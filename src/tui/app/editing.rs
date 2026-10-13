//! Table cell edits, checkbox toggles, and saving them to the file.

use super::*;

/// A pending table cell edit that hasn't been saved to file yet
#[derive(Debug, Clone)]
pub struct PendingEdit {
    /// Source line (0-indexed) where the table's section starts; table
    /// counting for this edit begins at this line
    pub section_start_line: usize,
    /// Which table within the section (0-indexed)
    pub table_index: usize,
    /// Row within the table (0 = header, 1+ = data rows, excludes separator)
    pub row: usize,
    /// Column within the table (0-indexed)
    pub col: usize,
    /// The cell's source before editing, exactly as written between its
    /// pipes, which undo writes back verbatim
    pub original_value: String,
    /// The new value after editing
    pub new_value: String,
}

impl App {
    /// Whether there are table edits not yet written to the file.
    pub fn has_unsaved_changes(&self) -> bool {
        !self.pending_edits.is_empty()
    }

    /// Drop all buffered edits.
    /// Put back every buffered edit, newest first, so the document is what
    /// was loaded. Clearing the list alone left the edits in the text, where
    /// the next save (a checkbox toggle) wrote them anyway.
    pub fn discard_pending_edits(&mut self) {
        while !self.pending_edits.is_empty() {
            if self.undo_last_edit().is_err() {
                self.pending_edits.pop();
            }
        }
    }

    /// Toggle a checkbox and save changes to the file
    pub(super) fn toggle_checkbox_and_save(
        &mut self,
        block_idx: usize,
        item_idx: usize,
        checked: bool,
    ) -> Result<(), String> {
        use crate::parser::content::parse_content;
        use crate::parser::output::Block;

        let clean = |raw: &str| -> String {
            raw.trim_start()
                .trim_start_matches("[x]")
                .trim_start_matches("[X]")
                .trim_start_matches("[ ]")
                .trim()
                .to_string()
        };

        // The selected element's block and item index are in the section as
        // displayed, where hidden frontmatter or display math can remove
        // whole blocks. Hiding never adds or removes a list item, though, so
        // the item's place among the section's task items is the same in the
        // source, and the source's own text is what the file is matched by.
        let task_items = |blocks: &[Block]| -> Vec<(usize, usize, String, Option<bool>)> {
            blocks
                .iter()
                .enumerate()
                .filter_map(|(b_i, block)| match block {
                    Block::List { items, .. } => Some((b_i, items)),
                    _ => None,
                })
                .flat_map(|(b_i, items)| {
                    items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| item.checked.is_some())
                        .map(move |(i_i, item)| (b_i, i_i, clean(&item.content), item.checked))
                })
                .collect()
        };
        let displayed = parse_content(&self.displayed_section_content(), 0);
        let ordinal = task_items(&displayed)
            .iter()
            .position(|(b_i, i_i, _, _)| (*b_i, *i_i) == (block_idx, item_idx))
            .ok_or_else(|| "Could not find checkbox content".to_string())?;
        let source = task_items(&parse_content(&self.current_section_content(), 0));
        let (_, _, target_text, _) = source
            .get(ordinal)
            .cloned()
            .ok_or_else(|| "Could not find checkbox content".to_string())?;

        // Its occurrence among identical checkboxes (same text, same state)
        // before it in this section, so duplicates toggle the right line.
        let occurrence = source[..ordinal]
            .iter()
            .filter(|(_, _, text, state)| *state == Some(checked) && *text == target_text)
            .count();

        // Apply the toggle on the in-memory content (which includes any
        // buffered table edits) so toggling can never drop them.
        let had_pending = self.has_unsaved_changes();
        let line_range = self.current_section_line_range();
        let new_content = crate::tui::edits::toggle_checkbox(
            &self.document.content,
            line_range,
            &target_text,
            checked,
            occurrence,
            crate::parser::utils::strip_markdown_inline,
        )?;

        self.write_document(&new_content)?;

        // Any buffered table edits were persisted along with the toggle
        if had_pending {
            self.pending_edits.clear();
        }

        // Save scroll position and interactive element index before reload
        let saved_scroll = self.content_scroll;
        let saved_element_idx = self.interactive_state.current_index;

        // Reload the document
        self.reload_current_file()?;

        // Re-index interactive elements
        self.reindex_interactive_elements();

        // Restore scroll position (clamped to valid range)
        self.content_scroll = saved_scroll.min(self.max_content_scroll());
        self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);

        // Restore interactive element selection if still valid
        if let Some(idx) = saved_element_idx
            && idx < self.interactive_state.elements.len()
        {
            self.interactive_state.current_index = Some(idx);
        }

        // IMPORTANT: Sync previous_selection to prevent update_content_metrics() from resetting scroll
        // After reload, load_document() sets previous_selection = None, but current selection is restored.
        // Without this sync, update_content_metrics() thinks selection changed and resets scroll to 0.
        self.previous_selection = self.outline_state.selected().map(|i| {
            self.outline_items
                .get(i)
                .and_then(|item| item.heading_index)
        });

        // Suppress file watcher for this save - we already reloaded internally
        // Without this, file watcher detects our save and triggers a second reload
        self.suppress_file_watch = true;

        let new_state = if checked { "unchecked" } else { "checked" };
        self.status_message = Some(if had_pending {
            format!(
                "✓ Checkbox {} and saved (pending edits included)",
                new_state
            )
        } else {
            format!("✓ Checkbox {} and saved", new_state)
        });

        Ok(())
    }

    /// 0-indexed line number of a byte offset in the document content.
    pub(super) fn line_of_offset(&self, offset: usize) -> usize {
        let clamped = offset.min(self.document.content.len());
        self.document.content[..clamped].matches('\n').count()
    }

    /// 0-indexed `[start, end)` source line range of the currently selected
    /// section (heading line through the line before the next heading at the
    /// same or higher level), or the whole document when nothing is selected.
    pub(super) fn current_section_line_range(&self) -> (usize, usize) {
        // Exclusive upper bound in the same 0-indexed line space as
        // `line_of_offset` (newline count), so start/end/total never disagree.
        let total = self.line_of_offset(self.document.content.len()) + 1;
        let Some(idx) = self.selected_heading_index() else {
            return (0, total);
        };
        let Some(heading) = self.document.headings.get(idx) else {
            return (0, total);
        };
        let start = self.line_of_offset(heading.offset);
        let end = self.document.headings[idx + 1..]
            .iter()
            .find(|h| h.level <= heading.level)
            .map(|h| self.line_of_offset(h.offset))
            .unwrap_or(total);
        (start, end)
    }

    /// Enter cell edit mode for the currently selected table cell
    pub fn enter_cell_edit_mode(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data()
            && let Some((row, col)) = self.interactive_state.get_table_position()
        {
            // Get current cell value
            let cell_value = if row == 0 {
                // Header row
                headers.get(col).cloned().unwrap_or_default()
            } else {
                // Data row
                rows.get(row - 1)
                    .and_then(|r| r.get(col))
                    .cloned()
                    .unwrap_or_default()
            };

            // Edit the cell as written, so formatting the parser strips from
            // its text (`**bold**`) survives the edit, and undo can put back
            // exactly what was there.
            let table_index = self.calculate_current_table_index()?;
            let source = crate::tui::edits::table_cell_source(
                &self.document.content,
                self.current_section_line_range().0,
                table_index,
                row,
                col,
            )?;
            // The cell found in the file has to be the one on screen. If the
            // two disagree, refusing is better than editing another cell.
            let letters = |s: &str| -> String {
                s.chars()
                    .filter(|c| c.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect()
            };
            if letters(&source) != letters(&cell_value) {
                return Err(
                    "Could not find this cell in the file, so it cannot be edited safely"
                        .to_string(),
                );
            }

            let editable = source.trim().replace("\\|", "|");
            self.cell_edit_value = editable.clone();
            self.cell_edit_original_value = editable;
            self.cell_edit_original_source = source;
            self.cell_edit_row = row;
            self.cell_edit_col = col;
            self.mode = AppMode::CellEdit;
            return Ok(());
        }
        Err("No cell selected for editing".to_string())
    }

    /// Sanitize table cell content to prevent markdown injection
    pub(super) fn sanitize_table_cell(value: &str) -> String {
        value
            .replace('|', "\\|") // Escape pipe characters (table delimiters)
            .replace(['\n', '\r'], " ") // Replace newlines and carriage returns
    }

    /// Buffer the edited cell value in memory (does not write to file)
    /// Use save_pending_edits_to_file() to write changes to disk
    pub fn save_edited_cell(&mut self) -> Result<(), String> {
        // Skip if no actual change was made
        if self.cell_edit_value == self.cell_edit_original_value {
            self.status_message = Some("No changes made".to_string());
            return Ok(());
        }

        // Sanitize the cell value to prevent table structure corruption
        let sanitized_value = Self::sanitize_table_cell(&self.cell_edit_value);

        // Calculate the table index for this edit
        let table_index = self.calculate_current_table_index()?;
        let section_start_line = self.current_section_line_range().0;

        // Apply the edit to the in-memory document content first; only
        // record it as pending once it has actually been applied.
        let new_content = crate::tui::edits::replace_table_cell(
            &self.document.content,
            section_start_line,
            table_index,
            self.cell_edit_row,
            self.cell_edit_col,
            &sanitized_value,
        )?;
        self.apply_edited_content(new_content);

        // Store the edit in the pending buffer for undo capability
        let pending_edit = PendingEdit {
            section_start_line,
            table_index,
            row: self.cell_edit_row,
            col: self.cell_edit_col,
            original_value: self.cell_edit_original_source.clone(),
            new_value: sanitized_value,
        };
        self.pending_edits.push(pending_edit);

        let edit_count = self.pending_edits.len();
        self.status_message = Some(format!(
            "✓ Cell updated ({} unsaved change{})",
            edit_count,
            if edit_count == 1 { "" } else { "s" }
        ));
        Ok(())
    }

    /// Calculate the table index (0-based, within the current section) for
    /// the currently selected table element. Paired with the section start
    /// line, this unambiguously identifies the table for the file writer.
    pub(super) fn calculate_current_table_index(&self) -> Result<usize, String> {
        use crate::parser::content::parse_content;
        use crate::parser::output::Block;

        // Get the current section content to find the right table
        let section_content = self.compute_displayed_section();

        // Parse to find the table block
        let blocks = parse_content(&section_content, 0);

        // Find the block index of the current table element
        if let Some(element) = self.interactive_state.current_element() {
            let block_idx = element.id.block_idx;

            if let Some(Block::Table { .. }) = blocks.get(block_idx) {
                // Count tables before this one in the section
                let tables_before_in_section: usize = blocks[..block_idx]
                    .iter()
                    .filter(|b| matches!(b, Block::Table { .. }))
                    .count();

                return Ok(tables_before_in_section);
            }
        }

        Err("Could not locate table".to_string())
    }

    /// Write `content` over the current file, the one place a TUI edit saves.
    ///
    /// Refuses when the document is not a file on disk (stdin or a fetched
    /// URL, whose placeholder path used to have a file named `<stdin>`
    /// written into the working directory), and when the file has changed
    /// since treemd last read or wrote it: the in-memory copy predates that
    /// change, so writing it would quietly undo whatever was done elsewhere.
    pub(super) fn write_document(&mut self, content: &str) -> Result<(), String> {
        let path = self.current_file_path.clone();
        if !(path.is_absolute() && path.is_file()) {
            return Err("Not a file on disk, so there is nowhere to save".to_string());
        }
        let on_disk = std::fs::read(&path).map_err(|e| format!("Failed to read file: {e}"))?;
        // Its lone `\r` line endings were read as `\n`, and writing that
        // back would change every line ending in the file.
        if crate::parser::has_lone_cr(&on_disk) {
            return Err(format!(
                "{} ends its lines with a bare CR (classic Mac); \
                 treemd shows it but does not save edits to it",
                self.filename
            ));
        }
        if content_hash(&on_disk) != self.disk_hash {
            return Err(format!(
                "{} changed on disk since it was opened; not saving over it. \
                 Undo the edit, or reopen the file to see the change",
                self.filename
            ));
        }

        // Our own write is not news to the watcher.
        self.suppress_file_watch = true;
        crate::fsutil::write_atomic(&path, content.as_bytes())
            .map_err(|e| format!("Failed to save file: {e}"))?;
        self.disk_hash = content_hash(content.as_bytes());
        Ok(())
    }

    /// Write all pending edits to the file
    pub fn save_pending_edits_to_file(&mut self) -> Result<(), String> {
        if !self.has_unsaved_changes() {
            self.status_message = Some("No changes to save".to_string());
            return Ok(());
        }

        let content = self.document.content.clone();
        self.write_document(&content)?;

        // Clear the pending edits buffer
        let edit_count = self.pending_edits.len();
        self.pending_edits.clear();

        self.status_message = Some(format!(
            "✓ Saved {} change{} to {}",
            edit_count,
            if edit_count == 1 { "" } else { "s" },
            self.filename
        ));
        Ok(())
    }

    /// Swap in content edited in place, by a table cell edit or its undo.
    ///
    /// Sections are cut from the content at their headings' byte offsets,
    /// and an edit moves every heading after it. Keeping the old parse
    /// sliced later sections at stale offsets: a shorter cell panicked on the
    /// next frame and lost the edit, a longer one cut sections short or
    /// split a character. The outline is left alone unless the headings
    /// themselves changed, which a first cell starting with `#` in a table
    /// without outer pipes can do.
    pub(super) fn apply_edited_content(&mut self, content: String) {
        let document = crate::parser::parse_markdown(&content);
        let same_headings = document.headings.len() == self.document.headings.len()
            && document
                .headings
                .iter()
                .zip(&self.document.headings)
                .all(|(new, old)| new.level == old.level && new.text == old.text);
        self.document = document;
        self.tree = self.document.build_tree();
        if !same_headings {
            let selected = self.outline_state.selected();
            self.rebuild_outline_items();
            let last = self.outline_items.len().checked_sub(1);
            self.outline_state
                .select(selected.zip(last).map(|(i, last)| i.min(last)));
            self.outline_scroll_state = ScrollbarState::new(self.outline_items.len());
            self.previous_selection = None;
        }
        self.metrics_dirty = true;
    }

    /// Undo the last pending edit
    pub fn undo_last_edit(&mut self) -> Result<(), String> {
        if let Some(edit) = self.pending_edits.pop() {
            // Put the original source back exactly, escapes and formatting
            // included. Writing the parser's text instead turned `x \| y`
            // into `x | y`, splitting the cell in two.
            let new_content = crate::tui::edits::replace_table_cell_source(
                &self.document.content,
                edit.section_start_line,
                edit.table_index,
                edit.row,
                edit.col,
                &edit.original_value,
            )?;

            self.apply_edited_content(new_content);

            if self.pending_edits.is_empty() {
                self.status_message = Some("✓ Undone - no unsaved changes".to_string());
            } else {
                let remaining = self.pending_edits.len();
                self.status_message = Some(format!(
                    "✓ Undone ({} unsaved change{} remaining)",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                ));
            }
            Ok(())
        } else {
            self.status_message = Some("Nothing to undo".to_string());
            Ok(())
        }
    }
}

#[cfg(test)]
mod save_tests {
    use super::*;

    fn app_for(path: &std::path::Path) -> App {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        App::new(
            crate::parser::parse_markdown(&content),
            "note.md".into(),
            path.to_path_buf(),
            Config::default(),
            ColorMode::Rgb,
            false,
        )
    }

    #[test]
    fn a_save_writes_the_file_and_keeps_its_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        std::fs::write(&path, "# t\n- [ ] a\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }

        let mut app = app_for(&path);
        app.write_document("# t\n- [x] a\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# t\n- [x] a\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o644);
        }

        // A second save from the state the first one left is fine too.
        app.write_document("# t\n- [ ] a\n").unwrap();
    }

    /// The in-memory copy predates an edit made elsewhere, so writing it
    /// would undo that edit. It has to refuse and leave the file alone.
    #[test]
    fn a_save_refuses_to_overwrite_a_change_made_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        std::fs::write(&path, "# t\n- [ ] a\n").unwrap();
        let mut app = app_for(&path);

        std::fs::write(&path, "# t\n- [ ] a\n- [ ] added elsewhere\n").unwrap();
        let error = app.write_document("# t\n- [x] a\n").unwrap_err();
        assert!(error.contains("changed on disk"), "{error}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# t\n- [ ] a\n- [ ] added elsewhere\n"
        );
    }

    /// A file with bare CR line endings is shown with them read as `\n`.
    /// Saving that would change every line ending, so it is refused with
    /// the reason, rather than as a change made on disk.
    #[test]
    fn a_file_with_bare_cr_line_endings_is_shown_but_not_saved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        std::fs::write(&path, "# t\r- [ ] a\r## u\r").unwrap();
        let mut app = app_for(&path);
        assert_eq!(app.document.headings.len(), 2);

        let error = app.write_document("# t\n- [x] a\n## u\n").unwrap_err();
        assert!(error.contains("bare CR"), "{error}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# t\r- [ ] a\r## u\r"
        );
    }

    /// Edit the first body cell of the first table in the selected section.
    fn edit_first_body_cell(app: &mut App, value: &str) {
        app.enter_interactive_mode();
        let table = app
            .interactive_state
            .elements
            .iter()
            .position(|e| matches!(e.element_type, ElementType::Table { .. }))
            .expect("a table in the section");
        app.interactive_state.current_index = Some(table);
        app.interactive_state.enter_table_mode().unwrap();
        app.interactive_state.table_move_down(1);
        app.enter_cell_edit_mode().unwrap();
        app.cell_edit_value = value.to_string();
        app.save_edited_cell().unwrap();
    }

    /// A cell edit moves every heading after it. Sections are cut at heading
    /// offsets, which the edit left stale: shortening a cell panicked on the
    /// next frame, and lengthening one cut the next section's text.
    #[test]
    fn a_cell_edit_keeps_the_sections_after_it_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        let original =
            "# A\n\n| a | b |\n|---|---|\n| a long cell value | 2 |\n\n# B\n\nlast 日本語\n";
        std::fs::write(&path, original).unwrap();
        let mut app = app_for(&path);

        edit_first_body_cell(&mut app, "x");
        app.select_outline_index(1);
        assert_eq!(app.current_section_content().trim(), "last 日本語");

        app.select_outline_index(0);
        edit_first_body_cell(&mut app, "a much, much longer cell value than before");
        app.select_outline_index(1);
        assert_eq!(app.current_section_content().trim(), "last 日本語");

        app.undo_last_edit().unwrap();
        app.undo_last_edit().unwrap();
        assert_eq!(app.document.content, original);
        assert_eq!(app.current_section_content().trim(), "last 日本語");
    }

    /// The document is reloaded from the file after the editor, so opening
    /// it lost buffered edits even when the editor changed nothing.
    #[test]
    fn the_editor_waits_for_unsaved_edits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        std::fs::write(&path, "# A\n\n| a | b |\n|---|---|\n| one | 2 |\n").unwrap();
        let mut app = app_for(&path);
        app.select_outline_index(0);
        edit_first_body_cell(&mut app, "x");

        let result = app.execute_action(crate::keybindings::Action::OpenInEditor);
        assert!(matches!(result, ActionResult::Continue));
        assert_eq!(app.pending_edits.len(), 1);
    }

    /// Discarding cleared the list of edits and left them in the text,
    /// where the next save wrote them anyway.
    #[test]
    fn discarding_edits_puts_the_text_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        let original = "# A\n\n| a | b |\n|---|---|\n| one | 2 |\n";
        std::fs::write(&path, original).unwrap();
        let mut app = app_for(&path);
        app.select_outline_index(0);
        edit_first_body_cell(&mut app, "x");
        edit_first_body_cell(&mut app, "y");

        app.discard_pending_edits();
        assert!(!app.has_unsaved_changes());
        assert_eq!(app.document.content, original);
    }

    /// Restored by text, a reload selected the first of two sections with
    /// the same name, so the next checkbox toggle went to the other one.
    #[test]
    fn a_reload_keeps_the_section_among_namesakes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap().join("note.md");
        std::fs::write(&path, "# Tasks\n\n- [ ] a\n\n# Tasks\n\n- [ ] c\n").unwrap();
        let mut app = app_for(&path);
        app.select_outline_index(1);

        std::fs::write(&path, "# Tasks\n\n- [ ] a\n\n# Tasks\n\n- [x] c\n").unwrap();
        assert!(app.reload_current_file().unwrap());
        assert_eq!(app.selected_heading_index(), Some(1));
    }

    /// Stdin and fetched documents have a placeholder path. A save used to
    /// write a file named `<stdin>` into the working directory.
    #[test]
    fn a_document_that_is_not_a_file_is_not_saved() {
        let mut app = App::new(
            crate::parser::parse_markdown("# t\n- [ ] a\n"),
            "stdin".into(),
            PathBuf::from("<stdin>"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        assert!(app.write_document("# t\n- [x] a\n").is_err());
        assert!(!std::path::Path::new("<stdin>").exists());
    }
}
