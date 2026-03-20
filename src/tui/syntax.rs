use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlighter {
    pub fn new(theme: &str, theme_dir: Option<PathBuf>) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let mut theme_set = ThemeSet::load_defaults();
        if let Some(dir) = theme_dir {
            // Load the user themes if any (silently ignore errors)
            let _ = theme_set.add_from_folder(dir);
        }

        let theme = theme_set.themes.get(theme).cloned().unwrap_or_else(|| {
            // Fallback to the first theme (we know there is one since `load_defaults was used`)
            theme_set.themes.first_entry().unwrap().get().clone()
        });

        Self { syntax_set, theme }
    }

    pub fn highlight_code(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        // Replace tabs with spaces to avoid terminal rendering artifacts during scrolling
        // Tabs can cause inconsistent display widths across different terminals
        let code = code.replace('\t', "    ");

        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();

        for line in LinesWithEndings::from(&code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<Span> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    let color = Color::Rgb(fg.r, fg.g, fg.b);
                    let mut ratatui_style = Style::default().fg(color);

                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::BOLD)
                    {
                        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                    }
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::ITALIC)
                    {
                        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                    }
                    if style
                        .font_style
                        .contains(syntect::highlighting::FontStyle::UNDERLINE)
                    {
                        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                    }

                    Span::styled(text.to_string(), ratatui_style)
                })
                .collect();

            lines.push(Line::from(spans));
        }

        lines
    }

    pub fn detect_language(info_string: &str) -> String {
        // Extract language from info string (e.g., "rust" from "```rust")
        info_string
            .split_whitespace()
            .next()
            .unwrap_or("text")
            .to_lowercase()
    }
}
