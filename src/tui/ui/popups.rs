//! Popup and overlay rendering for the TUI
//!
//! Every popup is drawn the same way: [`popup_frame`] clears the area and
//! draws a rounded border with the title, and lists inside one share
//! [`popup_list`]. Key hints are in the footer, which shows the ones for
//! the popup that is open, so popups do not repeat them.

use crate::tui::app::App;
use crate::tui::help_text;
use crate::tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph,
    Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::util::popup_area;

/// Clear `area` and draw a popup's frame there: rounded, its border in the
/// focus color and its title in bold, in the popup's own colors. Returns
/// the area inside, with a column of padding each side.
fn popup_frame(frame: &mut Frame, theme: &Theme, area: Rect, title: Line<'static>) -> Rect {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.modal_border()))
        .title(title)
        .padding(ratatui::widgets::Padding::horizontal(1))
        .style(theme.modal_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

/// A centered popup area sized to its content: `rows` by `cols` inside the
/// border, no larger than `max_pct` of `area` either way, and no smaller
/// than the minimums. A short list used to sit in a popup most of the
/// screen tall.
fn fitted_area(area: Rect, rows: usize, cols: usize, max_pct: u16, min: (u16, u16)) -> Rect {
    let limit = |size: u16| (u32::from(size) * u32::from(max_pct) / 100) as u16;
    // The border, plus the padding column on each side.
    let width = (cols as u16)
        .saturating_add(4)
        .clamp(min.0, limit(area.width).max(min.0));
    let height = (rows as u16)
        .saturating_add(2)
        .clamp(min.1, limit(area.height).max(min.1));
    let (width, height) = (width.min(area.width), height.min(area.height));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// A popup title: the name in bold, then any detail, dimmer.
fn popup_title(theme: &Theme, name: &str, detail: Option<String>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!(" {name} "),
        Style::default()
            .fg(theme.modal_title())
            .add_modifier(Modifier::BOLD),
    )];
    if let Some(detail) = detail {
        spans.push(Span::styled(
            format!("{detail} "),
            Style::default().fg(theme.modal_description()),
        ));
    }
    Line::from(spans)
}

/// A list in a popup: the selection marked with `▶` and drawn in the
/// selection color, kept in view with a row of context around it.
fn popup_list<'a>(theme: &Theme, items: Vec<ListItem<'a>>) -> List<'a> {
    List::new(items)
        .highlight_symbol("▶ ")
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(
            Style::default()
                .fg(theme.modal_selected_fg())
                .add_modifier(Modifier::BOLD),
        )
        .scroll_padding(1)
}

/// A popup's one-line text input: a prompt, the text, and the terminal
/// cursor after it while it is being typed in.
fn popup_input(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    prompt: &str,
    text: &str,
    typing: bool,
) {
    use unicode_width::UnicodeWidthStr;
    let text_style = if typing {
        Style::default()
            .fg(theme.modal_text())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.modal_description())
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                prompt.to_string(),
                Style::default().fg(theme.modal_key_fg()),
            ),
            Span::styled(text.to_string(), text_style),
        ])),
        area,
    );
    if typing && area.width > 0 {
        let x = area
            .x
            .saturating_add((prompt.width() + text.width()) as u16)
            .min(area.right().saturating_sub(1));
        frame.set_cursor_position((x, area.y));
    }
}

/// A dim italic line, for an empty state or a description.
fn popup_note(theme: &Theme, text: impl Into<String>) -> Paragraph<'static> {
    Paragraph::new(Span::styled(
        text.into(),
        Style::default()
            .fg(theme.modal_description())
            .add_modifier(Modifier::ITALIC),
    ))
}

/// A scrollbar on a popup's right border when its `total` rows overflow the
/// `visible` ones: a thumb, and no track or arrows.
fn popup_scrollbar(
    frame: &mut Frame,
    theme: &Theme,
    area: Rect,
    total: usize,
    visible: usize,
    position: usize,
) {
    if total <= visible {
        return;
    }
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(None)
        .thumb_symbol("┃")
        .style(Style::default().fg(theme.modal_border()));
    let mut state = ScrollbarState::new(total.saturating_sub(visible)).position(position);
    frame.render_stateful_widget(
        scrollbar,
        area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut state,
    );
}

/// Render the help popup with keyboard shortcuts
pub fn render_help_popup(frame: &mut Frame, app: &mut App, area: Rect) {
    // Min 40 cols for readability, min 10 rows for usable scroll area
    let popup_area = popup_area(area, 70, 80, 40, 10);
    let theme = app.theme.clone();
    let inner = popup_frame(frame, &theme, popup_area, popup_title(&theme, "Help", None));

    // Wrapped to the popup here, so the scroll limit and page size are the
    // rows actually drawn. They were the number of unwrapped help lines and
    // the content pane's height, so scrolling stopped short of the end, or
    // ran past it, depending on the terminal and the keybindings.
    let help_lines = help_text::build_help_text(&app.keybindings, &theme);
    let visible = usize::from(inner.height);
    let rows = crate::tui::content_view::ContentView::new(help_lines, inner.width, Vec::new()).rows;
    let max_scroll = rows.len().saturating_sub(visible);
    app.help_max_scroll = u16::try_from(max_scroll).unwrap_or(u16::MAX);
    app.help_page = u16::try_from(visible.max(1)).unwrap_or(u16::MAX);
    app.help_scroll = app.help_scroll.min(app.help_max_scroll);
    let scroll = usize::from(app.help_scroll);

    frame.render_widget(Paragraph::new(rows[scroll..].to_vec()), inner);
    popup_scrollbar(frame, &theme, popup_area, rows.len(), visible, scroll);
}

/// Where a link goes, as one short line.
fn link_target(link: &crate::parser::Link) -> String {
    use crate::parser::LinkTarget;
    match &link.target {
        LinkTarget::Anchor(a) => format!("#{a}"),
        LinkTarget::RelativeFile { path, anchor } => match anchor {
            Some(a) => format!("{}#{a}", path.display()),
            None => path.display().to_string(),
        },
        LinkTarget::WikiLink { target, .. } => format!("[[{target}]]"),
        LinkTarget::External(url) => super::util::truncate_with_ellipsis(url, 60),
    }
}

/// Render the link picker popup
///
/// The section's links, each numbered for its jump key, with where it goes.
/// A search filters them.
pub fn render_link_picker(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let picker = &app.link_picker;
    let searching = picker.active || !picker.query.is_empty();

    let detail = if searching {
        format!(
            "{} of {}",
            picker.filtered_indices.len(),
            app.links_in_view.len()
        )
    } else {
        app.links_in_view.len().to_string()
    };
    let widest = app
        .links_in_view
        .iter()
        .map(|link| link.text.chars().count() + link_target(link).chars().count() + 11)
        .max()
        .unwrap_or(20);
    let rows = picker.filtered_indices.len().max(1) + if searching { 2 } else { 0 };
    let popup_area = fitted_area(area, rows, widest, 80, (40, 5));
    let inner = popup_frame(
        frame,
        theme,
        popup_area,
        popup_title(theme, "Links", Some(detail)),
    );

    let [input, list_area] = Layout::vertical([
        Constraint::Length(if searching { 2 } else { 0 }),
        Constraint::Min(1),
    ])
    .areas(inner);
    if searching {
        popup_input(frame, theme, input, "/ ", &picker.query, picker.active);
    }

    if picker.filtered_indices.is_empty() {
        frame.render_widget(popup_note(theme, "No links match"), list_area);
        return;
    }

    let muted = Style::default().fg(theme.modal_description());
    let items: Vec<ListItem> = picker
        .filtered_indices
        .iter()
        .map(|&index| {
            let link = &app.links_in_view[index];
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:>2} ", index + 1), muted),
                Span::raw(link.text.clone()),
                Span::styled(format!("  → {}", link_target(link)), muted),
            ]))
        })
        .collect();
    let mut state = ListState::default().with_selected(picker.selected);
    frame.render_stateful_widget(popup_list(theme, items), list_area, &mut state);
}

/// Render the theme picker popup
///
/// Every theme, each with a strip of its own heading and accent colors, the
/// saved one ticked. Moving the selection previews the theme on the whole
/// screen; Enter keeps it and Esc goes back.
pub fn render_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::theme::ThemeName;

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
    let height = (names.len() as u16 + 4).min(area.height);
    let width = (name_width as u16 + 20).max(44).min(area.width);
    let popup_area = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let inner = popup_frame(frame, theme, popup_area, popup_title(theme, "Theme", None));

    // The list, then a blank row and the selected theme's description.
    let [list_area, _, about_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let mut state = ListState::default().with_selected(Some(app.theme_picker_selected));
    frame.render_stateful_widget(popup_list(theme, items), list_area, &mut state);

    let about = if selected.is_light() {
        format!("{} · light", selected.description())
    } else {
        selected.description().to_string()
    };
    frame.render_widget(popup_note(theme, about), about_area);
}

/// Render the cell edit overlay for table editing
pub fn render_cell_edit_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let width = (area.width / 2).max(24).min(area.width);
    let height = 3.min(area.height);
    let edit_area = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let detail = format!("row {}, column {}", app.cell_edit_row, app.cell_edit_col);
    let inner = popup_frame(
        frame,
        theme,
        edit_area,
        popup_title(theme, "Edit cell", Some(detail)),
    );
    popup_input(frame, theme, inner, "", &app.cell_edit_value, true);
}

/// A confirmation: a title, what is being asked, and each answer's key.
fn render_confirm(
    frame: &mut Frame,
    theme: &Theme,
    title: &str,
    message: &[String],
    options: &[(&str, &str)],
) {
    let height = (message.len() + options.len() + 4) as u16;
    let width = message
        .iter()
        .map(|line| line.len())
        .chain(
            options
                .iter()
                .map(|(key, label)| key.len() + label.len() + 4),
        )
        .max()
        .unwrap_or(20) as u16
        + 8;
    let screen = frame.area();
    let area = Rect::new(
        screen.x + screen.width.saturating_sub(width) / 2,
        screen.y + screen.height.saturating_sub(height) / 2,
        width.min(screen.width),
        height.min(screen.height),
    );
    let inner = popup_frame(frame, theme, area, popup_title(theme, title, None));

    let mut lines: Vec<Line> = message
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                line.clone(),
                Style::default().fg(theme.modal_text()),
            ))
        })
        .collect();
    lines.push(Line::from(""));
    for (key, label) in options {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{key:>5}"),
                Style::default()
                    .fg(theme.modal_key_fg())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {label}"),
                Style::default().fg(theme.modal_description()),
            ),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Left),
        inner.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }),
    );
}

fn changes(count: usize) -> String {
    format!(
        "You have {count} unsaved change{}.",
        if count == 1 { "" } else { "s" }
    )
}

/// Render file creation confirmation dialog
pub fn render_file_create_confirm(frame: &mut Frame, message: &str, theme: &Theme) {
    render_confirm(
        frame,
        theme,
        "Create file?",
        &[message.to_string()],
        &[("y", "Create it"), ("n/Esc", "Cancel")],
    );
}

/// Render the save width confirmation modal
pub fn render_save_width_confirm(frame: &mut Frame, width: u16, theme: &Theme) {
    render_confirm(
        frame,
        theme,
        "Save outline width",
        &[format!("Save width {width}% to the config file?")],
        &[("y", "Save"), ("n/Esc", "Cancel")],
    );
}

/// Render the save before quit confirmation modal
pub fn render_save_before_quit_confirm(frame: &mut Frame, edit_count: usize, theme: &Theme) {
    render_confirm(
        frame,
        theme,
        "Unsaved changes",
        &[changes(edit_count), "Save before quitting?".to_string()],
        &[
            ("y", "Save and quit"),
            ("q", "Quit without saving"),
            ("Esc", "Cancel"),
        ],
    );
}

/// Render the save before navigate confirmation modal
pub fn render_save_before_nav_confirm(frame: &mut Frame, edit_count: usize, theme: &Theme) {
    render_confirm(
        frame,
        theme,
        "Unsaved changes",
        &[
            changes(edit_count),
            "Save before leaving this file?".to_string(),
        ],
        &[
            ("y", "Save and go"),
            ("d", "Discard and go"),
            ("q", "Discard and quit"),
            ("Esc", "Cancel"),
        ],
    );
}

/// Render the command palette with fuzzy search
pub fn render_command_palette(frame: &mut Frame, app: &App, theme: &Theme) {
    use crate::tui::app::PALETTE_COMMANDS;

    let area = popup_area(frame.area(), 60, 50, 35, 10);
    let palette = &app.command_palette;
    let inner = popup_frame(
        frame,
        theme,
        area,
        popup_title(theme, "Commands", Some(palette.filtered.len().to_string())),
    );

    // The input, the commands as a list that scrolls to keep the selection
    // in view, and the selected command's description.
    let [input, list_area, _, about] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    popup_input(frame, theme, input, ": ", &palette.query, true);

    if palette.filtered.is_empty() {
        frame.render_widget(popup_note(theme, "No matching commands"), list_area);
        return;
    }

    let muted = Style::default().fg(theme.modal_description());
    let items: Vec<ListItem> = palette
        .filtered
        .iter()
        .map(|&index| {
            let command = &PALETTE_COMMANDS[index];
            let mut spans = vec![Span::raw(command.name)];
            if !command.aliases.is_empty() {
                spans.push(Span::styled(
                    format!("  {}", command.aliases.join(", ")),
                    muted,
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(palette.selected));
    frame.render_stateful_widget(popup_list(theme, items), list_area, &mut state);

    if let Some(&index) = palette.filtered.get(palette.selected) {
        frame.render_widget(
            popup_note(theme, PALETTE_COMMANDS[index].description),
            about,
        );
    }
}

/// Render the file picker modal
///
/// The directory's markdown files, then its subdirectories, each with a
/// trailing `/`. A search filters both.
pub fn render_file_picker(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let picker = &app.file_picker;
    let searching = picker.active || !picker.query.is_empty();

    let dir = app.effective_picker_dir();
    let dir_label = dir
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| dir.display().to_string(), str::to_string);
    let widest = picker
        .files
        .iter()
        .chain(&picker.dirs)
        .filter_map(|path| path.file_name())
        .map(|name| name.to_string_lossy().chars().count() + 8)
        .max()
        .unwrap_or(20);
    let rows = app.file_picker_item_count().max(1) + if searching { 2 } else { 0 };
    let popup_area = fitted_area(area, rows, widest, 70, (40, 5));
    let inner = popup_frame(
        frame,
        theme,
        popup_area,
        popup_title(theme, "Files", Some(format!("{dir_label}/"))),
    );

    let [input, list_area] = Layout::vertical([
        Constraint::Length(if searching { 2 } else { 0 }),
        Constraint::Min(1),
    ])
    .areas(inner);
    if searching {
        popup_input(frame, theme, input, "/ ", &picker.query, picker.active);
    }

    if app.file_picker_item_count() == 0 {
        let note = if picker.files.is_empty() && picker.dirs.is_empty() {
            "No markdown files or folders here"
        } else {
            "Nothing matches"
        };
        frame.render_widget(popup_note(theme, note), list_area);
        return;
    }

    let muted = Style::default().fg(theme.modal_description());
    let name = |path: &std::path::Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("?")
            .to_string()
    };
    let files = picker.filtered_file_indices.iter().map(|&index| {
        let path = &picker.files[index];
        let mut spans = vec![Span::raw(name(path))];
        if path == &app.current_file_path {
            spans.push(Span::styled("  open", muted));
        }
        ListItem::new(Line::from(spans))
    });
    let dirs = picker.filtered_dir_indices.iter().map(|&index| {
        ListItem::new(Line::from(Span::styled(
            format!("{}/", name(&picker.dirs[index])),
            Style::default().fg(theme.modal_key_fg()),
        )))
    });
    let items: Vec<ListItem> = files.chain(dirs).collect();
    let mut state = ListState::default().with_selected(picker.selected);
    frame.render_stateful_widget(popup_list(theme, items), list_area, &mut state);
}
