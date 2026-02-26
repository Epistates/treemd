//! Utility functions for UI rendering
//!
//! Pure functions for layout calculations, text parsing, and formatting.

use crate::parser::output::Alignment;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
        // Use single ellipsis character (…) which is 1 display width
        // Much more space-efficient than "..." (3 chars)
        if width > 3 {
            // Truncate with ellipsis: " text… " or "text…" depending on space
            let available = width.saturating_sub(2); // 1 for padding, 1 for ellipsis
            let mut truncated = String::new();
            let mut current_width = 0;
            for c in text.chars() {
                let char_width = c.width().unwrap_or(1);
                if current_width + char_width > available {
                    break;
                }
                truncated.push(c);
                current_width += char_width;
            }
            // Pad to fill remaining space
            let remaining = width.saturating_sub(current_width + 2); // +2 for " " and "…"
            return format!(" {}…{}", truncated, " ".repeat(remaining));
        }
        // Very narrow: just show what fits
        let mut truncated = String::new();
        let mut current_width = 0;
        for c in text.chars() {
            let char_width = c.width().unwrap_or(1);
            if current_width + char_width > width {
                break;
            }
            truncated.push(c);
            current_width += char_width;
        }
        return truncated;
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

/// Strip LaTeX math expressions and commands from content.
///
/// Removes LaTeX commands and math delimiters while preserving the text content.
/// For common math symbols and Greek letters, it uses Unicode approximations
/// to maintain readability (SOTA approach).
///
/// # Arguments
/// * `content` - The document content
///
/// # Returns
/// Content with LaTeX expressions removed or converted to Unicode
pub fn strip_latex(content: &str) -> String {
    use regex::{Captures, Regex};

    // 1. Convert common LaTeX symbols to Unicode approximations (SOTA)
    let mut result = content.to_string();

    // Greek letters and common math symbols
    let symbols = [
        (r"\\alpha", "α"), (r"\\beta", "β"), (r"\\gamma", "γ"), (r"\\delta", "δ"),
        (r"\\epsilon", "ε"), (r"\\zeta", "ζ"), (r"\\eta", "η"), (r"\\theta", "θ"),
        (r"\\iota", "ι"), (r"\\kappa", "κ"), (r"\\lambda", "λ"), (r"\\mu", "μ"),
        (r"\\nu", "ν"), (r"\\xi", "ξ"), (r"\\pi", "π"), (r"\\rho", "ρ"),
        (r"\\sigma", "σ"), (r"\\tau", "τ"), (r"\\upsilon", "υ"), (r"\\phi", "φ"),
        (r"\\chi", "χ"), (r"\\psi", "ψ"), (r"\\omega", "ω"),
        (r"\\Gamma", "Γ"), (r"\\Delta", "Δ"), (r"\\Theta", "Θ"), (r"\\Lambda", "Λ"),
        (r"\\Xi", "Ξ"), (r"\\Pi", "Π"), (r"\\Sigma", "Σ"), (r"\\Phi", "Φ"),
        (r"\\Psi", "Ψ"), (r"\\Omega", "Ω"),
        (r"\\sum", "∑"), (r"\\prod", "∏"), (r"\\int", "∫"), (r"\\infty", "∞"),
        (r"\\approx", "≈"), (r"\\neq", "≠"), (r"\\le", "≤"), (r"\\ge", "≥"),
        (r"\\pm", "±"), (r"\\times", "×"), (r"\\div", "÷"), (r"\\partial", "∂"),
        (r"\\nabla", "∇"), (r"\\forall", "∀"), (r"\\exists", "∃"), (r"\\in", "∈"),
        (r"\\notin", "∉"), (r"\\subset", "⊂"), (r"\\supset", "⊃"), (r"\\cup", "∪"),
        (r"\\cap", "∩"), (r"\\Rightarrow", "⇒"), (r"\\rightarrow", "→"),
        (r"\\Leftarrow", "⇐"), (r"\\leftarrow", "←"), (r"\\Leftrightarrow", "⇔"),
        (r"\\leftrightarrow", "↔"), (r"\\cdot", "·"), (r"\\dots", "…"),
    ];

    for (pattern, replacement) in symbols {
        // Match the command followed by a non-letter or end of string
        // Use a capture group for the trailing character so we can preserve it
        let re = Regex::new(&format!(r"{}([^a-zA-Z]|$)", pattern)).unwrap();
        let replacement_with_group = format!("{}$1", replacement);
        result = re.replace_all(&result, replacement_with_group.as_str()).to_string();
    }

    // Superscripts and subscripts (common ones)
    let sup_map = [
        ('0', '⁰'), ('1', '¹'), ('2', '²'), ('3', '³'), ('4', '⁴'),
        ('5', '⁵'), ('6', '⁶'), ('7', '⁷'), ('8', '⁸'), ('9', '⁹'),
        ('+', '⁺'), ('-', '⁻'), ('=', '⁼'), ('(', '⁽'), (')', '⁾'),
        ('n', 'ⁿ'), ('i', 'ⁱ'), ('x', 'ˣ'),
    ];
    let sub_map = [
        ('0', '₀'), ('1', '₁'), ('2', '₂'), ('3', '₃'), ('4', '₄'),
        ('5', '₅'), ('6', '₆'), ('7', '₇'), ('8', '₈'), ('9', '₉'),
        ('+', '₊'), ('-', '₋'), ('=', '₌'), ('(', '₍'), (')', '₎'),
        ('a', 'ₐ'), ('e', 'ₑ'), ('h', 'ₕ'), ('i', 'ᵢ'), ('j', 'ⱼ'),
        ('k', 'ₖ'), ('l', 'ₗ'), ('m', 'ₘ'), ('n', 'ₙ'), ('o', 'ₒ'),
        ('p', 'ₚ'), ('r', 'ᵣ'), ('s', 'ₛ'), ('t', 'ₜ'), ('u', 'ᵤ'),
        ('v', 'ᵥ'), ('x', 'ₓ'),
    ];

    // Replace ^x with superscript if x is in map
    let superscript = Regex::new(r"\^\{?([0-9+\-=()nix])\}?").unwrap();
    result = superscript.replace_all(&result, |caps: &Captures| {
        let val = caps[1].chars().next().unwrap();
        sup_map.iter().find(|(k, _)| *k == val).map(|(_, v)| v.to_string()).unwrap_or_else(|| caps[0].to_string())
    }).to_string();

    // Replace _x with subscript if x is in map
    let subscript = Regex::new(r"_\{?([0-9+\-=()aehijklmnoprstuvx])\}?").unwrap();
    result = subscript.replace_all(&result, |caps: &Captures| {
        let val = caps[1].chars().next().unwrap();
        sub_map.iter().find(|(k, _)| *k == val).map(|(_, v)| v.to_string()).unwrap_or_else(|| caps[0].to_string())
    }).to_string();

    // 2. Strip remaining delimiters and structural commands
    // Match display math ($$...$$) first - must be removed before inline math
    let display_math = Regex::new(r"\$\$[\s\S]*?\$\$").unwrap();
    result = display_math.replace_all(&result, "").to_string();

    // Match inline math ($...$)
    let inline_math = Regex::new(r"\$([^\$\n]+)\$").unwrap();
    result = inline_math.replace_all(&result, "$1").to_string();

    // Match \( ... \) and \[ ... \]
    let paren_math = Regex::new(r"\\\(([\s\S]*?)\\\)").unwrap();
    result = paren_math.replace_all(&result, "$1").to_string();
    let bracket_math = Regex::new(r"\\\[([\s\S]*?)\\\]").unwrap();
    result = bracket_math.replace_all(&result, "$1").to_string();

    // Match \begin{...}...\end{...} environments
    // For some environments (like equation/align), we keep the content but strip the tags
    let latex_env = Regex::new(r"(?s)^\s*\\begin\{[^}]+\}\s*(.*?)\s*\\end\{[^}]+\}\s*$").unwrap();
    result = latex_env.replace_all(&result, "$1").to_string();
    // Also handle inline environments
    let latex_env_inline = Regex::new(r"(?s)\\begin\{[^}]+\}(.*?)\\end\{[^}]+\}").unwrap();
    result = latex_env_inline.replace_all(&result, "$1").to_string();

    // Match font size commands (standalone, no args)
    let font_size_cmd = Regex::new(
        r"(?m)^\s*\\(tiny|scriptsize|footnotesize|small|normalsize|large|Large|LARGE|huge|Huge|HUGE|ssmall|miniscule)\s*$"
    ).unwrap();
    result = font_size_cmd.replace_all(&result, "").to_string();

    // Match standalone LaTeX commands on their own line
    let standalone_cmd = Regex::new(
        r"(?m)^\s*\\(newpage|clearpage|pagebreak|tableofcontents|maketitle|listoffigures|listoftables|appendix|frontmatter|mainmatter|backmatter|centering|raggedright|raggedleft|noindent|indent|par|bigskip|medskip|smallskip|vfill|hfill|newline|linebreak)\s*$"
    ).unwrap();
    result = standalone_cmd.replace_all(&result, "").to_string();

    // Match LaTeX commands with braces on their own line (preamble/setup)
    let cmd_with_args_line = Regex::new(
        r"(?m)^\s*\\(usepackage|documentclass|title|author|date|include|input|bibliography|bibliographystyle|setlength|renewcommand|newcommand|setcounter|addtocounter|pagenumbering|pagestyle|thispagestyle|geometry|hypersetup|definecolor|graphicspath|addbibresource|fontsize|sethlcolor|titlespacing|titleformat|captionsetup|lstset)(\[[^\]]*\])?(\{[^}]*\})+\s*$"
    ).unwrap();
    result = cmd_with_args_line.replace_all(&result, "").to_string();

    // Match inline commands with args that should be stripped entirely
    let cmd_with_args_inline = Regex::new(
        r"\\(label|ref|cite|eqref|pageref|vspace|hspace|phantom|hphantom|vphantom)\{[^}]*\}",
    )
    .unwrap();
    result = cmd_with_args_inline.replace_all(&result, "").to_string();

    // Match other common inline LaTeX commands and replace with content
    let text_formatting =
        Regex::new(r"\\(textbf|textit|emph|underline|texttt|hl|textsf|textsc|textsl)\{([^}]*)\}")
            .unwrap();
    result = text_formatting.replace_all(&result, "$2").to_string();

    // Match \textcolor{color}{text} and \colorbox{color}{text}
    let textcolor = Regex::new(r"\\textcolor\{[^}]*\}\{([^}]*)\}").unwrap();
    result = textcolor.replace_all(&result, "$1").to_string();
    let colorbox = Regex::new(r"\\colorbox\{[^}]*\}\{([^}]*)\}").unwrap();
    result = colorbox.replace_all(&result, "$1").to_string();

    result
}

/// Strip ALL lines starting with backslash (aggressive LaTeX filtering).
///
/// This is a simple catch-all for users whose documents have LaTeX commands
/// not covered by the standard filtering.
///
/// # Arguments
/// * `content` - The document content
///
/// # Returns
/// Content with all backslash-starting lines removed
pub fn strip_latex_aggressive(content: &str) -> String {
    use regex::Regex;

    // Match any line that starts with optional whitespace followed by backslash and letters
    let backslash_line = Regex::new(r"(?m)^\s*\\[a-zA-Z].*$").unwrap();
    backslash_line.replace_all(content, "").to_string()
}

/// Wrap text to a specific width, preserving word boundaries when possible.
///
/// Uses unicode-width for accurate terminal display measurement.
///
/// # Arguments
/// * `text` - The text to wrap
/// * `width` - The maximum width per line
///
/// # Returns
/// A vector of lines
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = word.width();

        // If word itself is wider than limit, we must break it
        if word_width > width {
            // Push whatever we have so far
            if !current_line.is_empty() {
                lines.push(std::mem::take(&mut current_line));
                current_width = 0;
            }

            let mut remaining = word;
            while !remaining.is_empty() {
                let mut chunk = String::new();
                let mut chunk_width = 0;
                for c in remaining.chars() {
                    let c_width = c.width().unwrap_or(1);
                    if chunk_width + c_width > width {
                        break;
                    }
                    chunk.push(c);
                    chunk_width += c_width;
                }
                if chunk.is_empty() { break; }
                let chunk_len = chunk.len();
                lines.push(chunk);
                remaining = &remaining[chunk_len..];
            }
            continue;
        }

        // Check if word fits on current line (plus a space)
        let space_needed = if current_line.is_empty() { 0 } else { 1 };
        if current_width + space_needed + word_width <= width {
            if space_needed > 0 {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        } else {
            // Doesn't fit, start new line
            lines.push(std::mem::take(&mut current_line));
            current_line = word.to_string();
            current_width = word_width;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(if text.trim().is_empty() && !text.is_empty() {
            text.to_string()
        } else {
            String::new()
        });
    }

    lines
}

/// Apply content filters based on configuration.
///
/// Strips frontmatter and/or LaTeX based on the provided flags.
///
/// # Arguments
/// * `content` - The document content
/// * `hide_frontmatter` - Whether to strip YAML frontmatter
/// * `hide_latex` - Whether to strip LaTeX expressions
/// * `latex_aggressive` - Whether to use aggressive filtering (strip all backslash lines)
///
/// # Returns
/// Filtered content
pub fn filter_content(
    content: &str,
    hide_frontmatter: bool,
    hide_latex: bool,
    latex_aggressive: bool,
) -> String {
    let mut result = content.to_string();

    if hide_frontmatter {
        result = strip_frontmatter(&result);
    }

    if hide_latex {
        result = strip_latex(&result);

        // Apply aggressive filtering if enabled (catches anything standard missed)
        if latex_aggressive {
            result = strip_latex_aggressive(&result);
        }
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
            assert_eq!(result, "The formula x² is quadratic");
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
            assert_eq!(result, "An equation:\n\ny = mx + b\n\ndone.");
        }

        #[test]
        fn test_greek_letters() {
            let content = "Angle $\\alpha$ and $\\beta$";
            let result = strip_latex(content);
            assert_eq!(result, "Angle α and β");
        }

        #[test]
        fn test_math_symbols() {
            let content = "$\\sum_{i=1}^n x_i \\approx \\int f(x) dx$";
            let result = strip_latex(content);
            // i=1 and n might not be mapped in sup/sub maps fully yet but let's check what we have
            // _i -> ᵢ, ^n -> ⁿ
            assert!(result.contains("∑"));
            assert!(result.contains("≈"));
            assert!(result.contains("∫"));
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

        #[test]
        fn test_standalone_commands() {
            let content = "Some text\n\\newpage\nMore text";
            let result = strip_latex(content);
            assert_eq!(result, "Some text\n\nMore text");
        }

        #[test]
        fn test_clearpage() {
            let content = "Chapter 1\n\\clearpage\nChapter 2";
            let result = strip_latex(content);
            assert_eq!(result, "Chapter 1\n\nChapter 2");
        }

        #[test]
        fn test_usepackage() {
            let content = "\\usepackage{amsmath}\nSome content";
            let result = strip_latex(content);
            assert_eq!(result, "\nSome content");
        }

        #[test]
        fn test_text_formatting_preserved() {
            // \textbf{} content should be preserved, just without the command
            let content = "This is \\textbf{bold} text";
            let result = strip_latex(content);
            assert_eq!(result, "This is bold text");
        }

        #[test]
        fn test_font_size_normalsize() {
            let content = "Some text\n\\normalsize\nMore text";
            let result = strip_latex(content);
            assert_eq!(result, "Some text\n\nMore text");
        }

        #[test]
        fn test_font_size_large() {
            let content = "\\Large\nHeading";
            let result = strip_latex(content);
            assert_eq!(result, "\nHeading");
        }

        #[test]
        fn test_font_size_tiny() {
            let content = "Normal\n\\tiny\nSmall text\n\\normalsize\nBack to normal";
            let result = strip_latex(content);
            assert_eq!(result, "Normal\n\nSmall text\n\nBack to normal");
        }

        #[test]
        fn test_centering() {
            let content = "\\centering\nCentered content";
            let result = strip_latex(content);
            assert_eq!(result, "\nCentered content");
        }

        #[test]
        fn test_label_ref_stripped() {
            let content = "See Figure \\ref{fig:example} for details.";
            let result = strip_latex(content);
            assert_eq!(result, "See Figure  for details.");
        }

        #[test]
        fn test_cite_stripped() {
            let content = "As shown by \\cite{smith2020} in their work.";
            let result = strip_latex(content);
            assert_eq!(result, "As shown by  in their work.");
        }

        #[test]
        fn test_vspace_hspace_stripped() {
            let content = "Text\\vspace{1em}More text\\hspace{2cm}End";
            let result = strip_latex(content);
            assert_eq!(result, "TextMore textEnd");
        }

        #[test]
        fn test_geometry_stripped() {
            let content = "\\geometry{margin=1in}\nDocument content";
            let result = strip_latex(content);
            assert_eq!(result, "\nDocument content");
        }
    }

    mod strip_latex_aggressive_tests {
        use super::*;

        #[test]
        fn test_aggressive_strips_any_backslash_line() {
            let content = "Normal text\n\\unknowncommand\nMore text";
            let result = strip_latex_aggressive(content);
            assert_eq!(result, "Normal text\n\nMore text");
        }

        #[test]
        fn test_aggressive_strips_with_args() {
            let content = "\\customcmd{arg}\nContent here";
            let result = strip_latex_aggressive(content);
            assert_eq!(result, "\nContent here");
        }

        #[test]
        fn test_aggressive_preserves_prose() {
            let content = "Regular text without backslash commands";
            let result = strip_latex_aggressive(content);
            assert_eq!(result, content);
        }

        #[test]
        fn test_aggressive_preserves_inline_backslash() {
            // Text with backslash not at line start should be preserved
            let content = "Some text with \\command inline";
            let result = strip_latex_aggressive(content);
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
            // Now uses single ellipsis character (…) instead of three dots
            assert!(result.contains("…"));
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
