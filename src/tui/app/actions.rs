//! Key dispatch: which mode a key is read in, and what each action does.

use super::*;

impl App {
    /// Get the current keybinding mode based on app state
    pub fn current_keybinding_mode(&self) -> KeybindingMode {
        // The image viewer covers everything, whatever mode opened it.
        if self.is_image_modal_open() {
            return KeybindingMode::ImageViewer;
        }
        // Then a popup in front of the mode
        match self.overlay {
            Some(Overlay::Help) => return KeybindingMode::Help,
            Some(Overlay::ThemePicker) => return KeybindingMode::ThemePicker,
            None => {}
        }

        // Then check app mode
        match self.mode {
            AppMode::Normal => {
                if self.outline_search == OutlineSearch::Typing {
                    KeybindingMode::Search
                } else {
                    // An applied outline search is navigated with the
                    // normal keys (j/k, n/N to cycle, s for a new search).
                    KeybindingMode::Normal
                }
            }
            AppMode::Interactive => {
                if self.interactive_state.is_in_table_mode() {
                    KeybindingMode::InteractiveTable
                } else {
                    KeybindingMode::Interactive
                }
            }
            AppMode::LinkFollow => {
                if self.link_picker.active {
                    KeybindingMode::LinkSearch
                } else {
                    KeybindingMode::LinkFollow
                }
            }
            AppMode::CellEdit => KeybindingMode::CellEdit,
            AppMode::ConfirmFileCreate
            | AppMode::ConfirmSaveWidth
            | AppMode::ConfirmSaveBeforeQuit
            | AppMode::ConfirmSaveBeforeNav => KeybindingMode::ConfirmDialog,
            AppMode::DocSearch => KeybindingMode::DocSearch,
            AppMode::CommandPalette => KeybindingMode::CommandPalette,
            AppMode::FilePicker => {
                if self.file_picker.active {
                    KeybindingMode::FileSearch
                } else {
                    KeybindingMode::FilePicker
                }
            }
        }
    }

    /// Get the action for a key press in the current mode
    pub fn get_action_for_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
        use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState};

        let mode = self.current_keybinding_mode();
        let event = KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };

        self.keybindings.dispatch(mode, event)
    }

    /// Execute an action, returning the result type
    ///
    /// Returns:
    /// - `ActionResult::Continue` - continue the main loop
    /// - `ActionResult::Quit` - exit the application
    /// - `ActionResult::RunEditor(PathBuf, Option<u32>)` - run editor on file at optional line
    pub fn execute_action(&mut self, action: Action) -> ActionResult {
        use Action::*;

        match action {
            // === Miscellaneous ===
            Noop => {}
            // === Application ===
            Quit => {
                // If in accepted outline search state, clear search instead of quitting
                if self.outline_search.is_on() {
                    self.search_query.clear();
                    self.filter_outline();
                    self.outline_search = OutlineSearch::Off;
                } else if self.has_unsaved_changes() {
                    // Prompt to save before quitting
                    self.mode = AppMode::ConfirmSaveBeforeQuit;
                } else {
                    return ActionResult::Quit;
                }
            }
            Suspend => {
                return ActionResult::Suspend;
            }
            Redraw => {
                return ActionResult::Redraw;
            }

            // === Navigation ===
            Next => {
                let count = self.take_count();
                if self.mode == AppMode::FilePicker {
                    for _ in 0..count {
                        self.next_file();
                    }
                } else {
                    for _ in 0..count {
                        self.next();
                    }
                }
            }
            Previous => {
                let count = self.take_count();
                if self.mode == AppMode::FilePicker {
                    for _ in 0..count {
                        self.previous_file();
                    }
                } else {
                    for _ in 0..count {
                        self.previous();
                    }
                }
            }
            // These keys are shared by Normal, Help and Interactive mode,
            // and act on what that mode shows. In help and interactive mode
            // `g` and `G` used to move the outline behind them, switching
            // section, after which Enter acted on an element from the old
            // one; and PageUp and PageDown did nothing in interactive mode.
            First => {
                self.clear_count();
                if self.overlay == Some(Overlay::Help) {
                    self.help_scroll = 0;
                } else if self.mode == AppMode::Interactive {
                    self.select_interactive_end(false);
                } else if self.mode == AppMode::DocSearch {
                    self.content_scroll = 0;
                } else {
                    self.first();
                }
            }
            Last => {
                self.clear_count();
                if self.overlay == Some(Overlay::Help) {
                    self.help_scroll = self.help_max_scroll;
                } else if self.mode == AppMode::Interactive {
                    self.select_interactive_end(true);
                } else if self.mode == AppMode::DocSearch {
                    self.content_scroll = self.max_content_scroll();
                } else {
                    self.last();
                }
            }
            PageDown => {
                self.clear_count();
                if self.overlay == Some(Overlay::Help) {
                    self.scroll_help_page_down();
                } else if self.mode == AppMode::Interactive {
                    self.scroll_page_down_interactive();
                    self.select_element_at_scroll();
                } else if self.mode == AppMode::DocSearch {
                    self.scroll_page_down_interactive();
                } else {
                    self.scroll_page_down();
                }
            }
            PageUp => {
                self.clear_count();
                if self.overlay == Some(Overlay::Help) {
                    self.scroll_help_page_up();
                } else if self.mode == AppMode::Interactive {
                    self.scroll_page_up_interactive();
                    self.select_element_at_scroll();
                } else if self.mode == AppMode::DocSearch {
                    self.scroll_page_up_interactive();
                } else {
                    self.scroll_page_up();
                }
            }
            JumpToParent => {
                self.clear_count();
                // In link mode the links listed have to follow the section,
                // or the picker kept the child section's links while the
                // pane showed the parent.
                if self.mode == AppMode::LinkFollow {
                    self.jump_to_parent_links();
                } else {
                    self.jump_to_parent();
                }
            }

            // === Outline ===
            Expand => self.expand(),
            Collapse => self.collapse(),
            ToggleExpand => self.toggle_expand(),
            ToggleFocus => self.toggle_focus(),
            ToggleFocusBack => self.toggle_focus_back(),
            ToggleOutline => self.toggle_outline(),
            OutlineWidthIncrease => self.cycle_outline_width(true),
            OutlineWidthDecrease => self.cycle_outline_width(false),
            ToggleTodoFilter => self.toggle_todo_filter(),
            ToggleHeadingMarkers => self.toggle_heading_markers(),

            // === Bookmarks ===
            SetBookmark => self.set_bookmark(),
            JumpToBookmark => self.jump_to_bookmark(),

            // === Mode Transitions ===
            EnterInteractiveMode => self.enter_interactive_mode(),
            ExitInteractiveMode => self.exit_interactive_mode(),
            EnterLinkFollowMode => self.enter_link_follow_mode(),
            EnterSearchMode => self.toggle_search(),
            EnterDocSearch => self.enter_doc_search(),
            ToggleSearchMode => self.toggle_search_mode(),
            ExitMode => self.exit_current_mode(),
            OpenCommandPalette => self.open_command_palette(),

            // === Link Navigation ===
            NextLink => self.next_link(),
            PreviousLink => self.previous_link(),
            FollowLink => match self.mode {
                AppMode::LinkFollow => {
                    if let Err(e) = self.follow_selected_link() {
                        self.status_message = Some(format!("✗ Error: {}", e));
                    }
                    self.update_content_metrics();
                }
                AppMode::FilePicker => {
                    if let Err(e) = self.select_file_from_picker() {
                        self.status_message = Some(format!("✗ Error: {}", e));
                    }
                    self.update_content_metrics();
                }
                _ => {}
            },
            LinkSearch => match self.mode {
                AppMode::LinkFollow => self.start_link_search(),
                AppMode::FilePicker => {
                    self.file_picker.active = true;
                }
                _ => {}
            },

            // === Interactive Mode ===
            InteractiveNext => {
                let count = self.take_count();
                if self.interactive_state.is_in_table_mode() {
                    // In table mode, move down within table
                    let (rows, cols) = self.get_table_dimensions();
                    for _ in 0..count {
                        self.interactive_state.table_move_down(rows);
                    }
                    self.status_message =
                        Some(self.interactive_state.table_status_text(rows + 1, cols));
                } else {
                    // Normal interactive mode, move to next element
                    for _ in 0..count {
                        self.interactive_state.next();
                    }
                    self.scroll_to_interactive_element(self.content_viewport_height);
                    self.status_message = Some(self.interactive_state.status_text());
                }
            }
            InteractivePrevious => {
                let count = self.take_count();
                if self.interactive_state.is_in_table_mode() {
                    // In table mode, move up within table
                    let (rows, cols) = self.get_table_dimensions();
                    for _ in 0..count {
                        self.interactive_state.table_move_up();
                    }
                    self.status_message =
                        Some(self.interactive_state.table_status_text(rows + 1, cols));
                } else {
                    // Normal interactive mode, move to previous element
                    for _ in 0..count {
                        self.interactive_state.previous();
                    }
                    self.scroll_to_interactive_element(self.content_viewport_height);
                    self.status_message = Some(self.interactive_state.status_text());
                }
            }
            InteractiveActivate => {
                self.clear_count();
                // In table mode, Enter edits the cell; otherwise activate the element
                if self.interactive_state.is_in_table_mode() {
                    if let Err(e) = self.enter_cell_edit_mode() {
                        self.status_message = Some(format!("✗ Error: {}", e));
                    }
                } else if let Err(e) = self.activate_interactive_element() {
                    self.status_message = Some(format!("✗ Error: {}", e));
                }
                self.update_content_metrics();
            }
            InteractiveNextLink => {
                let count = self.take_count();
                for _ in 0..count {
                    self.interactive_state.next();
                }
                self.scroll_to_interactive_element(self.content_viewport_height);
                self.status_message = Some(self.interactive_state.status_text());
            }
            InteractivePreviousLink => {
                let count = self.take_count();
                for _ in 0..count {
                    self.interactive_state.previous();
                }
                self.scroll_to_interactive_element(self.content_viewport_height);
                self.status_message = Some(self.interactive_state.status_text());
            }
            InteractiveLeft => {
                let count = self.take_count();
                for _ in 0..count {
                    self.table_navigate_left();
                }
            }
            InteractiveRight => {
                let count = self.take_count();
                for _ in 0..count {
                    self.table_navigate_right();
                }
            }
            CopyTableCell => {
                if let Err(e) = self.copy_table_cell() {
                    self.set_status_message(&format!("✗ {}", e));
                }
            }
            CopyTableRow => {
                if let Err(e) = self.copy_table_row() {
                    self.set_status_message(&format!("✗ {}", e));
                }
            }
            CopyTableMarkdown => {
                if let Err(e) = self.copy_table_markdown() {
                    self.set_status_message(&format!("✗ {}", e));
                }
            }

            // === View ===
            ToggleRawSource => self.toggle_raw_source(),
            ToggleMouseCapture => self.toggle_mouse_capture(),
            ToggleHelp => self.toggle_help(),
            ToggleThemePicker => self.toggle_theme_picker(),
            ApplyTheme => self.apply_selected_theme(),

            // === Clipboard ===
            CopyContent => self.copy_content(),
            CopyAnchor => self.copy_anchor(),

            // === File Operations ===
            GoBack => {
                // Check if there's anything to go back to
                if self.file_history.is_empty() {
                    return ActionResult::Continue;
                }
                // Check for unsaved changes
                if self.has_unsaved_changes() {
                    self.pending_navigation = Some(PendingNavigation::Back);
                    self.mode = AppMode::ConfirmSaveBeforeNav;
                } else if self.go_back().is_ok() {
                    self.update_content_metrics();
                }
            }
            GoForward => {
                // Check if there's anything to go forward to
                if self.file_future.is_empty() {
                    return ActionResult::Continue;
                }
                // Check for unsaved changes
                if self.has_unsaved_changes() {
                    self.pending_navigation = Some(PendingNavigation::Forward);
                    self.mode = AppMode::ConfirmSaveBeforeNav;
                } else if self.go_forward().is_ok() {
                    self.update_content_metrics();
                }
            }
            OpenInEditor => {
                // The editor works on the file, and the document is reloaded
                // from it afterwards, so buffered edits would be lost even
                // if the editor changed nothing.
                if self.has_unsaved_changes() {
                    self.status_message = Some(
                        "Save (:w) or undo (u) the unsaved edits before opening the editor"
                            .to_string(),
                    );
                    return ActionResult::Continue;
                }
                // Piped and fetched documents have a placeholder path, which
                // an editor would create as a file, or fetch again itself.
                if !(self.current_file_path.is_absolute() && self.current_file_path.is_file()) {
                    self.status_message = Some(
                        "This document is not a file on disk, so it cannot be edited".to_string(),
                    );
                    return ActionResult::Continue;
                }
                let line = if self.mode == AppMode::Interactive {
                    // In interactive mode, jump to the current element's source line
                    self.interactive_element_source_line()
                        .or_else(|| self.selected_heading_source_line())
                } else {
                    self.selected_heading_source_line()
                };
                return ActionResult::RunEditor(self.current_file_path.clone(), line);
            }
            UndoEdit => {
                self.clear_count();
                if let Err(e) = self.undo_last_edit() {
                    self.status_message = Some(format!("✗ Undo failed: {}", e));
                }
            }
            OpenFilePicker => {
                self.enter_file_picker();
            }
            ParentDirectory => {
                self.file_picker_parent_dir();
            }
            ToggleHidden => {
                self.show_hidden = !self.show_hidden;
                self.scan_markdown_files();
            }

            // === Dialog Actions ===
            ConfirmAction => {
                if let Some(result) = self.handle_confirm_action() {
                    return result;
                }
            }
            CancelAction => self.handle_cancel_action(),
            DiscardAndQuit => {
                if let Some(result) = self.handle_discard_and_quit() {
                    return result;
                }
            }
            DiscardAndContinue => {
                self.handle_discard_and_continue();
            }

            // === Jump to Heading by Number ===
            JumpToHeading1 => self.jump_to_heading(0),
            JumpToHeading2 => self.jump_to_heading(1),
            JumpToHeading3 => self.jump_to_heading(2),
            JumpToHeading4 => self.jump_to_heading(3),
            JumpToHeading5 => self.jump_to_heading(4),
            JumpToHeading6 => self.jump_to_heading(5),
            JumpToHeading7 => self.jump_to_heading(6),
            JumpToHeading8 => self.jump_to_heading(7),
            JumpToHeading9 => self.jump_to_heading(8),

            // === Jump to Link by Number ===
            JumpToLink1 => self.jump_to_link(0),
            JumpToLink2 => self.jump_to_link(1),
            JumpToLink3 => self.jump_to_link(2),
            JumpToLink4 => self.jump_to_link(3),
            JumpToLink5 => self.jump_to_link(4),
            JumpToLink6 => self.jump_to_link(5),
            JumpToLink7 => self.jump_to_link(6),
            JumpToLink8 => self.jump_to_link(7),
            JumpToLink9 => self.jump_to_link(8),

            // === Scroll (Content pane) ===
            ScrollDown => {
                let count = self.take_count();
                for _ in 0..count {
                    self.scroll_content_down();
                }
            }
            ScrollUp => {
                let count = self.take_count();
                for _ in 0..count {
                    self.scroll_content_up();
                }
            }

            // === Help Navigation ===
            HelpScrollDown => {
                self.clear_count();
                self.scroll_help_down();
            }
            HelpScrollUp => {
                self.clear_count();
                self.scroll_help_up();
            }

            // === Theme Picker Navigation ===
            ThemePickerNext => self.theme_picker_next(),
            ThemePickerPrevious => self.theme_picker_previous(),

            // === Search Input ===
            SearchBackspace => self.text_input_backspace(),

            // === Command Palette ===
            CommandPaletteNext => self.command_palette_next(),
            CommandPalettePrev => self.command_palette_prev(),
            CommandPaletteAutocomplete => self.command_palette_autocomplete(),

            // === Doc Search Navigation ===
            NextMatch => self.next_doc_match(),
            PrevMatch => self.prev_doc_match(),

            // === Image Viewer ===
            PreviousFrame => self.modal_prev_frame(),
            NextFrame => self.modal_next_frame(),
            TogglePlayback => self.modal_toggle_animation(),
        }

        ActionResult::Continue
    }

    /// Exit the current mode based on app state
    pub(super) fn exit_current_mode(&mut self) {
        // Close image modal if open
        if self.is_image_modal_open() {
            self.close_image_modal();
            return;
        }

        // Handle outline search - clear everything
        if self.outline_search.is_on() {
            self.search_query.clear();
            self.filter_outline();
            self.outline_search = OutlineSearch::Off;
            return;
        }

        match self.mode {
            AppMode::Interactive => {
                // If in table mode, exit table mode first (stay in interactive)
                if self.interactive_state.is_in_table_mode() {
                    self.interactive_state.exit_table_mode();
                    self.status_message = Some(self.interactive_state.status_text());
                } else {
                    self.exit_interactive_mode();
                }
            }
            AppMode::LinkFollow => {
                if self.link_picker.active {
                    self.stop_link_search();
                } else if !self.link_picker.query.is_empty() {
                    self.clear_link_search();
                } else {
                    self.exit_link_follow_mode();
                }
            }
            AppMode::DocSearch => {
                if self.doc_search.active {
                    self.cancel_doc_search();
                } else {
                    self.clear_doc_search();
                }
            }
            AppMode::CommandPalette => self.close_command_palette(),
            AppMode::CellEdit => {
                self.mode = AppMode::Interactive;
                self.status_message = Some("Editing cancelled".to_string());
            }
            AppMode::FilePicker => {
                if self.file_picker.active {
                    // Exit search mode, but stay in file picker
                    self.file_picker.active = false;
                    self.file_picker.query.clear();
                } else {
                    // Exit file picker entirely
                    self.mode = AppMode::Normal;
                    self.file_picker.query.clear();
                    self.file_picker.active = false;
                }
            }
            AppMode::Normal
            | AppMode::ConfirmFileCreate
            | AppMode::ConfirmSaveWidth
            | AppMode::ConfirmSaveBeforeQuit
            | AppMode::ConfirmSaveBeforeNav => {
                // In normal mode, show hint for quitting
                self.set_status_message("Press q to quit • : for commands • ? for help");
            }
        }
    }

    /// Handle confirm action based on current mode
    /// Returns Some(ActionResult) if the action should return early (e.g., quit)
    pub(super) fn handle_confirm_action(&mut self) -> Option<ActionResult> {
        // Handle outline search - accept and keep filtered results visible
        if self.outline_search == OutlineSearch::Typing {
            // Check if there are any matches (filtered items with matching text)
            let has_matches = if self.search_query.is_empty() {
                true // Empty query matches everything
            } else {
                // Check if any outline items match the query
                !self.outline_items.is_empty()
            };

            if has_matches {
                // Accept the search: the filter and its highlights stay,
                // and j/k, n/N move through it, or 's' starts a new search.
                self.outline_search = OutlineSearch::Applied;
            } else {
                // No matches - show status message and clear search
                self.status_message = Some(format!("Pattern not found: {}", self.search_query));
                self.outline_search = OutlineSearch::Off;
                self.search_query.clear();
                self.filter_outline(); // Restore full outline
            }
            return None;
        }

        match self.mode {
            AppMode::ConfirmFileCreate => {
                if let Err(e) = self.confirm_file_create() {
                    self.status_message = Some(format!("✗ Error: {}", e));
                }
            }
            AppMode::ConfirmSaveWidth => self.confirm_save_outline_width(),
            AppMode::ConfirmSaveBeforeQuit => {
                // Save pending changes and quit
                if let Err(e) = self.save_pending_edits_to_file() {
                    self.status_message = Some(format!("✗ Save failed: {}", e));
                    self.mode = AppMode::Normal;
                } else {
                    return Some(ActionResult::Quit);
                }
            }
            AppMode::ConfirmSaveBeforeNav => {
                // Save pending changes and then navigate
                if let Err(e) = self.save_pending_edits_to_file() {
                    self.status_message = Some(format!("✗ Save failed: {}", e));
                    self.mode = AppMode::Normal;
                    self.pending_navigation = None;
                } else {
                    // Execute the pending navigation
                    self.execute_pending_navigation();
                }
            }
            AppMode::DocSearch => self.accept_doc_search(),
            AppMode::CommandPalette => {
                // Execute command - propagate non-trivial results (Quit,
                // RunEditor, Redraw) to the main loop
                match self.execute_selected_command() {
                    ActionResult::Continue => {}
                    result => return Some(result),
                }
            }
            AppMode::CellEdit => {
                if let Err(e) = self.save_edited_cell() {
                    self.status_message = Some(format!("✗ Error saving: {}", e));
                } else {
                    self.mode = AppMode::Interactive;
                }
            }
            _ => {}
        }
        None
    }

    /// Handle cancel action based on current mode
    pub(super) fn handle_cancel_action(&mut self) {
        match self.mode {
            AppMode::ConfirmFileCreate => self.cancel_file_create(),
            AppMode::ConfirmSaveWidth => self.cancel_save_width_confirmation(),
            AppMode::ConfirmSaveBeforeQuit => {
                // Cancel quit - go back to normal mode
                self.mode = AppMode::Normal;
                self.status_message = Some("Quit cancelled".to_string());
            }
            AppMode::ConfirmSaveBeforeNav => {
                // Cancel navigation - go back to normal mode
                self.mode = AppMode::Normal;
                self.pending_navigation = None;
                self.status_message = Some("Navigation cancelled".to_string());
            }
            _ => self.exit_current_mode(),
        }
    }

    /// Handle discard and quit action (quit without saving)
    pub(super) fn handle_discard_and_quit(&mut self) -> Option<ActionResult> {
        match self.mode {
            AppMode::ConfirmSaveBeforeQuit => {
                // Discard changes and quit
                self.discard_pending_edits();
                Some(ActionResult::Quit)
            }
            AppMode::ConfirmSaveBeforeNav => {
                // Discard changes and quit (instead of navigating)
                self.discard_pending_edits();
                self.pending_navigation = None;
                Some(ActionResult::Quit)
            }
            _ => None,
        }
    }

    /// Handle discard and continue action (discard changes and proceed with navigation)
    pub(super) fn handle_discard_and_continue(&mut self) {
        match self.mode {
            AppMode::ConfirmSaveBeforeNav => {
                // Discard changes and navigate
                self.discard_pending_edits();
                self.execute_pending_navigation();
            }
            AppMode::ConfirmSaveBeforeQuit => {
                // In quit dialog, 'd' doesn't make sense - ignore or treat as quit
                // We'll ignore for now, user should use 'q' for quit without saving
            }
            _ => {}
        }
    }

    /// Execute the pending navigation action
    // Suggested guard collapse would call go_back/go_forward inside match guards
    // (side effects in guards) — keep the explicit ifs.
    #[allow(clippy::collapsible_match)]
    pub(super) fn execute_pending_navigation(&mut self) {
        let nav = self.pending_navigation.take();
        self.mode = AppMode::Normal;

        match nav {
            Some(PendingNavigation::Back) => {
                if self.go_back().is_ok() {
                    self.update_content_metrics();
                }
            }
            Some(PendingNavigation::Forward) => {
                if self.go_forward().is_ok() {
                    self.update_content_metrics();
                }
            }
            Some(PendingNavigation::LoadFile(path, anchor)) => {
                // Call internal load_file_internal that skips the unsaved check
                if let Err(e) = self.load_file_internal(&path, anchor.as_deref()) {
                    self.status_message = Some(format!("✗ {}", e));
                }
            }
            Some(PendingNavigation::OpenPicked(path)) => {
                if let Err(e) = self.open_picked_file(path) {
                    self.status_message = Some(format!("✗ {}", e));
                }
            }
            None => {}
        }
    }

    /// Navigate table left
    pub(super) fn table_navigate_left(&mut self) {
        if !self.interactive_state.is_in_table_mode() {
            return;
        }

        let (rows, cols) = self.get_table_dimensions();
        if cols > 0 {
            self.interactive_state.table_move_left();
            self.status_message = Some(self.interactive_state.table_status_text(rows + 1, cols));
        }
    }

    /// Navigate table right
    pub(super) fn table_navigate_right(&mut self) {
        if !self.interactive_state.is_in_table_mode() {
            return;
        }

        let (rows, cols) = self.get_table_dimensions();
        if cols > 0 {
            self.interactive_state.table_move_right(cols);
            self.status_message = Some(self.interactive_state.table_status_text(rows + 1, cols));
        }
    }

    /// Get table dimensions for current interactive element
    pub(super) fn get_table_dimensions(&self) -> (usize, usize) {
        if let Some(element) = self.interactive_state.current_element()
            && let crate::tui::interactive::ElementType::Table { rows, cols, .. } =
                &element.element_type
        {
            return (*rows, *cols);
        }
        (0, 0)
    }

    /// Jump to link by index in filtered list
    pub(super) fn jump_to_link(&mut self, idx: usize) {
        if let Some(display_idx) = self
            .link_picker
            .filtered_indices
            .iter()
            .position(|&i| i == idx)
        {
            self.link_picker.selected = Some(display_idx);
        }
    }
}
