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
    /// Debounce: ignore events within this duration of the last reload
    last_reload: Instant,
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
            last_reload: Instant::now(),
            debounce_duration: Duration::from_millis(100),
        })
    }

    /// Start watching a file. Stops watching any previously watched file.
    pub fn watch(&mut self, path: &std::path::Path) -> Result<(), notify::Error> {
        // Unwatch previous file if any
        if let Some(ref old_path) = self.current_path {
            let _ = self.watcher.unwatch(old_path);
        }

        // Watch the new file (non-recursive since it's a single file)
        self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        self.current_path = Some(path.to_path_buf());

        // Reset debounce timer
        self.last_reload = Instant::now();

        Ok(())
    }

    /// Stop watching the current file.
    #[allow(dead_code)]
    pub fn unwatch(&mut self) {
        if let Some(ref path) = self.current_path {
            let _ = self.watcher.unwatch(path);
        }
        self.current_path = None;
    }

    /// Check if the watched file has been modified.
    /// Returns true if a reload should be triggered.
    pub fn check_for_changes(&mut self) -> bool {
        // Drain all pending events
        let mut should_reload = false;

        loop {
            match self.receiver.try_recv() {
                Ok(Ok(event)) => {
                    // Check if this is a modification event we care about
                    if self.is_relevant_event(&event) {
                        should_reload = true;
                    }
                }
                Ok(Err(_)) => {
                    // Watch error, ignore
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Apply debouncing
        if should_reload {
            let now = Instant::now();
            if now.duration_since(self.last_reload) >= self.debounce_duration {
                self.last_reload = now;
                return true;
            }
        }

        false
    }

    /// Mark that a reload just happened (for debouncing after internal saves).
    #[allow(dead_code)]
    pub fn mark_reloaded(&mut self) {
        self.last_reload = Instant::now();
    }

    /// Check if an event is relevant for triggering a reload.
    fn is_relevant_event(&self, event: &Event) -> bool {
        let Some(ref watched_path) = self.current_path else {
            return false;
        };

        // Check if event path matches our watched file
        // Use multiple strategies to handle platform differences
        let matches_path =
            event.paths.iter().any(|event_path| {
                // Strategy 1: Exact path match
                if event_path == watched_path {
                    return true;
                }

                // Strategy 2: Canonicalized path match (handles symlinks, case differences)
                if let (Ok(event_canonical), Ok(watched_canonical)) =
                    (event_path.canonicalize(), watched_path.canonicalize())
                    && event_canonical == watched_canonical
                {
                    return true;
                }

                // Strategy 3: File name match (fallback for FSEvents quirks)
                // Only match if event is in same directory
                if let (
                    Some(event_name),
                    Some(watched_name),
                    Some(event_parent),
                    Some(watched_parent),
                ) = (
                    event_path.file_name(),
                    watched_path.file_name(),
                    event_path.parent(),
                    watched_path.parent(),
                ) && event_name == watched_name
                {
                    // Verify same directory (canonicalize to handle . and ..)
                    if let (Ok(ep), Ok(wp)) =
                        (event_parent.canonicalize(), watched_parent.canonicalize())
                    {
                        return ep == wp;
                    }
                }

                false
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
                // Atomic saves: write to temp then rename to target
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
}
