//! Default keybindings for treemd
//!
//! This module defines the default keybindings that are used when no
//! user configuration is provided. Uses keybinds-rs key string syntax.

use super::{Action, KeybindingMode, Keybindings};

/// Create the default keybindings configuration
pub fn default_keybindings() -> Keybindings {
    let mut kb = Keybindings::new();

    // Normal mode
    add_normal_mode(&mut kb);

    // Help mode
    add_help_mode(&mut kb);

    // Theme picker mode
    add_theme_picker_mode(&mut kb);

    // Interactive mode
    add_interactive_mode(&mut kb);

    // Interactive table mode
    add_interactive_table_mode(&mut kb);

    // Link follow mode
    add_link_follow_mode(&mut kb);

    // Link search mode
    add_link_search_mode(&mut kb);

    // Search mode
    add_search_mode(&mut kb);

    // Doc search mode
    add_doc_search_mode(&mut kb);

    // Command palette mode
    add_command_palette_mode(&mut kb);

    // Confirm dialog mode
    add_confirm_dialog_mode(&mut kb);

    // Cell edit mode
    add_cell_edit_mode(&mut kb);

    kb
}

/// Bind a key, panicking on invalid key syntax (only used for built-in defaults)
fn bind(kb: &mut Keybindings, mode: KeybindingMode, key: &str, action: Action) {
    kb.bind(mode, key, action)
        .unwrap_or_else(|e| panic!("Invalid default keybinding '{}': {}", key, e));
}

fn add_normal_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Normal;

    // Navigation
    bind(kb, Normal, "j", Next);
    bind(kb, Normal, "Down", Next);
    bind(kb, Normal, "k", Previous);
    bind(kb, Normal, "Up", Previous);
    bind(kb, Normal, "g", First);
    bind(kb, Normal, "G", Last);
    bind(kb, Normal, "d", PageDown);
    bind(kb, Normal, "u", PageUp);
    bind(kb, Normal, "p", JumpToParent);

    // Outline
    bind(kb, Normal, "Enter", ToggleExpand);
    bind(kb, Normal, "Space", ToggleExpand);
    bind(kb, Normal, "Tab", ToggleFocus);
    bind(kb, Normal, "Shift+Tab", ToggleFocusBack);
    bind(kb, Normal, "h", Collapse);
    bind(kb, Normal, "Left", Collapse);
    bind(kb, Normal, "l", Expand);
    bind(kb, Normal, "Right", Expand);
    bind(kb, Normal, "w", ToggleOutline);
    bind(kb, Normal, "[", OutlineWidthDecrease);
    bind(kb, Normal, "]", OutlineWidthIncrease);

    // Bookmarks
    bind(kb, Normal, "m", SetBookmark);
    bind(kb, Normal, "'", JumpToBookmark);

    // Mode transitions
    bind(kb, Normal, "i", EnterInteractiveMode);
    bind(kb, Normal, "f", EnterLinkFollowMode);
    bind(kb, Normal, "s", EnterSearchMode);
    bind(kb, Normal, "/", EnterDocSearch);
    bind(kb, Normal, ":", OpenCommandPalette);

    // View
    bind(kb, Normal, "r", ToggleRawSource);
    bind(kb, Normal, "t", ToggleThemePicker);
    bind(kb, Normal, "?", ToggleHelp);

    // Clipboard
    bind(kb, Normal, "y", CopyContent);
    bind(kb, Normal, "Y", CopyAnchor);

    // File operations
    bind(kb, Normal, "b", GoBack);
    bind(kb, Normal, "Backspace", GoBack);
    bind(kb, Normal, "F", GoForward);
    bind(kb, Normal, "e", OpenInEditor);

    // Application
    bind(kb, Normal, "q", Quit);
    bind(kb, Normal, "Escape", Quit);

    // Jump to heading by number
    bind(kb, Normal, "1", JumpToHeading1);
    bind(kb, Normal, "2", JumpToHeading2);
    bind(kb, Normal, "3", JumpToHeading3);
    bind(kb, Normal, "4", JumpToHeading4);
    bind(kb, Normal, "5", JumpToHeading5);
    bind(kb, Normal, "6", JumpToHeading6);
    bind(kb, Normal, "7", JumpToHeading7);
    bind(kb, Normal, "8", JumpToHeading8);
    bind(kb, Normal, "9", JumpToHeading9);

    // Search match navigation (when matches exist)
    bind(kb, Normal, "n", NextMatch);
    bind(kb, Normal, "N", PrevMatch);
}

fn add_help_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Help;

    // Navigation
    bind(kb, Help, "j", HelpScrollDown);
    bind(kb, Help, "Down", HelpScrollDown);
    bind(kb, Help, "k", HelpScrollUp);
    bind(kb, Help, "Up", HelpScrollUp);
    bind(kb, Help, "d", PageDown);
    bind(kb, Help, "PageDown", PageDown);
    bind(kb, Help, "u", PageUp);
    bind(kb, Help, "PageUp", PageUp);

    // Close help
    bind(kb, Help, "?", ToggleHelp);
    bind(kb, Help, "Escape", ToggleHelp);

    // Clipboard (available everywhere)
    bind(kb, Help, "y", CopyContent);
    bind(kb, Help, "Y", CopyAnchor);

    // Quit
    bind(kb, Help, "q", Quit);
}

fn add_theme_picker_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::ThemePicker;

    // Navigation
    bind(kb, ThemePicker, "j", ThemePickerNext);
    bind(kb, ThemePicker, "Down", ThemePickerNext);
    bind(kb, ThemePicker, "k", ThemePickerPrevious);
    bind(kb, ThemePicker, "Up", ThemePickerPrevious);

    // Actions
    bind(kb, ThemePicker, "Enter", ApplyTheme);
    bind(kb, ThemePicker, "Escape", ToggleThemePicker);

    // Clipboard (available everywhere)
    bind(kb, ThemePicker, "y", CopyContent);
    bind(kb, ThemePicker, "Y", CopyAnchor);

    // Quit
    bind(kb, ThemePicker, "q", Quit);
}

fn add_interactive_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Interactive;

    // Exit
    bind(kb, Interactive, "Escape", ExitInteractiveMode);
    bind(kb, Interactive, "i", ExitInteractiveMode);

    // Navigation
    bind(kb, Interactive, "j", InteractiveNext);
    bind(kb, Interactive, "Down", InteractiveNext);
    bind(kb, Interactive, "k", InteractivePrevious);
    bind(kb, Interactive, "Up", InteractivePrevious);

    // Link navigation within element
    bind(kb, Interactive, "Tab", InteractiveNextLink);
    bind(kb, Interactive, "Shift+Tab", InteractivePreviousLink);

    // Activate element
    bind(kb, Interactive, "Enter", InteractiveActivate);
    bind(kb, Interactive, "Space", InteractiveActivate);

    // Page navigation
    bind(kb, Interactive, "d", PageDown);
    bind(kb, Interactive, "PageDown", PageDown);
    bind(kb, Interactive, "u", PageUp);
    bind(kb, Interactive, "PageUp", PageUp);

    // Document search from interactive mode
    bind(kb, Interactive, "/", EnterDocSearch);

    // Search match navigation
    bind(kb, Interactive, "n", NextMatch);
    bind(kb, Interactive, "N", PrevMatch);

    // Clipboard
    bind(kb, Interactive, "y", CopyContent);

    // Undo last edit
    bind(kb, Interactive, "Ctrl+z", UndoEdit);

    // Quit
    bind(kb, Interactive, "q", Quit);
}

fn add_interactive_table_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::InteractiveTable;

    // Exit table mode
    bind(kb, InteractiveTable, "Escape", ExitMode);

    // Table navigation
    bind(kb, InteractiveTable, "h", InteractiveLeft);
    bind(kb, InteractiveTable, "Left", InteractiveLeft);
    bind(kb, InteractiveTable, "l", InteractiveRight);
    bind(kb, InteractiveTable, "Right", InteractiveRight);
    bind(kb, InteractiveTable, "j", InteractiveNext);
    bind(kb, InteractiveTable, "Down", InteractiveNext);
    bind(kb, InteractiveTable, "k", InteractivePrevious);
    bind(kb, InteractiveTable, "Up", InteractivePrevious);

    // Clipboard
    bind(kb, InteractiveTable, "y", CopyContent);
    bind(kb, InteractiveTable, "Y", CopyAnchor);

    // View toggle
    bind(kb, InteractiveTable, "r", ToggleRawSource);

    // Activate (follow link or edit cell)
    bind(kb, InteractiveTable, "Enter", InteractiveActivate);

    // Undo last edit
    bind(kb, InteractiveTable, "Ctrl+z", UndoEdit);

    // Quit
    bind(kb, InteractiveTable, "q", Quit);
}

fn add_link_follow_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::LinkFollow;

    // Exit
    bind(kb, LinkFollow, "Escape", ExitMode);

    // Navigation
    bind(kb, LinkFollow, "j", NextLink);
    bind(kb, LinkFollow, "Down", NextLink);
    bind(kb, LinkFollow, "Tab", NextLink);
    bind(kb, LinkFollow, "k", PreviousLink);
    bind(kb, LinkFollow, "Up", PreviousLink);
    bind(kb, LinkFollow, "Shift+Tab", PreviousLink);

    // Actions
    bind(kb, LinkFollow, "Enter", FollowLink);
    bind(kb, LinkFollow, "/", LinkSearch);
    bind(kb, LinkFollow, "p", JumpToParent);

    // Jump to link by number
    bind(kb, LinkFollow, "1", JumpToLink1);
    bind(kb, LinkFollow, "2", JumpToLink2);
    bind(kb, LinkFollow, "3", JumpToLink3);
    bind(kb, LinkFollow, "4", JumpToLink4);
    bind(kb, LinkFollow, "5", JumpToLink5);
    bind(kb, LinkFollow, "6", JumpToLink6);
    bind(kb, LinkFollow, "7", JumpToLink7);
    bind(kb, LinkFollow, "8", JumpToLink8);
    bind(kb, LinkFollow, "9", JumpToLink9);

    // Clipboard
    bind(kb, LinkFollow, "y", CopyContent);
    bind(kb, LinkFollow, "Y", CopyAnchor);

    // Quit
    bind(kb, LinkFollow, "q", Quit);
}

fn add_link_search_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::LinkSearch;

    // Exit search (back to link follow)
    bind(kb, LinkSearch, "Escape", ExitMode);

    // Select filtered result
    bind(kb, LinkSearch, "Enter", FollowLink);

    // Navigation while searching
    bind(kb, LinkSearch, "Down", NextLink);
    bind(kb, LinkSearch, "Up", PreviousLink);

    // Delete character
    bind(kb, LinkSearch, "Backspace", SearchBackspace);
}

fn add_search_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Search;

    // Exit search
    bind(kb, Search, "Escape", ExitMode);

    // Confirm search (select result)
    bind(kb, Search, "Enter", ConfirmAction);

    // Delete character
    bind(kb, Search, "Backspace", SearchBackspace);

    // Toggle to doc search
    bind(kb, Search, "Tab", ToggleSearchMode);
}

fn add_doc_search_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::DocSearch;

    // Exit search
    bind(kb, DocSearch, "Escape", ExitMode);

    // Accept search and enter navigation mode
    bind(kb, DocSearch, "Enter", ConfirmAction);

    // Delete character
    bind(kb, DocSearch, "Backspace", SearchBackspace);

    // Navigate matches (after accepting with Enter)
    bind(kb, DocSearch, "n", NextMatch);
    bind(kb, DocSearch, "N", PrevMatch);
    bind(kb, DocSearch, "Down", NextMatch);
    bind(kb, DocSearch, "Up", PrevMatch);

    // Toggle to outline search (while typing), or cycle matches (after Enter)
    bind(kb, DocSearch, "Tab", ToggleSearchMode);
    bind(kb, DocSearch, "Shift+Tab", PrevMatch);

    // Re-enter search input with /
    bind(kb, DocSearch, "/", EnterDocSearch);
}

fn add_command_palette_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::CommandPalette;

    // Exit
    bind(kb, CommandPalette, "Escape", ExitMode);

    // Execute selected command
    bind(kb, CommandPalette, "Enter", ConfirmAction);

    // Navigation
    bind(kb, CommandPalette, "Down", CommandPaletteNext);
    bind(kb, CommandPalette, "j", CommandPaletteNext);
    bind(kb, CommandPalette, "Up", CommandPalettePrev);
    bind(kb, CommandPalette, "k", CommandPalettePrev);

    // Autocomplete selected command
    bind(kb, CommandPalette, "Tab", CommandPaletteAutocomplete);

    // Delete character
    bind(kb, CommandPalette, "Backspace", SearchBackspace);
}

fn add_confirm_dialog_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::ConfirmDialog;

    // Confirm
    bind(kb, ConfirmDialog, "y", ConfirmAction);
    bind(kb, ConfirmDialog, "Y", ConfirmAction);
    bind(kb, ConfirmDialog, "Enter", ConfirmAction);

    // Cancel
    bind(kb, ConfirmDialog, "n", CancelAction);
    bind(kb, ConfirmDialog, "N", CancelAction);
    bind(kb, ConfirmDialog, "Escape", CancelAction);

    // Discard and quit (for unsaved changes dialogs)
    bind(kb, ConfirmDialog, "q", DiscardAndQuit);

    // Discard and continue (for navigation with unsaved changes)
    bind(kb, ConfirmDialog, "d", DiscardAndContinue);
}

fn add_cell_edit_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::CellEdit;

    // Cancel editing
    bind(kb, CellEdit, "Escape", CancelAction);

    // Confirm edit
    bind(kb, CellEdit, "Enter", ConfirmAction);

    // Delete character
    bind(kb, CellEdit, "Backspace", SearchBackspace);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_default_normal_mode() {
        let mut kb = default_keybindings();

        // Check some common bindings
        assert_eq!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('j'), KeyModifiers::NONE)
            ),
            Some(Action::Next)
        );
        assert_eq!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('k'), KeyModifiers::NONE)
            ),
            Some(Action::Previous)
        );
        assert_eq!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('q'), KeyModifiers::NONE)
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('?'), KeyModifiers::NONE)
            ),
            Some(Action::ToggleHelp)
        );
    }

    #[test]
    fn test_default_interactive_mode() {
        let mut kb = default_keybindings();

        assert_eq!(
            kb.dispatch(
                KeybindingMode::Interactive,
                make_key_event(KeyCode::Esc, KeyModifiers::NONE)
            ),
            Some(Action::ExitInteractiveMode)
        );
        assert_eq!(
            kb.dispatch(
                KeybindingMode::Interactive,
                make_key_event(KeyCode::Tab, KeyModifiers::NONE)
            ),
            Some(Action::InteractiveNextLink)
        );
    }

    #[test]
    fn test_all_modes_have_bindings() {
        let kb = default_keybindings();

        let modes = [
            KeybindingMode::Normal,
            KeybindingMode::Help,
            KeybindingMode::ThemePicker,
            KeybindingMode::Interactive,
            KeybindingMode::InteractiveTable,
            KeybindingMode::LinkFollow,
            KeybindingMode::LinkSearch,
            KeybindingMode::Search,
            KeybindingMode::DocSearch,
            KeybindingMode::CommandPalette,
            KeybindingMode::ConfirmDialog,
            KeybindingMode::CellEdit,
        ];

        for mode in modes {
            assert!(
                kb.get_mode_keybinds(mode).is_some(),
                "Mode {:?} has no bindings",
                mode
            );
            assert!(
                !kb.get_mode_keybinds(mode).unwrap().as_slice().is_empty(),
                "Mode {:?} has empty bindings",
                mode
            );
        }
    }
}
