//! Utility functions for UI rendering
//!
//! Pure functions for layout calculations, text parsing, and formatting.

use crate::parser::output::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// Calculate a popup area with minimum size constraints.
///
/// Returns a `Rect` that is centered within the parent area, sized as a
/// percentage but respecting minimum dimensions. If the parent is smaller
/// than the minimum, the popup will fill the available space.
///
/// # Arguments
/// * `area` - The parent area to center within
/// * `percent_x` - Width as a percentage of parent (0-100)
/// * `percent_y` - Height as a percentage of parent (0-100)
/// * `min_width` - Minimum width in columns (will not exceed parent width)
/// * `min_height` - Minimum height in rows (will not exceed parent height)
pub fn popup_area(
    area: Rect,
    percent_x: u16,
    percent_y: u16,
    min_width: u16,
    min_height: u16,
) -> Rect {
    // Calculate percentage-based dimensions
    let pct_width = area.width * percent_x / 100;
    let pct_height = area.height * percent_y / 100;

    // Apply minimum constraints, but don't exceed parent
    let width = pct_width.max(min_width).min(area.width);
    let height = pct_height.max(min_height).min(area.height);

    // Center the popup
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
}

/// Detect checkbox markers in text for task list items.
///
/// Parses text to identify task list checkbox patterns (`[x]`, `[X]`, `[ ]`).
///
/// # Arguments
/// * `text` - The text to parse
///
/// # Returns
/// A tuple of `(is_task, is_checked, remaining_text)`:
/// - `is_task`: true if a checkbox pattern was found
/// - `is_checked`: true if the checkbox is checked (`[x]` or `[X]`)
/// - `remaining_text`: the text after the checkbox marker (or original text if no checkbox)
pub fn detect_checkbox_in_text(text: &str) -> (bool, bool, &str) {
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

/// Align text within a fixed width using Unicode-aware width calculations.
///
/// Handles left, center, right, and none (defaults to left) alignments.
/// If text is longer than width, it will be truncated with ellipsis.
///
/// # Arguments
/// * `text` - The text to align
/// * `width` - The total width to align within (including padding)
/// * `alignment` - The alignment direction
///
/// # Returns
/// A string padded to the specified width with appropriate alignment.
pub fn align_text(text: &str, width: usize, alignment: &Alignment) -> String {
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

/// Highlight search matches within text, returning a Line with styled spans.
///
/// Performs case-insensitive matching and splits the text into segments,
/// applying the highlight style to matched portions.
///
/// # Arguments
/// * `text` - The text to search within
/// * `query` - The search query (case-insensitive)
/// * `base_style` - Style for non-matched text
/// * `highlight_style` - Style for matched text
///
/// # Returns
/// A vector of Spans with appropriate styling applied
pub fn highlight_search_matches(
    text: &str,
    query: &str,
    base_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    let mut spans = Vec::new();
    let mut last_end = 0;

    // Find all matches
    let mut search_start = 0;
    while let Some(rel_pos) = text_lower[search_start..].find(&query_lower) {
        let match_start = search_start + rel_pos;
        let match_end = match_start + query.len();

        // Verify char boundaries
        if !text.is_char_boundary(match_start) || !text.is_char_boundary(match_end) {
            search_start = match_start + 1;
            continue;
        }

        // Add text before match
        if match_start > last_end {
            spans.push(Span::styled(
                text[last_end..match_start].to_string(),
                base_style,
            ));
        }

        // Add highlighted match
        spans.push(Span::styled(
            text[match_start..match_end].to_string(),
            highlight_style,
        ));

        last_end = match_end;
        search_start = match_end;

        if search_start >= text.len() {
            break;
        }
    }

    // Add remaining text after last match
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }

    // If no matches found, return original text with base style
    if spans.is_empty() {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    spans
}

/// Build a Line with optional search highlighting.
///
/// Convenience wrapper that builds a complete Line, optionally with a prefix.
///
/// # Arguments
/// * `prefix` - Optional prefix spans to prepend
/// * `text` - The main text content
/// * `query` - Optional search query for highlighting
/// * `base_style` - Style for non-matched text
/// * `highlight_style` - Style for matched text
pub fn build_highlighted_line(
    prefix: Vec<Span<'static>>,
    text: &str,
    query: Option<&str>,
    base_style: Style,
    highlight_style: Style,
) -> Line<'static> {
    let mut spans = prefix;

    if let Some(q) = query {
        spans.extend(highlight_search_matches(
            text,
            q,
            base_style,
            highlight_style,
        ));
    } else {
        spans.push(Span::styled(text.to_string(), base_style));
    }

    Line::from(spans)
}

/// Strip YAML frontmatter from the beginning of a document.
///
/// Frontmatter must:
/// - Start at the very beginning of the document (possibly after leading newlines)
/// - Begin with `---` on its own line
/// - End with `---` on its own line
///
/// # Arguments
/// * `content` - The document content
///
/// # Returns
/// Content with frontmatter removed, or original content if no frontmatter found
pub fn strip_frontmatter(content: &str) -> String {
    // Frontmatter must start at the beginning (after optional whitespace/newlines)
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---") {
        return content.to_string();
    }

    // Find the closing ---
    // The pattern is: ---\n...\n---\n (or end of content)
    if let Some(rest) = trimmed.strip_prefix("---") {
        // Find the closing marker (must be on its own line)
        if let Some(end_pos) = rest.find("\n---") {
            // Skip past the closing ---
            let after_close = &rest[end_pos + 4..];
            // Also skip the newline after the closing --- if present
            let result = after_close.strip_prefix('\n').unwrap_or(after_close);
            return result.to_string();
        }
    }

    content.to_string()
}

/// Strip LaTeX math expressions from content.
///
/// Removes:
/// - Inline math: `$...$`
/// - Display math: `$$...$$`
/// - LaTeX environments: `\begin{...}...\end{...}`
///
/// # Arguments
/// * `content` - The document content
///
/// # Returns
/// Content with LaTeX expressions removed
pub fn strip_latex(content: &str) -> String {
    use regex::Regex;

    // Match display math ($$...$$) first - must be removed before inline math
    let display_math = Regex::new(r"\$\$[\s\S]*?\$\$").unwrap();
    let result = display_math.replace_all(content, "");

    // Match inline math ($...$) - simple pattern that requires content between dollars
    // Match: $ followed by non-empty content (no newlines, no unescaped $) ending with $
    // This avoids matching lone $ signs like currency
    let inline_math = Regex::new(r"\$([^\$\n]+)\$").unwrap();
    let result = inline_math.replace_all(&result, "");

    // Match \begin{...}...\end{...} environments
    let latex_env = Regex::new(r"\\begin\{[^}]+\}[\s\S]*?\\end\{[^}]+\}").unwrap();
    let result = latex_env.replace_all(&result, "");

    result.to_string()
}

/// Apply content filters based on configuration.
///
/// Strips frontmatter and/or LaTeX based on the provided flags.
///
/// # Arguments
/// * `content` - The document content
/// * `hide_frontmatter` - Whether to strip YAML frontmatter
/// * `hide_latex` - Whether to strip LaTeX expressions
///
/// # Returns
/// Filtered content
pub fn filter_content(content: &str, hide_frontmatter: bool, hide_latex: bool) -> String {
    let mut result = content.to_string();

    if hide_frontmatter {
        result = strip_frontmatter(&result);
    }

    if hide_latex {
        result = strip_latex(&result);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    mod strip_frontmatter_tests {
        use super::*;

        #[test]
        fn test_simple_frontmatter() {
            let content = "---\ntitle: Test\n---\n\n# Heading\n\nContent";
            let result = strip_frontmatter(content);
            assert_eq!(result, "\n# Heading\n\nContent");
        }

        #[test]
        fn test_no_frontmatter() {
            let content = "# Heading\n\nContent";
            let result = strip_frontmatter(content);
            assert_eq!(result, content);
        }

        #[test]
        fn test_frontmatter_with_yaml() {
            let content = "---\ntitle: My Doc\ntags:\n  - rust\n  - markdown\n---\n# Start";
            let result = strip_frontmatter(content);
            assert_eq!(result, "# Start");
        }

        #[test]
        fn test_frontmatter_not_at_start() {
            let content = "Some text\n---\ntitle: Test\n---\nMore text";
            let result = strip_frontmatter(content);
            assert_eq!(result, content); // Should not strip
        }
    }

    mod strip_latex_tests {
        use super::*;

        #[test]
        fn test_inline_math() {
            let content = "The formula $x^2$ is quadratic";
            let result = strip_latex(content);
            assert_eq!(result, "The formula  is quadratic");
        }

        #[test]
        fn test_display_math() {
            let content = "The equation:\n$$\nE = mc^2\n$$\nis famous.";
            let result = strip_latex(content);
            assert_eq!(result, "The equation:\n\nis famous.");
        }

        #[test]
        fn test_latex_environment() {
            let content = "An equation:\n\\begin{equation}\ny = mx + b\n\\end{equation}\ndone.";
            let result = strip_latex(content);
            assert_eq!(result, "An equation:\n\ndone.");
        }

        #[test]
        fn test_no_latex() {
            let content = "Regular text without math";
            let result = strip_latex(content);
            assert_eq!(result, content);
        }

        #[test]
        fn test_money_not_stripped() {
            // Currency like "$5" shouldn't match inline math pattern
            // because it would need content between the dollars
            let content = "It costs $5";
            let result = strip_latex(content);
            // This won't match because there's no closing $
            assert_eq!(result, content);
        }
    }

    mod detect_checkbox_tests {
        use super::*;

        #[test]
        fn test_checked_lowercase() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[x] Task done");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "Task done");
        }

        #[test]
        fn test_checked_uppercase() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[X] Also done");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "Also done");
        }

        #[test]
        fn test_unchecked() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[ ] Not done yet");
            assert!(is_task);
            assert!(!is_checked);
            assert_eq!(text, "Not done yet");
        }

        #[test]
        fn test_not_a_task() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("Regular text");
            assert!(!is_task);
            assert!(!is_checked);
            assert_eq!(text, "Regular text");
        }

        #[test]
        fn test_with_leading_whitespace() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("  [x] Indented task");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "Indented task");
        }

        #[test]
        fn test_empty_task() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[x]");
            assert!(is_task);
            assert!(is_checked);
            assert_eq!(text, "");
        }

        #[test]
        fn test_bracket_but_not_checkbox() {
            let (is_task, is_checked, text) = detect_checkbox_in_text("[a] Not a checkbox");
            assert!(!is_task);
            assert!(!is_checked);
            assert_eq!(text, "[a] Not a checkbox");
        }
    }

    mod align_text_tests {
        use super::*;

        #[test]
        fn test_left_align() {
            let result = align_text("Hi", 10, &Alignment::Left);
            assert_eq!(result, " Hi       ");
            assert_eq!(result.len(), 10);
        }

        #[test]
        fn test_right_align() {
            let result = align_text("Hi", 10, &Alignment::Right);
            assert_eq!(result, "       Hi ");
            assert_eq!(result.len(), 10);
        }

        #[test]
        fn test_center_align() {
            let result = align_text("Hi", 10, &Alignment::Center);
            assert_eq!(result, "    Hi    ");
            assert_eq!(result.len(), 10);
        }

        #[test]
        fn test_none_defaults_to_left() {
            let result = align_text("Hi", 10, &Alignment::None);
            assert_eq!(result, " Hi       ");
        }

        #[test]
        fn test_truncation_when_too_long() {
            let result = align_text("This is a very long text", 10, &Alignment::Left);
            assert!(result.contains("..."));
            // Should be truncated with ellipsis
        }

        #[test]
        fn test_exact_width() {
            let result = align_text("Test", 6, &Alignment::Left);
            assert_eq!(result, " Test ");
        }

        #[test]
        fn test_unicode_width() {
            // Japanese characters are typically 2 columns wide
            let result = align_text("日本", 10, &Alignment::Left);
            // "日本" is 4 columns wide (2 chars * 2 width each)
            // Result should be " 日本     " (1 space + 4 cols + 5 spaces = 10)
            assert_eq!(result.width(), 10);
        }

        #[test]
        fn test_center_odd_padding() {
            // When padding can't be split evenly, extra space goes to right
            let result = align_text("A", 10, &Alignment::Center);
            // "A" is 1 wide, 9 spaces to distribute: 4 left, 5 right
            assert_eq!(result, "    A     ");
        }
    }

    mod highlight_search_tests {
        use super::*;
        use ratatui::style::Color;

        #[test]
        fn test_no_match() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "xyz", base, highlight);
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content.as_ref(), "Hello World");
        }

        #[test]
        fn test_single_match() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "World", base, highlight);
            assert_eq!(spans.len(), 2);
            assert_eq!(spans[0].content.as_ref(), "Hello ");
            assert_eq!(spans[1].content.as_ref(), "World");
            assert_eq!(spans[1].style, highlight);
        }

        #[test]
        fn test_case_insensitive() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "world", base, highlight);
            assert_eq!(spans.len(), 2);
            assert_eq!(spans[1].content.as_ref(), "World"); // Preserves original case
        }

        #[test]
        fn test_multiple_matches() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("foo bar foo", "foo", base, highlight);
            assert_eq!(spans.len(), 3);
            assert_eq!(spans[0].content.as_ref(), "foo");
            assert_eq!(spans[1].content.as_ref(), " bar ");
            assert_eq!(spans[2].content.as_ref(), "foo");
        }

        #[test]
        fn test_empty_query() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello", "", base, highlight);
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content.as_ref(), "Hello");
        }

        #[test]
        fn test_match_at_start() {
            let base = Style::default().fg(Color::White);
            let highlight = Style::default().fg(Color::Yellow);
            let spans = highlight_search_matches("Hello World", "Hello", base, highlight);
            assert_eq!(spans.len(), 2);
            assert_eq!(spans[0].content.as_ref(), "Hello");
            assert_eq!(spans[0].style, highlight);
            assert_eq!(spans[1].content.as_ref(), " World");
        }
    }
}
