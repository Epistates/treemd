//! Popup and overlay rendering for the TUI
//!
//! Handles modal dialogs including help, link picker, search, theme selector,
//! and cell edit overlays.

use crate::tui::app::App;
use crate::tui::help_text;
use crate::tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
};

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

    // Build header with search info
    let header_text = if app.link_search_active || !app.link_search_query.is_empty() {
        format!(
            "Links ({}/{}) - /: search, Enter: follow, Esc: {}",
            app.filtered_link_indices.len(),
            app.links_in_view.len(),
            if app.link_search_active {
                "stop search"
            } else {
                "cancel"
            }
        )
    } else {
        format!(
            "Links in this section ({} found) - /: search, Tab/j/k: navigate, Enter: follow",
            app.links_in_view.len()
        )
    };

    // Create lines for display
    let mut lines = vec![Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(theme.modal_title())
            .add_modifier(Modifier::BOLD),
    )])];

    // Show search bar if active or has query
    if app.link_search_active || !app.link_search_query.is_empty() {
        let search_style = if app.link_search_active {
            Style::default()
                .fg(theme.modal_selected_fg())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.modal_description())
        };

        let cursor = if app.link_search_active { "▌" } else { "" };
        lines.push(Line::from(vec![
            Span::styled("Search: ", Style::default().fg(theme.modal_key_fg())),
            Span::styled(format!("{}{}", app.link_search_query, cursor), search_style),
        ]));
    }

    lines.push(Line::from(""));

    // Track the line index where selected link starts (for scrolling)
    let mut selected_line_start: u16 = 0;

    // Iterate over filtered links
    for (display_idx, &real_idx) in app.filtered_link_indices.iter().enumerate() {
        let link = &app.links_in_view[real_idx];
        let is_selected = app.selected_link_idx == Some(display_idx);

        // Track line position for selected item
        if is_selected {
            selected_line_start = lines.len() as u16;
        }

        // Format link number (show original index for jump commands)
        let number = format!("[{}] ", real_idx + 1);
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
                        .fg(theme.selection_indicator_fg)
                        .bg(theme.modal_selected_fg())
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
        if display_idx < app.filtered_link_indices.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    // Show "no matches" message if filter has no results
    if app.filtered_link_indices.is_empty() && !app.links_in_view.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No links match your search",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    // Add footer
    lines.push(Line::from(""));
    let footer_text = if app.link_search_active {
        "Type to filter • Enter: select • Esc: stop search • Backspace: delete"
    } else {
        "Tab/j/k: Navigate • /: Search • 1-9: Jump • p: Parent • Enter: Follow • Esc: Cancel"
    };
    lines.push(Line::from(vec![Span::styled(
        footer_text,
        Style::default()
            .fg(theme.modal_description())
            .add_modifier(Modifier::ITALIC),
    )]));

    let total_lines = lines.len();

    // Calculate scroll offset to keep selected link visible
    // Account for popup border (2 lines) and header (2 lines)
    let inner_height = popup_area.height.saturating_sub(2) as usize; // Border takes 2 lines
    let header_lines = 2; // Title + blank line
    let footer_lines = 2; // Blank + footer

    // Visible content area for links
    let visible_area = inner_height.saturating_sub(header_lines + footer_lines);

    // Calculate scroll offset
    let scroll_offset = if selected_line_start > 0 && visible_area > 0 {
        // Keep selected line roughly centered, but don't scroll past bounds
        let target_line = selected_line_start.saturating_sub(header_lines as u16);
        let center_offset = (visible_area / 2) as u16;

        if target_line > center_offset {
            let max_scroll = (total_lines.saturating_sub(inner_height)) as u16;
            (target_line.saturating_sub(center_offset)).min(max_scroll)
        } else {
            0
        }
    } else {
        0
    };

    let paragraph = Paragraph::new(lines.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.modal_border()))
                .title(" Link Navigator ")
                .style(Style::default().bg(theme.modal_bg())),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, popup_area);

    // Render scrollbar if content exceeds visible area
    if total_lines > inner_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(Style::default().fg(theme.modal_border()));

        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset as usize);

        frame.render_stateful_widget(
            scrollbar,
            popup_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Render the theme picker popup
pub fn render_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::theme::ThemeName;

    let theme = &app.theme;

    // All available themes
    let themes = [
        (
            ThemeName::OceanDark,
            "Ocean Dark",
            "Base16 Ocean with cool blues",
        ),
        (ThemeName::Nord, "Nord", "Arctic, north-bluish palette"),
        (
            ThemeName::Dracula,
            "Dracula",
            "Dark theme with vibrant colors",
        ),
        (
            ThemeName::Solarized,
            "Solarized",
            "Precision colors for machines and people",
        ),
        (
            ThemeName::Monokai,
            "Monokai",
            "Sublime Text's iconic scheme",
        ),
        (ThemeName::Gruvbox, "Gruvbox", "Retro groove color scheme"),
        (
            ThemeName::TokyoNight,
            "Tokyo Night",
            "Modern night theme for low-light",
        ),
        (
            ThemeName::CatppuccinMocha,
            "Catppuccin Mocha",
            "Soothing pastel theme for night coding",
        ),
    ];

    // Create centered popup area
    let popup_area = centered_area(area, 60, 50);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Create lines for each theme
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "Select Theme (j/k: preview, Enter: save, Esc: cancel)",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
    ];

    for (idx, (theme_name, name, description)) in themes.iter().enumerate() {
        let is_selected = idx == app.theme_picker_selected;
        // Show ✓ next to the saved theme (original), not the preview
        let saved_theme = app.theme_picker_original.unwrap_or(app.current_theme);
        let is_saved = *theme_name == saved_theme;

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

        let current_marker = if is_saved { " ✓" } else { "" };
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

/// Render file creation confirmation dialog
pub fn render_file_create_confirm(frame: &mut Frame, message: &str, theme: &Theme) {
    use crate::tui::ui::util::centered_area;

    // Create a centered dialog area (smaller than help/link picker)
    let area = centered_area(frame.area(), 50, 20);

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the dialog content
    let text = vec![
        Line::from(vec![Span::styled(
            "Create File?",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            message,
            Style::default().fg(theme.modal_text()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                " Create file  ",
                Style::default().fg(theme.modal_description()),
            ),
            Span::styled("[n/Esc]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Cancel", Style::default().fg(theme.modal_description())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm ")
                .title_style(Style::default().fg(theme.modal_title()))
                .border_style(Style::default().fg(theme.modal_border()))
                .style(Style::default().bg(theme.modal_bg())),
        );

    frame.render_widget(paragraph, area);
}

/// Render the save width confirmation modal
pub fn render_save_width_confirm(frame: &mut Frame, width: u16, theme: &Theme) {
    use crate::tui::ui::util::centered_area;

    // Create a centered dialog area
    let area = centered_area(frame.area(), 45, 18);

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the dialog content
    let text = vec![
        Line::from(vec![Span::styled(
            "Save Outline Width",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!("Save width {}% to config file?", width),
            Style::default().fg(theme.modal_text()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y/Enter]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Save  ", Style::default().fg(theme.modal_description())),
            Span::styled("[n/Esc]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Cancel", Style::default().fg(theme.modal_description())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm ")
                .title_style(Style::default().fg(theme.modal_title()))
                .border_style(Style::default().fg(theme.modal_border()))
                .style(Style::default().bg(theme.modal_bg())),
        );

    frame.render_widget(paragraph, area);
}

/// Render the save before quit confirmation modal
pub fn render_save_before_quit_confirm(frame: &mut Frame, edit_count: usize, theme: &Theme) {
    use crate::tui::ui::util::centered_area;

    // Create a centered dialog area
    let area = centered_area(frame.area(), 56, 26);

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the dialog content
    let text = vec![
        Line::from(vec![Span::styled(
            "Unsaved Changes",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(
                "You have {} unsaved change{}.",
                edit_count,
                if edit_count == 1 { "" } else { "s" }
            ),
            Style::default().fg(theme.modal_text()),
        )]),
        Line::from(vec![Span::styled(
            "Save before quitting?",
            Style::default().fg(theme.modal_text()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y/Enter]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Save & Quit", Style::default().fg(theme.modal_description())),
        ]),
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Quit without saving", Style::default().fg(theme.modal_description())),
        ]),
        Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Cancel", Style::default().fg(theme.modal_description())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Quit ")
                .title_style(Style::default().fg(theme.modal_title()))
                .border_style(Style::default().fg(theme.modal_border()))
                .style(Style::default().bg(theme.modal_bg())),
        );

    frame.render_widget(paragraph, area);
}

/// Render the save before navigate confirmation modal
pub fn render_save_before_nav_confirm(frame: &mut Frame, edit_count: usize, theme: &Theme) {
    use crate::tui::ui::util::centered_area;

    // Create a centered dialog area
    let area = centered_area(frame.area(), 58, 28);

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the dialog content
    let text = vec![
        Line::from(vec![Span::styled(
            "Unsaved Changes",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            format!(
                "You have {} unsaved change{}.",
                edit_count,
                if edit_count == 1 { "" } else { "s" }
            ),
            Style::default().fg(theme.modal_text()),
        )]),
        Line::from(vec![Span::styled(
            "Save before navigating?",
            Style::default().fg(theme.modal_text()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y/Enter]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Save & Navigate", Style::default().fg(theme.modal_description())),
        ]),
        Line::from(vec![
            Span::styled("[d]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Discard & Navigate", Style::default().fg(theme.modal_description())),
        ]),
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Discard & Quit", Style::default().fg(theme.modal_description())),
        ]),
        Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" Cancel", Style::default().fg(theme.modal_description())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm Navigation ")
                .title_style(Style::default().fg(theme.modal_title()))
                .border_style(Style::default().fg(theme.modal_border()))
                .style(Style::default().bg(theme.modal_bg())),
        );

    frame.render_widget(paragraph, area);
}

/// Render the command palette with fuzzy search
pub fn render_command_palette(frame: &mut Frame, app: &App, theme: &Theme) {
    use crate::tui::app::PALETTE_COMMANDS;
    use crate::tui::ui::util::centered_area;

    // Create a centered popup
    let area = centered_area(frame.area(), 60, 50);

    // Clear the area
    frame.render_widget(Clear, area);

    // Build the content
    let mut lines = vec![
        // Title
        Line::from(vec![Span::styled(
            "Command Palette",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        // Search input
        Line::from(vec![
            Span::styled(": ", Style::default().fg(theme.modal_key_fg())),
            Span::styled(&app.command_query, Style::default().fg(theme.modal_text())),
            Span::styled("█", Style::default().fg(Color::White)), // Cursor
        ]),
        Line::from(""),
    ];

    // Show filtered commands
    if app.command_filtered.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  No matching commands",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]));
    } else {
        for (display_idx, &cmd_idx) in app.command_filtered.iter().enumerate() {
            let cmd = &PALETTE_COMMANDS[cmd_idx];
            let is_selected = display_idx == app.command_selected;

            let prefix = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.modal_selected_marker())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.modal_text())
            };

            // Command name
            let mut spans = vec![Span::styled(prefix, style), Span::styled(cmd.name, style)];

            // Show aliases in dimmer text
            if !cmd.aliases.is_empty() {
                let aliases_str = format!(" ({})", cmd.aliases.join(", "));
                spans.push(Span::styled(
                    aliases_str,
                    Style::default().fg(theme.modal_description()),
                ));
            }

            lines.push(Line::from(spans));

            // Show description for selected item
            if is_selected {
                lines.push(Line::from(vec![Span::styled(
                    format!("    {}", cmd.description),
                    Style::default()
                        .fg(theme.modal_description())
                        .add_modifier(Modifier::ITALIC),
                )]));
            }
        }
    }

    // Footer with hints
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("↑↓/Tab", Style::default().fg(theme.modal_key_fg())),
        Span::styled(
            " navigate  ",
            Style::default().fg(theme.modal_description()),
        ),
        Span::styled("Enter", Style::default().fg(theme.modal_key_fg())),
        Span::styled(" execute  ", Style::default().fg(theme.modal_description())),
        Span::styled("Esc", Style::default().fg(theme.modal_key_fg())),
        Span::styled(" cancel", Style::default().fg(theme.modal_description())),
    ]));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.modal_border()))
            .style(Style::default().bg(theme.modal_bg())),
    );

    frame.render_widget(paragraph, area);
}
