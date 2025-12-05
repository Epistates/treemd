//! Help text generation from keybindings
//!
//! This module generates help text dynamically from the keybindings configuration,
//! ensuring that help always reflects the actual key mappings.

use crate::keybindings::{Action, KeybindingMode, Keybindings};
use crate::tui::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Key column width for keybindings
const KEY_COLUMN_WIDTH: usize = 11;

/// Build dynamic help text from keybindings configuration
pub fn build_dynamic_help_text(theme: &Theme, keybindings: &Keybindings) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Title and instructions
    lines.push(styled_title("treemd - Keyboard Shortcuts", theme));
    lines.push(styled_description(
        "Use j/k or ↓/↑ to scroll | Press Esc or ? to close",
        theme,
    ));
    lines.push(Line::from(""));

    // Normal mode keybindings
    add_mode_section(&mut lines, theme, keybindings, KeybindingMode::Normal, &[
        ("Navigation", &[
            Action::Next, Action::Previous, Action::First, Action::Last,
            Action::JumpToParent, Action::PageDown, Action::PageUp,
        ]),
        ("Tree Operations", &[
            Action::ToggleExpand, Action::Expand, Action::Collapse,
        ]),
        ("General", &[
            Action::ToggleFocus, Action::EnterSearchMode, Action::ToggleRawSource,
            Action::ToggleHelp, Action::Quit,
        ]),
        ("UX Features", &[
            Action::ToggleOutline, Action::OutlineWidthDecrease, Action::OutlineWidthIncrease,
            Action::JumpToHeading1, Action::SetBookmark, Action::JumpToBookmark,
        ]),
    ]);

    // Link following
    lines.push(styled_section("Link Following", theme));
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::EnterLinkFollowMode);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::LinkFollow, Action::NextLink);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::LinkFollow, Action::JumpToLink1);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::LinkFollow, Action::FollowLink);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::LinkFollow, Action::JumpToParent);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::GoBack);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::GoForward);
    lines.push(Line::from(""));

    // Interactive mode
    lines.push(styled_section("Interactive Mode", theme));
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::EnterInteractiveMode);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Interactive, Action::InteractiveNext);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Interactive, Action::PageUp);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Interactive, Action::InteractiveActivate);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Interactive, Action::CopyContent);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Interactive, Action::ExitInteractiveMode);
    lines.push(Line::from(""));

    // Table navigation
    lines.push(styled_section("Table Navigation", theme));
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::InteractiveTable, Action::InteractiveLeft);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::InteractiveTable, Action::InteractiveNext);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::InteractiveTable, Action::InteractiveActivate);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::InteractiveTable, Action::ExitMode);
    lines.push(Line::from(""));

    // Themes & Clipboard
    lines.push(styled_section("Themes & Clipboard", theme));
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::ToggleThemePicker);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::CopyContent);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::CopyAnchor);
    add_keybinding_line(&mut lines, theme, keybindings, KeybindingMode::Normal, Action::OpenInEditor);
    lines.push(Line::from(""));

    // Note
    lines.push(styled_note(
        "On Linux, install a clipboard manager (clipit, parcellite, xclip) for best results",
        theme,
    ));
    lines.push(Line::from(""));

    // Footer
    lines.push(styled_description(
        "Use j/k or ↓/↑ to scroll | Press Esc or ? to close",
        theme,
    ));

    lines
}

fn styled_title(text: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        text.to_string(),
        Style::default()
            .fg(theme.modal_title())
            .add_modifier(Modifier::BOLD),
    )])
}

fn styled_description(text: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        text.to_string(),
        Style::default()
            .fg(theme.modal_description())
            .add_modifier(Modifier::ITALIC),
    )])
}

fn styled_section(text: &str, _theme: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        text.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )])
}

fn styled_note(text: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "Note: ".to_string(),
            Style::default()
                .fg(theme.modal_selected_marker())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(text.to_string(), Style::default().fg(theme.modal_description())),
    ])
}

fn styled_keybinding(key: &str, desc: &str, theme: &Theme) -> Line<'static> {
    let formatted_key = format!("  {:<width$}", key, width = KEY_COLUMN_WIDTH);
    Line::from(vec![
        Span::styled(formatted_key, Style::default().fg(theme.modal_key_fg())),
        Span::raw(desc.to_string()),
    ])
}

fn add_mode_section(
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    keybindings: &Keybindings,
    mode: KeybindingMode,
    sections: &[(&str, &[Action])],
) {
    for (section_name, actions) in sections {
        lines.push(styled_section(section_name, theme));
        for action in *actions {
            add_keybinding_line(lines, theme, keybindings, mode, *action);
        }
        lines.push(Line::from(""));
    }
}

fn add_keybinding_line(
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    keybindings: &Keybindings,
    mode: KeybindingMode,
    action: Action,
) {
    let keys = keybindings.keys_for_action(mode, action);
    if keys.is_empty() {
        return;
    }

    // Format keys, limiting to first few to avoid long strings
    let key_strs: Vec<&str> = keys.iter().take(3).map(|s| s.as_str()).collect();
    let key_display = if keys.len() > 3 {
        format!("{}/...", key_strs.join("/"))
    } else {
        key_strs.join("/")
    };

    lines.push(styled_keybinding(&key_display, action.description(), theme));
}
