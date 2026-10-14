//! The content pane as screen rows.
//!
//! The renderer turns a section into styled lines; this wraps them to the
//! pane width once and keeps the result, so a frame only slices out the
//! visible rows. It also records where every line and block landed, which
//! is the one place anything that has to find a spot on screen (the
//! selected element, an image, a search match) looks it up.
//!
//! Before this, each frame re-rendered and re-wrapped the whole section,
//! about a second per keypress on a large document, and positions came from
//! a separate count of lines that knew nothing of wrapping, so a selection
//! below a wrapped paragraph scrolled to the wrong place and an image was
//! drawn over text.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A section as the renderer emits it: its lines, and the line each
/// top-level block starts on.
pub type RenderedLines = (Vec<Line<'static>>, Vec<usize>);

/// The content pane's work, kept between frames, each stage keyed on a hash
/// of everything it was built from.
#[derive(Debug, Default)]
pub struct ContentCache {
    /// The section as displayed, after frontmatter and LaTeX filtering.
    pub displayed: Option<(u64, std::sync::Arc<String>)>,
    /// The rendered lines, and the line each top-level block starts on.
    pub rendered: Option<(u64, std::sync::Arc<RenderedLines>)>,
    /// The lines wrapped to the pane, with search highlighting.
    pub view: Option<(u64, std::sync::Arc<ContentView>)>,
}

/// A rendered section, wrapped to a width.
#[derive(Debug, Default)]
pub struct ContentView {
    /// One entry per screen row.
    pub rows: Vec<Line<'static>>,
    /// For each line the renderer emitted, the first row it is drawn on,
    /// plus a final entry holding the row count.
    pub line_rows: Vec<usize>,
    /// For each top-level block, the line the renderer started it on.
    pub block_lines: Vec<usize>,
    /// The plain text of each line, before wrapping, for searching.
    pub line_text: Vec<String>,
    /// For each row, the line it belongs to and the byte offset into that
    /// line's text where it starts.
    pub row_origin: Vec<(usize, usize)>,
}

impl ContentView {
    /// Wrap `lines` to `width` columns. `block_lines` is where the renderer
    /// started each top-level block, in line indexes.
    pub fn new(lines: Vec<Line<'static>>, width: u16, block_lines: Vec<usize>) -> Self {
        let mut view = Self {
            block_lines,
            ..Self::default()
        };
        for (index, line) in lines.into_iter().enumerate() {
            view.line_rows.push(view.rows.len());
            view.line_text.push(
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect(),
            );
            for (row, offset) in wrap_line(line, width) {
                view.rows.push(row);
                view.row_origin.push((index, offset));
            }
        }
        view.line_rows.push(view.rows.len());
        view
    }

    /// The first row of `line`, clamped to the end.
    pub fn row_of_line(&self, line: usize) -> usize {
        let last = self.line_rows.len().saturating_sub(1);
        self.line_rows
            .get(line)
            .copied()
            .unwrap_or(self.line_rows[last])
    }

    /// The row holding byte `offset` of `line`'s text.
    pub fn row_of_offset(&self, line: usize, offset: usize) -> usize {
        let first = self.row_of_line(line);
        let end = self.row_of_line(line + 1);
        (first..end)
            .take_while(|&row| self.row_origin[row].1 <= offset)
            .last()
            .unwrap_or(first)
    }

    /// The rows an element covers, given the lines it was counted at.
    ///
    /// Elements are counted by the interactive index, which walks the same
    /// blocks without wrapping. Each element is placed relative to its own
    /// block's start as the renderer actually drew it, so a miscount in one
    /// block stays in that block instead of displacing everything below,
    /// and the result is then mapped through the wrapping.
    pub fn element_rows(
        &self,
        block_idx: usize,
        counted_block_line: usize,
        (start, end): (usize, usize),
    ) -> Option<(usize, usize)> {
        let block_line = *self.block_lines.get(block_idx)?;
        let next_block = self
            .block_lines
            .get(block_idx + 1)
            .copied()
            .unwrap_or(self.line_rows.len().saturating_sub(1));
        let first = block_line + start.saturating_sub(counted_block_line);
        let last = (block_line + end.saturating_sub(counted_block_line))
            .min(next_block)
            .max(first + 1);
        Some((self.row_of_line(first), self.row_of_line(last)))
    }
}

/// Columns between tab stops.
const TAB_WIDTH: usize = 4;

/// One grapheme of a line, with the style of the span it came from.
struct Cell<'a> {
    text: &'a str,
    width: usize,
    style: Style,
    /// Byte offset in the line's text.
    offset: usize,
}

/// Word-wrap one styled line to `width` columns, returning each row with
/// the byte offset in the line's text where it starts.
///
/// Breaks after whitespace where it can and inside a word only when the word
/// alone is wider than the row. Leading whitespace is kept, so indentation
/// and code survive, and no row is ever wider than `width`, which is what
/// lets a frame draw rows without wrapping them again.
fn wrap_line(line: Line<'static>, width: u16) -> Vec<(Line<'static>, usize)> {
    let width = usize::from(width);
    let Line {
        style: line_style,
        alignment,
        spans,
    } = line;

    let mut cells = Vec::new();
    let mut offset = 0;
    let mut column = 0;
    let mut has_tab = false;
    for span in &spans {
        for grapheme in span.content.graphemes(true) {
            // A tab is a control character, which the terminal buffer drops,
            // so outside code (which expands its own) tabs vanished from the
            // text. It becomes spaces to the next stop.
            if grapheme == "\t" {
                has_tab = true;
                let stop = TAB_WIDTH - column % TAB_WIDTH;
                for _ in 0..stop {
                    cells.push(Cell {
                        text: " ",
                        width: 1,
                        style: span.style,
                        offset,
                    });
                }
                column += stop;
                offset += grapheme.len();
                continue;
            }
            let width = grapheme.width();
            cells.push(Cell {
                text: grapheme,
                width,
                style: span.style,
                offset,
            });
            column += width;
            offset += grapheme.len();
        }
    }

    let build = |cells: &[Cell]| -> Line<'static> {
        let mut out: Vec<Span<'static>> = Vec::new();
        for cell in cells {
            match out.last_mut() {
                Some(span) if span.style == cell.style => {
                    span.content.to_mut().push_str(cell.text);
                }
                _ => out.push(Span::styled(cell.text.to_string(), cell.style)),
            }
        }
        let mut line = Line::from(out).style(line_style);
        line.alignment = alignment;
        line
    };

    let total: usize = cells.iter().map(|cell| cell.width).sum();
    if !has_tab && (width == 0 || total <= width) {
        let mut whole = Line::from(spans).style(line_style);
        whole.alignment = alignment;
        return vec![(whole, 0)];
    }
    if width == 0 || total <= width {
        return vec![(build(&cells), 0)];
    }

    let mut rows = Vec::new();
    let mut start = 0;
    while start < cells.len() {
        let mut used = 0;
        let mut end = start;
        // The last place in this row a break may go: just after whitespace.
        let mut last_break = None;
        while end < cells.len() && used + cells[end].width <= width {
            used += cells[end].width;
            if cells[end].text.chars().all(char::is_whitespace) {
                last_break = Some(end + 1);
            }
            end += 1;
        }
        if end < cells.len()
            && let Some(at) = last_break.filter(|&at| at > start)
        {
            end = at;
        }
        // A grapheme wider than the row still has to go somewhere.
        if end == start {
            end = start + 1;
        }
        rows.push((build(&cells[start..end]), cells[start].offset));
        start = end;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};

    fn text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn a_line_that_fits_is_one_row() {
        let view = ContentView::new(vec![Line::from("short"), Line::from("")], 20, vec![0]);
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.line_rows, [0, 1, 2]);
    }

    #[test]
    fn a_long_line_breaks_at_spaces_and_keeps_its_styles() {
        let bold = Style::default().fg(Color::Red);
        let line = Line::from(vec![
            Span::raw("alpha beta "),
            Span::styled("gamma", bold),
            Span::raw(" delta"),
        ]);
        let view = ContentView::new(vec![line, Line::from("next")], 11, vec![0]);
        let rows: Vec<String> = view.rows.iter().map(text).collect();
        assert_eq!(rows, ["alpha beta ", "gamma delta", "next"]);
        assert_eq!(view.rows[1].spans[0].style, bold);
        assert_eq!(view.line_rows, [0, 2, 3]);
        // Every row fits.
        assert!(view.rows.iter().all(|row| row.width() <= 11));
    }

    #[test]
    fn tabs_become_spaces_to_the_next_stop() {
        let view = ContentView::new(vec![Line::from("a\tb\t\tc")], 40, vec![0]);
        assert_eq!(text(&view.rows[0]), "a   b       c");
    }

    #[test]
    fn a_word_wider_than_the_row_is_split() {
        let view = ContentView::new(vec![Line::from("abcdefghij")], 4, vec![0]);
        let rows: Vec<String> = view.rows.iter().map(text).collect();
        assert_eq!(rows, ["abcd", "efgh", "ij"]);
        assert_eq!(view.row_origin, [(0, 0), (0, 4), (0, 8)]);
        assert_eq!(view.row_of_offset(0, 5), 1);
    }

    #[test]
    fn wide_characters_are_measured_by_their_columns() {
        let view = ContentView::new(vec![Line::from("日本語日本語")], 4, vec![0]);
        let rows: Vec<String> = view.rows.iter().map(text).collect();
        assert_eq!(rows, ["日本", "語日", "本語"]);
    }

    proptest::proptest! {
        /// No row is wider than the pane unless it is one grapheme too wide
        /// to fit anywhere, and the rows put back together are the line.
        #[test]
        fn wrapping_fits_the_width_and_loses_nothing(
            line in "[a-z \u{65e5}\u{672c}\u{1f600}-]{0,120}",
            width in 1u16..40,
        ) {
            let view = ContentView::new(vec![Line::from(line.clone())], width, vec![0]);
            for row in &view.rows {
                proptest::prop_assert!(
                    row.width() <= usize::from(width) || text(row).graphemes(true).count() == 1
                );
            }
            let joined: String = view.rows.iter().map(text).collect();
            proptest::prop_assert_eq!(joined, line);
            proptest::prop_assert_eq!(view.line_rows, vec![0, view.rows.len()]);
        }
    }

    /// An element is placed from its own block's drawn start, so a block
    /// counted one line short does not shift the next block's elements.
    #[test]
    fn element_rows_are_anchored_to_their_block() {
        let lines = (0..6).map(|i| Line::from(format!("line {i}"))).collect();
        let view = ContentView::new(lines, 40, vec![0, 3]);
        // Counted at line 2 when the renderer started block 1 at line 3.
        assert_eq!(view.element_rows(1, 2, (2, 3)), Some((3, 4)));
        // An end counted past the block is held to it.
        assert_eq!(view.element_rows(0, 0, (0, 9)), Some((0, 3)));
    }
}
