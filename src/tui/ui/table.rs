//! Table rendering for the TUI
//!
//! Handles rendering of markdown tables with proper alignment,
//! borders, selection highlighting, and cell navigation.

use crate::parser::output::Alignment;
use crate::tui::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::tui::ui::format_inline_markdown;
use crate::tui::ui::util::{align_text, terminal_width, wrap_text};

/// Context for rendering a table row
pub struct TableRenderContext<'a> {
    pub theme: &'a Theme,
    pub row_num: usize,
    pub is_header: bool,
    pub in_table_mode: bool,
    pub is_table_selected: bool,
    pub selected_cell: Option<(usize, usize)>,
}

/// Minimum column width (including padding) to maintain readability
const MIN_COL_WIDTH: usize = 3;

/// One space either side of a cell's text. Included in every column width, and
/// reserved again by `render_table_row` when it wraps, so the two must agree.
const CELL_PADDING: usize = 2;

/// Fit columns into `budget` by capping the widest ones, not by scaling all.
///
/// Scaling every column by the same ratio takes the same *fraction* from each,
/// which in cell terms means a 4-wide column loses almost nothing in absolute
/// space while being crushed past readability, and a 75-wide column keeps most
/// of its surplus. A two-character `ID` header ends up stacked vertically next
/// to a `Description` column that is still 64 cells wide.
///
/// Instead, find the largest cap where trimming every column to it fits, which
/// takes space only from columns above the cap. Narrow columns are untouched
/// until every wider one has come down to their size.
fn fit_columns_to_budget(col_widths: &mut [usize], budget: usize) {
    if col_widths.is_empty() || col_widths.iter().sum::<usize>() <= budget {
        return;
    }

    let capped_total = |cap: usize, w: &[usize]| -> usize {
        w.iter().map(|x| (*x).min(cap).max(MIN_COL_WIDTH)).sum()
    };

    // Largest cap that fits. If even an all-minimum table is too wide there is
    // nothing to find, and every column lands on the floor below.
    let mut lo = MIN_COL_WIDTH;
    let mut hi = col_widths.iter().copied().max().unwrap_or(MIN_COL_WIDTH);
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if capped_total(mid, col_widths) <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    for width in col_widths.iter_mut() {
        *width = (*width).min(lo).max(MIN_COL_WIDTH);
    }

    // An integral cap usually leaves a few cells over. Hand them back one each
    // to the columns that were actually clipped, so the space goes where
    // content was lost rather than padding a column that already fits.
    let mut slack = budget.saturating_sub(col_widths.iter().sum::<usize>());
    let clipped: Vec<usize> = col_widths
        .iter()
        .enumerate()
        .filter(|(_, w)| **w == lo)
        .map(|(i, _)| i)
        .collect();
    for idx in clipped {
        if slack == 0 {
            break;
        }
        col_widths[idx] += 1;
        slack -= 1;
    }
}

/// How wide a cell's text is on screen. Inline code is drawn without its
/// backticks, so they take no room.
fn cell_width(cell: &str) -> usize {
    if cell.contains('`') {
        terminal_width(&cell.replace('`', ""))
    } else {
        terminal_width(cell)
    }
}

/// Each column's natural width: its widest cell, header included.
///
/// A column was sized at 70% of its average cell and 30% of its widest, to
/// keep one long value from taking the table, so `Previous` wrapped in a
/// column of `Next` and `Previous` with the pane half empty. That only
/// matters when the table does not fit, and [`fit_columns_to_budget`] then
/// takes the space from the widest columns first.
fn calculate_column_widths(headers: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    headers
        .iter()
        .enumerate()
        .map(|(i, header)| {
            rows.iter()
                .filter_map(|row| row.get(i))
                .map(|cell| cell_width(cell))
                .fold(cell_width(header), usize::max)
                .max(1)
        })
        .collect()
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
/// * `available_width` - Optional maximum width to constrain table to
#[allow(clippy::too_many_arguments)]
pub fn render_table(
    headers: &[String],
    alignments: &[Alignment],
    rows: &[Vec<String>],
    theme: &Theme,
    is_selected: bool,
    in_table_mode: bool,
    selected_cell: Option<(usize, usize)>,
    available_width: Option<u16>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if headers.is_empty() {
        return lines;
    }

    let col_count = headers.len();

    // Calculate column widths using content-weighted approach
    let mut col_widths = calculate_column_widths(headers, rows);

    // One space each side. `render_table_row` always reserves this much when it
    // wraps cell text, so it has to stay inside the width for the whole of the
    // fitting below. Shaving it off here would not reclaim decoration, it would
    // just take two cells away from the content.
    for width in &mut col_widths {
        *width += CELL_PADDING;
    }

    // Shrink to fit if the table is wider than the space it has.
    if let Some(max_width) = available_width {
        let prefix_width = if in_table_mode || is_selected { 2 } else { 0 };
        let border_width = col_count + 1; // │ between and around columns
        let budget = usize::from(max_width).saturating_sub(border_width + prefix_width);
        fit_columns_to_budget(&mut col_widths, budget);
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
    let header_lines = render_table_row(
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
    lines.extend(header_lines);

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
        let row_lines = render_table_row(
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
        lines.extend(row_lines);
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
/// Supports multi-line cells via wrapping.
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
) -> Vec<Line<'static>> {
    // 1. Wrap each cell into multiple lines
    let mut wrapped_cells: Vec<Vec<String>> = Vec::new();
    let mut max_lines = 1;

    // A row is drawn with exactly the table's columns. The parser already
    // drops a row's extra cells, as GFM says to, but a longer row reaching
    // here would otherwise draw cells past the right border.
    for (i, cell) in cells.iter().take(col_widths.len()).enumerate() {
        let width = col_widths[i];
        // Column widths carry `CELL_PADDING`, so the text gets what is left.
        let content_width = width.saturating_sub(CELL_PADDING);
        let wrapped = if content_width > 0 {
            wrap_text(cell, content_width)
        } else {
            vec![String::new()]
        };
        max_lines = max_lines.max(wrapped.len());
        wrapped_cells.push(wrapped);
    }

    // 2. Render each line of the row
    let mut row_lines = Vec::new();

    for line_idx in 0..max_lines {
        let mut spans = Vec::new();

        // Add arrow or space to keep table aligned
        if ctx.in_table_mode {
            let is_selected_row = ctx.selected_cell.map(|(r, _)| r) == Some(ctx.row_num);
            if is_selected_row && line_idx == 0 {
                spans.push(Span::styled(
                    "→ ",
                    Style::default()
                        .fg(ctx.theme.selection_indicator_fg)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw("  "));
            }
        } else if ctx.is_table_selected {
            spans.push(Span::raw("  "));
        }

        spans.push(Span::styled(
            "│",
            Style::default().fg(ctx.theme.table_border),
        ));

        for (i, wrapped_cell) in wrapped_cells.iter().enumerate() {
            let width = col_widths.get(i).copied().unwrap_or(10);
            let alignment = alignments.get(i).unwrap_or(&Alignment::Left);

            // Get the text for this specific line of the cell, or empty string
            let line_text = wrapped_cell.get(line_idx).cloned().unwrap_or_default();

            // Determine if this specific cell is selected
            let is_selected = ctx.selected_cell == Some((ctx.row_num, i));

            let style = if is_selected {
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

            if !is_selected && line_text.contains('`') {
                // Render inline code spans with theme styling
                let formatted = format_inline_markdown(&line_text, ctx.theme);
                let rendered_width: usize =
                    formatted.iter().map(|s| terminal_width(&s.content)).sum();
                // The same one-space margin `align_text` gives a plain cell;
                // without it a code cell sat against the left border.
                let padding_total = width.saturating_sub(rendered_width);
                let (lead, trail) = match alignment {
                    Alignment::Right => (padding_total.saturating_sub(1), padding_total.min(1)),
                    Alignment::Center => {
                        let half = padding_total / 2;
                        (half, padding_total - half)
                    }
                    _ => (padding_total.min(1), padding_total.saturating_sub(1)),
                };
                if lead > 0 {
                    spans.push(Span::styled(" ".repeat(lead), style));
                }
                for span in formatted {
                    // Plain text spans: apply row/header style; styled spans (code etc.) keep their style
                    let effective_style =
                        if span.style == Style::default() || span.style == ctx.theme.text_style() {
                            style
                        } else {
                            span.style
                        };
                    spans.push(Span::styled(span.content.into_owned(), effective_style));
                }
                if trail > 0 {
                    spans.push(Span::styled(" ".repeat(trail), style));
                }
            } else {
                let cell_text = align_text(&line_text, width, alignment);
                spans.push(Span::styled(cell_text, style));
            }
            spans.push(Span::styled(
                "│",
                Style::default().fg(ctx.theme.table_border),
            ));
        }

        row_lines.push(Line::from(spans));
    }

    row_lines
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

        fn text(lines: &[Line]) -> Vec<String> {
            lines
                .iter()
                .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
                .collect()
        }

        /// A column was sized from a blend of its average and widest cell,
        /// so `Previous` wrapped beside `Next` with room to spare. A table
        /// that fits gets its natural widths.
        #[test]
        fn a_table_that_fits_wraps_nothing() {
            let headers = vec!["Key".to_string(), "Action".to_string()];
            let rows = vec![
                vec!["`j`".to_string(), "Next".to_string()],
                vec!["`k`".to_string(), "Previous".to_string()],
            ];
            let aligns = [Alignment::None, Alignment::None];
            let lines = render_table(
                &headers,
                &aligns,
                &rows,
                &test_theme(),
                false,
                false,
                None,
                Some(80),
            );
            let lines = text(&lines);
            // Top, header, separator, two rows, bottom: no wrapped rows.
            assert_eq!(lines.len(), 6, "{lines:#?}");
            assert_eq!(lines[3], "│ j   │ Next     │");
            assert_eq!(lines[4], "│ k   │ Previous │");
        }

        #[test]
        fn test_empty_headers_returns_empty() {
            let theme = test_theme();
            let lines = render_table(&[], &[], &[], &theme, false, false, None, None);
            assert!(lines.is_empty());
        }

        #[test]
        fn test_single_column_table() {
            let theme = test_theme();
            let headers = vec!["Name".to_string()];
            let alignments = vec![Alignment::Left];
            let rows = vec![vec!["Alice".to_string()], vec!["Bob".to_string()]];

            let lines = render_table(
                &headers,
                &alignments,
                &rows,
                &theme,
                false,
                false,
                None,
                None,
            );

            // Should have: top border, header, separator, 2 data rows, bottom border
            // Wrapping might add lines if text is tight, current implementation yields 7
            assert!(lines.len() >= 6);
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

            let lines = render_table(
                &headers,
                &alignments,
                &rows,
                &theme,
                false,
                false,
                None,
                None,
            );

            // Should have at least 6 lines
            assert!(lines.len() >= 6);
        }

        #[test]
        fn test_selected_table_adds_arrow() {
            let theme = test_theme();
            let headers = vec!["Col".to_string()];
            let rows = vec![vec!["Data".to_string()]];

            let _lines_unselected =
                render_table(&headers, &[], &rows, &theme, false, false, None, None);
            let lines_selected =
                render_table(&headers, &[], &rows, &theme, true, false, None, None);

            // Selected table should have arrow prefix on first line
            let first_selected = &lines_selected[0];

            // Selected version should have "→ " at the start
            assert!(first_selected.spans.iter().any(|s| s.content.contains("→")));
        }

        #[test]
        fn test_table_mode_shows_row_arrow() {
            let theme = test_theme();
            let headers = vec!["Col".to_string()];
            let rows = vec![vec!["Row1".to_string()], vec!["Row2".to_string()]];

            // Select cell at row 1, col 0
            let lines = render_table(&headers, &[], &rows, &theme, true, true, Some((1, 0)), None);

            // Find the row with the arrow
            assert!(
                lines
                    .iter()
                    .any(|l| l.spans.iter().any(|s| s.content.contains("→")))
            );
        }

        #[test]
        fn test_header_only_table() {
            let theme = test_theme();
            let headers = vec!["Header1".to_string(), "Header2".to_string()];
            let alignments = vec![Alignment::Left, Alignment::Right];
            let rows: Vec<Vec<String>> = vec![];

            let lines = render_table(
                &headers,
                &alignments,
                &rows,
                &theme,
                false,
                false,
                None,
                None,
            );

            // Should have: top border, header, separator, bottom border = 4 lines
            assert_eq!(lines.len(), 4);
        }

        #[test]
        fn test_halfwidth_katakana_table_widths_match_terminal_cells() {
            let theme = test_theme();
            let headers = vec!["Kana".to_string()];
            let rows = vec![vec!["ｶﾞ".to_string()], vec!["ﾊﾟ".to_string()]];

            let lines = render_table(
                &headers,
                &[Alignment::Left],
                &rows,
                &theme,
                false,
                false,
                None,
                Some(8),
            );

            for line in lines {
                let width: usize = line
                    .spans
                    .iter()
                    .map(|span| terminal_width(&span.content))
                    .sum();
                assert!(width <= 8, "line is {width} cells wide: {line:?}");
            }
        }

        #[test]
        fn test_table_width_constraint() {
            let theme = test_theme();
            let headers = vec![
                "Very Long Header Name".to_string(),
                "Another Long Header".to_string(),
            ];
            let alignments = vec![Alignment::Left, Alignment::Left];
            let rows = vec![vec![
                "Some content here".to_string(),
                "More content".to_string(),
            ]];

            // Without width constraint
            let lines_unconstrained = render_table(
                &headers,
                &alignments,
                &rows,
                &theme,
                false,
                false,
                None,
                None,
            );

            // With width constraint - table will wrap
            let lines_constrained = render_table(
                &headers,
                &alignments,
                &rows,
                &theme,
                false,
                false,
                None,
                Some(40),
            );

            // Constrained version should have MORE lines due to wrapping
            assert!(lines_constrained.len() >= lines_unconstrained.len());
        }

        /// Widths of each column, read back off the rendered top border.
        fn rendered_col_widths(lines: &[Line<'static>]) -> Vec<usize> {
            let border: String = lines[0]
                .spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>();
            border
                .trim_start_matches(|c| c != '┌')
                .trim_matches(['┌', '┐'])
                .split('┬')
                .map(|seg| seg.chars().count())
                .collect()
        }

        fn unequal_table() -> (Vec<String>, Vec<Alignment>, Vec<Vec<String>>) {
            let headers = ["ID", "OK", "Name", "Description"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let rows = (0..3)
                .map(|r| {
                    vec![
                        format!("{r}"),
                        "yes".to_string(),
                        format!("item-{r}"),
                        "a considerably longer description column that dominates the width"
                            .to_string(),
                    ]
                })
                .collect();
            (headers, vec![Alignment::Left; 4], rows)
        }

        #[test]
        fn narrow_columns_are_not_crushed_to_fit_a_dominant_one() {
            // Scaling every column by the same ratio drove `ID` and `OK` to the
            // floor, stacking two-letter headers vertically, while the
            // description column kept the bulk of its surplus.
            let theme = test_theme();
            let (headers, aligns, rows) = unequal_table();

            for width in [60u16, 80, 100] {
                let lines = render_table(
                    &headers,
                    &aligns,
                    &rows,
                    &theme,
                    false,
                    false,
                    None,
                    Some(width),
                );
                let widths = rendered_col_widths(&lines);
                assert_eq!(widths.len(), 4, "at {width}: {widths:?}");

                // "ID" and "OK" are 2 cells of content. They should never be
                // clipped while a wider column still has surplus to give.
                let description = widths[3];
                for (i, header) in ["ID", "OK"].iter().enumerate() {
                    assert!(
                        widths[i] >= header.len() || widths[i] >= description,
                        "at {width}: {header} got {} cells while description kept {description}: {widths:?}",
                        widths[i]
                    );
                }
            }
        }

        #[test]
        fn shrinking_never_exceeds_the_available_width() {
            let theme = test_theme();
            let (headers, aligns, rows) = unequal_table();

            // Below `col_count * MIN_COL_WIDTH` plus one border per edge there
            // is no layout that fits, so the floor is where the guarantee
            // starts: 4 columns at 3 cells plus 5 borders.
            let floor = (4 * MIN_COL_WIDTH + 5) as u16;

            for width in floor..=120 {
                let lines = render_table(
                    &headers,
                    &aligns,
                    &rows,
                    &theme,
                    false,
                    false,
                    None,
                    Some(width),
                );
                for line in &lines {
                    let rendered: usize =
                        line.spans.iter().map(|s| terminal_width(&s.content)).sum();
                    assert!(
                        rendered <= usize::from(width),
                        "at {width} a line rendered {rendered} cells wide"
                    );
                }
            }
        }

        #[test]
        fn test_seven_column_table_at_width_146_no_panic() {
            // Regression test for crash when MIN_COL_WIDTH clamping pushes
            // total column width over budget after proportional shrink
            let theme = test_theme();
            let headers = vec![
                "Protocol".to_string(),
                "Port(s)".to_string(),
                "Transport".to_string(),
                "Purpose".to_string(),
                "Encryption".to_string(),
                "Key Feature".to_string(),
                "Common Usage".to_string(),
            ];
            let alignments = vec![Alignment::Left; 7];
            let rows = vec![
                vec![
                    "HTTP".to_string(),
                    "80".to_string(),
                    "TCP".to_string(),
                    "Web".to_string(),
                    "No".to_string(),
                    "Stateless".to_string(),
                    "Websites".to_string(),
                ],
                vec![
                    "HTTPS".to_string(),
                    "443".to_string(),
                    "TCP".to_string(),
                    "Secure Web".to_string(),
                    "TLS/SSL".to_string(),
                    "Encrypted HTTP".to_string(),
                    "Secure websites".to_string(),
                ],
                vec![
                    "FTP".to_string(),
                    "20/21".to_string(),
                    "TCP".to_string(),
                    "File Transfer".to_string(),
                    "Optional".to_string(),
                    "Active/Passive".to_string(),
                    "File sharing".to_string(),
                ],
            ];

            // Test specific widths around the previously crashing point
            for width in [30, 50, 80, 100, 130, 140, 145, 146, 147, 150, 160, 180, 200] {
                let lines = render_table(
                    &headers,
                    &alignments,
                    &rows,
                    &theme,
                    false,
                    false,
                    None,
                    Some(width),
                );
                assert!(!lines.is_empty(), "Table should render at width {}", width);
            }
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

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);
            let line = &row_lines[0];

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

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);
            let line = &row_lines[0];

            // Header should have bold modifier
            let cell_span = line.spans.iter().find(|s| s.content.contains("Header"));
            assert!(cell_span.is_some());
            assert!(
                cell_span
                    .unwrap()
                    .style
                    .add_modifier
                    .contains(Modifier::BOLD)
            );
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

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);
            let line = &row_lines[0];

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

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);
            let line = &row_lines[0];

            // Should have arrow at start when row is selected in table mode
            assert!(line.spans[0].content.contains("→"));
        }

        #[test]
        fn test_row_wrapping() {
            let theme = test_theme();
            let cells = vec!["Very long text that should wrap".to_string()];
            let col_widths = vec![10]; // Small width will force wrapping
            let alignments = vec![Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 1,
                is_header: false,
                in_table_mode: false,
                is_table_selected: false,
                selected_cell: None,
            };

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);

            // Should have multiple lines due to wrapping
            assert!(row_lines.len() > 1);
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

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);
            let line = &row_lines[0];

            // Should have spaces, not arrow
            assert_eq!(line.spans[0].content, "  ");
        }

        #[test]
        fn test_header_inline_code_keeps_surrounding_header_style() {
            let theme = test_theme();
            let cells = vec!["Use `foo` now".to_string()];
            let col_widths = vec![20];
            let alignments = vec![Alignment::Left];

            let ctx = TableRenderContext {
                theme: &theme,
                row_num: 0,
                is_header: true,
                in_table_mode: false,
                is_table_selected: false,
                selected_cell: None,
            };

            let row_lines = render_table_row(&cells, &col_widths, &alignments, &ctx);
            let line = &row_lines[0];

            let trailing_text = line
                .spans
                .iter()
                .find(|span| span.content.as_ref() == " now")
                .expect("expected plain trailing text after inline code");

            assert!(
                trailing_text.style.add_modifier.contains(Modifier::BOLD),
                "plain text after inline code in a header cell should keep header styling"
            );
        }
    }
}
