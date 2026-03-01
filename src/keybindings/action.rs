//! Action definitions for keybindings
//!
//! This module defines all bindable actions in treemd.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// All bindable actions in treemd
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumIter, EnumString,
)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub enum Action {
    // === Navigation ===
    /// Move to next item in outline/list
    Next,
    /// Move to previous item in outline/list
    Previous,
    /// Jump to first item
    First,
    /// Jump to last item
    Last,
    /// Scroll down by page
    PageDown,
    /// Scroll up by page
    PageUp,
    /// Jump to parent heading in outline
    JumpToParent,

    // === Outline ===
    /// Expand collapsed heading
    Expand,
    /// Collapse expanded heading
    Collapse,
    /// Toggle expand/collapse state
    ToggleExpand,
    /// Toggle focus between outline and content
    ToggleFocus,
    /// Toggle focus backwards (Shift+Tab)
    ToggleFocusBack,
    /// Toggle outline visibility
    ToggleOutline,
    /// Increase outline width
    OutlineWidthIncrease,
    /// Decrease outline width
    OutlineWidthDecrease,
    /// Toggle filtering outline by open todos
    ToggleTodoFilter,

    // === Bookmarks ===
    /// Set bookmark at current position
    SetBookmark,
    /// Jump to bookmarked position
    JumpToBookmark,

    // === Mode Transitions ===
    /// Enter interactive element navigation mode
    EnterInteractiveMode,
    /// Exit interactive mode
    ExitInteractiveMode,
    /// Enter link following mode
    EnterLinkFollowMode,
    /// Enter search/filter mode
    EnterSearchMode,
    /// Enter document search mode
    EnterDocSearch,
    /// Toggle between outline and document search modes
    ToggleSearchMode,
    /// Exit current mode (generic escape)
    ExitMode,

    // === Link Navigation ===
    /// Move to next link
    NextLink,
    /// Move to previous link
    PreviousLink,
    /// Follow/activate the selected link
    FollowLink,
    /// Start link search/filter
    LinkSearch,

    // === Interactive Mode ===
    /// Move to next interactive element
    InteractiveNext,
    /// Move to previous interactive element
    InteractivePrevious,
    /// Activate/toggle the selected element
    InteractiveActivate,
    /// Move to next link within element
    InteractiveNextLink,
    /// Move to previous link within element
    InteractivePreviousLink,
    /// Navigate left in table
    InteractiveLeft,
    /// Navigate right in table
    InteractiveRight,

    // === View ===
    /// Toggle raw markdown source view
    ToggleRawSource,
    /// Toggle help popup
    ToggleHelp,
    /// Toggle theme picker
    ToggleThemePicker,
    /// Apply selected theme (in theme picker)
    ApplyTheme,

    // === Clipboard ===
    /// Copy current section content
    CopyContent,
    /// Copy anchor/heading text
    CopyAnchor,

    // === File Operations ===
    /// Navigate back in file history
    GoBack,
    /// Navigate forward in file history
    GoForward,
    /// Open current file in external editor
    OpenInEditor,
    /// Undo last table cell edit
    UndoEdit,
    /// Open file picker to switch between markdown files
    OpenFilePicker,
    /// Navigate to parent directory in file picker
    ParentDirectory,

    // === Dialog Actions ===
    /// Confirm action in dialog
    ConfirmAction,
    /// Cancel action in dialog
    CancelAction,
    /// Discard changes and quit (for unsaved changes dialogs)
    DiscardAndQuit,
    /// Discard changes and continue navigation (for unsaved changes dialogs)
    DiscardAndContinue,

    // === Application ===
    /// Quit the application
    Quit,
    /// Redraw the screen
    Redraw,

    // === Jump to Heading by Number ===
    JumpToHeading1,
    JumpToHeading2,
    JumpToHeading3,
    JumpToHeading4,
    JumpToHeading5,
    JumpToHeading6,
    JumpToHeading7,
    JumpToHeading8,
    JumpToHeading9,

    // === Jump to Link by Number ===
    JumpToLink1,
    JumpToLink2,
    JumpToLink3,
    JumpToLink4,
    JumpToLink5,
    JumpToLink6,
    JumpToLink7,
    JumpToLink8,
    JumpToLink9,

    // === Scroll (Content pane) ===
    /// Scroll content down one line
    ScrollDown,
    /// Scroll content up one line
    ScrollUp,

    // === Help Navigation ===
    /// Scroll help popup down
    HelpScrollDown,
    /// Scroll help popup up
    HelpScrollUp,

    // === Theme Picker Navigation ===
    /// Move to next theme in picker
    ThemePickerNext,
    /// Move to previous theme in picker
    ThemePickerPrevious,

    // === Search Input ===
    /// Delete last character in search
    SearchBackspace,

    // === Command Palette ===
    /// Open command palette
    OpenCommandPalette,
    /// Navigate to next command
    CommandPaletteNext,
    /// Navigate to previous command
    CommandPalettePrev,
    /// Autocomplete selected command
    CommandPaletteAutocomplete,

    // === Doc Search Navigation ===
    /// Next search match
    NextMatch,
    /// Previous search match
    PrevMatch,
}

impl Action {
    /// Get a human-readable description of the action
    pub fn description(&self) -> &'static str {
        match self {
            // Navigation
            Action::Next => "Move to next item",
            Action::Previous => "Move to previous item",
            Action::First => "Jump to first item",
            Action::Last => "Jump to last item",
            Action::PageDown => "Page down",
            Action::PageUp => "Page up",
            Action::JumpToParent => "Jump to parent heading",

            // Outline
            Action::Expand => "Expand heading",
            Action::Collapse => "Collapse heading",
            Action::ToggleExpand => "Toggle expand/collapse",
            Action::ToggleFocus => "Switch focus (outline/content)",
            Action::ToggleFocusBack => "Switch focus backwards",
            Action::ToggleOutline => "Toggle outline visibility",
            Action::OutlineWidthIncrease => "Increase outline width",
            Action::OutlineWidthDecrease => "Decrease outline width",
            Action::ToggleTodoFilter => "Filter by open todos",

            // Bookmarks
            Action::SetBookmark => "Set bookmark",
            Action::JumpToBookmark => "Jump to bookmark",

            // Mode transitions
            Action::EnterInteractiveMode => "Enter interactive mode",
            Action::ExitInteractiveMode => "Exit interactive mode",
            Action::EnterLinkFollowMode => "Enter link follow mode",
            Action::EnterSearchMode => "Search/filter headings",
            Action::EnterDocSearch => "Search document content",
            Action::ToggleSearchMode => "Toggle outline/content search",
            Action::ExitMode => "Exit current mode",

            // Link navigation
            Action::NextLink => "Next link",
            Action::PreviousLink => "Previous link",
            Action::FollowLink => "Follow link",
            Action::LinkSearch => "Search links",

            // Interactive mode
            Action::InteractiveNext => "Next element",
            Action::InteractivePrevious => "Previous element",
            Action::InteractiveActivate => "Activate element",
            Action::InteractiveNextLink => "Next link in element",
            Action::InteractivePreviousLink => "Previous link in element",
            Action::InteractiveLeft => "Navigate left (table)",
            Action::InteractiveRight => "Navigate right (table)",

            // View
            Action::ToggleRawSource => "Toggle raw source view",
            Action::ToggleHelp => "Toggle help",
            Action::ToggleThemePicker => "Open theme picker",
            Action::ApplyTheme => "Apply selected theme",

            // Clipboard
            Action::CopyContent => "Copy content",
            Action::CopyAnchor => "Copy heading/anchor",

            // File operations
            Action::GoBack => "Go back",
            Action::GoForward => "Go forward",
            Action::OpenInEditor => "Open in editor",
            Action::UndoEdit => "Undo last edit",
            Action::OpenFilePicker => "Open file picker",
            Action::ParentDirectory => "Go to parent directory",

            // Dialog
            Action::ConfirmAction => "Confirm",
            Action::CancelAction => "Cancel",
            Action::DiscardAndQuit => "Discard changes and quit",
            Action::DiscardAndContinue => "Discard changes and continue",

            // Application
            Action::Quit => "Quit",
            Action::Redraw => "Redraw screen",

            // Jump to heading
            Action::JumpToHeading1 => "Jump to heading 1",
            Action::JumpToHeading2 => "Jump to heading 2",
            Action::JumpToHeading3 => "Jump to heading 3",
            Action::JumpToHeading4 => "Jump to heading 4",
            Action::JumpToHeading5 => "Jump to heading 5",
            Action::JumpToHeading6 => "Jump to heading 6",
            Action::JumpToHeading7 => "Jump to heading 7",
            Action::JumpToHeading8 => "Jump to heading 8",
            Action::JumpToHeading9 => "Jump to heading 9",

            // Jump to link
            Action::JumpToLink1 => "Jump to link 1",
            Action::JumpToLink2 => "Jump to link 2",
            Action::JumpToLink3 => "Jump to link 3",
            Action::JumpToLink4 => "Jump to link 4",
            Action::JumpToLink5 => "Jump to link 5",
            Action::JumpToLink6 => "Jump to link 6",
            Action::JumpToLink7 => "Jump to link 7",
            Action::JumpToLink8 => "Jump to link 8",
            Action::JumpToLink9 => "Jump to link 9",

            // Scroll
            Action::ScrollDown => "Scroll down",
            Action::ScrollUp => "Scroll up",

            // Help navigation
            Action::HelpScrollDown => "Scroll help down",
            Action::HelpScrollUp => "Scroll help up",

            // Theme picker
            Action::ThemePickerNext => "Next theme",
            Action::ThemePickerPrevious => "Previous theme",

            // Search
            Action::SearchBackspace => "Delete character",

            // Command palette
            Action::OpenCommandPalette => "Open command palette",
            Action::CommandPaletteNext => "Next command",
            Action::CommandPalettePrev => "Previous command",
            Action::CommandPaletteAutocomplete => "Autocomplete command",

            // Doc search
            Action::NextMatch => "Next search match",
            Action::PrevMatch => "Previous search match",
        }
    }

    /// Get the category for grouping in help display
    pub fn category(&self) -> &'static str {
        match self {
            Action::Next
            | Action::Previous
            | Action::First
            | Action::Last
            | Action::PageDown
            | Action::PageUp
            | Action::JumpToParent => "Navigation",

            Action::Expand
            | Action::Collapse
            | Action::ToggleExpand
            | Action::ToggleFocus
            | Action::ToggleFocusBack
            | Action::ToggleOutline
            | Action::OutlineWidthIncrease
            | Action::OutlineWidthDecrease
            | Action::ToggleTodoFilter => "Outline",

            Action::SetBookmark | Action::JumpToBookmark => "Bookmarks",

            Action::EnterInteractiveMode
            | Action::ExitInteractiveMode
            | Action::EnterLinkFollowMode
            | Action::EnterSearchMode
            | Action::EnterDocSearch
            | Action::ToggleSearchMode
            | Action::ExitMode => "Modes",

            Action::NextLink | Action::PreviousLink | Action::FollowLink | Action::LinkSearch => {
                "Links"
            }

            Action::InteractiveNext
            | Action::InteractivePrevious
            | Action::InteractiveActivate
            | Action::InteractiveNextLink
            | Action::InteractivePreviousLink
            | Action::InteractiveLeft
            | Action::InteractiveRight => "Interactive",

            Action::ToggleRawSource
            | Action::ToggleHelp
            | Action::ToggleThemePicker
            | Action::ApplyTheme => "View",

            Action::CopyContent | Action::CopyAnchor => "Clipboard",

            Action::GoBack
            | Action::GoForward
            | Action::OpenInEditor
            | Action::UndoEdit
            | Action::OpenFilePicker
            | Action::ParentDirectory => "Files",

            Action::ConfirmAction
            | Action::CancelAction
            | Action::DiscardAndQuit
            | Action::DiscardAndContinue => "Dialog",

            Action::Quit | Action::Redraw => "Application",

            Action::JumpToHeading1
            | Action::JumpToHeading2
            | Action::JumpToHeading3
            | Action::JumpToHeading4
            | Action::JumpToHeading5
            | Action::JumpToHeading6
            | Action::JumpToHeading7
            | Action::JumpToHeading8
            | Action::JumpToHeading9 => "Jump to Heading",

            Action::JumpToLink1
            | Action::JumpToLink2
            | Action::JumpToLink3
            | Action::JumpToLink4
            | Action::JumpToLink5
            | Action::JumpToLink6
            | Action::JumpToLink7
            | Action::JumpToLink8
            | Action::JumpToLink9 => "Jump to Link",

            Action::ScrollDown | Action::ScrollUp => "Scroll",

            Action::HelpScrollDown | Action::HelpScrollUp => "Help",

            Action::ThemePickerNext | Action::ThemePickerPrevious => "Theme Picker",

            Action::SearchBackspace | Action::NextMatch | Action::PrevMatch => "Search",

            Action::OpenCommandPalette
            | Action::CommandPaletteNext
            | Action::CommandPalettePrev
            | Action::CommandPaletteAutocomplete => "Command Palette",
        }
    }
}
