//! Table rendering for the TUI
//!
//! Handles rendering of markdown tables with proper alignment,
//! borders, selection highlighting, and cell navigation.

use crate::parser::output::Alignment;
use crate::tui::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::util::align_text;

/// Context for rendering a table row
pub struct TableRenderContext<'a> {
    pub theme: &'a Theme,
    pub row_num: usize,
    pub is_header: bool,
    pub in_table_mode: bool,
    pub is_table_selected: bool,
    pub selected_cell: Option<(usize, usize)>,
}

/// Render a complete table with headers, alignments, and rows
///
/// # Arguments
/// * `headers` - Column headers
/// * `alignments` - Column alignments
/// * `rows` - Data rows
/// * `theme` - Color theme
/// * `is_selected` - Whether the table element is selected
/// * `in_table_mode` - Whether we're in table cell navigation mode
/// * `selected_cell` - Currently selected cell (row, col) if in table mode
pub fn render_table(
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
                .fg(theme.selection_indicator_fg)
                .bg(theme.selection_indicator_bg)
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
        Style::default().fg(theme.table_border),
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
        Style::default().fg(theme.table_border),
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
        Style::default().fg(theme.table_border),
    ));
    lines.push(Line::from(bottom_border_spans));

    lines
}

/// Render a single table row with proper alignment and styling
///
/// # Arguments
/// * `cells` - Cell contents for this row
/// * `col_widths` - Pre-calculated column widths
/// * `alignments` - Column alignments
/// * `ctx` - Rendering context with theme and selection state
pub fn render_table_row(
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
                    .fg(ctx.theme.selection_indicator_fg)
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
        Style::default().fg(ctx.theme.table_border),
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
                .fg(ctx.theme.link_selected_fg)
                .bg(ctx.theme.link_selected_bg)
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
            Style::default().fg(ctx.theme.table_border),
        ));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::theme::ThemeName;

    fn test_theme() -> Theme {
        Theme::from_name(ThemeName::OceanDark)
    }

    mod render_table_tests {
        use super::*;

        #[test]
        fn test_empty_headers_returns_empty() {
            let theme = test_theme();
            let lines = render_table(&[], &[], &[], &theme, false, false, None);
            assert!(lines.is_empty());
        }

        #[test]
        fn test_single_column_table() {
            let theme = test_theme();
            let headers = vec!["Name".to_string()];
            let alignments = vec![Alignment::Left];
            let rows = vec![vec!["Alice".to_string()], vec!["Bob".to_string()]];

            let lines = render_table(&headers, &alignments, &rows, &theme, false, false, None);

            // Should have: top border, header, separator, 2 data rows, bottom border = 6 lines
            assert_eq!(lines.len(), 6);
        }

        #[test]
        fn test_multi_column_table() {
            let theme = test_theme();
            let headers = vec!["Name".to_string(), "Age".to_string(), "City".to_string()];
            let alignments = vec![Alignment::Left, Alignment::Right, Alignment::Center];
            let rows = vec![
                vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
                vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
            ];

            let lines = render_table(&headers, &alignments, &rows, &theme, false, false, None);

            // Should have: top border, header, separator, 2 data rows, bottom border = 6 lines
            assert_eq!(lines.len(), 6);
        }

        #[test]
        fn test_selected_table_adds_arrow() {
            let theme = test_theme();
            let headers = vec!["Col".to_string()];
            let rows = vec![vec!["Data".to_string()]];

            let lines_unselected = render_table(&headers, &[], &rows, &theme, false, false, None);
            let lines_selected = render_table(&headers, &[], &rows, &theme, true, false, None);

            // Selected table should have arrow prefix on first line
            let first_unselected = &lines_unselected[0];
            let first_selected = &lines_selected[0];

            // Selected version should have "→ " at the start
            assert!(first_selected.spans.iter().any(|s| s.content.contains("→")));
            assert!(!first_unselected.spans.iter().any(|s| s.content.contains("→")));
        }

        #[test]
        fn test_table_mode_shows_row_arrow() {
            let theme = test_theme();
            let headers = vec!["Col".to_string()];
            let rows = vec![vec!["Row1".to_string()], vec!["Row2".to_string()]];

            // Select cell at row 1, col 0
            let lines = render_table(&headers, &[], &rows, &theme, true, true, Some((1, 0)));

            // Row 1 (first data row, which is lines[3] - after top, header, separator)
            // should have the arrow indicator
            let data_row_1 = &lines[3];
            assert!(data_row_1.spans.iter().any(|s| s.content.contains("→")));

            // Header row (lines[1]) should NOT have arrow
            let _header_row = &lines[1];
            // It has spacing but not the actual arrow character with bold styling
            // The arrow only appears on the selected row
        }

        #[test]
        fn test_header_only_table() {
            let theme = test_theme();
            let headers = vec!["Header1".to_string(), "Header2".to_string()];
            let alignments = vec![Alignment::Left, Alignment::Right];
            let rows: Vec<Vec<String>> = vec![];

            let lines = render_table(&headers, &alignments, &rows, &theme, false, false, None);

            // Should have: top border, header, separator, bottom border = 4 lines
            assert_eq!(lines.len(), 4);
        }
    }

    mod render_table_row_tests {
        use super::*;

        #[test]
        fn test_basic_row() {
            let theme = test_theme();
            let cells = vec!["A".to_string(), "B".to_string()];
            let col_widths = vec![5, 5];
            let alignments = vec![Alignment::Left, Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 0,
                is_header: false,
                in_table_mode: false,
                is_table_selected: false,
                selected_cell: None,
            };

            let line = render_table_row(&cells, &col_widths, &alignments, &ctx);

            // Should have spans for: │, cell1, │, cell2, │
            assert!(line.spans.len() >= 5);
        }

        #[test]
        fn test_header_row_styling() {
            let theme = test_theme();
            let cells = vec!["Header".to_string()];
            let col_widths = vec![10];
            let alignments = vec![Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 0,
                is_header: true,
                in_table_mode: false,
                is_table_selected: false,
                selected_cell: None,
            };

            let line = render_table_row(&cells, &col_widths, &alignments, &ctx);

            // Header should have bold modifier
            let cell_span = line.spans.iter().find(|s| s.content.contains("Header"));
            assert!(cell_span.is_some());
            assert!(cell_span.unwrap().style.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn test_selected_cell_highlighting() {
            let theme = test_theme();
            let cells = vec!["A".to_string(), "B".to_string()];
            let col_widths = vec![5, 5];
            let alignments = vec![Alignment::Left, Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 1,
                is_header: false,
                in_table_mode: true,
                is_table_selected: true,
                selected_cell: Some((1, 1)), // Select cell B
            };

            let line = render_table_row(&cells, &col_widths, &alignments, &ctx);

            // The selected cell should have a background color
            let cell_b_span = line.spans.iter().find(|s| s.content.contains("B"));
            assert!(cell_b_span.is_some());
            // Check it has the highlight background
            assert!(cell_b_span.unwrap().style.bg.is_some());
        }

        #[test]
        fn test_row_with_arrow_when_selected() {
            let theme = test_theme();
            let cells = vec!["Data".to_string()];
            let col_widths = vec![8];
            let alignments = vec![Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 1,
                is_header: false,
                in_table_mode: true,
                is_table_selected: true,
                selected_cell: Some((1, 0)),
            };

            let line = render_table_row(&cells, &col_widths, &alignments, &ctx);

            // Should have arrow at start when row is selected in table mode
            assert!(line.spans[0].content.contains("→"));
        }

        #[test]
        fn test_row_without_arrow_when_not_selected() {
            let theme = test_theme();
            let cells = vec!["Data".to_string()];
            let col_widths = vec![8];
            let alignments = vec![Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 2,
                is_header: false,
                in_table_mode: true,
                is_table_selected: true,
                selected_cell: Some((1, 0)), // Different row selected
            };

            let line = render_table_row(&cells, &col_widths, &alignments, &ctx);

            // Should have spaces, not arrow
            assert_eq!(line.spans[0].content, "  ");
        }
    }
}
