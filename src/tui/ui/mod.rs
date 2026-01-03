mod layout;
mod popups;
mod table;
mod util;

use layout::{DynamicLayout, Section};

use crate::tui::app::{App, AppMode, Focus};
use crate::tui::theme::Theme;
use popups::{
    render_cell_edit_overlay, render_command_palette, render_file_create_confirm,
    render_file_picker, render_help_popup, render_link_picker, render_save_before_nav_confirm,
    render_save_before_quit_confirm, render_save_width_confirm, render_theme_picker,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
};
use table::render_table;
use util::detect_checkbox_in_text;

pub fn render(frame: &mut Frame, app: &mut App) {
    // Update content metrics before rendering to ensure content height and scroll are correct
    app.update_content_metrics();

    // Clear expired status messages (auto-dismiss after timeout)
    app.clear_expired_status_message();

    let area = frame.area();

    // Create dynamic main layout
    // Show search bar if: outline search is active OR in document search mode (typing or viewing results)
    let show_search_bar = app.show_search || app.mode == AppMode::DocSearch;
    let main_layout = DynamicLayout::vertical(area)
        .section(Section::Title, Constraint::Length(2))
        .section_if(show_search_bar, Section::Search, Constraint::Length(3))
        .section(Section::Content, Constraint::Min(0))
        .section(Section::Status, Constraint::Length(1))
        .section(Section::Footer, Constraint::Length(1))
        .build();

    // Render title bar
    render_title_bar(frame, app, main_layout.require(Section::Title));

    // Render search bar if visible
    if let Some(search_area) = main_layout.get(Section::Search) {
        render_search_bar(frame, app, search_area);
    }

    // Create horizontal layout for outline and content (conditional based on outline visibility)
    let content_area = main_layout.require(Section::Content);

    // Update viewport height for scroll calculations (subtract 2 for block borders)
    app.set_viewport_height(content_area.height.saturating_sub(2));

    // Minimum widths: outline needs at least 20 cols to be usable, content needs at least 40
    const MIN_OUTLINE_WIDTH: u16 = 20;
    const MIN_CONTENT_WIDTH: u16 = 40;
    const MIN_TOTAL_WIDTH: u16 = MIN_OUTLINE_WIDTH + MIN_CONTENT_WIDTH;

    // Decide whether to show outline based on terminal width
    let effective_show_outline = app.show_outline && content_area.width >= MIN_TOTAL_WIDTH;

    let content_chunks = if effective_show_outline {
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

    // Render outline (left pane) only if effectively visible (user toggle AND enough width)
    if effective_show_outline {
        render_outline(frame, app, content_chunks[0]);
        // Render content (right pane)
        render_content(frame, app, content_chunks[1]);
    } else {
        // Full-width content
        render_content(frame, app, content_chunks[0]);
    }

    // Render status bar at bottom
    render_status_bar(frame, app, main_layout.require(Section::Status));

    // Render keybinding hints footer
    render_footer(frame, app, main_layout.require(Section::Footer));

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

    // Render image modal if viewing an image
    render_image_modal(frame, app, area);

    // Render link picker if in link follow mode with links
    if matches!(app.mode, crate::tui::app::AppMode::LinkFollow) && !app.links_in_view.is_empty() {
        render_link_picker(frame, app, area);
    }

    // Render file picker modal (FileSearch is only used as a fallback for old code)
    if matches!(app.mode, AppMode::FilePicker | AppMode::FileSearch) {
        render_file_picker(frame, app, area);
    }

    // Render file creation confirmation dialog
    if matches!(app.mode, AppMode::ConfirmFileCreate) {
        if let Some(message) = &app.pending_file_create_message {
            render_file_create_confirm(frame, message, &app.theme);
        }
    }

    // Render save width confirmation dialog
    if matches!(app.mode, AppMode::ConfirmSaveWidth) {
        render_save_width_confirm(frame, app.outline_width, &app.theme);
    }

    // Render save before quit confirmation dialog
    if matches!(app.mode, AppMode::ConfirmSaveBeforeQuit) {
        render_save_before_quit_confirm(frame, app.pending_edits.len(), &app.theme);
    }

    // Render save before navigate confirmation dialog
    if matches!(app.mode, AppMode::ConfirmSaveBeforeNav) {
        render_save_before_nav_confirm(frame, app.pending_edits.len(), &app.theme);
    }

    // Render command palette
    if matches!(app.mode, AppMode::CommandPalette) {
        render_command_palette(frame, app, &app.theme);
    }
}

fn render_title_bar(frame: &mut Frame, app: &App, area: Rect) {
    let heading_count = app.document.headings.len();
    let title_text = format!("treemd - {} - {} headings", app.filename, heading_count);

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(app.theme.title_bar_fg)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, area);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Unified search bar rendering for both outline and document search
    let is_doc_search = app.mode == AppMode::DocSearch;

    // Get the current query and display state
    let (query, is_active, match_info) = if is_doc_search {
        let info = if !app.doc_search_matches.is_empty() {
            let current = app.doc_search_current_idx.unwrap_or(0) + 1;
            let total = app.doc_search_matches.len();
            format!(" [{}/{}]", current, total)
        } else if !app.doc_search_query.is_empty() {
            " [no matches]".to_string()
        } else {
            String::new()
        };
        (&app.doc_search_query, app.doc_search_active, info)
    } else {
        // Outline search
        (&app.search_query, app.outline_search_active, String::new())
    };

    // Styling based on search type
    let (label, title, accent_color) = if is_doc_search {
        (
            "Find",
            " Content Search (Tab: switch to Outline) ",
            Color::Cyan,
        )
    } else {
        (
            "Filter",
            " Outline Search (Tab: switch to Content) ",
            Color::Yellow,
        )
    };

    // Build search bar with query
    let query_display = if is_active {
        format!("{}_", query)
    } else {
        query.clone()
    };

    let mut line_spans = vec![
        Span::raw(format!("{}: ", label)),
        Span::styled(
            query_display,
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(match_info),
    ];

    // Add hint text - consistent for both modes
    let hint = if is_active {
        "  (Esc, Ctrl+U)"
    } else {
        "  (Esc, Tab, /: edit)"
    };
    line_spans.push(Span::styled(
        hint.to_string(),
        Style::default().fg(Color::DarkGray),
    ));

    let paragraph = Paragraph::new(Line::from(line_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(accent_color))
                .title(title)
                .style(Style::default().bg(Color::Rgb(30, 30, 50))),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

fn render_outline(frame: &mut Frame, app: &mut App, area: Rect) {
    use crate::tui::app::DOCUMENT_OVERVIEW;
    use util::build_highlighted_line;

    let theme = &app.theme;
    let search_query = if app.show_search && !app.search_query.is_empty() {
        Some(app.search_query.as_str())
    } else {
        None
    };

    let items: Vec<ListItem> = app
        .outline_items
        .iter()
        .map(|item| {
            let indent = "  ".repeat(item.level.saturating_sub(1));

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
            let base_style = Style::default().fg(color);

            // Build prefix (indent + indicators + #'s)
            let prefix_text = if item.text == DOCUMENT_OVERVIEW {
                format!("{}{}{}📄 ", indent, expand_indicator, bookmark_indicator)
            } else {
                let hashes = "#".repeat(item.level);
                format!(
                    "{}{}{}{} ",
                    indent, expand_indicator, bookmark_indicator, hashes
                )
            };

            // Build line with search highlighting using shared utility
            let line = build_highlighted_line(
                vec![Span::styled(prefix_text, base_style)],
                &item.text,
                search_query,
                base_style,
                theme.search_match_style(),
            );

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
        .style(Style::default().fg(theme.scrollbar_fg));

    frame.render_stateful_widget(
        scrollbar,
        area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.outline_scroll_state,
    );
}

fn render_content(frame: &mut Frame, app: &mut App, area: Rect) {
    use crate::tui::app::AppMode;

    // Clone theme early to avoid borrow conflicts
    let theme = app.theme.clone();
    let block_style = theme.border_style(app.focus == Focus::Content);

    // Get content for selected section and determine title
    let (content_text, title) = if let Some(heading_text) = app.selected_heading_text() {
        let content = app
            .document
            .extract_section(heading_text)
            .unwrap_or_else(|| app.document.content.clone());

        // Build title with various indicators
        let raw_indicator = if app.show_raw_source { "[RAW] " } else { "" };
        let title = if app.mode == AppMode::LinkFollow && !app.links_in_view.is_empty() {
            format!(
                " {}{} [Links: {}] ",
                raw_indicator,
                heading_text,
                app.links_in_view.len()
            )
        } else {
            format!(" {}{} ", raw_indicator, heading_text)
        };

        (content, title)
    } else {
        let raw_indicator = if app.show_raw_source { "[RAW] " } else { "" };
        let title = if app.mode == AppMode::LinkFollow && !app.links_in_view.is_empty() {
            format!(
                " {}Content [Links: {}] ",
                raw_indicator,
                app.links_in_view.len()
            )
        } else {
            format!(" {}Content ", raw_indicator)
        };
        (app.document.content.clone(), title)
    };

    // Check if we should render raw source or enhanced markdown
    let mut rendered_text = if app.show_raw_source {
        // Raw source view - show unprocessed markdown
        render_raw_markdown(&content_text, &theme)
    } else {
        // Enhanced markdown rendering with syntax highlighting
        // Pre-extract what we need before passing app as mutable to avoid borrow conflicts
        let selected_element_id = if app.mode == AppMode::Interactive {
            app.interactive_state.current_element().map(|elem| elem.id)
        } else {
            None
        };
        // Clone interactive state to avoid keeping a borrow when passing app as mutable
        let interactive_state = app.interactive_state.clone();

        render_markdown_enhanced(
            &content_text,
            &app.highlighter,
            &theme,
            selected_element_id,
            Some(&interactive_state), // Pass cloned copy to release borrow
        )
    };

    // Apply search highlighting only for document/content search mode
    // Outline search (s) only filters headings, it doesn't highlight content
    if app.mode == AppMode::DocSearch && !app.doc_search_query.is_empty() {
        rendered_text = apply_search_highlighting(
            rendered_text,
            &app.doc_search_query,
            app.doc_search_current_idx,
            app.doc_search_matches.len(),
            &theme,
        );
    }

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

    // Render inline images (first image in content)
    render_inline_images(frame, app, area);

    // Render scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .style(Style::default().fg(theme.scrollbar_fg));

    frame.render_stateful_widget(
        scrollbar,
        area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.content_scroll_state.clone(),
    );
}

fn render_inline_images(frame: &mut Frame, app: &mut App, area: Rect) {
    use ratatui_image::{StatefulImage, Resize, FilterType};
    use crate::tui::interactive::ElementType;

    // Don't render inline when viewing modal
    if app.viewing_image_path.is_some() {
        return;
    }

    let theme = &app.theme;

    // Account for borders and padding
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });

    // Maximum image width: 80% of content area, but reasonable constraints
    let max_image_width = ((inner.width as usize * 80) / 100).max(20) as u16;
    // Max 12 lines per image to avoid covering too much text
    let max_image_height = 12u16;

    // Get currently selected image if in interactive mode
    let selected_image_id = if app.mode == crate::tui::app::AppMode::Interactive {
        app.interactive_state.current_element().and_then(|elem| {
            if matches!(elem.element_type, ElementType::Image { .. }) {
                Some(elem.id)
            } else {
                None
            }
        })
    } else {
        None
    };

    // Render all images that are visible in the current scroll viewport
    for elem in &app.interactive_state.elements {
        if let ElementType::Image { src, block_idx: _, .. } = &elem.element_type {
            let (line_start, line_end) = elem.line_range;

            // Check if this image is visible in current scroll window
            let scroll = app.content_scroll as usize;
            let viewport_height = app.content_viewport_height as usize;
            let viewport_end = scroll + viewport_height;

            // Skip if image is outside visible area
            if line_end < scroll || line_start >= viewport_end {
                continue;
            }

            // Calculate Y position: convert line number to screen coordinate
            // Line positions are relative to the full document, need to account for scroll
            let y_offset = if line_start >= scroll {
                (line_start - scroll) as u16
            } else {
                0
            };

            let image_y = inner.y + y_offset;

            // Only render if there's space on screen
            if image_y >= inner.bottom() {
                continue;
            }

            let available_height = inner.bottom().saturating_sub(image_y);
            if available_height < 3 {
                continue; // Not enough space for image
            }

            let image_height = available_height.min(max_image_height);

            // Resolve image path
            if let Ok(image_path) = app.resolve_image_path(src) {
                // Load and render the image
                if let Ok(img_data) = crate::tui::image_cache::ImageCache::extract_first_frame(&image_path) {
                    if let Some(picker) = &mut app.picker {
                        let protocol = picker.new_resize_protocol(img_data);
                        let resize = Resize::Scale(Some(FilterType::Triangle));

                        // Check if this image is selected
                        let is_selected = selected_image_id == Some(elem.id);

                        // Calculate image area - add border space when selected
                        let image_area = Rect {
                            x: inner.x,
                            y: image_y,
                            width: max_image_width.min(inner.width),
                            height: image_height,
                        };

                        // If selected, render a selection border around the image
                        let render_area = if is_selected {
                            let border_style = Style::default()
                                .fg(theme.selection_indicator_fg)
                                .bg(theme.selection_indicator_bg)
                                .add_modifier(Modifier::BOLD);

                            let border = Block::default()
                                .borders(Borders::ALL)
                                .border_style(border_style)
                                .title(" ▶ Selected ")
                                .title_alignment(ratatui::layout::Alignment::Left);

                            // Render border first
                            frame.render_widget(border.clone(), image_area);

                            // Return inner area for image (inside border)
                            border.inner(image_area)
                        } else {
                            image_area
                        };

                        let img_widget = StatefulImage::new().resize(resize);
                        let mut protocol_state = protocol;
                        frame.render_stateful_widget(img_widget, render_area, &mut protocol_state);
                    }
                }
            }
        }
    }
}

fn render_image_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    use ratatui_image::{StatefulImage, Resize, FilterType};
    use std::time::Duration;

    // Must have frames available
    if !app.is_image_modal_open() || app.modal_gif_frames.is_empty() {
        return;
    }

    // Clone theme colors we need before any mutable borrows
    let theme_background = app.theme.background;
    let theme_foreground = app.theme.foreground;
    let theme_heading_1 = app.theme.heading_1;

    let is_multi_frame = app.modal_gif_frames.len() > 1;

    // Calculate modal area - centered on screen with padding
    let modal_width = (area.width * 80) / 100;
    let modal_height = (area.height * 80) / 100;
    let modal_x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = area.y + (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Get inner area (inside modal border)
    let inner_area = Rect {
        x: modal_area.x + 1,
        y: modal_area.y + 1,
        width: modal_area.width.saturating_sub(2),
        height: modal_area.height.saturating_sub(2),
    };

    // Try to start Kitty native animation for multi-frame GIFs.
    // Kitty handles frame timing internally - no flicker!
    if is_multi_frame && !app.has_kitty_animation() && app.use_kitty_animation {
        // Start animation at center of inner area
        let image_col = inner_area.x + inner_area.width / 4;
        let image_row = inner_area.y + inner_area.height / 4;
        app.start_kitty_animation(image_col, image_row);
    }

    // Check if Kitty is handling animation
    let kitty_animating = app.has_kitty_animation();

    // For software animation (non-Kitty terminals), handle frame timing
    let is_animating = is_multi_frame && !app.modal_animation_paused && !kitty_animating;
    if is_animating {
        if let Some(last_update) = app.modal_last_frame_update {
            let current_frame = &app.modal_gif_frames[app.modal_frame_index];
            let frame_delay = Duration::from_millis(current_frame.delay_ms as u64);

            if last_update.elapsed() >= frame_delay {
                app.modal_frame_index = (app.modal_frame_index + 1) % app.modal_gif_frames.len();
                app.modal_last_frame_update = Some(std::time::Instant::now());
            }
        }
    }

    // Only create a new protocol when frame actually changes (software animation only).
    // Skip this entirely when Kitty handles animation.
    if !kitty_animating {
        let needs_new_protocol = app.modal_last_rendered_frame != Some(app.modal_frame_index);
        if needs_new_protocol {
            if let Some(picker) = &mut app.picker {
                let current_img = app.modal_gif_frames[app.modal_frame_index].image.clone();
                app.viewing_image_state = Some(picker.new_resize_protocol(current_img));
                app.modal_last_rendered_frame = Some(app.modal_frame_index);
            }
        }
    }

    // Get the active protocol for sizing (even Kitty needs this for layout)
    if let Some(protocol_state) = &mut app.viewing_image_state {
        // Calculate image area
        let resize = Resize::Scale(Some(FilterType::Triangle));

        let image_size = protocol_state.size_for(resize.clone(), inner_area);
        let image_area = Rect {
            x: inner_area.x + (inner_area.width.saturating_sub(image_size.width)) / 2,
            y: inner_area.y + (inner_area.height.saturating_sub(image_size.height)) / 2,
            width: image_size.width,
            height: image_size.height,
        };

        // Render background ONLY to areas outside the image to prevent flicker.
        // During animation, we avoid overwriting the image area entirely.
        let bg_style = Style::default()
            .bg(theme_background)
            .fg(theme_foreground)
            .add_modifier(Modifier::DIM);

        // Render background in 4 regions around the modal (L-shaped areas)
        // Top strip (above modal)
        if modal_area.y > area.y {
            let top_bg = Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: modal_area.y - area.y,
            };
            frame.render_widget(ratatui::widgets::Block::default().style(bg_style), top_bg);
        }
        // Bottom strip (below modal)
        let modal_bottom = modal_area.y + modal_area.height;
        if modal_bottom < area.y + area.height {
            let bottom_bg = Rect {
                x: area.x,
                y: modal_bottom,
                width: area.width,
                height: (area.y + area.height) - modal_bottom,
            };
            frame.render_widget(ratatui::widgets::Block::default().style(bg_style), bottom_bg);
        }
        // Left strip (left of modal, between top and bottom)
        if modal_area.x > area.x {
            let left_bg = Rect {
                x: area.x,
                y: modal_area.y,
                width: modal_area.x - area.x,
                height: modal_area.height,
            };
            frame.render_widget(ratatui::widgets::Block::default().style(bg_style), left_bg);
        }
        // Right strip (right of modal, between top and bottom)
        let modal_right = modal_area.x + modal_area.width;
        if modal_right < area.x + area.width {
            let right_bg = Rect {
                x: modal_right,
                y: modal_area.y,
                width: (area.x + area.width) - modal_right,
                height: modal_area.height,
            };
            frame.render_widget(ratatui::widgets::Block::default().style(bg_style), right_bg);
        }

        // Build title with frame info and controls for GIFs
        let title = if is_multi_frame {
            let state = if app.modal_animation_paused { "⏸" } else { "▶" };
            // Show Kitty indicator when using native animation
            let mode = if kitty_animating { "Kitty" } else { "GIF" };
            format!(
                " {} {}/{} {} | ←/→:step Space:play/pause q:close ",
                mode,
                app.modal_frame_index + 1,
                app.modal_gif_frames.len(),
                state
            )
        } else {
            " Image | q/Esc: Close ".to_string()
        };

        // Render modal border (but NOT over image area during animation)
        let modal_border = ratatui::widgets::Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme_heading_1).add_modifier(Modifier::BOLD))
            .title(title)
            .title_alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().bg(theme_background).fg(theme_foreground));

        // When Kitty handles animation, avoid overwriting the image area.
        // The terminal manages the animation layer directly.
        let avoid_image_area = is_animating || kitty_animating;

        // Only render the border frame (not the interior) during animation
        // to avoid overwriting the previous image before new one is drawn
        if avoid_image_area {
            // Render border edges only, preserving image area
            render_border_only(frame, &modal_border, modal_area, image_area);
        } else {
            // Static image or paused - safe to render full modal
            frame.render_widget(modal_border, modal_area);
        }

        // Render image via ratatui-image ONLY when Kitty is NOT handling animation.
        // For Kitty animation, the terminal renders the image directly via graphics protocol.
        if !kitty_animating {
            let img_widget = StatefulImage::new().resize(resize);
            frame.render_stateful_widget(img_widget, image_area, protocol_state);
        }
    }
}

/// Render only the border portions of a block, avoiding the image area.
/// This prevents flickering during GIF animation by not overwriting
/// the previous frame before the new one is drawn.
fn render_border_only(frame: &mut Frame, block: &ratatui::widgets::Block, modal_area: Rect, image_area: Rect) {
    use ratatui::widgets::Widget;

    // Top border row (full width of modal)
    let top_row = Rect {
        x: modal_area.x,
        y: modal_area.y,
        width: modal_area.width,
        height: 1,
    };
    block.clone().render(top_row, frame.buffer_mut());

    // Bottom border row (full width of modal)
    if modal_area.height > 1 {
        let bottom_row = Rect {
            x: modal_area.x,
            y: modal_area.y + modal_area.height - 1,
            width: modal_area.width,
            height: 1,
        };
        block.clone().render(bottom_row, frame.buffer_mut());
    }

    // Left border column (between top and bottom, avoiding image)
    if modal_area.height > 2 {
        let middle_height = modal_area.height - 2;
        // Left side - from border to image start
        let left_strip_width = image_area.x.saturating_sub(modal_area.x);
        if left_strip_width > 0 {
            let left_strip = Rect {
                x: modal_area.x,
                y: modal_area.y + 1,
                width: left_strip_width,
                height: middle_height,
            };
            block.clone().render(left_strip, frame.buffer_mut());
        }

        // Right side - from image end to border
        let image_right = image_area.x + image_area.width;
        let modal_right = modal_area.x + modal_area.width;
        if image_right < modal_right {
            let right_strip = Rect {
                x: image_right,
                y: modal_area.y + 1,
                width: modal_right - image_right,
                height: middle_height,
            };
            block.clone().render(right_strip, frame.buffer_mut());
        }

        // Top padding (above image, inside border)
        let padding_top_height = image_area.y.saturating_sub(modal_area.y + 1);
        if padding_top_height > 0 && image_area.width > 0 {
            let top_pad = Rect {
                x: image_area.x,
                y: modal_area.y + 1,
                width: image_area.width,
                height: padding_top_height,
            };
            block.clone().render(top_pad, frame.buffer_mut());
        }

        // Bottom padding (below image, inside border)
        let image_bottom = image_area.y + image_area.height;
        let modal_inner_bottom = modal_area.y + modal_area.height - 1;
        if image_bottom < modal_inner_bottom {
            let bottom_pad = Rect {
                x: image_area.x,
                y: image_bottom,
                width: image_area.width,
                height: modal_inner_bottom - image_bottom,
            };
            block.clone().render(bottom_pad, frame.buffer_mut());
        }
    }
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

    let status_text = if app.mode == AppMode::Interactive {
        // Interactive mode status with position info
        let total = app.interactive_state.elements.len();
        let current = app
            .interactive_state
            .current_index
            .map(|i| i + 1)
            .unwrap_or(0);
        let percentage = if total > 0 && current > 0 {
            current * 100 / total
        } else {
            0
        };

        // Get element-specific hint (shows current element info)
        let element_hint = app.interactive_state.get_status_hint();

        format!(
            " [INTERACTIVE] {}/{} ({}%) • {}",
            current, total, percentage, element_hint
        )
    } else if app.mode == AppMode::LinkFollow {
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
                    "Link {}/{}: \"{}\" → {}",
                    selected, link_count, link.text, target_str
                )
            } else {
                format!("Link {}/{}", selected, link_count)
            }
        } else {
            "No links in current section".to_string()
        };

        format!(" [LINKS] {} ", link_info)
    } else {
        // Normal mode status - show position based on focus
        let (focus_indicator, position_info) = match app.focus {
            Focus::Outline => {
                let selected_idx = app.outline_state.selected().unwrap_or(0);
                let total = app.outline_items.len();
                let percentage = if total > 0 {
                    (selected_idx + 1) * 100 / total
                } else {
                    0
                };
                (
                    "Outline",
                    format!("{}/{} ({}%)", selected_idx + 1, total, percentage),
                )
            }
            Focus::Content => {
                // Show content scroll position
                let scroll_pos = app.content_scroll as usize;
                let content_height = app.content_height as usize;
                let percentage = if content_height > 0 {
                    ((scroll_pos + 1) * 100 / content_height).min(100)
                } else {
                    0
                };
                (
                    "Content",
                    format!("Line {} ({}%)", scroll_pos + 1, percentage),
                )
            }
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
            " [{}] {}{}{} • {}",
            focus_indicator, position_info, bookmark_indicator, history_indicator, outline_status
        )
    };

    let theme_name = format!(" • Theme:{}", app.theme.name);
    let raw_indicator = if app.show_raw_source { " [RAW]" } else { "" };
    let status_text = format!("{}{}{}", status_text, theme_name, raw_indicator);

    let status_style = if app.mode == AppMode::Interactive {
        Style::default()
            .bg(Color::Rgb(80, 60, 120))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if app.mode == AppMode::LinkFollow {
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

/// Render the footer with context-aware keybinding hints
fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    use crate::tui::app::AppMode;

    let theme = &app.theme;

    // Define keybindings based on current mode
    let keys: Vec<(&str, &str)> = match app.mode {
        AppMode::Interactive => {
            // Check if we're in table mode
            if app.interactive_state.is_in_table_mode() {
                vec![
                    ("j/k", "Row"),
                    ("h/l", "Col"),
                    ("e", "Edit"),
                    ("y", "Copy"),
                    ("Esc", "Exit Table"),
                ]
            } else {
                // Get current element type for context-specific hints
                use crate::tui::interactive::ElementType;
                let element_hint = match app.interactive_state.current_element() {
                    Some(elem) => match &elem.element_type {
                        ElementType::Checkbox { .. } => {
                            vec![("j/k", "Navigate"), ("Space", "Toggle"), ("Esc", "Exit")]
                        }
                        ElementType::Table { .. } => {
                            vec![
                                ("j/k", "Navigate"),
                                ("Enter", "Enter Table"),
                                ("y", "Copy"),
                                ("Esc", "Exit"),
                            ]
                        }
                        ElementType::Link { .. } => {
                            vec![
                                ("j/k", "Navigate"),
                                ("Enter", "Follow"),
                                ("y", "Copy URL"),
                                ("Esc", "Exit"),
                            ]
                        }
                        ElementType::Details { .. } => {
                            vec![("j/k", "Navigate"), ("Enter", "Expand"), ("Esc", "Exit")]
                        }
                        ElementType::CodeBlock { .. } => {
                            vec![("j/k", "Navigate"), ("y", "Copy"), ("Esc", "Exit")]
                        }
                        ElementType::Image { .. } => {
                            vec![("j/k", "Navigate"), ("Enter", "Open"), ("Esc", "Exit")]
                        }
                    },
                    None => vec![("j/k", "Navigate"), ("Enter", "Action"), ("Esc", "Exit")],
                };
                element_hint
            }
        }
        AppMode::LinkFollow => {
            vec![
                ("Tab", "Next Link"),
                ("1-9", "Jump"),
                ("Enter", "Follow"),
                ("y", "Copy URL"),
                ("Esc", "Exit"),
            ]
        }
        AppMode::DocSearch => {
            vec![
                ("n/N", "Next/Prev"),
                ("Tab", "Outline Search"),
                ("Enter", "Accept"),
                ("Esc", "Cancel"),
            ]
        }
        AppMode::CellEdit => {
            vec![("Enter", "Save"), ("Esc", "Cancel")]
        }
        AppMode::CommandPalette => {
            vec![("j/k", "Navigate"), ("Enter", "Select"), ("Esc", "Cancel")]
        }
        _ => {
            // Normal mode - show based on focus
            match app.focus {
                Focus::Outline => {
                    vec![
                        ("j/k", "Navigate"),
                        ("Enter", "Select"),
                        ("/", "Search"),
                        ("i", "Interactive"),
                        ("f", "Links"),
                        ("?", "Help"),
                    ]
                }
                Focus::Content => {
                    vec![
                        ("j/k", "Scroll"),
                        ("/", "Search"),
                        ("i", "Interactive"),
                        ("f", "Links"),
                        ("y", "Copy"),
                        ("?", "Help"),
                    ]
                }
            }
        }
    };

    // Build styled spans using flat_map pattern
    let spans: Vec<Span> = keys
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(format!(" {} ", key), theme.help_key_style()),
                Span::styled(format!("{} ", desc), theme.help_desc_style()),
            ]
        })
        .collect();

    let line = Line::from(spans);
    let footer = Paragraph::new(line).style(theme.footer_style());

    frame.render_widget(footer, area);
}

use crate::parser::content::parse_content;
use crate::parser::output::{Block as ContentBlock, InlineElement};
use crate::parser::utils::parse_inline_html;
use crate::tui::syntax::SyntaxHighlighter;

/// Render raw markdown source with line numbers
fn render_raw_markdown(content: &str, theme: &Theme) -> Text<'static> {
    let lines: Vec<Line<'static>> = content
        .lines()
        .enumerate()
        .map(|(idx, line)| {
            // Line number with subtle styling (using border color for subtlety)
            let line_num = Span::styled(
                format!("{:4} │ ", idx + 1),
                Style::default().fg(theme.border_unfocused),
            );
            // Replace tabs with spaces to avoid terminal rendering artifacts
            let line_content = line.replace('\t', "    ");
            // Raw content with plain text styling
            let content_span = Span::styled(line_content, Style::default().fg(theme.foreground));
            Line::from(vec![line_num, content_span])
        })
        .collect();

    Text::from(lines)
}

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
                ..
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

                // Add selection indicator if selected (with background for visibility)
                if is_block_selected {
                    formatted.insert(
                        0,
                        Span::styled(
                            "→ ",
                            Style::default()
                                .fg(theme.selection_indicator_fg)
                                .bg(theme.selection_indicator_bg)
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

                // Add selection indicator (with background for visibility)
                if is_block_selected {
                    formatted.insert(
                        0,
                        Span::styled(
                            "→ ",
                            Style::default()
                                .fg(theme.selection_indicator_fg)
                                .bg(theme.selection_indicator_bg)
                                .add_modifier(Modifier::BOLD),
                        ),
                    );
                }

                lines.push(Line::from(formatted));

                // If paragraph contains images, add blank lines to reserve space for them
                // Images will be rendered on top at this position, so we need to push text below down
                let has_images = inline.iter().any(|elem| matches!(elem, InlineElement::Image { .. }));
                if has_images {
                    // Reserve space for image (max 12 lines + 1 blank line separator)
                    for _ in 0..13 {
                        lines.push(Line::from(vec![]));
                    }
                }
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
                            .fg(theme.selection_indicator_fg)
                            .bg(theme.selection_indicator_bg)
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
                    // Check if this specific list item (checkbox) is selected
                    let is_item_selected = selected_element_id
                        .map(|id| id.block_idx == block_idx && id.sub_idx == Some(idx))
                        .unwrap_or(false);

                    // Check if a link within this list item is selected
                    use crate::tui::interactive::{LINK_ITEM_MULTIPLIER, LINK_OFFSET};
                    let selected_link_inline_idx = selected_element_id.and_then(|id| {
                        if id.block_idx == block_idx {
                            id.sub_idx.and_then(|sub| {
                                // Decode: check if this is a link sub_idx for this item
                                let item_link_base = idx * LINK_ITEM_MULTIPLIER + LINK_OFFSET;
                                let next_item_link_base =
                                    (idx + 1) * LINK_ITEM_MULTIPLIER + LINK_OFFSET;
                                if sub >= LINK_OFFSET
                                    && sub >= item_link_base
                                    && sub < next_item_link_base
                                {
                                    Some(sub - item_link_base)
                                } else {
                                    None
                                }
                            })
                        } else {
                            None
                        }
                    });

                    // Determine which line should have the pointer based on selected link's line_offset
                    let selected_line_offset: Option<usize> =
                        selected_link_inline_idx.and_then(|inline_idx| {
                            item.inline.get(inline_idx).and_then(|elem| {
                                if let InlineElement::Link { line_offset, .. } = elem {
                                    // Use line_offset if provided, otherwise default to 0
                                    Some(line_offset.unwrap_or(0))
                                } else {
                                    None
                                }
                            })
                        });

                    // For checkboxes, always select line 0; for links, use their line_offset
                    let pointer_line = if is_item_selected {
                        Some(0)
                    } else {
                        selected_line_offset
                    };

                    // Check if content has nested items (contains newlines with indentation)
                    let has_nested = item.content.contains('\n');

                    if has_nested {
                        // Render multi-line item with nested items
                        let content_lines = item.content.lines();
                        for (line_idx, line) in content_lines.enumerate() {
                            // Check if this specific line should have the pointer
                            let show_pointer = pointer_line == Some(line_idx);

                            if line_idx == 0 {
                                // First line: use regular list marker
                                let mut spans = vec![];

                                // Pointer replaces leading spaces, not prepended
                                if show_pointer {
                                    spans.push(Span::styled(
                                        "→ ",
                                        Style::default()
                                            .fg(theme.selection_indicator_fg)
                                            .bg(theme.selection_indicator_bg)
                                            .add_modifier(Modifier::BOLD),
                                    ));
                                } else {
                                    spans.push(Span::raw("  "));
                                }

                                let prefix = if let Some(checked) = item.checked {
                                    let checkbox = if checked { "☑" } else { "☐" };
                                    format!("{} ", checkbox)
                                } else if *ordered {
                                    format!("{}. ", idx + 1)
                                } else {
                                    "• ".to_string()
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

                                    let mut spans = vec![];

                                    // Calculate indent: reserve 2 chars for pointer at appropriate depth
                                    // Base indent (2) + nested indent, with pointer replacing last 2 chars
                                    let total_indent = indent_count + 2;
                                    if show_pointer {
                                        // Indent up to pointer position, then pointer
                                        let pre_pointer_indent =
                                            " ".repeat(total_indent.saturating_sub(2));
                                        spans.push(Span::raw(pre_pointer_indent));
                                        spans.push(Span::styled(
                                            "→ ",
                                            Style::default()
                                                .fg(theme.selection_indicator_fg)
                                                .bg(theme.selection_indicator_bg)
                                                .add_modifier(Modifier::BOLD),
                                        ));
                                    } else {
                                        spans.push(Span::raw(" ".repeat(total_indent)));
                                    }

                                    let marker = if is_task {
                                        // Task list item with checkbox
                                        if checked { "☑ " } else { "☐ " }
                                    } else {
                                        // Regular bullet
                                        "• "
                                    };

                                    let formatted =
                                        format_inline_markdown(text_after_marker, theme);
                                    spans.push(Span::styled(
                                        marker,
                                        Style::default().fg(theme.list_bullet),
                                    ));
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
                            render_inline_elements(&item.inline, theme, selected_link_inline_idx)
                        } else {
                            format_inline_markdown(&item.content, theme)
                        };

                        let mut spans = vec![];

                        // Pointer replaces leading spaces, not prepended
                        if pointer_line.is_some() {
                            spans.push(Span::styled(
                                "→ ",
                                Style::default()
                                    .fg(theme.selection_indicator_fg)
                                    .bg(theme.selection_indicator_bg)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            spans.push(Span::raw("  "));
                        }

                        let prefix = if let Some(checked) = item.checked {
                            let checkbox = if checked { "☑" } else { "☐" };
                            format!("{} ", checkbox)
                        } else if *ordered {
                            format!("{}. ", idx + 1)
                        } else {
                            "• ".to_string()
                        };

                        spans.push(Span::styled(prefix, Style::default().fg(theme.list_bullet)));
                        spans.extend(formatted);
                        lines.push(Line::from(spans));
                    }

                    // Render nested blocks within this list item (e.g., code blocks)
                    use crate::tui::interactive::{
                        CODE_BLOCK_OFFSET, IMAGE_OFFSET, ITEM_MULTIPLIER, NESTED_MULTIPLIER,
                        TABLE_OFFSET,
                    };
                    for (nested_idx, nested_block) in item.blocks.iter().enumerate() {
                        // Check if this nested block is selected
                        let is_nested_selected = selected_element_id
                            .map(|id| {
                                if id.block_idx != block_idx {
                                    return false;
                                }
                                if let Some(sub) = id.sub_idx {
                                    // Decode the sub_idx to check if it matches this nested block
                                    let base =
                                        idx * ITEM_MULTIPLIER + nested_idx * NESTED_MULTIPLIER;
                                    sub == base + CODE_BLOCK_OFFSET
                                        || sub == base + TABLE_OFFSET
                                        || sub == base + IMAGE_OFFSET
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);

                        let nested_lines = render_block_to_lines(nested_block, highlighter, theme);
                        for (line_idx, nested_line) in nested_lines.into_iter().enumerate() {
                            let mut indented_spans = vec![];

                            // Add selection indicator on first line of nested block
                            if is_nested_selected && line_idx == 0 {
                                indented_spans.push(Span::styled(
                                    "→ ",
                                    Style::default()
                                        .fg(theme.selection_indicator_fg)
                                        .bg(theme.selection_indicator_bg)
                                        .add_modifier(Modifier::BOLD),
                                ));
                                indented_spans.push(Span::raw("   ")); // 3 spaces (5 - 2 for arrow)
                            } else {
                                indented_spans.push(Span::raw("     ")); // 5 spaces indent
                            }

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
            ContentBlock::Image { alt, src, .. } => {
                // Create placeholder space for image
                // The actual image will be rendered as a StatefulImage widget
                let mut img_line = vec![];
                if is_block_selected {
                    img_line.push(Span::styled(
                        "→ ",
                        Style::default()
                            .fg(theme.selection_indicator_fg)
                            .bg(theme.selection_indicator_bg)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                img_line.push(Span::styled(
                    "🖼 ",
                    Style::default().fg(Color::Rgb(150, 150, 150)),
                ));
                img_line.push(Span::styled(
                    alt.clone(),
                    Style::default()
                        .fg(Color::Rgb(100, 150, 200))
                        .add_modifier(Modifier::ITALIC),
                ));
                img_line.push(Span::raw(" "));
                img_line.push(Span::styled(
                    format!("({})", src),
                    Style::default().fg(Color::Gray),
                ));
                lines.push(Line::from(img_line));

                // Add empty lines as placeholder for image height (16 cells)
                for _ in 0..16 {
                    lines.push(Line::from(""));
                }
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

                // Add selection indicator (with background for visibility)
                if is_block_selected {
                    summary_spans.push(Span::styled(
                        "→ ",
                        Style::default()
                            .fg(theme.selection_indicator_fg)
                            .bg(theme.selection_indicator_bg)
                            .add_modifier(Modifier::BOLD),
                    ));
                }

                // Show ▼ when expanded, ▶ when collapsed
                let indicator = if is_expanded { "▼ " } else { "▶ " };
                summary_spans.push(Span::styled(
                    indicator,
                    Style::default().fg(theme.list_bullet),
                ));

                // Parse and render inline HTML in summary (e.g., <strong>Navigation</strong>)
                let summary_elements = parse_inline_html(summary);
                let rendered_summary = render_inline_elements(&summary_elements, theme, None);
                summary_spans.extend(rendered_summary);

                lines.push(Line::from(summary_spans));

                // Only render nested content if expanded
                if is_expanded {
                    for (nested_idx, nested_block) in nested.iter().enumerate() {
                        // Check if this nested block is selected
                        let nested_sub_idx = crate::tui::interactive::DETAILS_NESTED_BASE
                            + nested_idx * crate::tui::interactive::DETAILS_NESTED_MULTIPLIER;

                        // Check various offsets for different block types
                        let table_id = nested_sub_idx + crate::tui::interactive::TABLE_OFFSET;
                        let code_id = nested_sub_idx + crate::tui::interactive::CODE_BLOCK_OFFSET;
                        let image_id = nested_sub_idx + crate::tui::interactive::IMAGE_OFFSET;

                        let is_nested_selected = selected_element_id
                            .map(|sel_id| {
                                sel_id.block_idx == block_idx
                                    && sel_id.sub_idx.map_or(false, |sub| {
                                        sub == table_id
                                            || sub == code_id
                                            || sub == image_id
                                            || (sub
                                                >= nested_sub_idx
                                                    + crate::tui::interactive::LINK_OFFSET
                                                && sub
                                                    < nested_sub_idx
                                                        + crate::tui::interactive::LINK_OFFSET
                                                        + 100)
                                    })
                            })
                            .unwrap_or(false);

                        // Handle tables specially to preserve interactive rendering
                        if let ContentBlock::Table {
                            headers: nested_headers,
                            alignments: nested_alignments,
                            rows: nested_rows,
                        } = nested_block
                        {
                            // Check if this specific table is selected and in table mode
                            let is_this_table_selected = selected_element_id
                                .map(|sel_id| {
                                    sel_id.block_idx == block_idx
                                        && sel_id.sub_idx == Some(table_id)
                                })
                                .unwrap_or(false);

                            let (in_table_mode, selected_cell) = if is_this_table_selected {
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
                                nested_headers,
                                nested_alignments,
                                nested_rows,
                                theme,
                                is_this_table_selected,
                                in_table_mode,
                                selected_cell,
                            );

                            for nested_line in table_lines {
                                let mut spans = vec![Span::raw("  ")]; // Indent
                                spans.extend(nested_line.spans);
                                lines.push(Line::from(spans));
                            }
                        } else {
                            // Other block types use the standard renderer
                            let nested_lines =
                                render_block_to_lines(nested_block, highlighter, theme);
                            for (line_idx, nested_line) in nested_lines.into_iter().enumerate() {
                                let mut spans = vec![];

                                // Add selection indicator for first line of selected nested block
                                if is_nested_selected && line_idx == 0 {
                                    spans.push(Span::styled(
                                        "→ ",
                                        Style::default()
                                            .fg(theme.selection_indicator_fg)
                                            .bg(theme.selection_indicator_bg)
                                            .add_modifier(Modifier::BOLD),
                                    ));
                                } else {
                                    spans.push(Span::raw("  ")); // Indent
                                }

                                spans.extend(nested_line.spans);
                                lines.push(Line::from(spans));
                            }
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

/// Apply search highlighting to rendered text while preserving original span styles.
/// This function overlays search highlight styles on top of existing styling (links, bold, etc.)
fn apply_search_highlighting(
    text: Text<'static>,
    query: &str,
    current_match_idx: Option<usize>,
    total_matches: usize,
    theme: &Theme,
) -> Text<'static> {
    if query.is_empty() {
        return text;
    }

    let query_lower = query.to_lowercase();
    let mut new_lines = Vec::new();
    let mut match_counter = 0usize;

    for line in text.lines.into_iter() {
        // Build span index: (byte_start, byte_end, span_index)
        let mut span_ranges: Vec<(usize, usize, usize)> = Vec::new();
        let mut byte_pos = 0;
        for (idx, span) in line.spans.iter().enumerate() {
            let span_len = span.content.len();
            span_ranges.push((byte_pos, byte_pos + span_len, idx));
            byte_pos += span_len;
        }

        // Join all spans to get the full line text for searching
        let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let full_text_lower = full_text.to_lowercase();

        // Find all occurrences of the query in this line (non-overlapping)
        let mut matches_in_line: Vec<(usize, usize)> = Vec::new();
        let mut search_start = 0;
        let query_len = query.len();

        while let Some(rel_pos) = full_text_lower[search_start..].find(&query_lower) {
            let byte_start = search_start + rel_pos;
            let byte_end = byte_start + query_len;

            // Verify we're on valid char boundaries
            if full_text.is_char_boundary(byte_start) && full_text.is_char_boundary(byte_end) {
                matches_in_line.push((byte_start, byte_end));
            }

            search_start = byte_end;
            if search_start >= full_text_lower.len() {
                break;
            }
        }

        if matches_in_line.is_empty() {
            // No matches in this line - keep original
            new_lines.push(line);
        } else {
            // Rebuild line with highlighted matches while preserving original styles
            let mut new_spans: Vec<Span<'static>> = Vec::new();

            // Process each original span and split it at match boundaries
            for (span_start, span_end, span_idx) in &span_ranges {
                let original_span = &line.spans[*span_idx];
                let original_style = original_span.style;
                let span_text = original_span.content.as_ref();

                // Find which matches overlap with this span
                let mut current_pos = 0; // position within the span

                for (match_start, match_end) in &matches_in_line {
                    // Skip matches that are entirely before this span
                    if *match_end <= *span_start {
                        continue;
                    }
                    // Stop if match is entirely after this span
                    if *match_start >= *span_end {
                        break;
                    }

                    let is_current = total_matches > 0 && current_match_idx == Some(match_counter);

                    // Calculate positions relative to span
                    let rel_match_start = match_start.saturating_sub(*span_start);
                    let rel_match_end = (*match_end).min(*span_end) - *span_start;

                    // Add text before the match (with original style)
                    if current_pos < rel_match_start {
                        if let Some(before_text) =
                            safe_slice(span_text, current_pos, rel_match_start)
                        {
                            if !before_text.is_empty() {
                                new_spans
                                    .push(Span::styled(before_text.to_string(), original_style));
                            }
                        }
                    }

                    // Add the matched portion (with search highlight style)
                    let highlight_style = if is_current {
                        theme.search_current_style()
                    } else {
                        theme.search_match_style()
                    };

                    let actual_start = rel_match_start.max(current_pos);
                    if let Some(match_text) = safe_slice(span_text, actual_start, rel_match_end) {
                        if !match_text.is_empty() {
                            new_spans.push(Span::styled(match_text.to_string(), highlight_style));
                        }
                    }

                    current_pos = rel_match_end;

                    // Only increment counter when we finish the match (match_end <= span_end)
                    if *match_end <= *span_end {
                        match_counter += 1;
                    }
                }

                // Add remaining text after all matches in this span (with original style)
                if current_pos < span_text.len() {
                    if let Some(after_text) = safe_slice(span_text, current_pos, span_text.len()) {
                        if !after_text.is_empty() {
                            new_spans.push(Span::styled(after_text.to_string(), original_style));
                        }
                    }
                }
            }

            new_lines.push(Line::from(new_spans));
        }
    }

    Text::from(new_lines)
}

/// Safely slice a string at byte boundaries, returning None if boundaries are invalid
fn safe_slice(s: &str, start: usize, end: usize) -> Option<&str> {
    if start > end || end > s.len() {
        return None;
    }
    if !s.is_char_boundary(start) || !s.is_char_boundary(end) {
        return None;
    }
    Some(&s[start..end])
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
            ..
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
            let mut summary_spans =
                vec![Span::styled("▶ ", Style::default().fg(theme.list_bullet))];

            // Parse and render inline HTML in summary (e.g., <strong>Navigation</strong>)
            let summary_elements = parse_inline_html(summary);
            let rendered_summary = render_inline_elements(&summary_elements, theme, None);
            summary_spans.extend(rendered_summary);

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
        ContentBlock::Table {
            headers,
            alignments,
            rows,
        } => {
            // Render table (non-interactive, no selection)
            let table_lines = render_table(headers, alignments, rows, theme, false, false, None);
            lines.extend(table_lines);
        }
        ContentBlock::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                let marker = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "• ".to_string()
                };

                // Render item content
                let item_spans = if !item.inline.is_empty() {
                    render_inline_elements(&item.inline, theme, None)
                } else {
                    format_inline_markdown(&item.content, theme)
                };

                let mut line_spans =
                    vec![Span::styled(marker, Style::default().fg(theme.list_bullet))];
                line_spans.extend(item_spans);
                lines.push(Line::from(line_spans));

                // Render nested blocks (indented)
                for nested in &item.blocks {
                    let nested_lines = render_block_to_lines(nested, highlighter, theme);
                    for nested_line in nested_lines {
                        let mut spans = vec![Span::raw("  ")];
                        spans.extend(nested_line.spans);
                        lines.push(Line::from(spans));
                    }
                }
            }
        }
        ContentBlock::Blockquote { content, blocks } => {
            // Render blockquote with > prefix
            let formatted = format_inline_markdown(content, theme);
            let mut quote_spans = vec![Span::styled(
                "│ ",
                Style::default().fg(theme.blockquote_border),
            )];
            quote_spans.extend(formatted);
            lines.push(Line::from(quote_spans));

            // Render nested blocks
            for nested in blocks {
                let nested_lines = render_block_to_lines(nested, highlighter, theme);
                for nested_line in nested_lines {
                    let mut spans = vec![Span::styled(
                        "│ ",
                        Style::default().fg(theme.blockquote_border),
                    )];
                    spans.extend(nested_line.spans);
                    lines.push(Line::from(spans));
                }
            }
        }
        ContentBlock::Image { alt, src, .. } => {
            let image_spans = vec![
                Span::styled("🖼 ", Style::default().fg(theme.link_fg)),
                Span::styled(
                    format!("{} ({})", alt, src),
                    Style::default()
                        .fg(theme.link_fg)
                        .add_modifier(Modifier::ITALIC),
                ),
            ];
            lines.push(Line::from(image_spans));
        }
        ContentBlock::HorizontalRule => {
            lines.push(Line::from(vec![Span::styled(
                "─".repeat(40),
                Style::default().fg(Color::Rgb(80, 80, 100)),
            )]));
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
                    // Add selection indicator before selected link (with background for visibility)
                    spans.push(Span::styled(
                        "▸ ",
                        Style::default()
                            .fg(theme.selection_indicator_fg)
                            .bg(theme.selection_indicator_bg)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                let style = if is_selected {
                    // Highlighted selected link - matches table cell selection style
                    Style::default()
                        .fg(theme.link_selected_fg)
                        .bg(theme.link_selected_bg)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
                } else {
                    // Normal link style
                    Style::default()
                        .fg(theme.link_fg)
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
            InlineElement::Image { .. } => {
                // Images are rendered separately, not as placeholder text
                // This allows them to appear in-place without text alongside
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
        // Check for markdown link [text](url)
        if chars[i] == '[' {
            // Look for the closing ] and opening (
            let mut j = i + 1;
            let mut link_text = String::new();
            while j < chars.len() && chars[j] != ']' {
                link_text.push(chars[j]);
                j += 1;
            }
            // Check if followed by (url)
            if j + 1 < chars.len() && chars[j] == ']' && chars[j + 1] == '(' {
                let mut k = j + 2;
                let mut url = String::new();
                while k < chars.len() && chars[k] != ')' {
                    url.push(chars[k]);
                    k += 1;
                }
                if k < chars.len() && chars[k] == ')' {
                    // Valid link found
                    if !current.is_empty() {
                        spans.push(Span::raw(current.clone()));
                        current.clear();
                    }
                    // Render link text with link styling
                    spans.push(Span::styled(
                        link_text,
                        Style::default()
                            .fg(theme.link_fg)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    i = k + 1; // Move past the closing )
                    continue;
                }
            }
            // Not a valid link, treat [ as regular character
            current.push(chars[i]);
            i += 1;
        }
        // Check for inline code `code`
        else if chars[i] == '`' {
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
