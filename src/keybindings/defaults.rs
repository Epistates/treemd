//! Default keybindings for treemd
//!
//! This module defines the default keybindings that are used when no
//! user configuration is provided.

use super::{Action, KeyBinding, KeybindingMode, Keybindings};
use crossterm::event::KeyCode;

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

    // Confirm dialog mode
    add_confirm_dialog_mode(&mut kb);

    kb
}

fn add_normal_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Normal;

    // Navigation
    kb.set(Normal, KeyBinding::key(KeyCode::Char('j')), Next);
    kb.set(Normal, KeyBinding::key(KeyCode::Down), Next);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('k')), Previous);
    kb.set(Normal, KeyBinding::key(KeyCode::Up), Previous);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('g')), First);
    kb.set(Normal, KeyBinding::shift(KeyCode::Char('G')), Last);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('d')), PageDown);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('u')), PageUp);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('p')), JumpToParent);

    // Outline
    kb.set(Normal, KeyBinding::key(KeyCode::Enter), ToggleExpand);
    kb.set(Normal, KeyBinding::key(KeyCode::Char(' ')), ToggleExpand);
    kb.set(Normal, KeyBinding::key(KeyCode::Tab), ToggleFocus);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('h')), Collapse);
    kb.set(Normal, KeyBinding::key(KeyCode::Left), Collapse);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('l')), Expand);
    kb.set(Normal, KeyBinding::key(KeyCode::Right), Expand);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('w')), ToggleOutline);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('[')), OutlineWidthDecrease);
    kb.set(Normal, KeyBinding::key(KeyCode::Char(']')), OutlineWidthIncrease);

    // Bookmarks
    kb.set(Normal, KeyBinding::key(KeyCode::Char('m')), SetBookmark);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('\'')), JumpToBookmark);

    // Mode transitions
    kb.set(Normal, KeyBinding::key(KeyCode::Char('i')), EnterInteractiveMode);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('f')), EnterLinkFollowMode);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('/')), EnterSearchMode);

    // View
    kb.set(Normal, KeyBinding::key(KeyCode::Char('r')), ToggleRawSource);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('t')), ToggleThemePicker);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('?')), ToggleHelp);

    // Clipboard
    kb.set(Normal, KeyBinding::key(KeyCode::Char('y')), CopyContent);
    kb.set(Normal, KeyBinding::shift(KeyCode::Char('Y')), CopyAnchor);

    // File operations
    kb.set(Normal, KeyBinding::key(KeyCode::Char('b')), GoBack);
    kb.set(Normal, KeyBinding::key(KeyCode::Backspace), GoBack);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('e')), OpenInEditor);

    // Application
    kb.set(Normal, KeyBinding::key(KeyCode::Char('q')), Quit);
    kb.set(Normal, KeyBinding::key(KeyCode::Esc), Quit);

    // Jump to heading by number
    kb.set(Normal, KeyBinding::key(KeyCode::Char('1')), JumpToHeading1);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('2')), JumpToHeading2);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('3')), JumpToHeading3);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('4')), JumpToHeading4);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('5')), JumpToHeading5);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('6')), JumpToHeading6);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('7')), JumpToHeading7);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('8')), JumpToHeading8);
    kb.set(Normal, KeyBinding::key(KeyCode::Char('9')), JumpToHeading9);
}

fn add_help_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Help;

    // Navigation
    kb.set(Help, KeyBinding::key(KeyCode::Char('j')), HelpScrollDown);
    kb.set(Help, KeyBinding::key(KeyCode::Down), HelpScrollDown);
    kb.set(Help, KeyBinding::key(KeyCode::Char('k')), HelpScrollUp);
    kb.set(Help, KeyBinding::key(KeyCode::Up), HelpScrollUp);

    // Close help
    kb.set(Help, KeyBinding::key(KeyCode::Char('?')), ToggleHelp);
    kb.set(Help, KeyBinding::key(KeyCode::Esc), ToggleHelp);

    // Clipboard (available everywhere)
    kb.set(Help, KeyBinding::key(KeyCode::Char('y')), CopyContent);
    kb.set(Help, KeyBinding::shift(KeyCode::Char('Y')), CopyAnchor);

    // Quit
    kb.set(Help, KeyBinding::key(KeyCode::Char('q')), Quit);
}

fn add_theme_picker_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::ThemePicker;

    // Navigation
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Char('j')), ThemePickerNext);
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Down), ThemePickerNext);
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Char('k')), ThemePickerPrevious);
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Up), ThemePickerPrevious);

    // Actions
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Enter), ApplyTheme);
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Esc), ToggleThemePicker);

    // Clipboard (available everywhere)
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Char('y')), CopyContent);
    kb.set(ThemePicker, KeyBinding::shift(KeyCode::Char('Y')), CopyAnchor);

    // Quit
    kb.set(ThemePicker, KeyBinding::key(KeyCode::Char('q')), Quit);
}

fn add_interactive_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Interactive;

    // Exit
    kb.set(Interactive, KeyBinding::key(KeyCode::Esc), ExitInteractiveMode);
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('i')), ExitInteractiveMode);

    // Navigation
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('j')), InteractiveNext);
    kb.set(Interactive, KeyBinding::key(KeyCode::Down), InteractiveNext);
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('k')), InteractivePrevious);
    kb.set(Interactive, KeyBinding::key(KeyCode::Up), InteractivePrevious);

    // Link navigation within element
    kb.set(Interactive, KeyBinding::key(KeyCode::Tab), InteractiveNextLink);
    kb.set(Interactive, KeyBinding::key(KeyCode::BackTab), InteractivePreviousLink);

    // Activate element
    kb.set(Interactive, KeyBinding::key(KeyCode::Enter), InteractiveActivate);
    kb.set(Interactive, KeyBinding::key(KeyCode::Char(' ')), InteractiveActivate);

    // Page navigation
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('d')), PageDown);
    kb.set(Interactive, KeyBinding::key(KeyCode::PageDown), PageDown);
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('u')), PageUp);
    kb.set(Interactive, KeyBinding::key(KeyCode::PageUp), PageUp);

    // Clipboard
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('y')), CopyContent);

    // Quit
    kb.set(Interactive, KeyBinding::key(KeyCode::Char('q')), Quit);
}

fn add_interactive_table_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::InteractiveTable;

    // Exit table mode
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Esc), ExitMode);

    // Table navigation
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('h')), InteractiveLeft);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Left), InteractiveLeft);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('l')), InteractiveRight);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Right), InteractiveRight);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('j')), InteractiveNext);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Down), InteractiveNext);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('k')), InteractivePrevious);
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Up), InteractivePrevious);

    // Clipboard
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('y')), CopyContent);
    kb.set(InteractiveTable, KeyBinding::shift(KeyCode::Char('Y')), CopyAnchor);

    // View toggle
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('r')), ToggleRawSource);

    // Activate (follow link or edit cell)
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Enter), InteractiveActivate);

    // Quit
    kb.set(InteractiveTable, KeyBinding::key(KeyCode::Char('q')), Quit);
}

fn add_link_follow_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::LinkFollow;

    // Exit
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Esc), ExitMode);

    // Navigation
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('j')), NextLink);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Down), NextLink);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Tab), NextLink);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('k')), PreviousLink);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Up), PreviousLink);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::BackTab), PreviousLink);

    // Actions
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Enter), FollowLink);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('/')), LinkSearch);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('p')), JumpToParent);

    // Jump to link by number
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('1')), JumpToLink1);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('2')), JumpToLink2);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('3')), JumpToLink3);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('4')), JumpToLink4);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('5')), JumpToLink5);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('6')), JumpToLink6);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('7')), JumpToLink7);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('8')), JumpToLink8);
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('9')), JumpToLink9);

    // Clipboard
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('y')), CopyContent);
    kb.set(LinkFollow, KeyBinding::shift(KeyCode::Char('Y')), CopyAnchor);

    // Quit
    kb.set(LinkFollow, KeyBinding::key(KeyCode::Char('q')), Quit);
}

fn add_link_search_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::LinkSearch;

    // Exit search (back to link follow)
    kb.set(LinkSearch, KeyBinding::key(KeyCode::Esc), ExitMode);

    // Select filtered result
    kb.set(LinkSearch, KeyBinding::key(KeyCode::Enter), FollowLink);

    // Navigation while searching
    kb.set(LinkSearch, KeyBinding::key(KeyCode::Down), NextLink);
    kb.set(LinkSearch, KeyBinding::key(KeyCode::Up), PreviousLink);

    // Delete character
    kb.set(LinkSearch, KeyBinding::key(KeyCode::Backspace), SearchBackspace);
}

fn add_search_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::Search;

    // Exit search
    kb.set(Search, KeyBinding::key(KeyCode::Esc), ExitMode);

    // Confirm search (select result)
    kb.set(Search, KeyBinding::key(KeyCode::Enter), ConfirmAction);

    // Delete character
    kb.set(Search, KeyBinding::key(KeyCode::Backspace), SearchBackspace);
}

fn add_confirm_dialog_mode(kb: &mut Keybindings) {
    use Action::*;
    use KeybindingMode::ConfirmDialog;

    // Confirm
    kb.set(ConfirmDialog, KeyBinding::key(KeyCode::Char('y')), ConfirmAction);
    kb.set(ConfirmDialog, KeyBinding::shift(KeyCode::Char('Y')), ConfirmAction);
    kb.set(ConfirmDialog, KeyBinding::key(KeyCode::Enter), ConfirmAction);

    // Cancel
    kb.set(ConfirmDialog, KeyBinding::key(KeyCode::Char('n')), CancelAction);
    kb.set(ConfirmDialog, KeyBinding::shift(KeyCode::Char('N')), CancelAction);
    kb.set(ConfirmDialog, KeyBinding::key(KeyCode::Esc), CancelAction);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_default_normal_mode() {
        let kb = default_keybindings();

        // Check some common bindings
        assert_eq!(
            kb.get_action(KeybindingMode::Normal, KeyCode::Char('j'), KeyModifiers::NONE),
            Some(Action::Next)
        );
        assert_eq!(
            kb.get_action(KeybindingMode::Normal, KeyCode::Char('k'), KeyModifiers::NONE),
            Some(Action::Previous)
        );
        assert_eq!(
            kb.get_action(KeybindingMode::Normal, KeyCode::Char('q'), KeyModifiers::NONE),
            Some(Action::Quit)
        );
        assert_eq!(
            kb.get_action(KeybindingMode::Normal, KeyCode::Char('?'), KeyModifiers::NONE),
            Some(Action::ToggleHelp)
        );
    }

    #[test]
    fn test_default_interactive_mode() {
        let kb = default_keybindings();

        assert_eq!(
            kb.get_action(KeybindingMode::Interactive, KeyCode::Esc, KeyModifiers::NONE),
            Some(Action::ExitInteractiveMode)
        );
        assert_eq!(
            kb.get_action(KeybindingMode::Interactive, KeyCode::Tab, KeyModifiers::NONE),
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
            KeybindingMode::ConfirmDialog,
        ];

        for mode in modes {
            assert!(
                kb.get_mode_bindings(mode).is_some(),
                "Mode {:?} has no bindings",
                mode
            );
            assert!(
                !kb.get_mode_bindings(mode).unwrap().is_empty(),
                "Mode {:?} has empty bindings",
                mode
            );
        }
    }
}
