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
        key: &'static str,
        desc: &'static str,
    },
    Note(&'static str),
    Blank,
}

impl HelpLine {
    /// Convert this help line to a styled ratatui Line
    pub fn to_line(&self, theme: &Theme) -> Line<'static> {
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
            HelpLine::KeyBinding { key, desc } => {
                let formatted_key = format!("  {:<width$}", key, width = KEY_COLUMN_WIDTH);
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

const fn keybinding(key: &'static str, desc: &'static str) -> HelpLine {
    HelpLine::KeyBinding { key, desc }
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
    keybinding("j/↓", "Move down"),
    keybinding("k/↑", "Move up"),
    keybinding("g", "Jump to top"),
    keybinding("G", "Jump to bottom"),
    keybinding("p", "Jump to parent heading"),
    keybinding("d", "Page down (content)"),
    keybinding("u", "Page up (content)"),
    blank(),
    // Tree Operations
    section("Tree Operations"),
    keybinding("Enter/Space", "Toggle expand/collapse"),
    keybinding("l/→", "Expand heading"),
    keybinding("h/←", "Collapse (or parent if no children)"),
    blank(),
    // General
    section("General"),
    keybinding("Tab", "Switch between Outline and Content"),
    keybinding("/", "Search/filter headings"),
    keybinding("r", "Toggle raw source view"),
    keybinding("?", "Toggle this help"),
    keybinding("q/Esc", "Quit"),
    blank(),
    // UX Features
    section("UX Features"),
    keybinding("w", "Toggle outline visibility (full-width content)"),
    keybinding("[ ]", "Decrease/increase outline width (20%, 30%, 40%)"),
    keybinding("1-9", "Jump to heading 1-9"),
    keybinding("m", "Set bookmark (shows ⚑ indicator)"),
    keybinding("'", "Jump to bookmarked position"),
    blank(),
    // Link Following
    section("Link Following"),
    keybinding("f", "Enter link follow mode"),
    keybinding("Tab", "Cycle through links (in link mode)"),
    keybinding("1-9", "Jump to link by number (in link mode)"),
    keybinding("Enter", "Follow selected link (in link mode)"),
    keybinding("p", "Jump to parent's links (stay in link mode)"),
    keybinding("b/Bksp", "Go back to previous file"),
    keybinding("F", "Go forward in navigation history"),
    blank(),
    // Interactive Mode
    section("Interactive Mode"),
    keybinding("i", "Enter interactive mode (navigate elements)"),
    keybinding("Tab/j/k", "Next element | Shift+Tab/k to go back"),
    keybinding("u/d", "Page up/down (PgUp/PgDn also work)"),
    keybinding("Enter", "Activate element (toggle/follow/edit)"),
    keybinding("Space", "Toggle checkboxes/details blocks"),
    keybinding("y", "Copy element (code/cell/link)"),
    keybinding("hjkl", "Navigate table cells (in table mode)"),
    keybinding("Enter", "Edit table cell (in table mode)"),
    keybinding("Esc", "Exit interactive mode"),
    blank(),
    // Themes & Clipboard
    section("Themes & Clipboard"),
    keybinding("t", "Cycle color theme"),
    keybinding("y", "Copy current section content (works in all modes)"),
    keybinding("Y", "Copy anchor link (works in all modes)"),
    keybinding("e", "Edit file in default editor ($VISUAL or $EDITOR)"),
    blank(),
    // Note
    note("On Linux, install a clipboard manager (clipit, parcellite, xclip) for best results"),
    blank(),
    // Footer
    description("Use j/k or ↓/↑ to scroll | Press Esc or ? to close"),
];

/// Build the help text with theme colors applied
pub fn build_help_text(theme: &Theme) -> Vec<Line<'static>> {
    HELP_LINES.iter().map(|line| line.to_line(theme)).collect()
}
