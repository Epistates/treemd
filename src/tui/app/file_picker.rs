//! The file picker.

use super::*;

/// File picker state — files and directories listing, search, selection.
#[derive(Debug, Default)]
pub struct FilePickerState {
    pub files: Vec<PathBuf>,
    pub dirs: Vec<PathBuf>,
    pub filtered_file_indices: Vec<usize>,
    pub filtered_dir_indices: Vec<usize>,
    /// Selected index into the combined dirs+files list.
    pub selected: Option<usize>,
    pub query: String,
    /// Whether the search input is active (cursor visible).
    pub active: bool,
}

impl App {
    /// Check if a path has a markdown file extension
    pub(super) fn is_markdown_extension(path: &std::path::Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext_lower = ext.to_lowercase();
                ext_lower == "md" || ext_lower == "markdown" || ext_lower == "mdown"
            })
            .unwrap_or(false)
    }

    /// Get the effective file picker directory (custom or cwd)
    pub fn effective_picker_dir(&self) -> PathBuf {
        self.file_picker_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    /// Navigate file picker to the given directory, clearing search and resetting selection
    pub(super) fn navigate_picker_to_dir(&mut self, dir: PathBuf) {
        self.file_picker_dir = Some(dir);
        self.file_picker.query.clear();
        self.scan_markdown_files(); // update_file_filter() resets selected_file_idx
    }

    /// Scan current directory for .md files and subdirectories (non-recursive, alphabetically sorted)
    pub fn scan_markdown_files(&mut self) {
        use std::fs;

        let dir = self.effective_picker_dir();

        let mut files = Vec::new();
        let mut dirs = Vec::new();

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let ft = entry.file_type().ok();
                let path = entry.path();
                let is_visible = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| self.show_hidden || !name.starts_with('.'))
                    .unwrap_or(false);
                if !is_visible {
                    continue;
                }
                if ft.map(|t| t.is_file()).unwrap_or(false) && Self::is_markdown_extension(&path) {
                    files.push(path);
                } else if ft.map(|t| t.is_dir()).unwrap_or(false) {
                    dirs.push(path);
                }
            }
        }

        files.sort();
        dirs.sort();
        self.file_picker.files = files;
        self.file_picker.dirs = dirs;

        self.update_file_filter();
    }

    /// Update filtered file and directory lists based on search query
    pub fn update_file_filter(&mut self) {
        fn filter_by_name(paths: &[PathBuf], query: &str) -> Vec<usize> {
            paths
                .iter()
                .enumerate()
                .filter(|(_, path)| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|name| name.to_lowercase().contains(query))
                        .unwrap_or(false)
                })
                .map(|(idx, _)| idx)
                .collect()
        }

        if self.file_picker.query.is_empty() {
            self.file_picker.filtered_file_indices = (0..self.file_picker.files.len()).collect();
            self.file_picker.filtered_dir_indices = (0..self.file_picker.dirs.len()).collect();
        } else {
            let query_lower = self.file_picker.query.to_lowercase();
            self.file_picker.filtered_file_indices =
                filter_by_name(&self.file_picker.files, &query_lower);
            self.file_picker.filtered_dir_indices =
                filter_by_name(&self.file_picker.dirs, &query_lower);
        }

        // Combined count: files + directories
        let combined_count = self.file_picker.filtered_file_indices.len()
            + self.file_picker.filtered_dir_indices.len();

        // Reset selection if current is out of bounds
        if let Some(sel) = self.file_picker.selected {
            if sel >= combined_count {
                self.file_picker.selected = if combined_count == 0 { None } else { Some(0) };
            }
        } else if combined_count > 0 {
            self.file_picker.selected = Some(0);
        }
    }

    /// Get the total number of items in the file picker (files + dirs)
    pub fn file_picker_item_count(&self) -> usize {
        self.file_picker.filtered_file_indices.len() + self.file_picker.filtered_dir_indices.len()
    }

    /// Navigate to parent directory in file picker
    pub fn file_picker_parent_dir(&mut self) {
        let current_dir = self.effective_picker_dir();
        if let Some(parent) = current_dir.parent() {
            self.navigate_picker_to_dir(parent.to_path_buf());
        }
    }

    /// Enter file picker mode
    pub fn enter_file_picker(&mut self) {
        self.scan_markdown_files();

        // Highlight current file if present
        if let Some(current_idx) = self
            .file_picker
            .files
            .iter()
            .position(|p| p == &self.current_file_path)
        {
            self.file_picker.selected = Some(current_idx);
        } else if !self.file_picker.filtered_file_indices.is_empty() {
            self.file_picker.selected = Some(0);
        }

        self.mode = AppMode::FilePicker;
    }

    /// Select file from picker and load it (or navigate into directory)
    pub fn select_file_from_picker(&mut self) -> Result<(), String> {
        let selected_display_idx = self.file_picker.selected.ok_or("No file selected")?;
        let file_count = self.file_picker.filtered_file_indices.len();

        // Check if selection is in the directory range
        if selected_display_idx >= file_count {
            let dir_display_idx = selected_display_idx - file_count;
            let real_dir_idx = self
                .file_picker
                .filtered_dir_indices
                .get(dir_display_idx)
                .ok_or("Invalid directory selection")?;
            let dir_path = self.file_picker.dirs[*real_dir_idx].clone();
            self.navigate_picker_to_dir(dir_path);
            return Ok(());
        }

        let real_idx = self
            .file_picker
            .filtered_file_indices
            .get(selected_display_idx)
            .ok_or("Invalid selection")?;
        let file_path = self.file_picker.files[*real_idx].clone();

        // Don't reload if it's already the current file
        if file_path == self.current_file_path {
            self.mode = AppMode::Normal;
            self.file_picker.query.clear();
            self.file_picker.active = false;
            return Ok(());
        }

        // Ask about unsaved edits first, as following a link does. Opening the
        // file straight away left the edits pending against the new file, so
        // quitting reported them there and undo applied them to it.
        if self.has_unsaved_changes() {
            self.file_picker.query.clear();
            self.file_picker.active = false;
            self.pending_navigation = Some(PendingNavigation::OpenPicked(file_path));
            self.mode = AppMode::ConfirmSaveBeforeNav;
            return Ok(());
        }

        self.open_picked_file(file_path)
    }

    /// Open a file chosen in the file picker, recording the current one in
    /// the history.
    pub(super) fn open_picked_file(&mut self, file_path: PathBuf) -> Result<(), String> {
        // Read it before touching the history, so a file that fails to open
        // leaves no entry to go back to.
        let content = crate::input::read_file(&file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        let document = crate::parser::parse_markdown(&content);

        // Save current state to history
        let current_state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_history.push(current_state);
        self.file_future.clear(); // Clear forward history when navigating to new file

        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.md")
            .to_string();

        self.load_document(document, filename, file_path);
        self.link_root = link_root_for(
            &self.current_file_path,
            self.config.security.trust_piped_documents,
        );

        // Exit picker mode
        self.mode = AppMode::Normal;
        self.file_picker.query.clear();
        self.file_picker.active = false;

        Ok(())
    }

    /// Cycle to the next item in file picker (files + dirs)
    pub fn next_file(&mut self) {
        let total = self.file_picker_item_count();
        if self.mode == AppMode::FilePicker && total > 0 {
            self.file_picker.selected = Some(match self.file_picker.selected {
                Some(idx) => {
                    if idx >= total - 1 {
                        0 // Wrap to first
                    } else {
                        idx + 1
                    }
                }
                None => 0,
            });
        }
    }

    /// Cycle to the previous item in file picker (files + dirs)
    pub fn previous_file(&mut self) {
        let total = self.file_picker_item_count();
        if self.mode == AppMode::FilePicker && total > 0 {
            self.file_picker.selected = Some(match self.file_picker.selected {
                Some(idx) => {
                    if idx == 0 {
                        total - 1 // Wrap to last
                    } else {
                        idx - 1
                    }
                }
                None => 0,
            });
        }
    }
}
