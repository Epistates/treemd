mod layout;
mod popups;
mod table;
mod util;

use layout::{DynamicLayout, Section};

use crate::tui::app::{App, Focus};
use crate::tui::theme::Theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
};
use popups::{render_cell_edit_overlay, render_help_popup, render_link_picker, render_theme_picker};
use table::render_table;
use util::detect_checkbox_in_text;

pub fn render(frame: &mut Frame, app: &mut App) {
    // Update content metrics before rendering to ensure content height and scroll are correct
    app.update_content_metrics();

    let area = frame.area();

    // Create dynamic main layout
    let main_layout = DynamicLayout::vertical(area)
        .section(Section::Title, Constraint::Length(2))
        .section_if(app.show_search, Section::Search, Constraint::Length(3))
        .section(Section::Content, Constraint::Min(0))
        .section(Section::Status, Constraint::Length(1))
        .build();

    // Render title bar
    render_title_bar(frame, app, main_layout.require(Section::Title));

    // Render search bar if visible
    if let Some(search_area) = main_layout.get(Section::Search) {
        render_search_bar(frame, app, search_area);
    }

    // Create horizontal layout for outline and content (conditional based on outline visibility)
    let content_area = main_layout.require(Section::Content);
    let content_chunks = if app.show_outline {
        let content_width = 100 - app.outline_width;
        Layout::horizontal([
            Constraint::Percentage(app.outline_width),
            Constraint::Percentage(content_width),
        ])
        .split(content_area)
    } else {
        // Full-width content when outline is hidden
        Layout::horizontal([Constraint::Percentage(100)]).split(content_area)
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
    render_status_bar(frame, app, main_layout.require(Section::Status));

    // Render help popup if shown
    if app.show_help {
        render_help_popup(frame, app, area);
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

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
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

    frame.render_widget(paragraph, area);
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

use crate::parser::content::parse_content;
use crate::parser::output::{Block as ContentBlock, InlineElement};
use crate::tui::syntax::SyntaxHighlighter;

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
        // Check if any element in this block is selected (block-level or inline)
        let is_block_selected = selected_element_id
            .map(|id| id.block_idx == block_idx)
            .unwrap_or(false);

        // Get the selected inline element index within this block (if any)
        let selected_inline_idx = selected_element_id
            .filter(|id| id.block_idx == block_idx)
            .and_then(|id| id.sub_idx);

        match block {
            ContentBlock::Heading {
                level,
                content,
                inline,
            } => {
                // Render sub-heading with appropriate styling
                let mut formatted = if !inline.is_empty() {
                    render_inline_elements(inline, theme, selected_inline_idx)
                } else {
                    format_inline_markdown(content, theme)
                };

                // Apply heading style to all spans
                let heading_style = Style::default()
                    .fg(theme.heading_color(*level))
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

                for span in &mut formatted {
                    span.style = heading_style;
                }

                // Add selection indicator if selected
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
            ContentBlock::Paragraph { content, inline } => {
                let mut formatted = if !inline.is_empty() {
                    render_inline_elements(inline, theme, selected_inline_idx)
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
                        // Simple single-line item (or item with nested blocks)
                        let formatted = if !item.inline.is_empty() {
                            render_inline_elements(&item.inline, theme, None)
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

                    // Render nested blocks within this list item (e.g., code blocks)
                    for nested_block in &item.blocks {
                        let nested_lines =
                            render_block_to_lines(nested_block, highlighter, theme);
                        for nested_line in nested_lines {
                            // Add indentation for nested content (align with list item text)
                            let mut indented_spans = vec![Span::raw("     ")]; // 5 spaces indent
                            indented_spans.extend(nested_line.spans);
                            lines.push(Line::from(indented_spans));
                        }
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
        ContentBlock::Heading {
            level,
            content,
            inline,
        } => {
            // Render heading with appropriate styling
            let mut formatted = if !inline.is_empty() {
                render_inline_elements(inline, theme, None)
            } else {
                format_inline_markdown(content, theme)
            };

            // Apply heading style to all spans
            let heading_style = Style::default()
                .fg(theme.heading_color(*level))
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

            for span in &mut formatted {
                span.style = heading_style;
            }

            lines.push(Line::from(formatted));
        }
        ContentBlock::Paragraph { content, inline } => {
            let formatted = if !inline.is_empty() {
                render_inline_elements(inline, theme, None)
            } else {
                format_inline_markdown(content, theme)
            };
            lines.push(Line::from(formatted));
        }
        ContentBlock::Code {
            language, content, ..
        } => {
            let lang_str = language.as_deref().unwrap_or("");

            // Opening fence
            lines.push(Line::from(vec![Span::styled(
                format!("```{}", lang_str),
                theme.code_fence_style(),
            )]));

            // Highlighted code
            let highlighted = highlighter.highlight_code(content, lang_str);
            lines.extend(highlighted);

            // Closing fence
            lines.push(Line::from(vec![Span::styled(
                "```".to_string(),
                theme.code_fence_style(),
            )]));
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

fn render_inline_elements(
    elements: &[InlineElement],
    theme: &Theme,
    selected_inline_idx: Option<usize>,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    for (idx, element) in elements.iter().enumerate() {
        let is_selected = selected_inline_idx == Some(idx);

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
                if is_selected {
                    // Add selection indicator before selected link
                    spans.push(Span::styled(
                        "▸ ",
                        Style::default()
                            .fg(Color::Rgb(100, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                let style = if is_selected {
                    // Highlighted selected link - matches table cell selection style
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(100, 200, 255))
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    // Normal link style
                    Style::default()
                        .fg(Color::Rgb(100, 150, 255))
                        .add_modifier(Modifier::UNDERLINED)
                };
                spans.push(Span::styled(text.clone(), style));
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
                if is_selected {
                    // Add selection indicator before selected image
                    spans.push(Span::styled(
                        "▸ ",
                        Style::default()
                            .fg(Color::Rgb(100, 200, 255))
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                let style = if is_selected {
                    // Highlighted selected image
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(100, 200, 255))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(180, 180, 200))
                };
                spans.push(Span::styled(format!("🖼 {}", alt), style));
            }
        }
    }

    if spans.is_empty() {
        spans.push(Span::raw(""));
    }

    spans
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
