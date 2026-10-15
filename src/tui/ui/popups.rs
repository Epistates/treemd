//! Popup and overlay rendering for the TUI
//!
//! Handles modal dialogs including help, link picker, search, theme selector,
//! and cell edit overlays.

use crate::tui::app::App;
use crate::tui::help_text;
use crate::tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::util::popup_area;

/// Render the help popup with keyboard shortcuts
pub fn render_help_popup(frame: &mut Frame, app: &mut App, area: Rect) {
    // Min 40 cols for readability, min 10 rows for usable scroll area
    let popup_area = popup_area(area, 70, 80, 40, 10);
    let theme = app.theme.clone();

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Wrapped to the popup here, so the scroll limit and page size are the
    // rows actually drawn. They were the number of unwrapped help lines and
    // the content pane's height, so scrolling stopped short of the end, or
    // ran past it, depending on the terminal and the keybindings.
    let help_lines = help_text::build_help_text(&app.keybindings, &theme);
    let visible = usize::from(popup_area.height.saturating_sub(2));
    let rows = crate::tui::content_view::ContentView::new(
        help_lines,
        popup_area.width.saturating_sub(2),
        Vec::new(),
    )
    .rows;
    let max_scroll = rows.len().saturating_sub(visible);
    app.help_max_scroll = u16::try_from(max_scroll).unwrap_or(u16::MAX);
    app.help_page = u16::try_from(visible.max(1)).unwrap_or(u16::MAX);
    app.help_scroll = app.help_scroll.min(app.help_max_scroll);
    let scroll = usize::from(app.help_scroll);

    let paragraph = Paragraph::new(rows[scroll..].to_vec()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.modal_border()))
            .title(" Help ")
            .style(theme.modal_style()),
    );

    frame.render_widget(paragraph, popup_area);

    // Render scrollbar for help
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .thumb_symbol("┃")
        .track_symbol(Some("│"))
        .style(Style::default().fg(theme.modal_border()));

    let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);

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
    // Min 30 cols for link text, min 8 rows for header + a few links + footer
    let popup_area = popup_area(area, 80, 60, 30, 8);

    // Clear background
    frame.render_widget(Clear, popup_area);

    // Build header with search info
    let header_text = if app.link_picker.active || !app.link_picker.query.is_empty() {
        format!(
            "Links ({}/{}) - /: search, Enter: follow, Esc: {}",
            app.link_picker.filtered_indices.len(),
            app.links_in_view.len(),
            if app.link_picker.active {
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
    if app.link_picker.active || !app.link_picker.query.is_empty() {
        let search_style = if app.link_picker.active {
            Style::default()
                .fg(theme.modal_selected_fg())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.modal_description())
        };

        let cursor = if app.link_picker.active { "▌" } else { "" };
        lines.push(Line::from(vec![
            Span::styled("Search: ", Style::default().fg(theme.modal_key_fg())),
            Span::styled(format!("{}{}", app.link_picker.query, cursor), search_style),
        ]));
    }

    lines.push(Line::from(""));

    // Track the line index where selected link starts (for scrolling)
    let mut selected_line_start: u16 = 0;

    // Iterate over filtered links
    for (display_idx, &real_idx) in app.link_picker.filtered_indices.iter().enumerate() {
        let link = &app.links_in_view[real_idx];
        let is_selected = app.link_picker.selected == Some(display_idx);

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
            LinkTarget::External(url) => super::util::truncate_with_ellipsis(url, 50),
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
        if display_idx < app.link_picker.filtered_indices.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    // Show "no matches" message if filter has no results
    if app.link_picker.filtered_indices.is_empty() && !app.links_in_view.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No links match your search",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    // Add footer
    lines.push(Line::from(""));
    let footer_text = if app.link_picker.active {
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

    let inner_height = popup_area.height.saturating_sub(2) as usize; // Border takes 2 lines
    let (rows, scroll_offset) = wrap_and_scroll(
        lines,
        popup_area.width.saturating_sub(2),
        inner_height,
        selected_line_start as usize,
    );
    let total_lines = rows.len();

    let paragraph = Paragraph::new(rows[scroll_offset..].to_vec()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.modal_border()))
            .title(" Link Navigator ")
            .style(theme.modal_style()),
    );

    frame.render_widget(paragraph, popup_area);

    // Render scrollbar if content exceeds visible area
    if total_lines > inner_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .thumb_symbol("┃")
            .track_symbol(Some("│"))
            .style(Style::default().fg(theme.modal_border()));

        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset);

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
///
/// Every theme, each with a strip of its own heading and accent colors, the
/// saved one ticked. Moving the selection previews the theme on the whole
/// screen; Enter keeps it and Esc goes back.
pub fn render_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::theme::ThemeName;
    use ratatui::widgets::{HighlightSpacing, List, ListItem, ListState};

    let theme = &app.theme;
    let saved_theme = app.theme_picker_original.unwrap_or(app.current_theme);
    let names: Vec<ThemeName> = ThemeName::all().collect();
    let name_width = names
        .iter()
        .map(|name| name.display_name().len())
        .max()
        .unwrap_or(0);

    let items: Vec<ListItem> = names
        .iter()
        .map(|&name| {
            let sample = Theme::for_terminal(name, app.color_mode);
            let mut spans = vec![Span::raw(format!("{:<name_width$}  ", name.display_name()))];
            for color in [
                sample.heading_1,
                sample.heading_2,
                sample.heading_3,
                sample.heading_4,
                sample.border_focused,
            ] {
                spans.push(Span::styled("●", Style::default().fg(color)));
            }
            if name == saved_theme {
                spans.push(Span::styled(
                    "  ✓",
                    Style::default().fg(theme.modal_key_fg()),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let selected = names
        .get(app.theme_picker_selected)
        .copied()
        .unwrap_or(app.current_theme);
    let height = (names.len() as u16 + 5).min(area.height);
    let width = (name_width as u16 + 22).max(44).min(area.width);
    let popup_area = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.modal_border()))
        .title(Span::styled(
            " Theme ",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(
            Line::from(vec![
                Span::styled(" j/k", Style::default().fg(theme.modal_key_fg())),
                Span::styled(" preview  ", Style::default().fg(theme.modal_description())),
                Span::styled("Enter", Style::default().fg(theme.modal_key_fg())),
                Span::styled(" save  ", Style::default().fg(theme.modal_description())),
                Span::styled("Esc", Style::default().fg(theme.modal_key_fg())),
                Span::styled(" cancel ", Style::default().fg(theme.modal_description())),
            ])
            .centered(),
        )
        .style(theme.modal_style());
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // The list, then a blank row and the selected theme's description.
    let [list_area, _, about_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let list = List::new(items)
        .highlight_symbol("▶ ")
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(
            Style::default()
                .fg(theme.modal_selected_fg())
                .add_modifier(Modifier::BOLD),
        )
        .scroll_padding(1);
    let mut state = ListState::default().with_selected(Some(app.theme_picker_selected));
    frame.render_stateful_widget(list, list_area, &mut state);

    let about = if selected.is_light() {
        format!("  {} · light", selected.description())
    } else {
        format!("  {}", selected.description())
    };
    frame.render_widget(
        Paragraph::new(about).style(
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        ),
        about_area,
    );
}

/// Render the cell edit overlay for table editing
pub fn render_cell_edit_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // Create centered popup area with safe bounds
    // Min 20 cols for input text, min 5 rows for the dialog
    let min_width = 20u16;
    let min_height = 5u16;
    let width = (area.width / 2).max(min_width).min(area.width);
    let height = min_height.min(area.height);

    // Center the popup safely
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    let edit_area = Rect {
        x,
        y,
        width,
        height,
    };

    // Clear background
    frame.render_widget(Clear, edit_area);

    // Create edit display
    let edit_prefix = format!("Edit Cell [{},{}]: ", app.cell_edit_row, app.cell_edit_col);
    let edit_text = format!("{edit_prefix}{}", app.cell_edit_value);

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
            Style::default().fg(theme.modal_text()),
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
            .style(theme.modal_style()),
    );

    frame.render_widget(paragraph, edit_area);

    if edit_area.width > 2 {
        use unicode_width::UnicodeWidthStr;
        let cursor_offset =
            edit_prefix.width() as u16 + app.cell_edit_value.as_str().width() as u16;
        let cursor_x = edit_area
            .x
            .saturating_add(1)
            .saturating_add(cursor_offset)
            .min(edit_area.right().saturating_sub(2));
        frame.set_cursor_position((cursor_x, edit_area.y.saturating_add(3)));
    }
}

/// Render file creation confirmation dialog
pub fn render_file_create_confirm(frame: &mut Frame, message: &str, theme: &Theme) {
    // Create a centered dialog area (smaller than help/link picker)
    // Min 30 cols for message, min 7 rows for dialog content
    let area = popup_area(frame.area(), 50, 20, 30, 7);

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
                .style(theme.modal_style()),
        );

    frame.render_widget(paragraph, area);
}

/// Render the save width confirmation modal
pub fn render_save_width_confirm(frame: &mut Frame, width: u16, theme: &Theme) {
    // Create a centered dialog area
    // Min 28 cols for text, min 7 rows for dialog content
    let area = popup_area(frame.area(), 45, 18, 28, 7);

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
                .style(theme.modal_style()),
        );

    frame.render_widget(paragraph, area);
}

/// Render the save before quit confirmation modal
pub fn render_save_before_quit_confirm(frame: &mut Frame, edit_count: usize, theme: &Theme) {
    // Create a centered dialog area
    // Min 30 cols for text, min 10 rows for all options
    let area = popup_area(frame.area(), 56, 26, 30, 10);

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
            Span::styled(
                " Save & Quit",
                Style::default().fg(theme.modal_description()),
            ),
        ]),
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                " Quit without saving",
                Style::default().fg(theme.modal_description()),
            ),
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
                .style(theme.modal_style()),
        );

    frame.render_widget(paragraph, area);
}

/// Render the save before navigate confirmation modal
pub fn render_save_before_nav_confirm(frame: &mut Frame, edit_count: usize, theme: &Theme) {
    // Create a centered dialog area
    // Min 32 cols for text, min 11 rows for all options
    let area = popup_area(frame.area(), 58, 28, 32, 11);

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
            Span::styled(
                " Save & Navigate",
                Style::default().fg(theme.modal_description()),
            ),
        ]),
        Line::from(vec![
            Span::styled("[d]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                " Discard & Navigate",
                Style::default().fg(theme.modal_description()),
            ),
        ]),
        Line::from(vec![
            Span::styled("[q]", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                " Discard & Quit",
                Style::default().fg(theme.modal_description()),
            ),
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
                .style(theme.modal_style()),
        );

    frame.render_widget(paragraph, area);
}

/// Render the command palette with fuzzy search
pub fn render_command_palette(frame: &mut Frame, app: &App, theme: &Theme) {
    use crate::tui::app::PALETTE_COMMANDS;
    use ratatui::layout::{Constraint, Layout};
    use ratatui::widgets::{List, ListItem, ListState};

    // Create a centered popup
    // Min 35 cols for command names, min 10 rows for header + a few commands
    let area = popup_area(frame.area(), 60, 50, 35, 10);

    // Clear the area
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.modal_border()))
        .style(theme.modal_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Title and input above, hints below, and the commands in between as a
    // list that scrolls to keep the selection in view. Drawn as one
    // paragraph, the commands past the bottom of the popup were cut off and
    // the selection moved out of sight after a few steps on a small terminal.
    let [header, commands, footer] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(inner);

    let header_lines = vec![
        Line::from(vec![Span::styled(
            "Command Palette",
            Style::default()
                .fg(theme.modal_title())
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(": ", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                &app.command_palette.query,
                Style::default().fg(theme.modal_text()),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(header_lines), header);

    if app.command_palette.filtered.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "  No matching commands",
                Style::default()
                    .fg(theme.modal_description())
                    .add_modifier(Modifier::ITALIC),
            )])),
            commands,
        );
    } else {
        let items: Vec<ListItem> = app
            .command_palette
            .filtered
            .iter()
            .enumerate()
            .map(|(display_idx, &cmd_idx)| {
                let cmd = &PALETTE_COMMANDS[cmd_idx];
                let is_selected = display_idx == app.command_palette.selected;

                let prefix = if is_selected { "▸ " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(theme.modal_selected_marker())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.modal_text())
                };

                // Command name, with aliases in dimmer text
                let mut spans = vec![Span::styled(prefix, style), Span::styled(cmd.name, style)];
                if !cmd.aliases.is_empty() {
                    spans.push(Span::styled(
                        format!(" ({})", cmd.aliases.join(", ")),
                        Style::default().fg(theme.modal_description()),
                    ));
                }
                let mut lines = vec![Line::from(spans)];

                // Show description for selected item
                if is_selected {
                    lines.push(Line::from(vec![Span::styled(
                        format!("    {}", cmd.description),
                        Style::default()
                            .fg(theme.modal_description())
                            .add_modifier(Modifier::ITALIC),
                    )]));
                }
                ListItem::new(lines)
            })
            .collect();

        let mut state = ListState::default().with_selected(Some(app.command_palette.selected));
        frame.render_stateful_widget(List::new(items).scroll_padding(1), commands, &mut state);
    }

    // Footer with hints. Tab completes the command name; it does not move.
    let footer_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("↑↓", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                " navigate  ",
                Style::default().fg(theme.modal_description()),
            ),
            Span::styled("Tab", Style::default().fg(theme.modal_key_fg())),
            Span::styled(
                " complete  ",
                Style::default().fg(theme.modal_description()),
            ),
            Span::styled("Enter", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" execute  ", Style::default().fg(theme.modal_description())),
            Span::styled("Esc", Style::default().fg(theme.modal_key_fg())),
            Span::styled(" cancel", Style::default().fg(theme.modal_description())),
        ]),
    ];
    frame.render_widget(Paragraph::new(footer_lines), footer);

    if inner.width > 0 {
        use unicode_width::UnicodeWidthStr;
        let cursor_offset = 2 + app.command_palette.query.as_str().width() as u16;
        let cursor_x = inner
            .x
            .saturating_add(cursor_offset)
            .min(inner.right().saturating_sub(1));
        frame.set_cursor_position((cursor_x, header.y.saturating_add(2)));
    }
}

/// A popup's lines wrapped to `width`, and the row to scroll to so that the
/// line at `selected_line` sits near the middle of `height` rows.
///
/// The scroll used to be worked out in lines while the paragraph wrapped
/// them, so with long entries above it the selection was scrolled to a row
/// that was no longer where it was drawn, and went out of view.
fn wrap_and_scroll(
    lines: Vec<Line<'_>>,
    width: u16,
    height: usize,
    selected_line: usize,
) -> (Vec<Line<'static>>, usize) {
    let owned = lines
        .into_iter()
        .map(|line| {
            let mut owned = Line::from(
                line.spans
                    .into_iter()
                    .map(|span| Span::styled(span.content.into_owned(), span.style))
                    .collect::<Vec<_>>(),
            )
            .style(line.style);
            owned.alignment = line.alignment;
            owned
        })
        .collect();
    let view = crate::tui::content_view::ContentView::new(owned, width, Vec::new());
    let selected = view.row_of_line(selected_line);
    let max_scroll = view.rows.len().saturating_sub(height);
    let scroll = selected.saturating_sub(height / 2).min(max_scroll);
    (view.rows, scroll)
}

/// Render the file picker modal
pub fn render_file_picker(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // Create centered popup area
    let popup_area = popup_area(area, 80, 60, 30, 8);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let file_count = app.file_picker.filtered_file_indices.len();
    let dir_count = app.file_picker.filtered_dir_indices.len();
    let total_items = app.file_picker_item_count();

    // Build header with search info
    let effective_dir = app.effective_picker_dir();
    let dir_label = effective_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(".")
        .to_string();

    let header_text = if app.file_picker.active || !app.file_picker.query.is_empty() {
        format!(
            "{}: {}/{} files, {} dirs - Enter: open, Esc: {}",
            dir_label,
            file_count,
            app.file_picker.files.len(),
            dir_count,
            if app.file_picker.active {
                "stop search"
            } else {
                "cancel"
            }
        )
    } else {
        format!(
            "{}: {} files, {} dirs",
            dir_label,
            app.file_picker.files.len(),
            app.file_picker.dirs.len()
        )
    };

    let mut lines = vec![Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(theme.modal_title())
            .add_modifier(Modifier::BOLD),
    )])];

    // Show search bar if active or has query
    if app.file_picker.active || !app.file_picker.query.is_empty() {
        let search_style = if app.file_picker.active {
            Style::default()
                .fg(theme.modal_selected_fg())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.modal_description())
        };

        let cursor = if app.file_picker.active { "▌" } else { "" };
        lines.push(Line::from(vec![
            Span::styled("Filter: ", Style::default().fg(theme.modal_key_fg())),
            Span::styled(format!("{}{}", app.file_picker.query, cursor), search_style),
        ]));
    }

    lines.push(Line::from(""));

    let mut selected_line_start: u16 = 0;

    // Iterate over filtered files
    for (display_idx, &real_idx) in app.file_picker.filtered_file_indices.iter().enumerate() {
        let file_path = &app.file_picker.files[real_idx];
        let is_selected = app.file_picker.selected == Some(display_idx);
        let is_current = file_path == &app.current_file_path;

        if is_selected {
            selected_line_start = lines.len() as u16;
        }

        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let current_marker = if is_current { " [current]" } else { "" };
        let number = format!("[{}] ", real_idx + 1);

        if is_selected {
            lines.push(Line::from(vec![
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(theme.selection_indicator_fg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(number, Style::default().fg(theme.modal_key_fg())),
                Span::styled(
                    filename,
                    Style::default()
                        .fg(theme.selection_indicator_fg)
                        .bg(theme.modal_selected_fg())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    current_marker,
                    Style::default()
                        .fg(theme.modal_description())
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        } else {
            let text_style = if is_current {
                Style::default()
                    .fg(theme.modal_text())
                    .add_modifier(Modifier::ITALIC)
            } else {
                Style::default().fg(theme.modal_text())
            };

            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(number, Style::default().fg(theme.modal_description())),
                Span::styled(filename, text_style),
                Span::styled(
                    current_marker,
                    Style::default()
                        .fg(theme.modal_description())
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
    }

    // Separator between files and directories
    if !app.file_picker.filtered_file_indices.is_empty()
        && !app.file_picker.filtered_dir_indices.is_empty()
    {
        lines.push(Line::from(vec![Span::styled(
            "── Directories ──",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    // Iterate over filtered directories
    for (dir_display_idx, &real_dir_idx) in app.file_picker.filtered_dir_indices.iter().enumerate()
    {
        let combined_idx = file_count + dir_display_idx;
        let dir_path = &app.file_picker.dirs[real_dir_idx];
        let is_selected = app.file_picker.selected == Some(combined_idx);

        if is_selected {
            selected_line_start = lines.len() as u16;
        }

        let dirname = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let display_name = format!("[DIR] {}/", dirname);

        if is_selected {
            lines.push(Line::from(vec![
                Span::styled(
                    "▶ ",
                    Style::default()
                        .fg(theme.selection_indicator_fg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    display_name,
                    Style::default()
                        .fg(theme.selection_indicator_fg)
                        .bg(theme.modal_selected_fg())
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(display_name, Style::default().fg(theme.modal_description())),
            ]));
        }
    }

    // Show "no matches" message
    if total_items == 0 && (!app.file_picker.files.is_empty() || !app.file_picker.dirs.is_empty()) {
        lines.push(Line::from(vec![Span::styled(
            "No items match your search",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]));
    } else if total_items == 0 {
        lines.push(Line::from(vec![Span::styled(
            "Empty directory",
            Style::default()
                .fg(theme.modal_description())
                .add_modifier(Modifier::ITALIC),
        )]));
    }

    // Footer
    lines.push(Line::from(""));
    let footer_text = if app.file_picker.active {
        "Type to filter • Enter: select • Esc: stop search • Backspace: delete"
    } else {
        "j/k: Navigate • /: Filter • Enter: Open • Backspace: Parent dir • h: Hidden • Esc: Cancel"
    };
    lines.push(Line::from(vec![Span::styled(
        footer_text,
        Style::default()
            .fg(theme.modal_description())
            .add_modifier(Modifier::ITALIC),
    )]));

    let total_lines = lines.len();
    let inner_height = popup_area.height.saturating_sub(2) as usize;
    let header_lines = if app.file_picker.active || !app.file_picker.query.is_empty() {
        3
    } else {
        2
    };
    let footer_lines = 2;
    let visible_area = inner_height.saturating_sub(header_lines + footer_lines);

    let scroll_offset = if selected_line_start > 0 && visible_area > 0 {
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

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.modal_border()))
            .title(" File Picker ")
            .style(theme.modal_style()),
    );

    frame.render_widget(paragraph.scroll((scroll_offset, 0)), popup_area);

    // Scrollbar if needed
    if total_lines > inner_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .thumb_symbol("┃")
            .track_symbol(Some("│"))
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
