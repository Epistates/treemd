use crate::keybindings::{
    Action::{self, *},
    KeybindingMode::{self, *},
    Keybindings,
};
use crate::tui::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Key column width for keybindings
const KEY_COLUMN_WIDTH: usize = 11;

#[derive(Debug, Clone, Copy)]
pub enum HelpLine {
    Title(&'static str),
    Description(&'static str),
    SectionHeader(&'static str),
    KeyBinding {
        prefix: &'static str,
        mode: KeybindingMode,
        actions: &'static [Action],
        desc: &'static str,
    },
    Note(&'static str),
    Blank,
}

impl HelpLine {
    fn format_action_keys(
        keybindings: &Keybindings,
        mode: KeybindingMode,
        actions: &'static [Action],
    ) -> String {
        actions
            .iter()
            .filter_map(|action| {
                keybindings
                    .keys_for_action(mode, *action)
                    .into_iter()
                    .next()
            })
            .collect::<Vec<String>>()
            .join("/")
    }

    /// Convert this help line to a styled ratatui Line
    pub fn to_line(self, keybindings: &Keybindings, theme: &Theme) -> Line<'static> {
        match self {
            HelpLine::Title(text) => Line::from(vec![Span::styled(
                text.to_string(),
                Style::default()
                    .fg(theme.modal_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            HelpLine::Description(text) => Line::from(vec![Span::styled(
                text.to_string(),
                Style::default()
                    .fg(theme.modal_description())
                    .add_modifier(Modifier::ITALIC),
            )]),
            HelpLine::SectionHeader(text) => Line::from(vec![Span::styled(
                text.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            HelpLine::KeyBinding {
                prefix,
                mode,
                actions,
                desc,
            } => {
                let key = Self::format_action_keys(keybindings, mode, actions);
                let key_width = KEY_COLUMN_WIDTH.saturating_sub(prefix.len());
                let formatted_key = format!("  {}{:<width$}", prefix, key, width = key_width);
                Line::from(vec![
                    Span::styled(formatted_key, Style::default().fg(theme.modal_key_fg())),
                    Span::raw(desc.to_string()),
                ])
            }
            HelpLine::Note(text) => Line::from(vec![
                Span::styled(
                    "Note: ".to_string(),
                    Style::default()
                        .fg(theme.modal_selected_marker())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    text.to_string(),
                    Style::default().fg(theme.modal_description()),
                ),
            ]),
            HelpLine::Blank => Line::from(""),
        }
    }
}

const fn title(text: &'static str) -> HelpLine {
    HelpLine::Title(text)
}

const fn description(text: &'static str) -> HelpLine {
    HelpLine::Description(text)
}

const fn section(text: &'static str) -> HelpLine {
    HelpLine::SectionHeader(text)
}

const fn keybinding(
    mode: KeybindingMode,
    actions: &'static [Action],
    desc: &'static str,
) -> HelpLine {
    HelpLine::KeyBinding {
        prefix: "",
        mode,
        actions,
        desc,
    }
}

const fn prefixed_keybinding(
    prefix: &'static str,
    mode: KeybindingMode,
    actions: &'static [Action],
    desc: &'static str,
) -> HelpLine {
    HelpLine::KeyBinding {
        prefix,
        mode,
        actions,
        desc,
    }
}

const fn note(text: &'static str) -> HelpLine {
    HelpLine::Note(text)
}

const fn blank() -> HelpLine {
    HelpLine::Blank
}

pub const HELP_LINES: &[HelpLine] = &[
    // Title and instructions
    title("treemd - Keyboard Shortcuts"),
    description("Use j/k or ↓/↑ to scroll | Press Esc or ? to close"),
    blank(),
    // Navigation section
    section("Navigation"),
    keybinding(Normal, &[Next], "Move down"),
    keybinding(Normal, &[Previous], "Move up"),
    keybinding(Normal, &[First], "Jump to top"),
    keybinding(Normal, &[Last], "Jump to bottom"),
    keybinding(Normal, &[JumpToParent], "Jump to parent heading"),
    keybinding(Normal, &[PageDown], "Page down (content)"),
    keybinding(Normal, &[PageUp], "Page up (content)"),
    blank(),
    // Tree Operations
    section("Tree Operations"),
    keybinding(Normal, &[ToggleExpand], "Toggle expand/collapse"),
    keybinding(Normal, &[Expand], "Expand heading"),
    keybinding(Normal, &[Collapse], "Collapse (or parent if no children)"),
    blank(),
    // General
    section("General"),
    keybinding(Normal, &[ToggleFocus], "Switch between Outline and Content"),
    keybinding(Normal, &[EnterDocSearch], "Search document content"),
    keybinding(Normal, &[EnterSearchMode], "Filter outline headings"),
    keybinding(Search, &[ExitMode], "Clear search"),
    keybinding(Search, &[ConfirmAction], "Confirm search"),
    keybinding(
        Normal,
        &[NextMatch, PrevMatch],
        "Next/previous search match",
    ),
    keybinding(Normal, &[OpenFilePicker], "Open file picker"),
    keybinding(Normal, &[ToggleRawSource], "Toggle raw source view"),
    keybinding(Normal, &[ToggleHelp], "Toggle this help"),
    keybinding(Normal, &[Quit], "Quit"),
    blank(),
    // UX Features
    section("UX Features"),
    keybinding(
        Normal,
        &[ToggleOutline],
        "Toggle outline visibility (full-width content)",
    ),
    keybinding(
        Normal,
        &[OutlineWidthDecrease, OutlineWidthIncrease],
        "Decrease/increase outline width (20%, 30%, 40%)",
    ),
    keybinding(
        Normal,
        &[OpenCommandPalette],
        "Open command palette (fuzzy search commands)",
    ),
    prefixed_keybinding(
        "[N]",
        Normal,
        &[Next, Previous],
        "Move N items (vim count prefix, e.g., 5j)",
    ),
    keybinding(
        Normal,
        &[ToggleHeadingMarkers],
        "Toggle heading markers in outline",
    ),
    keybinding(Normal, &[SetBookmark], "Set bookmark (shows ⚑ indicator)"),
    keybinding(Normal, &[JumpToBookmark], "Jump to bookmarked position"),
    blank(),
    // Link Following
    section("Link Following"),
    keybinding(Normal, &[EnterLinkFollowMode], "Enter link follow mode"),
    keybinding(
        LinkFollow,
        &[NextLink],
        "Cycle through links (in link mode)",
    ),
    prefixed_keybinding(
        "[1-9]",
        LinkFollow,
        &[],
        "Jump to link by number (in link mode)",
    ),
    keybinding(
        LinkFollow,
        &[FollowLink],
        "Follow selected link (in link mode)",
    ),
    keybinding(
        LinkFollow,
        &[JumpToParent],
        "Jump to parent's links (stay in link mode)",
    ),
    keybinding(Normal, &[GoBack], "Go back to previous file"),
    keybinding(Normal, &[GoForward], "Go forward in navigation history"),
    blank(),
    // Interactive Mode
    section("Interactive Mode"),
    keybinding(
        Normal,
        &[EnterInteractiveMode],
        "Enter interactive mode (navigate elements)",
    ),
    keybinding(
        Interactive,
        &[InteractiveNext, InteractivePrevious],
        "Next/previous element",
    ),
    keybinding(Interactive, &[PageUp, PageDown], "Page up/down"),
    keybinding(
        Interactive,
        &[InteractiveActivate],
        "Activate element (toggle/follow/edit)",
    ),
    keybinding(Interactive, &[CopyContent], "Copy element (code/cell/link)"),
    keybinding(
        InteractiveTable,
        &[
            InteractiveLeft,
            InteractiveNext,
            InteractivePrevious,
            InteractiveRight,
        ],
        "Navigate table cells (in table mode)",
    ),
    keybinding(
        InteractiveTable,
        &[InteractiveActivate],
        "Edit table cell (in table mode)",
    ),
    keybinding(InteractiveTable, &[ExitMode], "Exit table navigation"),
    blank(),
    // Themes & Clipboard
    section("Themes & Clipboard"),
    keybinding(Normal, &[ToggleThemePicker], "Toggle theme picker"),
    keybinding(
        Normal,
        &[CopyContent],
        "Copy current section content (works in all modes)",
    ),
    keybinding(
        Normal,
        &[CopyAnchor],
        "Copy anchor link (works in all modes)",
    ),
    keybinding(
        Normal,
        &[OpenInEditor],
        "Edit file in default editor ($VISUAL or $EDITOR)",
    ),
    blank(),
    // Note
    note("On Linux, install a clipboard manager (clipit, parcellite, xclip) for best results"),
    blank(),
    // Footer
    description("Use j/k or ↓/↑ to scroll | Press Esc or ? to close"),
];

/// Build the help text with theme colors applied
pub fn build_help_text(keybindings: &Keybindings, theme: &Theme) -> Vec<Line<'static>> {
    HELP_LINES
        .iter()
        .map(|line| line.to_line(keybindings, theme))
        .collect()
}
