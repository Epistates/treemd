//! Following links and anchors, loading linked files, and file history.

use super::*;

/// Link picker state (`f` to open).
#[derive(Debug, Default)]
pub struct LinkPickerState {
    pub filtered_indices: Vec<usize>,
    pub selected: Option<usize>,
    pub query: String,
    pub active: bool,
}

/// The link root for a document at `path`: the enclosing git work tree if
/// there is one, otherwise the document's own directory. See
/// [`App::confine_link_path`].
///
/// A piped document is not from any directory and may be from anywhere,
/// so unless `trust_piped` its links stay in the working directory itself.
pub(super) fn link_root_for(path: &std::path::Path, trust_piped: bool) -> PathBuf {
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if path == std::path::Path::new(crate::input::STDIN_PATH) && !trust_piped {
        return dir;
    }
    dir.ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(std::path::Path::to_path_buf)
        .unwrap_or(dir)
}

/// Whether a link from a document may be handed to the system opener: web
/// links only. Every place that opens a URL checks this one.
pub(super) fn is_openable_url(url: &str) -> bool {
    let lower = url.get(..8).unwrap_or(url).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Saved state for file navigation history
#[derive(Debug, Clone)]
pub struct FileState {
    pub path: PathBuf,
    pub document: Document,
    pub filename: String,
    pub selected_heading: Option<String>,
    pub content_scroll: usize,
    pub outline_state_selected: Option<usize>,
}

impl App {
    /// Enter link follow mode - extract links from current section and highlight them
    pub fn enter_link_follow_mode(&mut self) {
        // Extract content for current section
        let content = self.current_section_content();

        // Extract all links from the content
        self.links_in_view = extract_links(&content);

        // Initialize filtered indices to show all links
        self.link_picker.filtered_indices = (0..self.links_in_view.len()).collect();
        self.link_picker.query.clear();
        self.link_picker.active = false;

        // Always enter mode, even if no links (so user sees "no links" message)
        self.mode = AppMode::LinkFollow;

        // Select first link if any exist
        if !self.link_picker.filtered_indices.is_empty() {
            self.link_picker.selected = Some(0);
        } else {
            self.link_picker.selected = None;
        }
    }

    /// Exit link follow mode and return to normal mode
    pub fn exit_link_follow_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.links_in_view.clear();
        self.link_picker.filtered_indices.clear();
        self.link_picker.selected = None;
        self.link_picker.query.clear();
        self.link_picker.active = false;
        // Don't clear status message here - let it display for a moment
    }

    /// Start link search mode
    pub fn start_link_search(&mut self) {
        if self.mode == AppMode::LinkFollow {
            self.link_picker.active = true;
        }
    }

    /// Stop link search mode (but keep the filter)
    pub fn stop_link_search(&mut self) {
        self.link_picker.active = false;
    }

    /// Clear link search and show all links
    pub fn clear_link_search(&mut self) {
        self.link_picker.query.clear();
        self.link_picker.active = false;
        self.update_link_filter();
    }

    /// Add a character to the link search query
    pub fn link_search_push(&mut self, c: char) {
        self.link_picker.query.push(c);
        self.update_link_filter();
    }

    /// Remove the last character from the link search query
    pub fn link_search_pop(&mut self) {
        self.link_picker.query.pop();
        self.update_link_filter();
    }

    /// Update the filtered link indices based on the search query
    pub(super) fn update_link_filter(&mut self) {
        let query = self.link_picker.query.to_lowercase();

        if query.is_empty() {
            // Show all links when no search query
            self.link_picker.filtered_indices = (0..self.links_in_view.len()).collect();
        } else {
            // Filter links by text or URL containing the query
            self.link_picker.filtered_indices = self
                .links_in_view
                .iter()
                .enumerate()
                .filter(|(_, link)| {
                    link.text.to_lowercase().contains(&query)
                        || link.target.as_str().to_lowercase().contains(&query)
                })
                .map(|(idx, _)| idx)
                .collect();
        }

        // Update selection to stay within filtered results
        if self.link_picker.filtered_indices.is_empty() {
            self.link_picker.selected = None;
        } else if let Some(idx) = self.link_picker.selected {
            if idx >= self.link_picker.filtered_indices.len() {
                self.link_picker.selected = Some(0);
            }
        } else {
            self.link_picker.selected = Some(0);
        }
    }

    /// Cycle to the next link (Tab in link follow mode)
    pub fn next_link(&mut self) {
        if self.mode == AppMode::LinkFollow && !self.link_picker.filtered_indices.is_empty() {
            self.link_picker.selected = Some(match self.link_picker.selected {
                Some(idx) => {
                    if idx >= self.link_picker.filtered_indices.len() - 1 {
                        0 // Wrap to first
                    } else {
                        idx + 1
                    }
                }
                None => 0,
            });
        }
    }

    /// Cycle to the previous link (Shift+Tab in link follow mode)
    pub fn previous_link(&mut self) {
        if self.mode == AppMode::LinkFollow && !self.link_picker.filtered_indices.is_empty() {
            self.link_picker.selected = Some(match self.link_picker.selected {
                Some(idx) => {
                    if idx == 0 {
                        self.link_picker.filtered_indices.len() - 1 // Wrap to last
                    } else {
                        idx - 1
                    }
                }
                None => 0,
            });
        }
    }

    /// Jump to parent heading while staying in link follow mode
    pub fn jump_to_parent_links(&mut self) {
        if self.mode == AppMode::LinkFollow {
            // First, jump to parent in outline
            if let Some(current_idx) = self.outline_state.selected()
                && current_idx < self.outline_items.len()
            {
                let current_level = self.outline_items[current_idx].level;

                // Search backwards for a heading with lower level (parent)
                for i in (0..current_idx).rev() {
                    if self.outline_items[i].level < current_level {
                        // Jump to parent in outline
                        self.select_outline_index(i);

                        // Now extract links from parent's content
                        let content = self.current_section_content();
                        self.links_in_view = extract_links(&content);

                        // Reset link selection
                        if !self.links_in_view.is_empty() {
                            self.link_picker.selected = Some(0);
                            self.status_message = Some(format!(
                                "✓ Jumped to parent ({} links found)",
                                self.links_in_view.len()
                            ));
                        } else {
                            self.link_picker.selected = None;
                            self.status_message = Some("⚠ Parent has no links".to_string());
                        }

                        return;
                    }
                }

                // If no parent found (already at top-level)
                self.status_message = Some("⚠ Already at top-level heading".to_string());
            }
        }
    }

    /// Get the currently selected link (from filtered list)
    pub fn get_selected_link(&self) -> Option<&Link> {
        self.link_picker
            .selected
            .and_then(|idx| self.link_picker.filtered_indices.get(idx))
            .and_then(|&real_idx| self.links_in_view.get(real_idx))
    }

    /// Handle loading a relative file link, resolving markdown extensions and fallbacks.
    ///
    /// Returns `true` if the caller should exit its current mode (link-follow or interactive).
    pub(super) fn resolve_relative_file_link(
        &mut self,
        path: &std::path::Path,
        anchor: &Option<String>,
    ) -> Result<bool, String> {
        let has_md_extension = Self::is_markdown_extension(path);

        if has_md_extension {
            self.load_file(path, anchor.as_deref())?;
            // Only signal exit if we're not prompting for file creation
            Ok(self.mode != AppMode::ConfirmFileCreate)
        } else {
            // No markdown extension — try .md, then as-is, then prompt to create
            let md_path = PathBuf::from(format!("{}.md", path.display()));

            if self
                .confine_link_path(&md_path)
                .is_ok_and(|absolute_md_path| absolute_md_path.exists())
            {
                self.load_file(&md_path, anchor.as_deref())?;
                Ok(true)
            } else {
                let absolute_path = self.confine_link_path(path)?;

                if absolute_path.exists() {
                    // Non-markdown file — open in editor
                    self.pending_editor_file = Some(absolute_path);
                    Ok(true)
                } else {
                    // File doesn't exist — prompt to create markdown file
                    let relative_path = if path.extension().is_none() {
                        PathBuf::from(format!("{}.md", path.display()))
                    } else {
                        path.to_path_buf()
                    };
                    self.load_file(&relative_path, anchor.as_deref())?;
                    Ok(self.mode != AppMode::ConfirmFileCreate)
                }
            }
        }
    }

    /// Follow the currently selected link
    pub fn follow_selected_link(&mut self) -> Result<(), String> {
        let link = match self.get_selected_link() {
            Some(link) => link.clone(),
            None => return Err("No link selected".to_string()),
        };

        match link.target {
            crate::parser::LinkTarget::Anchor(anchor) => {
                // Jump to heading in current document
                self.jump_to_anchor(&anchor)?;
                self.status_message = Some(format!("✓ Jumped to #{}", anchor));
                self.exit_link_follow_mode();
                Ok(())
            }
            crate::parser::LinkTarget::RelativeFile { path, anchor } => {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                if self.resolve_relative_file_link(&path, &anchor)? {
                    self.status_message = Some(format!("✓ Opened {}", filename));
                    self.exit_link_follow_mode();
                }
                Ok(())
            }
            crate::parser::LinkTarget::WikiLink { target, .. } => {
                // Try to find and load the wikilinked file
                self.load_wikilink(&target)?;
                // Only exit link follow mode if we're not prompting for file creation
                if self.mode != AppMode::ConfirmFileCreate {
                    self.status_message = Some(format!("✓ Opened [[{}]]", target));
                    self.exit_link_follow_mode();
                }
                Ok(())
            }
            crate::parser::LinkTarget::External(url) => {
                // Try to open in default browser, if it is a web link. Any
                // other scheme is only copied: `mailto:` can carry an
                // `attach=` that some mail clients honor.
                let open_result = if is_openable_url(&url) {
                    open::that(&url)
                } else {
                    Err(std::io::Error::other("not a web link"))
                };

                // Also copy to clipboard as backup
                let mut clipboard_success = false;
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    clipboard_success = clipboard.set_text(url.clone()).is_ok();
                }

                // Set status message
                self.status_message = match (open_result, clipboard_success) {
                    (Ok(_), true) => Some(format!(
                        "✓ Opened {} in browser (also copied to clipboard)",
                        url
                    )),
                    (Ok(_), false) => Some(format!("✓ Opened {} in browser", url)),
                    (Err(_), true) => Some(format!(
                        "⚠ Could not open browser, URL copied to clipboard: {}",
                        url
                    )),
                    (Err(_), false) => Some(format!("✗ Failed to open URL: {}", url)),
                };

                self.exit_link_follow_mode();
                Ok(())
            }
        }
    }

    /// Select the heading an in-document link points at, by anchor
    /// (`#mixed-links-test`) or heading text (`[[#Mixed Links Test]]`).
    ///
    /// Every heading is searched, not just the outline's visible rows, so a
    /// heading under a collapsed one is found (its ancestors are expanded).
    /// As on GitHub, `#name-1` is the second heading whose anchor is `name`.
    pub(super) fn jump_to_anchor(&mut self, anchor: &str) -> Result<(), String> {
        use crate::parser::SlugStyle;

        let anchor_lower = anchor.to_lowercase();
        let anchors = self.document.heading_slugs(SlugStyle::Treemd);

        // Strategy 1: Normalized anchor match. The anchor from markdown
        // links is already normalized (lowercase, dashes). A link may be
        // written for treemd or copied from GitHub, which keeps `_`
        // (`#snake_case`), so either style finds the heading, whatever the
        // config's style for making anchors is.
        // Strategy 2: Direct heading text match (case-insensitive).
        // Wikilinks like [[#Mixed Links Test]] keep the heading's text.
        // Strategy 3: `name-N`, the Nth repeat of anchor `name`.
        let found = anchors
            .iter()
            .position(|a| *a == anchor_lower)
            .or_else(|| {
                self.document
                    .heading_slugs(SlugStyle::Github)
                    .iter()
                    .position(|a| *a == anchor_lower)
            })
            .or_else(|| {
                self.document
                    .headings
                    .iter()
                    .position(|heading| heading.text.eq_ignore_ascii_case(anchor))
            });
        let found = found.or_else(|| {
            let (base, n) = anchor_lower.rsplit_once('-')?;
            let n: usize = n.parse().ok()?;
            anchors
                .iter()
                .enumerate()
                .filter(|(_, a)| *a == base)
                .nth(n)
                .map(|(index, _)| index)
        });

        let index = found.ok_or_else(|| format!("Heading '{}' not found", anchor))?;
        self.reveal_heading(index);
        self.select_by_heading_index(index);
        Ok(())
    }

    /// Load a file by relative path (checks for unsaved changes first)
    ///
    /// Security: Validates path to prevent directory traversal attacks.
    /// Files must be within the current file's directory or its subdirectories.
    pub(super) fn load_file(
        &mut self,
        relative_path: &std::path::Path,
        anchor: Option<&str>,
    ) -> Result<(), String> {
        // Check for unsaved changes before navigating to a different file
        if self.has_unsaved_changes() {
            self.pending_navigation = Some(PendingNavigation::LoadFile(
                relative_path.to_path_buf(),
                anchor.map(|s| s.to_string()),
            ));
            self.mode = AppMode::ConfirmSaveBeforeNav;
            return Ok(()); // Not an error - we're asking user to confirm
        }

        self.load_file_internal(relative_path, anchor)
    }

    /// Where a link to `relative` lands, if that stays under the link root.
    /// Every link to a file goes through this, whether it opens the file,
    /// hands it to the editor, or creates it.
    ///
    /// The link root is the git work tree the document belongs to, or the
    /// directory treemd was opened on when there is none, so `../README.md`
    /// from `docs/guide.md` works the way it does on GitHub. A leading `/`
    /// is relative to that root, as GitHub reads it too. A drive or UNC
    /// prefix is refused.
    ///
    /// The link is resolved one component at a time against the real
    /// filesystem: a component that exists is canonicalized, so a symlink is
    /// judged by where it points, and `..` steps up from that real location,
    /// as the OS resolves it. A component that does not exist yet is kept as
    /// written, for a file about to be created. The result has to sit under
    /// the root. Judging only the full path let a target that did not exist
    /// yet through, so with `docs` a symlink out of the repository, creating
    /// `docs/new.md` wrote wherever `docs` pointed.
    pub(super) fn confine_link_path(&self, relative: &std::path::Path) -> Result<PathBuf, String> {
        use std::path::{Component, Path};

        let outside = || "Path escapes the document's directory tree".to_string();

        // A fetched document's relative links point into its site, not this
        // machine; resolved here they opened, or offered to create, files
        // in the working directory on the word of a page from the network.
        if self.is_remote_document() {
            return Err("Links to files in a fetched document are not followed".to_string());
        }

        // A stdin or fetched document has no directory of its own; its links
        // resolve against the working directory.
        let base = self
            .current_file_path
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut resolved = if relative.has_root() {
            self.link_root.clone()
        } else {
            base.canonicalize()
                .map_err(|e| format!("Cannot resolve the document's directory: {e}"))?
        };

        for component in relative.components() {
            match component {
                Component::RootDir | Component::CurDir => {}
                Component::Prefix(_) => {
                    return Err("Absolute paths are not allowed for security reasons".to_string());
                }
                Component::ParentDir => {
                    if !resolved.pop() {
                        return Err(outside());
                    }
                }
                Component::Normal(name) => {
                    resolved.push(name);
                    // `symlink_metadata` rather than `exists`, so a dangling
                    // symlink counts as present and then fails to resolve,
                    // instead of being written through later.
                    if resolved.symlink_metadata().is_ok() {
                        resolved = resolved
                            .canonicalize()
                            .map_err(|_| "Link target cannot be resolved".to_string())?;
                    }
                }
            }
        }

        if !resolved.starts_with(&self.link_root) {
            return Err(outside());
        }
        Ok(resolved)
    }

    /// Internal file loading - skips unsaved changes check
    ///
    /// Security: the path goes through [`Self::confine_link_path`], so the
    /// file must be within the current file's directory or below it.
    pub(super) fn load_file_internal(
        &mut self,
        relative_path: &std::path::Path,
        anchor: Option<&str>,
    ) -> Result<(), String> {
        let absolute_path = self.confine_link_path(relative_path)?;

        // Check if file exists - if not, prompt to create it
        if !absolute_path.exists() {
            self.pending_file_create = Some(absolute_path.clone());
            self.pending_file_create_message = Some(format!(
                "File '{}' does not exist. Create it?",
                relative_path.display()
            ));
            self.mode = AppMode::ConfirmFileCreate;
            return Ok(()); // Not an error - we're asking user to confirm
        }

        // Parse the new file
        let new_document = crate::parser::parse_file(&absolute_path)
            .map_err(|e| format!("Failed to load file: {}", e))?;

        let new_filename = absolute_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Save current state to history
        self.save_to_history();

        // Load new document
        self.load_document(new_document, new_filename, absolute_path);

        // Jump to anchor if specified
        if let Some(anchor_name) = anchor {
            let _ = self.jump_to_anchor(anchor_name);
        }

        Ok(())
    }

    /// Find and load a wikilinked file
    ///
    /// Supports formats:
    /// - `[[filename]]` - load file (tries .md, .markdown extensions)
    /// - `[[filename#anchor]]` - load file and jump to anchor
    /// - `[[#anchor]]` - jump to anchor in current document
    /// - `[[path/to/file]]` - load file with path (e.g., `[[diary/notes.md]]`)
    ///
    /// Security: Path traversal (..) and absolute paths are blocked.
    /// The `load_file()` function provides additional security validation.
    pub(super) fn load_wikilink(&mut self, target: &str) -> Result<(), String> {
        // Handle anchor-only wikilinks (e.g., [[#section]])
        if let Some(anchor) = target.strip_prefix('#') {
            // Jump to heading in current document
            self.jump_to_anchor(anchor)?;
            self.status_message = Some(format!("✓ Jumped to #{}", anchor));
            return Ok(());
        }

        // Split target into file and optional anchor (e.g., "file#section" -> ("file", Some("section")))
        let (file_target, anchor) = if let Some((file, anchor)) = target.split_once('#') {
            (file, Some(anchor))
        } else {
            (target, None)
        };

        // Security: Reject path traversal attempts
        if file_target.contains("..") {
            return Err("WikiLinks cannot contain path traversal (..)".to_string());
        }

        // Security: Reject absolute paths
        if file_target.starts_with('/') {
            return Err("WikiLinks cannot be absolute paths".to_string());
        }

        // Security: Reject Windows absolute paths (drive letters)
        #[cfg(windows)]
        if file_target.len() >= 2 && file_target.chars().nth(1) == Some(':') {
            return Err("WikiLinks cannot be absolute paths".to_string());
        }

        // Normalize backslashes to forward slashes for cross-platform compatibility
        let file_target = file_target.replace('\\', "/");

        // Check if target already has a markdown extension
        let file_target_lower = file_target.to_lowercase();
        let has_md_extension = file_target_lower.ends_with(".md")
            || file_target_lower.ends_with(".markdown")
            || file_target_lower.ends_with(".mdown");

        // Try various extensions (only add extensions if target doesn't already have one)
        let candidates: Vec<String> = if has_md_extension {
            // Already has markdown extension - just try as-is
            vec![file_target.to_string()]
        } else {
            // Try with various extensions
            vec![
                format!("{}.md", file_target),
                format!("{}.markdown", file_target),
                file_target.to_string(),
            ]
        };

        for candidate in &candidates {
            // Symlinks and anything resolving outside the directory are skipped
            let Ok(path) = self.confine_link_path(std::path::Path::new(candidate)) else {
                continue;
            };
            if path.exists() {
                return self.load_file(&PathBuf::from(candidate), anchor);
            }
        }

        // File not found - prompt to create it (default to .md extension if not already present)
        let default_filename = if has_md_extension {
            file_target.to_string()
        } else {
            format!("{}.md", file_target)
        };
        let new_path = self.confine_link_path(std::path::Path::new(&default_filename))?;
        self.pending_file_create = Some(new_path);
        self.pending_file_create_message = Some(format!(
            "Wikilink '[[{}]]' not found. Create '{}'?",
            target, default_filename
        ));
        self.mode = AppMode::ConfirmFileCreate;
        Ok(()) // Not an error - we're asking user to confirm
    }

    /// Save current state to history before navigating away
    pub(super) fn save_to_history(&mut self) {
        let state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_history.push(state);

        // Clear forward history when navigating to a new file
        self.file_future.clear();
    }

    /// Load a new document and update all related state
    pub(super) fn load_document(&mut self, document: Document, filename: String, path: PathBuf) {
        // Signal file watcher if path changed
        if self.current_file_path != path {
            self.file_path_changed = true;
        }

        self.disk_hash = content_hash(document.content.as_bytes());
        self.document = document;
        self.filename = filename;
        self.current_file_path = path;

        // Rebuild tree and outline (with overview entry if applicable)
        self.tree = self.document.build_tree();
        self.rebuild_outline_items();

        // Reset selection to first item
        let mut outline_state = ListState::default();
        if !self.outline_items.is_empty() {
            outline_state.select(Some(0));
        }
        self.outline_state = outline_state;
        self.outline_scroll_state = ScrollbarState::new(self.outline_items.len());

        // Reset content scroll
        self.content_scroll = 0;
        let content_lines = self.document.content.lines().count();
        self.content_height = content_lines;
        self.content_scroll_state = ScrollbarState::new(content_lines);

        // Clear previous selection tracking
        self.previous_selection = None;
        // Document changed — force a metrics recompute on the next render.
        self.metrics_dirty = true;

        // Index interactive elements (links, images, etc.) even in normal mode
        // This allows inline images to render without entering interactive mode
        let content = self.document.content.clone();
        use crate::parser::content::parse_content;
        let blocks = parse_content(&content, 0);
        self.index_interactive_elements(&blocks);
        self.populate_image_cache();

        // Detect LaTeX content for status hint
        self.latex_detected = crate::tui::ui::util::contains_latex(&content);

        if self.latex_filter_active() && !self.latex_hint_shown {
            self.latex_hint_shown = true;
            self.set_status_message("LaTeX detected · filtered via hide_latex in config");
        }
    }

    /// Navigate back in file history
    pub fn go_back(&mut self) -> Result<(), String> {
        let previous_state = self
            .file_history
            .pop()
            .ok_or("No previous file in history")?;

        // Save current state to future stack
        let current_state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_future.push(current_state);

        // Restore previous state
        self.restore_file_state(previous_state);

        Ok(())
    }

    /// Navigate forward in file history
    pub fn go_forward(&mut self) -> Result<(), String> {
        let next_state = self.file_future.pop().ok_or("No next file in history")?;

        // Save current state to history stack
        let current_state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_history.push(current_state);

        // Restore next state
        self.restore_file_state(next_state);

        Ok(())
    }

    /// Restore a file state from history
    pub(super) fn restore_file_state(&mut self, state: FileState) {
        self.load_document(state.document, state.filename, state.path);

        // Restore selection and scroll position
        if let Some(selected_idx) = state.outline_state_selected
            && selected_idx < self.outline_items.len()
        {
            self.select_outline_index(selected_idx);
        }

        // The section is laid out for the new selection first: done on the
        // next frame instead, that reset the scroll restored here to the top.
        self.update_content_metrics();
        self.content_scroll = state.content_scroll.min(self.max_content_scroll());
        self.content_scroll_state = self.content_scroll_state.position(state.content_scroll);
    }

    /// Reload current file from disk (used after external editing).
    ///
    /// Returns `Ok(false)` when the on-disk content is identical to the
    /// in-memory document (e.g. the watcher echoing back our own save),
    /// in which case nothing is reloaded.
    pub fn reload_current_file(&mut self) -> Result<bool, String> {
        // Save current state to restore after reload
        let current_selection = self.selected_heading_text().map(|s| s.to_string());
        let current_index = self.selected_heading_index();
        let current_scroll = self.content_scroll;

        // Reload the file
        let content = crate::input::read_file(&self.current_file_path)
            .map_err(|e| format!("Failed to reload file: {}", e))?;

        if content == self.document.content {
            return Ok(false);
        }

        let document = crate::parser::parse_markdown(&content);
        let filename = self
            .current_file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        self.load_document(document, filename, self.current_file_path.clone());

        // Restore the same heading: by its position while the heading there
        // still reads the same, which tells apart sections that share a name
        // (several `## Tasks`), and by text otherwise. By text alone, a
        // checkbox toggle in the second `## Tasks` moved the selection to the
        // first, and the next toggle went there.
        let same_place = current_index.filter(|&i| {
            self.document
                .headings
                .get(i)
                .is_some_and(|h| Some(h.text.as_str()) == current_selection.as_deref())
        });
        if let Some(index) = same_place {
            self.select_by_heading_index(index);
        } else if let Some(heading) = current_selection {
            self.select_by_text(&heading);
        }

        // Restore scroll position (may be adjusted if content changed)
        if current_scroll < self.content_height {
            self.content_scroll = current_scroll;
            self.content_scroll_state = self.content_scroll_state.position(current_scroll);
        }

        Ok(true)
    }

    /// Confirm file creation and open the new file
    pub fn confirm_file_create(&mut self) -> Result<(), String> {
        if let Some(path) = self.pending_file_create.take() {
            // Create parent directories if needed
            if let Some(parent) = path.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }

            // Create the file with default content
            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled");
            let default_content = format!("# {}\n\n", filename);

            std::fs::write(&path, &default_content)
                .map_err(|e| format!("Failed to create file: {}", e))?;

            self.pending_file_create_message = None;
            self.mode = AppMode::Normal;

            // Load the newly created file via its path relative to the
            // current document's directory — using only the file name here
            // would resolve to the wrong directory for wikilinks like
            // [[diary/notes]]
            let current_dir = self
                .current_file_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_default();
            let relative_path = path
                .strip_prefix(&current_dir)
                .map(PathBuf::from)
                .unwrap_or_else(|_| path.clone());

            self.load_file(&relative_path, None)?;
            self.status_message = Some(format!("✓ Created and opened {}", relative_path.display()));
            self.exit_link_follow_mode();
        }
        Ok(())
    }

    /// Cancel file creation and return to previous mode
    pub fn cancel_file_create(&mut self) {
        self.pending_file_create = None;
        self.pending_file_create_message = None;
        self.mode = AppMode::Normal;
        self.status_message = Some("File creation cancelled".to_string());
    }

    /// Whether the document came from a URL or `github:` rather than this
    /// machine; its placeholder path is the address it was fetched from.
    pub(super) fn is_remote_document(&self) -> bool {
        crate::input::is_remote_spec(&self.current_file_path.to_string_lossy())
    }

    /// Whether the document was piped in and the config does not trust
    /// piped documents (`security.trust_piped_documents`).
    pub(super) fn is_untrusted_piped_document(&self) -> bool {
        self.current_file_path == std::path::Path::new(crate::input::STDIN_PATH)
            && !self.config.security.trust_piped_documents
    }
}

#[cfg(test)]
mod link_boundary_tests {
    use super::*;
    use std::path::Path;

    fn app_in(dir: &Path) -> App {
        let path = dir.join("doc.md");
        std::fs::write(&path, "# doc\n").unwrap();
        App::new(
            crate::parser::parse_markdown("# doc\n"),
            "doc.md".into(),
            path,
            Config::default(),
            ColorMode::Rgb,
            false,
        )
    }

    #[test]
    fn a_link_inside_the_directory_resolves() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("notes")).unwrap();
        std::fs::write(root.join("notes/data.csv"), "a,b").unwrap();
        let app = app_in(&root);

        assert_eq!(
            app.confine_link_path(Path::new("notes/data.csv")).unwrap(),
            root.join("notes/data.csv")
        );
        // Not there yet, so it would be created, but inside.
        assert!(
            app.confine_link_path(Path::new("notes/new/page.md"))
                .is_ok()
        );
    }

    #[test]
    fn links_cannot_leave_the_link_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let app = app_in(&root);
        // No git tree here, so the root is the document's directory.
        assert!(app.confine_link_path(Path::new("../secret.txt")).is_err());
        assert!(app.confine_link_path(Path::new("a/../../b")).is_err());
        // A leading `/` is relative to the root, never the filesystem's.
        assert_eq!(
            app.confine_link_path(Path::new("/etc/hosts")).unwrap(),
            root.join("etc/hosts")
        );
    }

    /// `../README.md` from a document in a subdirectory is the ordinary
    /// way a repository's docs link back up, and was refused outright.
    #[test]
    fn a_link_may_climb_within_its_git_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::create_dir(root.join("docs")).unwrap();
        std::fs::write(root.join("README.md"), "# r\n").unwrap();
        let app = app_in(&root.join("docs"));

        assert_eq!(
            app.confine_link_path(Path::new("../README.md")).unwrap(),
            root.join("README.md")
        );
        assert_eq!(
            app.confine_link_path(Path::new("/README.md")).unwrap(),
            root.join("README.md")
        );
        assert!(
            app.confine_link_path(Path::new("../../outside.md"))
                .is_err()
        );
    }

    /// The link used to be checked only once it existed, so a file that did
    /// not exist yet under a symlinked directory was created wherever the
    /// symlink pointed.
    #[cfg(unix)]
    #[test]
    fn a_symlinked_directory_cannot_carry_a_link_outside() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.join("docs")).unwrap();
        std::os::unix::fs::symlink(root.join("missing"), root.join("dangling")).unwrap();
        let app = app_in(&root);

        assert!(app.confine_link_path(Path::new("docs/pwn.sh")).is_err());
        assert!(
            app.confine_link_path(Path::new("docs/deeper/pwn.md"))
                .is_err()
        );
        assert!(app.confine_link_path(Path::new("dangling/x.md")).is_err());
    }

    /// A link to an existing non-markdown file went straight to the editor
    /// without any of the checks a markdown link gets.
    #[test]
    fn a_non_markdown_link_outside_is_not_opened_in_the_editor() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_in(&dir.path().canonicalize().unwrap());

        assert!(
            app.resolve_relative_file_link(&PathBuf::from("../secret.txt"), &None)
                .is_err()
        );
        // Root-relative, so never the real /etc/hosts.
        let _ = app.resolve_relative_file_link(&PathBuf::from("/etc/hosts"), &None);
        assert!(
            app.pending_editor_file
                .as_deref()
                .is_none_or(|p| p != Path::new("/etc/hosts"))
        );
        assert!(app.pending_editor_file.is_none());
    }

    /// A fetched document's links and images name places on its site. Read
    /// against this machine, a page could open local files or show local
    /// images by path.
    #[test]
    fn a_fetched_document_reaches_nothing_on_this_machine() {
        let app = App::new(
            crate::parser::parse_markdown("# doc\n"),
            "README.md".into(),
            PathBuf::from("https://example.com/README.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        assert!(app.confine_link_path(Path::new("notes.md")).is_err());
        assert!(app.resolve_image_path("/etc/passwd").is_err());
        assert!(app.resolve_image_path("diagram.png").is_err());
    }

    fn piped_app(trusted: bool) -> App {
        let mut config = Config::default();
        config.security.trust_piped_documents = trusted;
        App::new(
            crate::parser::parse_markdown("# doc\n"),
            "stdin".into(),
            PathBuf::from(crate::input::STDIN_PATH),
            config,
            ColorMode::Rgb,
            false,
        )
    }

    /// A piped document can come from anywhere (`curl ... | treemd`). Its
    /// images could be loaded by absolute path, so a page could show any
    /// image on disk. Its links and images now stay in the working
    /// directory unless the config trusts piped documents.
    #[test]
    fn a_piped_document_stays_in_the_working_directory() {
        let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        let app = piped_app(false);
        assert_eq!(app.link_root, cwd);
        assert!(app.resolve_image_path("/etc/passwd").is_err());
        assert!(app.resolve_image_path("../outside.png").is_err());
        assert_eq!(
            app.resolve_image_path("docs/diagram.png").unwrap(),
            cwd.join("docs/diagram.png")
        );

        let trusted = piped_app(true);
        assert_eq!(
            trusted.resolve_image_path("/etc/passwd").unwrap(),
            Path::new("/etc/passwd")
        );
    }

    #[test]
    fn only_web_links_are_handed_to_the_system_opener() {
        assert!(is_openable_url("https://example.com"));
        assert!(is_openable_url("HTTP://example.com"));
        assert!(!is_openable_url("mailto:a@b.c?attach=/etc/passwd"));
        assert!(!is_openable_url("file:///etc/passwd"));
        assert!(!is_openable_url("vscode://x"));
        assert!(!is_openable_url("javascript:alert(1)"));
    }
}
