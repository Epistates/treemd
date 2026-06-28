//! Pure text transformations for in-place file edits (checkbox toggling,
//! table cell editing).
//!
//! These functions are kept free of I/O and `App` state so the file-mutation
//! logic can be unit-tested directly. They preserve the document's original
//! line endings (LF or CRLF, per line) and the presence/absence of a trailing
//! newline, and they are code-fence aware so edits never land inside fenced
//! code blocks that merely look like tables or task lists.

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

/// Split a table row on unescaped `|` characters, returning the segments
/// between pipes. `\|` (an escaped pipe inside a cell) does not split.
fn split_row_segments(line: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut backslashes = 0usize;

    for ch in line.chars() {
        if ch == '|' && backslashes.is_multiple_of(2) {
            segments.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
        backslashes = if ch == '\\' { backslashes + 1 } else { 0 };
    }
    segments.push(current);
    segments
}

/// True if a table line is a header/body separator row like `| --- | :-: |`.
fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    let segments = split_row_segments(trimmed);
    // Drop the empty segments produced by the leading/trailing pipes
    let cells: Vec<&str> = segments
        .iter()
        .map(|s| s.trim())
        .skip(1)
        .take(segments.len().saturating_sub(2))
        .collect();

    !cells.is_empty()
        && cells.iter().all(|cell| {
            let body = cell
                .strip_prefix(':')
                .unwrap_or(cell)
                .strip_suffix(':')
                .unwrap_or_else(|| cell.strip_prefix(':').unwrap_or(cell));
            !body.is_empty() && body.chars().all(|c| c == '-')
        })
}

/// True if a line (outside code fences) is part of a pipe table.
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 1
}

/// Replace a specific cell in a table row line, respecting escaped pipes.
/// Returns `None` when `col` is out of range for the row.
fn replace_cell_in_row(line: &str, col: usize, new_value: &str) -> Option<String> {
    let mut segments = split_row_segments(line);
    // Format: | cell0 | cell1 |  →  ["", " cell0 ", " cell1 ", ""]
    let target = col + 1;
    if target >= segments.len().saturating_sub(1) {
        return None;
    }
    segments[target] = format!(" {} ", new_value);
    Some(segments.join("|"))
}

/// Replace a table cell in `content`.
///
/// Tables are counted starting at `section_start_line` (0-indexed); fenced
/// code blocks anywhere in the document are skipped. `table_index` is the
/// 0-based table index *within the section*, `row` 0 is the header row
/// (separator rows are not counted), `col` is the 0-based column.
pub fn replace_table_cell(
    content: &str,
    section_start_line: usize,
    table_index: usize,
    row: usize,
    col: usize,
    new_value: &str,
) -> Result<String, String> {
    let lines = split_lines_keep_endings(content);
    let mut out = String::with_capacity(content.len() + new_value.len());
    let mut fences = FenceTracker::default();
    let mut in_table = false;
    let mut table_row_idx = 0usize;
    // Counts tables seen from section_start_line onward; starts "one before"
    // so the first table found becomes index 0.
    let mut current_table: Option<usize> = None;
    let mut modified = false;

    for (idx, (text, ending)) in lines.iter().enumerate() {
        let in_fence = fences.feed(text);
        let pipe_line = !in_fence && idx >= section_start_line && is_table_line(text);

        if pipe_line {
            if !in_table {
                // A pipe-delimited block is only a GFM table when its header
                // row is immediately followed by a delimiter row. This mirrors
                // the parser used to compute `table_index`, so the two never
                // disagree on what counts as a table; bare `|a|b|` blocks are
                // emitted as plain content.
                let is_real_table = lines
                    .get(idx + 1)
                    .map(|(next, _)| is_table_line(next) && is_separator_row(next))
                    .unwrap_or(false);
                if !is_real_table {
                    out.push_str(text);
                    out.push_str(ending);
                    continue;
                }
                in_table = true;
                table_row_idx = 0;
                current_table = Some(current_table.map_or(0, |t| t + 1));
            }

            if is_separator_row(text) {
                out.push_str(text);
                out.push_str(ending);
                continue;
            }

            if !modified && current_table == Some(table_index) && table_row_idx == row {
                match replace_cell_in_row(text, col, new_value) {
                    Some(new_line) => {
                        out.push_str(&new_line);
                        modified = true;
                    }
                    None => {
                        return Err(format!(
                            "Column {} out of range in table {} row {}",
                            col, table_index, row
                        ));
                    }
                }
            } else {
                out.push_str(text);
            }
            out.push_str(ending);
            table_row_idx += 1;
        } else {
            in_table = false;
            out.push_str(text);
            out.push_str(ending);
        }
    }

    if modified {
        Ok(out)
    } else {
        Err(format!(
            "Table {} not found or row {} not found",
            table_index, row
        ))
    }
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

    #[test]
    fn separator_detection() {
        assert!(is_separator_row("| --- | --- |"));
        assert!(is_separator_row("| :-: | ---: |"));
        assert!(!is_separator_row("| a --- b | c |"));
        assert!(!is_separator_row("| |"));
    }
}
