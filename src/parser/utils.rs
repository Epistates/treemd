//! Utility functions for markdown parsing.
//!
//! Shared helper functions used across the parser module.

use crate::parser::output::InlineElement;

/// Strip inline markdown formatting (bold, italic, code, strikethrough) from text.
///
/// This is useful when comparing heading text extracted from events (which strips formatting)
/// against the raw markdown source (which contains formatting).
///
/// Handles: `**bold**`, `__bold__`, `*italic*`, `_italic_` (with snake_case protection),
/// `` `code` ``, and `~~strikethrough~~`.
///
/// # Examples
///
/// ```
/// # use treemd::parser::utils::strip_markdown_inline;
/// assert_eq!(strip_markdown_inline("**bold** text"), "bold text");
/// assert_eq!(strip_markdown_inline("`code` here"), "code here");
/// assert_eq!(strip_markdown_inline("snake_case_var"), "snake_case_var");
/// ```
pub fn strip_markdown_inline(text: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;

    static PATTERNS: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        vec![
            (Regex::new(r"~~(.+?)~~").unwrap(), "$1"),
            (Regex::new(r"\*\*(.+?)\*\*").unwrap(), "$1"),
            (Regex::new(r"__(.+?)__").unwrap(), "$1"),
            (Regex::new(r"`([^`]+)`").unwrap(), "$1"),
            (Regex::new(r"\*(.+?)\*").unwrap(), "$1"),
            (
                Regex::new(r"(^|[^a-zA-Z0-9])_([^_]+)_([^a-zA-Z0-9]|$)").unwrap(),
                "$1$2$3",
            ),
        ]
    });

    let mut result = text.to_string();
    for (pattern, replacement) in patterns {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    result
}

/// Parse inline HTML tags into InlineElements.
///
/// Converts HTML tags like `<strong>`, `<b>`, `<em>`, `<i>`, `<code>` into
/// their corresponding InlineElement types for proper rendering.
///
/// # Examples
///
/// ```
/// # use treemd::parser::utils::parse_inline_html;
/// let elements = parse_inline_html("<strong>Navigation</strong>");
/// assert_eq!(elements.len(), 1);
/// ```
pub fn parse_inline_html(html: &str) -> Vec<InlineElement> {
    use regex::Regex;
    use std::sync::OnceLock;

    // Pattern for each tag type (can't use backreferences in Rust regex)
    static STRONG_PATTERN: OnceLock<Regex> = OnceLock::new();
    static B_PATTERN: OnceLock<Regex> = OnceLock::new();
    static EM_PATTERN: OnceLock<Regex> = OnceLock::new();
    static I_PATTERN: OnceLock<Regex> = OnceLock::new();
    static CODE_PATTERN: OnceLock<Regex> = OnceLock::new();

    let strong_re = STRONG_PATTERN.get_or_init(|| Regex::new(r"<strong>(.*?)</strong>").unwrap());
    let b_re = B_PATTERN.get_or_init(|| Regex::new(r"<b>(.*?)</b>").unwrap());
    let em_re = EM_PATTERN.get_or_init(|| Regex::new(r"<em>(.*?)</em>").unwrap());
    let i_re = I_PATTERN.get_or_init(|| Regex::new(r"<i>(.*?)</i>").unwrap());
    let code_re = CODE_PATTERN.get_or_init(|| Regex::new(r"<code>(.*?)</code>").unwrap());

    // Collect all matches with their positions and types
    #[derive(Debug)]
    struct TagMatch {
        start: usize,
        end: usize,
        content: String,
        tag_type: TagType,
    }

    #[derive(Debug)]
    enum TagType {
        Strong,
        Emphasis,
        Code,
    }

    let mut matches: Vec<TagMatch> = Vec::new();

    // Find all strong tags
    for cap in strong_re.captures_iter(html) {
        let m = cap.get(0).unwrap();
        matches.push(TagMatch {
            start: m.start(),
            end: m.end(),
            content: cap.get(1).unwrap().as_str().to_string(),
            tag_type: TagType::Strong,
        });
    }

    // Find all b tags
    for cap in b_re.captures_iter(html) {
        let m = cap.get(0).unwrap();
        matches.push(TagMatch {
            start: m.start(),
            end: m.end(),
            content: cap.get(1).unwrap().as_str().to_string(),
            tag_type: TagType::Strong,
        });
    }

    // Find all em tags
    for cap in em_re.captures_iter(html) {
        let m = cap.get(0).unwrap();
        matches.push(TagMatch {
            start: m.start(),
            end: m.end(),
            content: cap.get(1).unwrap().as_str().to_string(),
            tag_type: TagType::Emphasis,
        });
    }

    // Find all i tags
    for cap in i_re.captures_iter(html) {
        let m = cap.get(0).unwrap();
        matches.push(TagMatch {
            start: m.start(),
            end: m.end(),
            content: cap.get(1).unwrap().as_str().to_string(),
            tag_type: TagType::Emphasis,
        });
    }

    // Find all code tags
    for cap in code_re.captures_iter(html) {
        let m = cap.get(0).unwrap();
        matches.push(TagMatch {
            start: m.start(),
            end: m.end(),
            content: cap.get(1).unwrap().as_str().to_string(),
            tag_type: TagType::Code,
        });
    }

    // Sort by start position; on tie, prefer the longer match (outer tag)
    // so nested inner matches are dropped instead of duplicating content.
    matches.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    // Drop any match whose range is contained in the previous accepted one —
    // e.g. `<em>` inside `<b><em>x</em></b>` gets discarded so we don't emit
    // both Strong("<em>x</em>") and Emphasis("x").
    let mut filtered: Vec<TagMatch> = Vec::with_capacity(matches.len());
    for m in matches {
        if let Some(prev) = filtered.last()
            && m.start < prev.end
        {
            continue; // nested or overlapping — skip
        }
        filtered.push(m);
    }

    // Build elements
    let mut elements = Vec::new();
    let mut last_end = 0;

    for tag_match in filtered {
        // Add any text before this tag
        if tag_match.start > last_end {
            let text = &html[last_end..tag_match.start];
            if !text.is_empty() {
                elements.push(InlineElement::Text {
                    value: text.to_string(),
                });
            }
        }

        // Add the styled element
        match tag_match.tag_type {
            TagType::Strong => {
                elements.push(InlineElement::Strong {
                    value: tag_match.content,
                });
            }
            TagType::Emphasis => {
                elements.push(InlineElement::Emphasis {
                    value: tag_match.content,
                });
            }
            TagType::Code => {
                elements.push(InlineElement::Code {
                    value: tag_match.content,
                });
            }
        }

        last_end = tag_match.end;
    }

    // Add any remaining text after the last tag
    if last_end < html.len() {
        let text = &html[last_end..];
        if !text.is_empty() {
            elements.push(InlineElement::Text {
                value: text.to_string(),
            });
        }
    }

    // If no tags were found, return the whole string as text
    if elements.is_empty() && !html.is_empty() {
        elements.push(InlineElement::Text {
            value: html.to_string(),
        });
    }

    elements
}

/// Extract the heading level from a line of markdown text.
///
/// Returns `Some(level)` if the line is a valid heading (1-6 #'s followed by whitespace),
/// or `None` otherwise.
///
/// # Examples
///
/// ```
/// # use treemd::parser::utils::get_heading_level;
/// assert_eq!(get_heading_level("# Title"), Some(1));
/// assert_eq!(get_heading_level("## Section"), Some(2));
/// assert_eq!(get_heading_level("not a heading"), None);
/// assert_eq!(get_heading_level("#NoSpace"), None);
/// ```
pub fn get_heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let mut level = 0;

    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else if ch.is_whitespace() {
            return if level > 0 && level <= 6 {
                Some(level)
            } else {
                None
            };
        } else {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_inline() {
        assert_eq!(strip_markdown_inline("**bold**"), "bold");
        assert_eq!(strip_markdown_inline("__bold__"), "bold");
        assert_eq!(strip_markdown_inline("*italic*"), "italic");
        assert_eq!(strip_markdown_inline("_italic_"), "italic");
        assert_eq!(strip_markdown_inline("`code`"), "code");
        assert_eq!(strip_markdown_inline("~~strikethrough~~"), "strikethrough");
        assert_eq!(
            strip_markdown_inline("**bold** and *italic*"),
            "bold and italic"
        );
        assert_eq!(strip_markdown_inline("**_bold italic_**"), "bold italic");
        assert_eq!(
            strip_markdown_inline("*`code in italic`*"),
            "code in italic"
        );
        assert_eq!(
            strip_markdown_inline("**turbocli-parser** (850 LOC)"),
            "turbocli-parser (850 LOC)"
        );
        assert_eq!(strip_markdown_inline("snake_case_var"), "snake_case_var");
        assert_eq!(strip_markdown_inline("plain text"), "plain text");
    }

    #[test]
    fn test_get_heading_level() {
        assert_eq!(get_heading_level("# Title"), Some(1));
        assert_eq!(get_heading_level("## Section"), Some(2));
        assert_eq!(get_heading_level("### Subsection"), Some(3));
        assert_eq!(get_heading_level("#### Level 4"), Some(4));
        assert_eq!(get_heading_level("##### Level 5"), Some(5));
        assert_eq!(get_heading_level("###### Level 6"), Some(6));

        // Invalid cases
        assert_eq!(get_heading_level("not a heading"), None);
        assert_eq!(get_heading_level("#NoSpace"), None);
        assert_eq!(get_heading_level("####### Too many"), None);
        assert_eq!(get_heading_level("  ## Indented"), Some(2)); // Trimmed
    }

    #[test]
    fn test_parse_inline_html_strong() {
        let elements = parse_inline_html("<strong>Navigation</strong>");
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], InlineElement::Strong { value } if value == "Navigation"));
    }

    #[test]
    fn test_parse_inline_html_bold() {
        let elements = parse_inline_html("<b>Bold text</b>");
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], InlineElement::Strong { value } if value == "Bold text"));
    }

    #[test]
    fn test_parse_inline_html_emphasis() {
        let elements = parse_inline_html("<em>Italic</em>");
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], InlineElement::Emphasis { value } if value == "Italic"));
    }

    #[test]
    fn test_parse_inline_html_code() {
        let elements = parse_inline_html("<code>fn main()</code>");
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], InlineElement::Code { value } if value == "fn main()"));
    }

    #[test]
    fn test_parse_inline_html_mixed() {
        let elements = parse_inline_html("Before <strong>bold</strong> after");
        assert_eq!(elements.len(), 3);
        assert!(matches!(&elements[0], InlineElement::Text { value } if value == "Before "));
        assert!(matches!(&elements[1], InlineElement::Strong { value } if value == "bold"));
        assert!(matches!(&elements[2], InlineElement::Text { value } if value == " after"));
    }

    #[test]
    fn test_parse_inline_html_plain_text() {
        let elements = parse_inline_html("No HTML here");
        assert_eq!(elements.len(), 1);
        assert!(matches!(&elements[0], InlineElement::Text { value } if value == "No HTML here"));
    }

    #[test]
    fn test_parse_inline_html_nested_tags_no_duplication() {
        // Outer wins; inner tag is dropped so its content isn't emitted twice.
        let elements = parse_inline_html("<b><em>nested</em></b>");
        assert_eq!(
            elements.len(),
            1,
            "nested tags should yield a single element, not two"
        );
        assert!(
            matches!(&elements[0], InlineElement::Strong { value } if value == "<em>nested</em>")
        );
    }

    #[test]
    fn test_parse_inline_html_overlap_keeps_outer() {
        let elements = parse_inline_html("Pre <strong>outer <em>inner</em> tail</strong> post");
        // Should produce: text, strong, text — without a duplicate Emphasis arm.
        assert_eq!(elements.len(), 3);
        assert!(matches!(&elements[0], InlineElement::Text { value } if value == "Pre "));
        assert!(matches!(&elements[1], InlineElement::Strong { value } if value.contains("inner")));
        assert!(matches!(&elements[2], InlineElement::Text { value } if value == " post"));
    }
}
