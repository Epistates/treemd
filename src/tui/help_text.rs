use crate::keybindings::{
    Action::{self, *},
    KeybindingMode::{self, *},
    Keybindings,
};
use crate::tui::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Key column width for keybindings
const KEY_COLUMN_WIDTH: usize = 14;

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
    /// The keys for `actions`: one action's first two keys (`o C-o`), or
    /// the first key of each of several (`[/]`).
    fn format_action_keys(
        keybindings: &Keybindings,
        mode: KeybindingMode,
        actions: &'static [Action],
    ) -> String {
        if let [action] = actions {
            return keybindings
                .keys_for_action(mode, *action)
                .into_iter()
                .take(2)
                .collect::<Vec<String>>()
                .join(" ");
        }
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
    title("treemd keys"),
    description("Every key can be remapped in the config; see the README."),
    blank(),
    section("Moving around"),
    keybinding(
        Normal,
        &[Next],
        "Next heading, or scroll down in the content",
    ),
    keybinding(Normal, &[Previous], "Previous heading, or scroll up"),
    keybinding(Normal, &[First], "First"),
    keybinding(Normal, &[Last], "Last"),
    keybinding(Normal, &[JumpToParent], "Parent heading"),
    keybinding(Normal, &[PageDown], "Page down"),
    keybinding(Normal, &[PageUp], "Page up"),
    keybinding(Normal, &[ScrollDown, ScrollUp], "Scroll the content a line"),
    keybinding(Normal, &[ToggleFocus], "Switch between outline and content"),
    keybinding(Normal, &[ToggleFocusBack], "Switch back"),
    prefixed_keybinding("[N]", Normal, &[Next, Previous], "Move N at once, as in 5j"),
    blank(),
    section("Outline"),
    keybinding(Normal, &[ToggleExpand], "Fold or unfold a heading"),
    keybinding(Normal, &[Expand], "Unfold"),
    keybinding(Normal, &[Collapse], "Fold, or go to the parent"),
    keybinding(Normal, &[ToggleOutline], "Hide or show the outline"),
    keybinding(
        Normal,
        &[OutlineWidthDecrease, OutlineWidthIncrease],
        "Narrower or wider outline",
    ),
    keybinding(
        Normal,
        &[ToggleHeadingMarkers],
        "Show or hide the # markers",
    ),
    keybinding(Normal, &[ToggleTodoFilter], "Only headings with open todos"),
    keybinding(Normal, &[SetBookmark], "Bookmark this heading"),
    keybinding(Normal, &[JumpToBookmark], "Go to the bookmark"),
    blank(),
    section("Search"),
    keybinding(Normal, &[EnterDocSearch], "Search this section"),
    keybinding(Normal, &[EnterSearchMode], "Filter the outline's headings"),
    keybinding(Normal, &[NextMatch, PrevMatch], "Next or previous match"),
    keybinding(Search, &[ToggleSearchMode], "Switch between the two"),
    keybinding(
        Search,
        &[ConfirmAction],
        "Keep the search and move through it",
    ),
    keybinding(Normal, &[ExitMode], "Clear a search or filter"),
    blank(),
    section("Links"),
    keybinding(Normal, &[EnterLinkFollowMode], "List this section's links"),
    keybinding(
        LinkFollow,
        &[NextLink, PreviousLink],
        "Next or previous link",
    ),
    prefixed_keybinding("[1-9]", LinkFollow, &[], "Link by its number"),
    keybinding(LinkFollow, &[FollowLink], "Follow it"),
    keybinding(LinkFollow, &[Action::LinkSearch], "Search the links"),
    keybinding(LinkFollow, &[JumpToParent], "The parent section's links"),
    keybinding(Normal, &[GoBack], "Back to the previous file"),
    keybinding(Normal, &[GoForward], "Forward again"),
    blank(),
    section("Interactive mode"),
    keybinding(
        Normal,
        &[EnterInteractiveMode],
        "Select checkboxes, links, tables, code",
    ),
    keybinding(
        Interactive,
        &[InteractiveNext, InteractivePrevious],
        "Next or previous element",
    ),
    keybinding(
        Interactive,
        &[InteractiveActivate],
        "Toggle, follow, open or enter it",
    ),
    keybinding(Interactive, &[CopyContent], "Copy it"),
    keybinding(Interactive, &[OpenInEditor], "Edit it in your editor"),
    keybinding(Interactive, &[UndoEdit], "Undo the last table edit"),
    keybinding(
        InteractiveTable,
        &[
            InteractiveLeft,
            InteractiveNext,
            InteractivePrevious,
            InteractiveRight,
        ],
        "Move between cells in a table",
    ),
    keybinding(InteractiveTable, &[InteractiveActivate], "Edit the cell"),
    keybinding(
        InteractiveTable,
        &[CopyTableCell, CopyTableRow, CopyTableMarkdown],
        "Copy the cell, the row, or the table",
    ),
    blank(),
    section("Images"),
    keybinding(
        Interactive,
        &[InteractiveActivate],
        "Open an image full screen",
    ),
    keybinding(ImageViewer, &[TogglePlayback], "Play or pause a GIF"),
    keybinding(
        ImageViewer,
        &[PreviousFrame, NextFrame],
        "Step through its frames",
    ),
    blank(),
    section("View"),
    keybinding(Normal, &[ToggleThemePicker], "Choose a theme"),
    keybinding(Normal, &[ToggleRawSource], "Show the markdown source"),
    keybinding(Normal, &[ToggleMouseCapture], "Let the mouse select text"),
    keybinding(Normal, &[OpenCommandPalette], "Run any command by name"),
    keybinding(Normal, &[Redraw], "Redraw the screen"),
    blank(),
    section("Files and clipboard"),
    keybinding(Normal, &[OpenFilePicker], "Open another file"),
    keybinding(Normal, &[OpenInEditor], "Edit this file in your editor"),
    keybinding(Normal, &[CopyContent], "Copy the section"),
    keybinding(Normal, &[CopyAnchor], "Copy a link to the heading"),
    keybinding(Normal, &[Suspend], "Suspend to the shell (fg returns)"),
    keybinding(Normal, &[ToggleHelp], "This help"),
    keybinding(Normal, &[Quit], "Quit"),
    blank(),
    note("On Linux, a clipboard manager (clipit, parcellite) keeps copies after quitting"),
];

/// Build the help text with theme colors applied
pub fn build_help_text(keybindings: &Keybindings, theme: &Theme) -> Vec<Line<'static>> {
    HELP_LINES
        .iter()
        .map(|line| line.to_line(keybindings, theme))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// Every action the main view binds a key to is in the help. The help
    /// left out the todo filter, line scrolling, and Shift+Tab, among others.
    #[test]
    fn help_lists_every_main_view_key() {
        let keybindings = Keybindings::default();
        let listed: Vec<Action> = HELP_LINES
            .iter()
            .filter_map(|line| match line {
                HelpLine::KeyBinding {
                    mode: Normal,
                    actions,
                    ..
                } => Some(*actions),
                _ => None,
            })
            .flatten()
            .copied()
            .collect();
        let missing: Vec<Action> = Action::iter()
            .filter(|action| !keybindings.keys_for_action(Normal, *action).is_empty())
            .filter(|action| !listed.contains(action))
            .collect();
        assert!(missing.is_empty(), "not in the help: {missing:?}");
    }
}
