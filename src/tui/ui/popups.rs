//! Popup and overlay rendering for the TUI
//!
//! Handles modal dialogs including help, link picker, search, theme selector,
//! and cell edit overlays.

use crate::tui::app::App;
use crate::tui::help_text;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};
use ratatui::Frame;

use super::util::centered_area;

/// Render the help popup with keyboard shortcuts
pub fn render_help_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_area(area, 70, 80);
    let theme = &app.theme;

    // Clear the area
    frame.render_widget(Clear, popup_area);

    let help_lines = help_text::build_help_text(theme);
    let help_text_len = help_lines.len();

    let paragraph = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.modal_border()))
                .title(" Help ")
                .style(Style::default().bg(theme.modal_bg())),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0));

    frame.render_widget(paragraph, popup_area);

    // Render scrollbar for help
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .style(Style::default().fg(theme.modal_border()));

    let mut scrollbar_state = ScrollbarState::new(help_text_len).position(app.help_scroll as usize);

    frame.render_stateful_widget(
        scrollbar,
        popup_area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
}

/// Render the link picker popup
pub fn render_link_picker(frame: &mut Frame, app: &App, area: Rect) {
    use crate::parser::LinkTarget;

    let theme = &app.theme;

    // Create centered popup area (smaller than full screen)
    let popup_area = centered_area(area, 80, 60);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create lines for each link
    let mut lines = vec![
        Line::from(vec![Span::styled(
            format!(
                "Links in this section ({} found) - Tab/j/k to navigate, Enter to follow, Esc to cancel",
                app.links_in_view.len()
            ),
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    for (idx, link) in app.links_in_view.iter().enumerate() {
        let is_selected = app.selected_link_idx == Some(idx);

        // Format link number and text
        let number = format!("[{}] ", idx + 1);
        let link_text = &link.text;

        // Format target
        let target_str = match &link.target {
            LinkTarget::Anchor(a) => format!("#{}", a),
            LinkTarget::RelativeFile { path, anchor } => {
                if let Some(a) = anchor {
                    format!("{}#{}", path.display(), a)
                } else {
                    path.display().to_string()
                }
            }
            LinkTarget::WikiLink { target, .. } => format!("[[{}]]", target),
            LinkTarget::External(url) => {
                if url.len() > 50 {
                    format!("{}...", &url[..47])
                } else {
                    url.clone()
                }
            }
        };

        // Different styles for selected vs unselected
        if is_selected {
            lines.push(Line::from(vec![
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(theme.modal_selected_marker())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    number,
                    Style::default()
                        .fg(theme.modal_key_fg())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    link_text.clone(),
                    Style::default()
                        .fg(theme.modal_selected_fg())
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::styled(
                    format!(" → {}", target_str),
                    Style::default()
                        .fg(theme.modal_description())
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(number, Style::default().fg(theme.modal_description())),
                Span::styled(link_text.clone(), Style::default().fg(theme.modal_text())),
                Span::styled(
                    format!(" → {}", target_str),
                    Style::default().fg(theme.modal_description()),
                ),
            ]));
        }

        // Add blank line between links
        if idx < app.links_in_view.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    // Add footer
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Tab/j/k: Navigate • 1-9: Jump • p: Parent • Enter: Follow • Esc: Cancel",
        Style::default()
            .fg(theme.modal_description())
            .add_modifier(Modifier::ITALIC),
    )]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.modal_border()))
                .title(" Link Navigator ")
                .style(Style::default().bg(theme.modal_bg())),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

/// Render the theme picker popup
pub fn render_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::theme::ThemeName;

    let theme = &app.theme;

    // All available themes
    let themes = [
        (ThemeName::OceanDark, "Ocean Dark", "Base16 Ocean with cool blues"),
        (ThemeName::Nord, "Nord", "Arctic, north-bluish palette"),
        (ThemeName::Dracula, "Dracula", "Dark theme with vibrant colors"),
        (ThemeName::Solarized, "Solarized", "Precision colors for machines and people"),
        (ThemeName::Monokai, "Monokai", "Sublime Text's iconic scheme"),
        (ThemeName::Gruvbox, "Gruvbox", "Retro groove color scheme"),
        (ThemeName::TokyoNight, "Tokyo Night", "Modern night theme for low-light"),
        (ThemeName::CatppuccinMocha, "Catppuccin Mocha", "Soothing pastel theme for night coding"),
    ];

    // Create centered popup area
    let popup_area = centered_area(area, 60, 50);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create lines for each theme
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "Select Theme (j/k to navigate, Enter to apply, Esc to cancel)",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
    ];

    for (idx, (theme_name, name, description)) in themes.iter().enumerate() {
        let is_selected = idx == app.theme_picker_selected;
        let is_current = *theme_name == app.current_theme;

        let (prefix, style) = if is_selected {
            (
                "▶ ",
                Style::default()
                    .fg(theme.modal_selected_fg())
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default().fg(theme.modal_text()))
        };

        let current_marker = if is_current { " ✓" } else { "" };
        let line_text = format!("{}{}{}", prefix, name, current_marker);

        lines.push(Line::from(vec![Span::styled(line_text, style)]));

        // Add description on next line if selected
        if is_selected {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", description),
                Style::default()
                    .fg(theme.modal_description())
                    .add_modifier(Modifier::ITALIC),
            )]));
        }
    }

    lines.push(Line::from(""));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.modal_border()))
                .title(" Theme Selector ")
                .style(Style::default().bg(theme.modal_bg())),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}

/// Render the cell edit overlay for table editing
pub fn render_cell_edit_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // Create centered popup area
    let edit_area = Rect {
        x: area.x + area.width / 4,
        y: area.y + area.height / 3,
        width: area.width / 2,
        height: 5,
    };

    // Clear background
    frame.render_widget(Clear, edit_area);

    // Create edit display
    let edit_text = format!(
        "Edit Cell [{},{}]: {}_",
        app.cell_edit_row, app.cell_edit_col, app.cell_edit_value
    );

    let paragraph = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            "Edit Table Cell",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            edit_text,
            Style::default().fg(Color::White),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter: Save • Esc: Cancel",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.modal_border()))
            .style(Style::default().bg(theme.modal_bg())),
    );

    frame.render_widget(paragraph, edit_area);
}
