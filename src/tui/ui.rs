use crate::tui::app::{App, Focus};
use crate::tui::help_text;
use crate::tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};

pub fn render(frame: &mut Frame, app: &mut App) {
    // Update content metrics before rendering to ensure content height and scroll are correct
    app.update_content_metrics();

    let area = frame.area();

    // Create main layout with title bar and content
    let main_chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(area);

    // Render title bar
    render_title_bar(frame, app, main_chunks[0]);

    // Create horizontal layout for outline and content (conditional based on outline visibility)
    let content_chunks = if app.show_outline {
        let content_width = 100 - app.outline_width;
        Layout::horizontal([
            Constraint::Percentage(app.outline_width),
            Constraint::Percentage(content_width),
        ])
        .split(main_chunks[1])
    } else {
        // Full-width content when outline is hidden
        Layout::horizontal([Constraint::Percentage(100)]).split(main_chunks[1])
    };

    // Render outline (left pane) only if visible
    if app.show_outline {
        render_outline(frame, app, content_chunks[0]);
        // Render content (right pane)
        render_content(frame, app, content_chunks[1]);
    } else {
        // Full-width content
        render_content(frame, app, content_chunks[0]);
    }

    // Render status bar at bottom
    let status_area = Rect {
        x: area.x,
        y: area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    render_status_bar(frame, app, status_area);

    // Render help popup if shown
    if app.show_help {
        render_help_popup(frame, app, area);
    }

    // Render search overlay if shown
    if app.show_search {
        render_search_overlay(frame, app, area);
    }

    // Render theme picker if shown
    if app.show_theme_picker {
        render_theme_picker(frame, app, area);
    }

    // Render cell edit overlay if in cell edit mode
    if matches!(app.mode, crate::tui::app::AppMode::CellEdit) {
        render_cell_edit_overlay(frame, app, area);
    }

    // Render link picker if in link follow mode with links
    if matches!(app.mode, crate::tui::app::AppMode::LinkFollow) && !app.links_in_view.is_empty() {
        render_link_picker(frame, app, area);
    }
}

fn render_title_bar(frame: &mut Frame, app: &App, area: Rect) {
    let heading_count = app.document.headings.len();
    let title_text = format!("treemd - {} - {} headings", app.filename, heading_count);

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Rgb(100, 200, 255))
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, area);
}

fn render_outline(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;

    let items: Vec<ListItem> = app
        .outline_items
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.level.saturating_sub(1));
            let prefix = "#".repeat(item.level);

            // Show expand/collapse indicator if heading has children
            let expand_indicator = if item.has_children {
                if item.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };

            // Show bookmark indicator if this item's text matches the bookmark
            let bookmark_indicator = if app.bookmark_position.as_deref() == Some(&item.text) {
                "⚑ "
            } else {
                ""
            };

            // Color headings by level using theme
            let color = theme.heading_color(item.level);

            let text = format!(
                "{}{}{}{} {}",
                indent, expand_indicator, bookmark_indicator, prefix, item.text
            );
            let line = Line::from(Span::styled(text, Style::default().fg(color)));

            ListItem::new(line)
        })
        .collect();

    let block_style = theme.border_style(app.focus == Focus::Outline);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(block_style)
                .title(" Outline "),
        )
        .style(theme.content_style())
        .highlight_style(theme.selection_style())
        .highlight_symbol("► ");

    frame.render_stateful_widget(list, area, &mut app.outline_state);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .style(Style::default().fg(Color::Rgb(80, 80, 100)));

    frame.render_stateful_widget(
        scrollbar,
        area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.outline_scroll_state,
    );
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::app::AppMode;

    let theme = &app.theme;
    let block_style = theme.border_style(app.focus == Focus::Content);

    // Get content for selected section and determine title
    let (content_text, title) = if let Some(heading_text) = app.selected_heading_text() {
        let content = app
            .document
            .extract_section(heading_text)
            .unwrap_or_else(|| app.document.content.clone());

        // Add link count to title if in link follow mode
        let title = if app.mode == AppMode::LinkFollow && !app.links_in_view.is_empty() {
            format!(" {} [Links: {}] ", heading_text, app.links_in_view.len())
        } else {
            format!(" {} ", heading_text)
        };

        (content, title)
    } else {
        let title = if app.mode == AppMode::LinkFollow && !app.links_in_view.is_empty() {
            format!(" Content [Links: {}] ", app.links_in_view.len())
        } else {
            " Content ".to_string()
        };
        (app.document.content.clone(), title)
    };

    // Enhanced markdown rendering with syntax highlighting
    // Pass interactive state if in interactive mode
    let (selected_element_id, interactive_state_ref) = if app.mode == AppMode::Interactive {
        (
            app.interactive_state.current_element().map(|elem| elem.id),
            Some(&app.interactive_state),
        )
    } else {
        (None, None)
    };

    let rendered_text = render_markdown_enhanced(
        &content_text,
        &app.highlighter,
        theme,
        selected_element_id,
        interactive_state_ref,
    );

    let paragraph = Paragraph::new(rendered_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(block_style)
                .title(title),
        )
        .style(theme.content_style())
        .wrap(Wrap { trim: false })
        .scroll((app.content_scroll, 0));

    frame.render_widget(paragraph, area);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .style(Style::default().fg(Color::Rgb(80, 80, 100)));

    frame.render_stateful_widget(
        scrollbar,
        area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.content_scroll_state.clone(),
    );
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::app::AppMode;

    // If there's a status message, display it prominently
    if let Some(ref msg) = app.status_message {
        let status = Paragraph::new(msg.clone()).style(
            Style::default()
                .bg(Color::Rgb(0, 80, 120))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(status, area);
        return;
    }

    let status_text = if app.mode == AppMode::LinkFollow {
        // Link follow mode status
        let link_count = app.links_in_view.len();
        let selected = app.selected_link_idx.map(|i| i + 1).unwrap_or(0);

        let link_info = if link_count > 0 {
            // Show current link details
            let current_link = app
                .selected_link_idx
                .and_then(|idx| app.links_in_view.get(idx));

            if let Some(link) = current_link {
                use crate::parser::LinkTarget;
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
                        // Truncate long URLs
                        if url.len() > 40 {
                            format!("{}...", &url[..37])
                        } else {
                            url.clone()
                        }
                    }
                };

                format!(
                    "Link {}/{}: \"{}\" → {} • Tab:Next • Enter:Follow • Esc:Exit",
                    selected, link_count, link.text, target_str
                )
            } else {
                format!(
                    "Link {}/{} • Tab:Next • 1-9:Jump • Enter:Follow • Esc:Exit",
                    selected, link_count
                )
            }
        } else {
            "No links in current section • Press Esc to exit".to_string()
        };

        format!(" [LINK FOLLOW MODE] {} ", link_info)
    } else {
        // Normal mode status
        let focus_indicator = match app.focus {
            Focus::Outline => "Outline",
            Focus::Content => "Content",
        };

        let selected_idx = app.outline_state.selected().unwrap_or(0);
        let total = app.outline_items.len();
        let percentage = if total > 0 {
            (selected_idx + 1) * 100 / total
        } else {
            0
        };

        let outline_status = if app.show_outline {
            format!("Outline:{}%", app.outline_width)
        } else {
            "Outline:Hidden".to_string()
        };

        let bookmark_indicator = if app.bookmark_position.is_some() {
            " ⚑"
        } else {
            ""
        };

        let history_indicator = if !app.file_history.is_empty() {
            format!(" ← {} ", app.file_history.len())
        } else {
            "".to_string()
        };

        format!(
            " [{}] {}/{} ({}%){}{} • {} • i:Interactive • f:Links • b:Back • w:View • []:Size • m:Mark • y/Y:Copy • t:Theme • ?:Help ",
            focus_indicator,
            selected_idx + 1,
            total,
            percentage,
            bookmark_indicator,
            history_indicator,
            outline_status
        )
    };

    let theme_name = format!(" Theme:{} ", app.theme.name);
    let status_text = format!("{}{}", status_text, theme_name);

    let status_style = if app.mode == AppMode::LinkFollow {
        Style::default()
            .bg(Color::Rgb(0, 100, 0))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        app.theme.status_bar_style()
    };

    let status = Paragraph::new(status_text).style(status_style);

    frame.render_widget(status, area);
}

fn render_help_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_area(area, 70, 80);
    let theme = &app.theme;

    // Clear the area
    frame.render_widget(Clear, popup_area);

    // Build help text with theme colors
    let help_text = help_text::build_help_text(theme);

    let help_text_len = help_text.len();

    let paragraph = Paragraph::new(help_text)
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

fn centered_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn render_link_picker(frame: &mut Frame, app: &App, area: Rect) {
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
                    link_text,
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
                Span::styled(link_text, Style::default().fg(theme.modal_text())),
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

use crate::parser::content::parse_content;
use crate::parser::output::{Alignment, Block as ContentBlock, InlineElement};
use crate::tui::syntax::SyntaxHighlighter;
use unicode_width::UnicodeWidthStr;

fn render_markdown_enhanced(
    content: &str,
    highlighter: &SyntaxHighlighter,
    theme: &Theme,
    selected_element_id: Option<crate::tui::interactive::ElementId>,
    interactive_state: Option<&crate::tui::interactive::InteractiveState>,
) -> Text<'static> {
    let mut lines = Vec::new();

    // Parse content into structured blocks
    let blocks = parse_content(content, 0);

    for (block_idx, block) in blocks.iter().enumerate() {
        let is_block_selected = selected_element_id
            .map(|id| id.block_idx == block_idx && id.sub_idx.is_none())
            .unwrap_or(false);

        match block {
            ContentBlock::Paragraph { content, inline } => {
                let mut formatted = if !inline.is_empty() {
                    render_inline_elements(inline, theme)
                } else {
                    format_inline_markdown(content, theme)
                };

                // Add selection indicator
                if is_block_selected {
                    formatted.insert(
                        0,
                        Span::styled(
                            "→ ",
                            Style::default()
                                .fg(Color::Rgb(100, 200, 255))
                                .add_modifier(Modifier::BOLD),
                        ),
                    );
                }

                lines.push(Line::from(formatted));
            }
            ContentBlock::Code {
                language, content, ..
            } => {
                // Opening fence
                let lang_str = language.as_deref().unwrap_or("");

                let mut fence_spans = vec![];
                if is_block_selected {
                    fence_spans.push(Span::styled(
                        "→ ",
                        Style::default()
                            .fg(Color::Rgb(100, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                fence_spans.push(Span::styled(
                    format!("```{}", lang_str),
                    theme.code_fence_style(),
                ));

                lines.push(Line::from(fence_spans));

                // Highlighted code
                let highlighted = highlighter.highlight_code(content, lang_str);
                lines.extend(highlighted);

                // Closing fence
                lines.push(Line::from(vec![Span::styled(
                    "```".to_string(),
                    theme.code_fence_style(),
                )]));
            }
            ContentBlock::List { ordered, items } => {
                for (idx, item) in items.iter().enumerate() {
                    // Check if this specific list item is selected
                    let is_item_selected = selected_element_id
                        .map(|id| id.block_idx == block_idx && id.sub_idx == Some(idx))
                        .unwrap_or(false);

                    // Check if content has nested items (contains newlines with indentation)
                    let has_nested = item.content.contains('\n');

                    if has_nested {
                        // Render multi-line item with nested items
                        let content_lines = item.content.lines();
                        for (line_idx, line) in content_lines.enumerate() {
                            if line_idx == 0 {
                                // First line: use regular list marker
                                let mut spans = vec![];

                                // Add selection indicator for checkboxes
                                if is_item_selected {
                                    spans.push(Span::styled(
                                        "→ ",
                                        Style::default()
                                            .fg(Color::Rgb(100, 200, 255))
                                            .add_modifier(Modifier::BOLD),
                                    ));
                                }

                                let prefix = if let Some(checked) = item.checked {
                                    let checkbox = if checked { "☑" } else { "☐" };
                                    format!("  {} ", checkbox)
                                } else if *ordered {
                                    format!("  {}. ", idx + 1)
                                } else {
                                    "  • ".to_string()
                                };
                                let formatted = format_inline_markdown(line, theme);
                                spans.push(Span::styled(
                                    prefix,
                                    Style::default().fg(theme.list_bullet),
                                ));
                                spans.extend(formatted);
                                lines.push(Line::from(spans));
                            } else {
                                // Nested items: detect indentation and add bullet/checkbox
                                let trimmed = line.trim_start();
                                let indent_count = line.len() - trimmed.len();
                                if indent_count > 0 {
                                    // Check if this is a task list item by looking for checkbox in text
                                    let (is_task, checked, text_after_marker) =
                                        detect_checkbox_in_text(trimmed);

                                    let indent = " ".repeat(indent_count + 2); // Base indent + 2

                                    let marker = if is_task {
                                        // Task list item with checkbox
                                        if checked { "☑ " } else { "☐ " }
                                    } else {
                                        // Regular bullet
                                        "• "
                                    };

                                    let formatted =
                                        format_inline_markdown(text_after_marker, theme);
                                    let mut spans = vec![
                                        Span::raw(indent),
                                        Span::styled(
                                            marker,
                                            Style::default().fg(theme.list_bullet),
                                        ),
                                    ];
                                    spans.extend(formatted);
                                    lines.push(Line::from(spans));
                                } else {
                                    // Empty line or continuation
                                    lines.push(Line::from(line.to_string()));
                                }
                            }
                        }
                    } else {
                        // Simple single-line item
                        let formatted = if !item.inline.is_empty() {
                            render_inline_elements(&item.inline, theme)
                        } else {
                            format_inline_markdown(&item.content, theme)
                        };

                        let mut spans = vec![];

                        // Add selection indicator for checkboxes
                        if is_item_selected {
                            spans.push(Span::styled(
                                "→ ",
                                Style::default()
                                    .fg(Color::Rgb(100, 200, 255))
                                    .add_modifier(Modifier::BOLD),
                            ));
                        }

                        let prefix = if let Some(checked) = item.checked {
                            let checkbox = if checked { "☑" } else { "☐" };
                            format!("  {} ", checkbox)
                        } else if *ordered {
                            format!("  {}. ", idx + 1)
                        } else {
                            "  • ".to_string()
                        };

                        spans.push(Span::styled(prefix, Style::default().fg(theme.list_bullet)));
                        spans.extend(formatted);
                        lines.push(Line::from(spans));
                    }
                }
            }
            ContentBlock::Blockquote {
                content,
                blocks: nested,
            } => {
                // If we have nested blocks, render them recursively
                if !nested.is_empty() {
                    for nested_block in nested {
                        let nested_lines = render_block_to_lines(nested_block, highlighter, theme);
                        for nested_line in nested_lines {
                            let mut spans = vec![Span::styled(
                                "│ ",
                                Style::default().fg(theme.blockquote_border),
                            )];
                            spans.extend(nested_line.spans.into_iter().map(|span| {
                                Span::styled(
                                    span.content,
                                    span.style
                                        .fg(theme.blockquote_fg)
                                        .add_modifier(Modifier::ITALIC),
                                )
                            }));
                            lines.push(Line::from(spans));
                        }
                    }
                } else {
                    // Fallback to raw content
                    for line in content.lines() {
                        let formatted = format_inline_markdown(line, theme);
                        let mut spans = vec![Span::styled(
                            "│ ",
                            Style::default().fg(theme.blockquote_border),
                        )];
                        spans.extend(formatted.into_iter().map(|span| {
                            Span::styled(
                                span.content,
                                span.style
                                    .fg(theme.blockquote_fg)
                                    .add_modifier(Modifier::ITALIC),
                            )
                        }));
                        lines.push(Line::from(spans));
                    }
                }
            }
            ContentBlock::Table {
                headers,
                alignments,
                rows,
            } => {
                // Get selected cell position if in table navigation mode
                let (in_table_mode, selected_cell) = if is_block_selected {
                    let in_mode = interactive_state
                        .map(|state| state.is_in_table_mode())
                        .unwrap_or(false);
                    let cell = if in_mode {
                        interactive_state.and_then(|state| state.get_table_position())
                    } else {
                        None
                    };
                    (in_mode, cell)
                } else {
                    (false, None)
                };

                let table_lines = render_table(
                    headers,
                    alignments,
                    rows,
                    theme,
                    is_block_selected,
                    in_table_mode,
                    selected_cell,
                );
                lines.extend(table_lines);
            }
            ContentBlock::Image { alt, .. } => {
                // Render image as placeholder with alt text only
                let mut image_spans = vec![];
                if is_block_selected {
                    image_spans.push(Span::styled(
                        "→ ",
                        Style::default()
                            .fg(Color::Rgb(100, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                image_spans.push(Span::styled(
                    "🖼 ",
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                ));
                image_spans.push(Span::styled(
                    alt.clone(),
                    Style::default()
                        .fg(Color::Rgb(100, 150, 200))
                        .add_modifier(Modifier::ITALIC),
                ));
                lines.push(Line::from(image_spans));
            }
            ContentBlock::Details {
                summary,
                blocks: nested,
                ..
            } => {
                // Check if this details block is expanded
                let element_id = crate::tui::interactive::ElementId {
                    block_idx,
                    sub_idx: None,
                };
                let is_expanded = interactive_state
                    .map(|state| state.is_details_expanded(element_id))
                    .unwrap_or(false);

                // Render details block with expand/collapse indicator
                let mut summary_spans = vec![];

                // Add selection indicator
                if is_block_selected {
                    summary_spans.push(Span::styled(
                        "→ ",
                        Style::default()
                            .fg(Color::Rgb(100, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    ));
                }

                // Show ▼ when expanded, ▶ when collapsed
                let indicator = if is_expanded { "▼ " } else { "▶ " };
                summary_spans.push(Span::styled(
                    indicator,
                    Style::default().fg(theme.list_bullet),
                ));
                summary_spans.push(Span::styled(
                    summary.clone(),
                    Style::default()
                        .fg(theme.heading_color(3))
                        .add_modifier(Modifier::BOLD),
                ));

                lines.push(Line::from(summary_spans));

                // Only render nested content if expanded
                if is_expanded {
                    for nested_block in nested {
                        let nested_lines = render_block_to_lines(nested_block, highlighter, theme);
                        for nested_line in nested_lines {
                            let mut spans = vec![Span::raw("  ")]; // Indent
                            spans.extend(nested_line.spans);
                            lines.push(Line::from(spans));
                        }
                    }
                }
            }
            ContentBlock::HorizontalRule => {
                lines.push(Line::from(vec![Span::styled(
                    "─".repeat(60),
                    Style::default().fg(Color::Rgb(80, 80, 100)),
                )]));
            }
        }

        // Add blank line after most blocks for spacing
        lines.push(Line::from(""));
    }

    Text::from(lines)
}

fn render_block_to_lines(
    block: &ContentBlock,
    highlighter: &SyntaxHighlighter,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match block {
        ContentBlock::Paragraph { content, inline } => {
            let formatted = if !inline.is_empty() {
                render_inline_elements(inline, theme)
            } else {
                format_inline_markdown(content, theme)
            };
            lines.push(Line::from(formatted));
        }
        ContentBlock::Code {
            language, content, ..
        } => {
            let lang_str = language.as_deref().unwrap_or("");
            let highlighted = highlighter.highlight_code(content, lang_str);
            lines.extend(highlighted);
        }
        ContentBlock::Details {
            summary,
            blocks: nested,
            ..
        } => {
            // Render details with collapsed indicator
            let summary_spans = vec![
                Span::styled("▶ ", Style::default().fg(theme.list_bullet)),
                Span::styled(
                    summary.clone(),
                    Style::default()
                        .fg(theme.heading_color(3))
                        .add_modifier(Modifier::BOLD),
                ),
            ];
            lines.push(Line::from(summary_spans));

            // Render nested content (indented)
            for nested_block in nested {
                let nested_lines = render_block_to_lines(nested_block, highlighter, theme);
                for nested_line in nested_lines {
                    let mut spans = vec![Span::raw("  ")];
                    spans.extend(nested_line.spans);
                    lines.push(Line::from(spans));
                }
            }
        }
        _ => {
            // For other blocks, use simple text rendering
            lines.push(Line::from(vec![Span::raw("")]));
        }
    }

    lines
}

fn render_inline_elements(elements: &[InlineElement], theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    for element in elements {
        match element {
            InlineElement::Text { value } => {
                spans.push(Span::styled(value.clone(), theme.text_style()));
            }
            InlineElement::Strong { value } => {
                spans.push(Span::styled(value.clone(), theme.bold_style()));
            }
            InlineElement::Emphasis { value } => {
                spans.push(Span::styled(value.clone(), theme.italic_style()));
            }
            InlineElement::Code { value } => {
                spans.push(Span::styled(value.clone(), theme.inline_code_style()));
            }
            InlineElement::Link { text, .. } => {
                spans.push(Span::styled(
                    text.clone(),
                    Style::default()
                        .fg(Color::Rgb(100, 150, 255))
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            InlineElement::Strikethrough { value } => {
                spans.push(Span::styled(
                    value.clone(),
                    Style::default()
                        .fg(Color::Rgb(120, 120, 120))
                        .add_modifier(Modifier::CROSSED_OUT),
                ));
            }
            InlineElement::Image { alt, .. } => {
                spans.push(Span::styled(
                    format!("🖼 {}", alt),
                    Style::default().fg(Color::Rgb(150, 150, 180)),
                ));
            }
        }
    }

    if spans.is_empty() {
        spans.push(Span::raw(""));
    }

    spans
}

fn render_table(
    headers: &[String],
    alignments: &[Alignment],
    rows: &[Vec<String>],
    theme: &Theme,
    is_selected: bool,
    in_table_mode: bool,
    selected_cell: Option<(usize, usize)>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if headers.is_empty() {
        return lines;
    }

    // Calculate column widths using Unicode display width
    let col_count = headers.len();
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.width()).collect();

    for row in rows {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            col_widths[i] = col_widths[i].max(cell.width());
        }
    }

    // Add padding
    for width in &mut col_widths {
        *width += 2; // 1 space on each side
    }

    // Top border (add selection indicator or spacing)
    let mut top_border_spans = vec![];

    if in_table_mode {
        // In table mode, add spacing to align with row arrows
        top_border_spans.push(Span::raw("  "));
    } else if is_selected {
        // Not in table nav mode: show arrow if table is selected as element
        top_border_spans.push(Span::styled(
            "→ ",
            Style::default()
                .fg(Color::Rgb(100, 200, 255))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let mut top_border = String::from("┌");
    for (i, &width) in col_widths.iter().enumerate() {
        top_border.push_str(&"─".repeat(width));
        if i < col_widths.len() - 1 {
            top_border.push('┬');
        }
    }
    top_border.push('┐');
    top_border_spans.push(Span::styled(
        top_border,
        Style::default().fg(Color::Rgb(100, 100, 120)),
    ));
    lines.push(Line::from(top_border_spans));

    // Header row (row 0)
    let header_line = render_table_row(
        headers,
        &col_widths,
        alignments,
        &TableRenderContext {
            theme,
            row_num: 0,
            is_header: true,
            in_table_mode,
            is_table_selected: is_selected,
            selected_cell,
        },
    );
    lines.push(header_line);

    // Header separator
    let mut separator_spans = vec![];
    if in_table_mode || is_selected {
        separator_spans.push(Span::raw("  "));
    }
    let mut separator = String::from("├");
    for (i, &width) in col_widths.iter().enumerate() {
        separator.push_str(&"─".repeat(width));
        if i < col_widths.len() - 1 {
            separator.push('┼');
        }
    }
    separator.push('┤');
    separator_spans.push(Span::styled(
        separator,
        Style::default().fg(Color::Rgb(100, 100, 120)),
    ));
    lines.push(Line::from(separator_spans));

    // Data rows
    for (row_idx, row) in rows.iter().enumerate() {
        let data_row = row_idx + 1; // +1 because row 0 is header
        let row_line = render_table_row(
            row,
            &col_widths,
            alignments,
            &TableRenderContext {
                theme,
                row_num: data_row,
                is_header: false,
                in_table_mode,
                is_table_selected: is_selected,
                selected_cell,
            },
        );
        lines.push(row_line);
    }

    // Bottom border
    let mut bottom_border_spans = vec![];
    if in_table_mode || is_selected {
        bottom_border_spans.push(Span::raw("  "));
    }
    let mut bottom_border = String::from("└");
    for (i, &width) in col_widths.iter().enumerate() {
        bottom_border.push_str(&"─".repeat(width));
        if i < col_widths.len() - 1 {
            bottom_border.push('┴');
        }
    }
    bottom_border.push('┘');
    bottom_border_spans.push(Span::styled(
        bottom_border,
        Style::default().fg(Color::Rgb(100, 100, 120)),
    ));
    lines.push(Line::from(bottom_border_spans));

    lines
}

struct TableRenderContext<'a> {
    theme: &'a Theme,
    row_num: usize,
    is_header: bool,
    in_table_mode: bool,
    is_table_selected: bool,
    selected_cell: Option<(usize, usize)>,
}

fn render_table_row(
    cells: &[String],
    col_widths: &[usize],
    alignments: &[Alignment],
    ctx: &TableRenderContext,
) -> Line<'static> {
    let mut spans = Vec::new();

    // Add arrow or space to keep table aligned when selected or in table mode
    if ctx.in_table_mode {
        // In table mode: show arrow on selected row, spaces on others
        let is_selected_row = ctx.selected_cell.map(|(r, _)| r) == Some(ctx.row_num);
        if is_selected_row {
            spans.push(Span::styled(
                "→ ",
                Style::default()
                    .fg(Color::Rgb(100, 200, 255))
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw("  ")); // Two spaces to match arrow width
        }
    } else if ctx.is_table_selected {
        // Table selected but not in nav mode: add spacing to align with top arrow
        spans.push(Span::raw("  "));
    }

    spans.push(Span::styled(
        "│",
        Style::default().fg(Color::Rgb(100, 100, 120)),
    ));

    for (i, cell) in cells.iter().enumerate() {
        let width = col_widths.get(i).copied().unwrap_or(10);
        let alignment = alignments.get(i).unwrap_or(&Alignment::Left);

        let cell_text = align_text(cell, width, alignment);

        // Determine if this specific cell is selected
        let is_selected = ctx.selected_cell == Some((ctx.row_num, i));

        let style = if is_selected {
            // Highlighted selected cell
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(100, 200, 255))
                .add_modifier(Modifier::BOLD)
        } else if ctx.is_header {
            Style::default()
                .fg(ctx.theme.heading_color(3))
                .add_modifier(Modifier::BOLD)
        } else {
            ctx.theme.text_style()
        };

        spans.push(Span::styled(cell_text, style));
        spans.push(Span::styled(
            "│",
            Style::default().fg(Color::Rgb(100, 100, 120)),
        ));
    }

    Line::from(spans)
}

/// Detect checkbox markers in text and return (is_task, checked, text_after_marker)
fn detect_checkbox_in_text(text: &str) -> (bool, bool, &str) {
    let trimmed = text.trim_start();

    // Check for [x] or [X] (checked)
    if let Some(stripped) = trimmed
        .strip_prefix("[x]")
        .or_else(|| trimmed.strip_prefix("[X]"))
    {
        return (true, true, stripped.trim_start());
    }

    // Check for [ ] (unchecked)
    if let Some(stripped) = trimmed.strip_prefix("[ ]") {
        return (true, false, stripped.trim_start());
    }

    // Not a task list item
    (false, false, text)
}

fn align_text(text: &str, width: usize, alignment: &Alignment) -> String {
    // Use Unicode display width instead of character/byte length
    let text_width = text.width();

    // If text is longer than width, truncate it
    if text_width >= width {
        // TODO: Proper Unicode-aware truncation
        if width > 5 {
            // Approximate truncation - not perfect but better than nothing
            let approx_chars = width.saturating_sub(5);
            let truncated = text.chars().take(approx_chars).collect::<String>();
            return format!(" {}... ", truncated);
        }
        return format!(" {} ", text);
    }

    // Width includes padding we added earlier
    let content_width = width;

    match alignment {
        Alignment::Left | Alignment::None => {
            // Left-aligned: " text     "
            let right_padding = content_width.saturating_sub(text_width + 1);
            format!(" {}{}", text, " ".repeat(right_padding))
        }
        Alignment::Center => {
            // Center-aligned: "  text   "
            let total_padding = content_width.saturating_sub(text_width);
            let left_pad = total_padding / 2;
            let right_pad = total_padding - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        Alignment::Right => {
            // Right-aligned: "     text "
            let left_padding = content_width.saturating_sub(text_width + 1);
            format!("{}{} ", " ".repeat(left_padding), text)
        }
    }
}

fn format_inline_markdown<'a>(text: &str, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for inline code `code`
        if chars[i] == '`' {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '`' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip closing `
            }
            spans.push(Span::styled(code, theme.inline_code_style()));
        }
        // Check for bold **text**
        else if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            i += 2;
            let mut bold_text = String::new();
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold_text.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2; // Skip closing **
            }
            spans.push(Span::styled(bold_text, theme.bold_style()));
        }
        // Check for italic *text*
        else if chars[i] == '*' {
            if !current.is_empty() {
                spans.push(Span::raw(current.clone()));
                current.clear();
            }
            i += 1;
            let mut italic_text = String::new();
            while i < chars.len() && chars[i] != '*' {
                italic_text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip closing *
            }
            spans.push(Span::styled(italic_text, theme.italic_style()));
        } else {
            current.push(chars[i]);
            i += 1;
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, theme.text_style()));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), theme.text_style()));
    }

    spans
}

fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let search_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4).max(40),
        height: 3,
    };

    frame.render_widget(Clear, search_area);

    let search_text = format!("Search: {}_", app.search_query);
    let paragraph = Paragraph::new(search_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Filter Headings ")
                .style(Style::default().bg(Color::Rgb(30, 30, 50))),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, search_area);
}

fn render_theme_picker(frame: &mut Frame, app: &App, area: Rect) {
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

fn render_cell_edit_overlay(frame: &mut Frame, app: &App, area: Rect) {
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
