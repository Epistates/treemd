//! Pure text transformations for in-place file edits (checkbox toggling,
//! table cell editing).
//!
//! These functions are kept free of I/O and `App` state so the file-mutation
//! logic can be unit-tested directly. They preserve the document's original
//! line endings (LF or CRLF, per line) and the presence/absence of a trailing
//! newline, and they are code-fence aware so edits never land inside fenced
//! code blocks that merely look like tables or task lists.

use std::ops::Range;

/// Tracks fenced code blocks (``` or ~~~) while scanning lines.
#[derive(Default)]
struct FenceTracker {
    /// Open fence: (fence char, fence length)
    open: Option<(char, usize)>,
}

impl FenceTracker {
    /// Feed a line to the tracker. Returns true if the line is *inside* a
    /// fenced code block (the delimiter lines themselves count as inside).
    fn feed(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if let Some((ch, len)) = self.open {
            // A closing fence: same char, at least as long, nothing but the
            // fence char and trailing whitespace.
            let run = trimmed.chars().take_while(|&c| c == ch).count();
            if indent <= 3 && run >= len && trimmed[run..].trim().is_empty() {
                self.open = None;
            }
            true
        } else {
            for ch in ['`', '~'] {
                let run = trimmed.chars().take_while(|&c| c == ch).count();
                if indent <= 3 && run >= 3 {
                    // Backtick fences can't have backticks in the info string
                    if ch == '`' && trimmed[run..].contains('`') {
                        continue;
                    }
                    self.open = Some((ch, run));
                    return true;
                }
            }
            false
        }
    }
}

/// Split a line into (text, line_ending) pairs, preserving each line's own
/// terminator so edits never normalize line endings.
fn split_lines_keep_endings(content: &str) -> Vec<(&str, &str)> {
    content
        .split_inclusive('\n')
        .map(|seg| {
            if let Some(text) = seg.strip_suffix("\r\n") {
                (text, "\r\n")
            } else if let Some(text) = seg.strip_suffix('\n') {
                (text, "\n")
            } else {
                (seg, "")
            }
        })
        .collect()
}

/// Byte ranges of the top-level tables in `markdown`, in document order.
///
/// Located by pulldown-cmark, the parser turbovault-parser runs, with the
/// same options, so what counts as a table here is what counted as one when
/// the user selected it: pipe-less rows (`a | b`) included, a table nested in
/// a list item or a quote excluded, a fence that merely looks like one
/// skipped. Re-detecting tables with a line heuristic disagreed with the
/// parser on all three, and an index from one applied to the other edited a
/// different table.
fn top_level_tables(markdown: &str) -> Vec<Range<usize>> {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut depth = 0usize;
    let mut tables = Vec::new();
    for (event, range) in Parser::new_ext(markdown, options).into_offset_iter() {
        match event {
            Event::Start(Tag::Table(_)) if depth == 0 => {
                tables.push(range);
                depth += 1;
            }
            Event::Start(_) => depth += 1,
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    tables
}

/// Byte ranges of the cells in one table row, between its unescaped pipes,
/// for rows written with or without outer pipes. `\|` does not split.
fn cell_spans(line: &str) -> Vec<Range<usize>> {
    let mut pipes = Vec::new();
    let mut backslashes = 0usize;
    for (i, ch) in line.char_indices() {
        if ch == '|' && backslashes.is_multiple_of(2) {
            pipes.push(i);
        }
        backslashes = if ch == '\\' { backslashes + 1 } else { 0 };
    }

    let leading = line.trim_start().starts_with('|');
    let (mut start, inner) = match pipes.split_first() {
        Some((&first, rest)) if leading => (first + 1, rest),
        _ => (0, pipes.as_slice()),
    };
    let mut spans = Vec::new();
    for &pipe in inner {
        spans.push(start..pipe);
        start = pipe + 1;
    }
    if !line[start..].trim().is_empty() {
        spans.push(start..line.len());
    }
    spans
}

/// Byte offset in `content` where 0-indexed line `line` starts.
fn line_start(content: &str, line: usize) -> Option<usize> {
    if line == 0 {
        return Some(0);
    }
    content
        .match_indices('\n')
        .nth(line - 1)
        .map(|(i, _)| i + 1)
}

/// The byte range in `content` of one cell's source text, between its pipes,
/// surrounding spaces included.
///
/// `table_index` is the 0-based top-level table counting from line
/// `section_start_line` (0-indexed), `row` 0 is the header row and the
/// delimiter row is not counted, `col` is 0-based.
fn locate_table_cell(
    content: &str,
    section_start_line: usize,
    table_index: usize,
    row: usize,
    col: usize,
) -> Result<Range<usize>, String> {
    let section_start = line_start(content, section_start_line)
        .ok_or_else(|| format!("Line {section_start_line} is past the end of the document"))?;
    let section = &content[section_start..];
    let table = top_level_tables(section)
        .into_iter()
        .nth(table_index)
        .ok_or_else(|| format!("Table {table_index} not found"))?;

    // Row 0 is the header, line 1 the delimiter row, then the body rows.
    let line_number = if row == 0 { 0 } else { row + 1 };
    // The parser starts the table after the header's indentation; cells are
    // split from the start of the line, or a pipe-less header's first cell
    // loses its leading spaces.
    let table_start = section_start + table.start;
    let mut offset = content[..table_start].rfind('\n').map_or(0, |i| i + 1);
    let (line, line_offset) = content[offset..section_start + table.end]
        .split_inclusive('\n')
        .map(|segment| {
            let at = offset;
            offset += segment.len();
            (segment.trim_end_matches(['\n', '\r']), at)
        })
        .nth(line_number)
        .ok_or_else(|| format!("Row {row} not found in table {table_index}"))?;

    let span = cell_spans(line)
        .into_iter()
        .nth(col)
        .ok_or_else(|| format!("Column {col} out of range in table {table_index} row {row}"))?;
    Ok(line_offset + span.start..line_offset + span.end)
}

/// The source text of one table cell, exactly as written between its pipes:
/// formatting, escapes and surrounding spaces included. See
/// [`replace_table_cell`] for how the cell is addressed.
pub fn table_cell_source(
    content: &str,
    section_start_line: usize,
    table_index: usize,
    row: usize,
    col: usize,
) -> Result<String, String> {
    locate_table_cell(content, section_start_line, table_index, row, col)
        .map(|range| content[range].to_string())
}

/// Replace one table cell's source with `source`, verbatim. Undo uses this
/// with what [`table_cell_source`] returned, so the cell comes back byte for
/// byte, escapes and formatting included.
pub fn replace_table_cell_source(
    content: &str,
    section_start_line: usize,
    table_index: usize,
    row: usize,
    col: usize,
    source: &str,
) -> Result<String, String> {
    let range = locate_table_cell(content, section_start_line, table_index, row, col)?;
    let mut out = String::with_capacity(content.len() + source.len());
    out.push_str(&content[..range.start]);
    out.push_str(source);
    out.push_str(&content[range.end..]);
    Ok(out)
}

/// Replace a table cell in `content` with `new_value`, padded by a space on
/// each side.
///
/// Tables are counted from `section_start_line` (0-indexed), top-level ones
/// only, the way the parser reports them; fenced code is never a table.
/// `table_index` is the 0-based table *within the section*, `row` 0 is the
/// header row (the delimiter row is not counted), `col` is the 0-based
/// column. `new_value` must already have its pipes escaped.
pub fn replace_table_cell(
    content: &str,
    section_start_line: usize,
    table_index: usize,
    row: usize,
    col: usize,
    new_value: &str,
) -> Result<String, String> {
    replace_table_cell_source(
        content,
        section_start_line,
        table_index,
        row,
        col,
        &format!(" {new_value} "),
    )
}

/// A task-list line parsed into its components.
struct TaskLine<'a> {
    /// Byte offset of the state character inside `[ ]`/`[x]`/`[X]`
    state_offset: usize,
    checked: bool,
    /// Text after the checkbox marker
    text: &'a str,
}

/// Parse a task-list line like `- [ ] foo`, `* [x] bar`, `3. [X] baz`.
fn parse_task_line(line: &str) -> Option<TaskLine<'_>> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();

    // Bullet: -, *, + or an ordered marker like `3.` / `3)`
    let after_bullet = if let Some(rest) = trimmed
        .strip_prefix('-')
        .or_else(|| trimmed.strip_prefix('*'))
        .or_else(|| trimmed.strip_prefix('+'))
    {
        rest
    } else {
        let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return None;
        }
        let rest = &trimmed[digits..];
        rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?
    };

    let spaces = after_bullet.len() - after_bullet.trim_start().len();
    if spaces == 0 {
        return None;
    }
    let marker = after_bullet.trim_start();

    let mut chars = marker.chars();
    if chars.next() != Some('[') {
        return None;
    }
    let state = chars.next()?;
    if chars.next() != Some(']') {
        return None;
    }
    let checked = match state {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    // Marker must be followed by whitespace or end of line
    let after_marker = &marker[3..];
    if !after_marker.is_empty() && !after_marker.starts_with([' ', '\t']) {
        return None;
    }

    let state_offset = indent + (trimmed.len() - marker.len()) + 1;
    Some(TaskLine {
        state_offset,
        checked,
        text: after_marker.trim(),
    })
}

/// Toggle the `occurrence`-th checkbox (0-based) within `line_range`
/// (0-indexed, end-exclusive) whose stripped text equals `target_text` and
/// whose current state equals `current_checked`.
///
/// `strip` is applied to each candidate line's text before comparison
/// (the caller passes its inline-markdown stripper).
pub fn toggle_checkbox(
    content: &str,
    line_range: (usize, usize),
    target_text: &str,
    current_checked: bool,
    occurrence: usize,
    strip: impl Fn(&str) -> String,
) -> Result<String, String> {
    let lines = split_lines_keep_endings(content);
    let mut out = String::with_capacity(content.len());
    let mut fences = FenceTracker::default();
    let mut seen = 0usize;
    let mut toggled = false;

    for (idx, (text, ending)) in lines.iter().enumerate() {
        let in_fence = fences.feed(text);
        let in_range = idx >= line_range.0 && idx < line_range.1;

        if !toggled
            && !in_fence
            && in_range
            && let Some(task) = parse_task_line(text)
            && task.checked == current_checked
            && strip(task.text) == target_text
        {
            if seen == occurrence {
                let new_state = if current_checked { ' ' } else { 'x' };
                out.push_str(&text[..task.state_offset]);
                out.push(new_state);
                out.push_str(&text[task.state_offset + 1..]);
                out.push_str(ending);
                toggled = true;
                continue;
            }
            seen += 1;
        }

        out.push_str(text);
        out.push_str(ending);
    }

    if toggled {
        Ok(out)
    } else {
        Err(format!("Checkbox not found in file: '{}'", target_text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_strip(s: &str) -> String {
        s.to_string()
    }

    // --- checkbox toggling ---

    #[test]
    fn toggles_simple_checkbox() {
        let content = "# T\n\n- [ ] task one\n- [ ] task two\n";
        let out = toggle_checkbox(content, (0, 4), "task two", false, 0, no_strip).unwrap();
        assert_eq!(out, "# T\n\n- [ ] task one\n- [x] task two\n");
    }

    #[test]
    fn toggles_correct_duplicate_by_occurrence() {
        let content = "- [ ] same\n- [ ] same\n- [ ] same\n";
        let out = toggle_checkbox(content, (0, 3), "same", false, 1, no_strip).unwrap();
        assert_eq!(out, "- [ ] same\n- [x] same\n- [ ] same\n");
    }

    #[test]
    fn respects_section_line_range() {
        let content = "# A\n- [ ] dup\n# B\n- [ ] dup\n";
        // Only section B's lines are in range
        let out = toggle_checkbox(content, (2, 4), "dup", false, 0, no_strip).unwrap();
        assert_eq!(out, "# A\n- [ ] dup\n# B\n- [x] dup\n");
    }

    #[test]
    fn matches_checked_state() {
        // First "dup" is already checked; we're checking an unchecked one
        let content = "- [x] dup\n- [ ] dup\n";
        let out = toggle_checkbox(content, (0, 2), "dup", false, 0, no_strip).unwrap();
        assert_eq!(out, "- [x] dup\n- [x] dup\n");
    }

    #[test]
    fn unchecks_capital_x_without_touching_text() {
        let content = "- [X] see [x] note\n";
        let out = toggle_checkbox(content, (0, 1), "see [x] note", true, 0, no_strip).unwrap();
        assert_eq!(out, "- [ ] see [x] note\n");
    }

    #[test]
    fn supports_star_plus_and_ordered_bullets() {
        let content = "* [ ] star\n+ [ ] plus\n3. [ ] ordered\n";
        let out = toggle_checkbox(content, (0, 3), "star", false, 0, no_strip).unwrap();
        assert!(out.starts_with("* [x] star\n"));
        let out = toggle_checkbox(content, (0, 3), "plus", false, 0, no_strip).unwrap();
        assert!(out.contains("+ [x] plus\n"));
        let out = toggle_checkbox(content, (0, 3), "ordered", false, 0, no_strip).unwrap();
        assert!(out.contains("3. [x] ordered\n"));
    }

    #[test]
    fn ignores_checkboxes_inside_code_fences() {
        let content = "```\n- [ ] fake\n```\n- [ ] real\n";
        let out = toggle_checkbox(content, (0, 4), "real", false, 0, no_strip).unwrap();
        assert_eq!(out, "```\n- [ ] fake\n```\n- [x] real\n");
        // The fenced one is never a candidate
        assert!(toggle_checkbox(content, (0, 3), "fake", false, 0, no_strip).is_err());
    }

    #[test]
    fn preserves_crlf_and_missing_trailing_newline() {
        let content = "- [ ] a\r\n- [ ] b";
        let out = toggle_checkbox(content, (0, 2), "b", false, 0, no_strip).unwrap();
        assert_eq!(out, "- [ ] a\r\n- [x] b");
    }

    #[test]
    fn errors_when_not_found() {
        assert!(toggle_checkbox("- [ ] a\n", (0, 1), "zzz", false, 0, no_strip).is_err());
    }

    // --- table cell replacement ---

    #[test]
    fn replaces_cell_in_simple_table() {
        let content = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let out = replace_table_cell(content, 0, 0, 1, 1, "X").unwrap();
        assert_eq!(out, "| A | B |\n| --- | --- |\n| 1 | X |\n");
    }

    #[test]
    fn replaces_header_cell() {
        let content = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let out = replace_table_cell(content, 0, 0, 0, 0, "H").unwrap();
        assert_eq!(out, "| H | B |\n| --- | --- |\n| 1 | 2 |\n");
    }

    #[test]
    fn skips_pipe_lines_inside_code_fences() {
        let content = "```\n| not | table |\n| --- | --- |\n| x | y |\n```\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let out = replace_table_cell(content, 0, 0, 1, 0, "Z").unwrap();
        assert!(out.contains("| not | table |"));
        assert!(out.contains("| Z | 2 |"));
    }

    #[test]
    fn skips_tilde_fences() {
        let content = "~~~\n| fake | t |\n~~~\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let out = replace_table_cell(content, 0, 0, 1, 1, "Q").unwrap();
        assert!(out.contains("| fake | t |"));
        assert!(out.contains("| 1 | Q |"));
    }

    #[test]
    fn counts_tables_only_from_section_start() {
        let content = "| P | Q |\n| --- | --- |\n| p | q |\n\n# Section\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        // Section starts at line 4; table 0 within the section is the second table
        let out = replace_table_cell(content, 4, 0, 1, 0, "N").unwrap();
        assert!(out.contains("| p | q |"));
        assert!(out.contains("| N | 2 |"));
    }

    #[test]
    fn data_row_containing_dashes_is_not_a_separator() {
        let content = "| A | B |\n| --- | --- |\n| pre --- post | 2 |\n| 3 | 4 |\n";
        // Row 2 is the second data row (header=0, data rows 1, 2)
        let out = replace_table_cell(content, 0, 0, 2, 0, "X").unwrap();
        assert_eq!(
            out,
            "| A | B |\n| --- | --- |\n| pre --- post | 2 |\n| X | 4 |\n"
        );
    }

    #[test]
    fn respects_escaped_pipes_in_cells() {
        let content = "| A | B | C |\n| --- | --- | --- |\n| has \\| pipe | 2 | 3 |\n";
        let out = replace_table_cell(content, 0, 0, 1, 2, "Z").unwrap();
        assert_eq!(
            out,
            "| A | B | C |\n| --- | --- | --- |\n| has \\| pipe | 2 | Z |\n"
        );
    }

    #[test]
    fn preserves_trailing_newline_absence_and_crlf() {
        let content = "| A |\r\n| --- |\r\n| 1 |";
        let out = replace_table_cell(content, 0, 0, 1, 0, "X").unwrap();
        assert_eq!(out, "| A |\r\n| --- |\r\n| X |");
    }

    #[test]
    fn errors_on_missing_table() {
        assert!(replace_table_cell("plain text\n", 0, 0, 0, 0, "x").is_err());
    }

    #[test]
    fn bare_pipe_block_without_separator_is_not_counted_as_a_table() {
        // The leading `|a|b|` block has no delimiter row, so it must not shift
        // the table index — matching the parser's table detection.
        let content = "| a | b |\n| c | d |\n\n| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let out = replace_table_cell(content, 0, 0, 1, 0, "Z").unwrap();
        assert!(out.contains("| a | b |"));
        assert!(out.contains("| c | d |"));
        assert!(out.contains("| Z | 2 |"));
    }

    #[test]
    fn errors_on_out_of_range_column() {
        let content = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let err = replace_table_cell(content, 0, 0, 1, 9, "X").unwrap_err();
        assert!(err.contains("out of range"));
    }

    /// The parser counts a table without outer pipes, and a table nested in
    /// a list item is not a top-level table. A line heuristic disagreed on
    /// both, so the index the app computed from the parser edited a
    /// different table.
    #[test]
    fn tables_are_counted_the_way_the_parser_counts_them() {
        let content = "\
# S

a | b
--|--
1 | 2

- item

  | in | list |
  |----|------|
  | x  | y    |

| C | D |
|---|---|
| 3 | 4 |
";
        // Table 1 is `| C | D |`: the pipe-less one is table 0, and the one
        // inside the list item is not counted.
        let out = replace_table_cell(content, 0, 1, 1, 0, "EDITED").unwrap();
        assert!(out.contains("| EDITED | 4 |"), "{out}");
        assert!(out.contains("1 | 2"));
        assert!(out.contains("| x  | y    |"));

        // And the pipe-less table edits in place.
        let out = replace_table_cell(content, 0, 0, 1, 1, "Z").unwrap();
        assert!(out.contains("\n1 | Z \n"), "{out}");
    }

    /// A table of `cells` (the first row is the header), with or without
    /// outer pipes, between two paragraphs.
    fn table_doc(cells: &[Vec<String>], outer: bool, newline: &str) -> String {
        let row = |cells: &[String]| {
            let inner = cells.join(" | ");
            if outer { format!("| {inner} |") } else { inner }
        };
        let rule = row(&vec!["---".to_string(); cells[0].len()]);
        let mut lines = vec!["Intro.".to_string(), String::new(), row(&cells[0]), rule];
        lines.extend(cells[1..].iter().map(|cells| row(cells)));
        lines.extend([String::new(), "Outro.".to_string(), String::new()]);
        lines.join(newline)
    }

    proptest::proptest! {
        /// An edit changes one line of the document, and putting the cell's
        /// source back gives the document it started from.
        #[test]
        fn a_cell_edit_is_local_and_undoes_exactly(
            cells in proptest::collection::vec(
                proptest::collection::vec("[a-z0-9]{1,6}( [a-z0-9]{1,6})?", 3),
                2..6,
            ),
            outer in proptest::bool::ANY,
            crlf in proptest::bool::ANY,
            pick in (0usize..16, 0usize..3),
            value in "[a-z0-9]{1,8}",
        ) {
            let content = table_doc(&cells, outer, if crlf { "\r\n" } else { "\n" });
            let (row, col) = (pick.0 % cells.len(), pick.1);

            let original = table_cell_source(&content, 0, 0, row, col).unwrap();
            let edited = replace_table_cell(&content, 0, 0, row, col, &value).unwrap();
            let before: Vec<&str> = content.split('\n').collect();
            let after: Vec<&str> = edited.split('\n').collect();
            proptest::prop_assert_eq!(before.len(), after.len());
            let changed = before.iter().zip(&after).filter(|(a, b)| a != b).count();
            proptest::prop_assert!(changed <= 1, "{changed} lines changed:\n{edited}");

            let restored = replace_table_cell_source(&edited, 0, 0, row, col, &original).unwrap();
            proptest::prop_assert_eq!(restored, content);
        }
    }

    /// Undo restores a cell from its source text, byte for byte. Restoring
    /// the parser's text instead dropped an escaped pipe's backslash and
    /// split the cell in two.
    #[test]
    fn a_cell_round_trips_through_its_source() {
        let content = "| A | B |\n|---|---|\n| x \\| y | **bold** |\n";
        let original = table_cell_source(content, 0, 0, 1, 0).unwrap();
        assert_eq!(original, " x \\| y ");
        assert_eq!(
            table_cell_source(content, 0, 0, 1, 1).unwrap(),
            " **bold** "
        );

        let edited = replace_table_cell(content, 0, 0, 1, 0, "new").unwrap();
        let restored = replace_table_cell_source(&edited, 0, 0, 1, 0, &original).unwrap();
        assert_eq!(restored, content);
    }

    /// Found by the property above: the parser starts a table after its
    /// header's indentation, so once an edit gave a pipe-less header a
    /// leading space, undo put the cell back one space to the right.
    #[test]
    fn an_indented_header_cell_undoes_exactly() {
        let content = "Intro.\n\n a | b\n--- | ---\nc | d\n";
        let original = table_cell_source(content, 0, 0, 0, 0).unwrap();
        assert_eq!(original, " a ");
        let edited = replace_table_cell(content, 0, 0, 0, 0, "x").unwrap();
        assert_eq!(edited, "Intro.\n\n x | b\n--- | ---\nc | d\n");
        let restored = replace_table_cell_source(&edited, 0, 0, 0, 0, &original).unwrap();
        assert_eq!(restored, content);
    }

    #[test]
    fn a_fence_that_looks_like_a_table_is_not_one() {
        let content = "```\n| A | B |\n|---|---|\n| 1 | 2 |\n```\n\n| C |\n|---|\n| 3 |\n";
        let out = replace_table_cell(content, 0, 0, 1, 0, "X").unwrap();
        assert!(out.contains("| 1 | 2 |"));
        assert!(out.contains("| X |"));
    }
}
