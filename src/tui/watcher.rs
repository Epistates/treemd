//! File system watcher for live reload functionality.
//!
//! Watches the currently open file for changes and notifies the TUI
//! to reload when modifications are detected.

use notify::{
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{AccessKind, AccessMode, ModifyKind, RenameMode},
};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// Manages file watching for live reload.
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    current_path: Option<PathBuf>,
    /// `current_path` and its directory resolved through symlinks, once, so
    /// events are not resolved against them one by one.
    canonical_path: Option<PathBuf>,
    canonical_dir: Option<PathBuf>,
    /// The directory path actually being watched (parent of current_path)
    watched_dir: Option<PathBuf>,
    /// Timestamp of the first relevant event in the current debounce window
    debounce_start: Option<Instant>,
    debounce_duration: Duration,
}

impl FileWatcher {
    /// Create a new file watcher.
    pub fn new() -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::channel();
        let watcher = notify::recommended_watcher(tx)?;

        Ok(Self {
            watcher,
            receiver: rx,
            current_path: None,
            canonical_path: None,
            canonical_dir: None,
            watched_dir: None,
            debounce_start: None,
            debounce_duration: Duration::from_millis(100),
        })
    }

    /// Start watching a file. Stops watching any previously watched file.
    /// Watches the parent directory to support atomic saves (e.g. Helix).
    pub fn watch(&mut self, path: &std::path::Path) -> Result<(), notify::Error> {
        // Unwatch previous directory if any
        if let Some(ref old_dir) = self.watched_dir {
            let _ = self.watcher.unwatch(old_dir);
        }

        // Watch the parent directory of the file for atomic save support
        let dir_path = path.parent().ok_or_else(|| {
            notify::Error::generic("Cannot watch a file with no parent directory")
        })?;
        self.watcher.watch(dir_path, RecursiveMode::NonRecursive)?;
        self.current_path = Some(path.to_path_buf());
        self.canonical_path = path.canonicalize().ok();
        self.canonical_dir = dir_path.canonicalize().ok();
        self.watched_dir = Some(dir_path.to_path_buf());
        self.debounce_start = None;

        Ok(())
    }

    /// Stop watching the current file.
    #[allow(dead_code)]
    pub fn unwatch(&mut self) {
        if let Some(ref dir) = self.watched_dir {
            let _ = self.watcher.unwatch(dir);
        }
        self.current_path = None;
        self.canonical_path = None;
        self.canonical_dir = None;
        self.watched_dir = None;
        self.debounce_start = None;
    }

    /// Check if the watched file has been modified.
    /// Returns true if a reload should be triggered.
    ///
    /// Uses non-blocking drain with time-based debouncing: the first relevant
    /// event starts a debounce window, and we only signal a reload once the
    /// window has elapsed on a subsequent poll. This avoids blocking the event loop.
    pub fn check_for_changes(&mut self) -> bool {
        // Drain all pending events (non-blocking)
        let mut saw_relevant = false;

        loop {
            match self.receiver.try_recv() {
                Ok(Ok(event)) => {
                    if self.is_relevant_event(&event) {
                        saw_relevant = true;
                    }
                }
                Ok(Err(_)) => {
                    // Watch error, ignore
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        if saw_relevant {
            // Start debounce window if not already running
            self.debounce_start.get_or_insert_with(Instant::now);
        }

        // Check if debounce window has elapsed
        if let Some(start) = self.debounce_start
            && start.elapsed() >= self.debounce_duration
        {
            self.debounce_start = None;
            return true;
        }

        false
    }

    /// Check if an event is relevant for triggering a reload.
    fn is_relevant_event(&self, event: &Event) -> bool {
        let Some(ref watched_path) = self.current_path else {
            return false;
        };

        // Check if event path matches our watched file. Every event in the
        // watched directory comes through here, so the name is compared
        // first: anything else in the directory is rejected without a
        // system call. Canonicalizing both sides of every event cost two.
        let matches_path = event.paths.iter().any(|event_path| {
            if event_path == watched_path || self.canonical_path.as_ref() == Some(event_path) {
                return true;
            }
            let same_name = match (event_path.file_name(), watched_path.file_name()) {
                (Some(event_name), Some(watched_name)) => {
                    event_name == watched_name || event_name.eq_ignore_ascii_case(watched_name)
                }
                _ => false,
            };
            if !same_name {
                return false;
            }
            // The same file through a symlink or a different case; or, when
            // it does not exist at this instant (an atomic save between its
            // rename steps), a file of that name in the same directory.
            match event_path.canonicalize() {
                Ok(canonical) => self.canonical_path.as_ref() == Some(&canonical),
                Err(_) => event_path
                    .parent()
                    .and_then(|parent| parent.canonicalize().ok())
                    .is_some_and(|parent| self.canonical_dir.as_ref() == Some(&parent)),
            }
        });

        if !matches_path {
            return false;
        }

        // Check event kind - be permissive to catch various save patterns
        matches!(
            event.kind,
            // Direct data modifications
            EventKind::Modify(ModifyKind::Data(_))
                | EventKind::Modify(ModifyKind::Any)
                // File closed after write
                | EventKind::Access(AccessKind::Close(AccessMode::Write))
                // File created (new file or recreated)
                | EventKind::Create(_)
                // Atomic saves: write to temp then rename (or variant)
                | EventKind::Modify(ModifyKind::Name(RenameMode::To))
                | EventKind::Modify(ModifyKind::Name(RenameMode::Any))
        )
    }

    /// Get the currently watched path.
    #[allow(dead_code)]
    pub fn current_path(&self) -> Option<&PathBuf> {
        self.current_path.as_ref()
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create file watcher")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }

    fn modified(path: &std::path::Path) -> Event {
        Event::new(EventKind::Modify(ModifyKind::Data(
            notify::event::DataChange::Content,
        )))
        .add_path(path.to_path_buf())
    }

    #[test]
    fn only_events_for_the_watched_file_count() {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("note.md");
        let other = dir.path().join("other.md");
        std::fs::write(&note, "a").unwrap();
        std::fs::write(&other, "b").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&note).unwrap();

        assert!(watcher.is_relevant_event(&modified(&note)));
        assert!(!watcher.is_relevant_event(&modified(&other)));
        // Reported through a symlinked directory, still the same file.
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(dir.path(), &link).unwrap();
            assert!(watcher.is_relevant_event(&modified(&link.join("note.md"))));
        }
        // Not a write.
        let read = Event::new(EventKind::Access(AccessKind::Read)).add_path(note.clone());
        assert!(!watcher.is_relevant_event(&read));
    }
}
