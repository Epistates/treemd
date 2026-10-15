use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::tui::terminal_compat::ColorMode;
use crate::tui::theme::rgb_to_256;

const DEFAULT_CODE_THEME: &str = "base16-ocean.dark";

/// What to paint behind a code block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodeBlockBackground {
    /// The code theme's own background, so the block agrees with its token
    /// colors. This is what an unset `code_block_bg` resolves to.
    #[default]
    FromTheme,
    /// An explicit color from config.
    Color(Color),
    /// Paint nothing, leaving code on the terminal background.
    Off,
}

/// Soft cap on cached entries before the cache resets. Each entry is a small
/// `Vec<Line>` so 256 covers virtually any document while bounding memory.
const CACHE_LIMIT: usize = 256;

/// Longest code line, in bytes, that is syntax highlighted.
const MAX_HIGHLIGHTED_LINE: usize = 4096;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
    /// Background for the code block area, or `None` to leave it unpainted.
    /// Config override if one was given, otherwise the code theme's own
    /// background so it agrees with the token colors.
    code_block_bg: Option<Color>,
    /// What the config asked for behind code blocks, kept so a new code
    /// theme can work its background out again.
    background: CodeBlockBackground,
    /// The directory of the user's own `.tmTheme` files.
    theme_dir: Option<PathBuf>,
    /// What the terminal can display. Syntect always reports 24-bit color, so
    /// every color leaving this type is mapped through it.
    color_mode: ColorMode,
    /// Cached highlight results keyed by `hash((content, language))`.
    /// `RefCell` because highlight_code takes `&self` and is called from render.
    cache: RefCell<HashMap<u64, Vec<Line<'static>>>>,
}

/// Map a syntect color to something the terminal can actually show.
///
/// Syntect themes are always 24-bit, so on a 256-color terminal every token
/// color and the block background have to be quantized the same way
/// `Theme::for_terminal` quantizes the rest of the UI.
fn adapt(color: Color, mode: ColorMode) -> Color {
    match mode {
        ColorMode::Rgb => color,
        ColorMode::Indexed256 => rgb_to_256(color),
    }
}

/// The code theme called `name`, from the built-in set or `theme_dir`,
/// falling back to the default with a warning.
fn load_theme(name: &str, theme_dir: Option<&PathBuf>) -> Theme {
    let mut theme_set = ThemeSet::load_defaults();
    if let Some(dir) = theme_dir
        && let Ok(paths) = ThemeSet::discover_theme_paths(dir)
    {
        for path in paths {
            match ThemeSet::get_theme(&path) {
                Ok(theme) => {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        theme_set.themes.insert(name.to_owned(), theme);
                    }
                }
                Err(e) => {
                    eprintln!("warning: skipping theme {}: {}", path.display(), e);
                }
            }
        }
    }

    theme_set
        .themes
        .get(name)
        .or_else(|| {
            if name != DEFAULT_CODE_THEME {
                eprintln!(
                    "warning: code theme '{}' not found, using '{}'",
                    name, DEFAULT_CODE_THEME
                );
            }
            theme_set.themes.get(DEFAULT_CODE_THEME)
        })
        .cloned()
        .expect("syntect default themes must contain base16-ocean.dark")
}

impl SyntaxHighlighter {
    pub fn new(
        theme: &str,
        theme_dir: Option<PathBuf>,
        background: CodeBlockBackground,
        color_mode: ColorMode,
    ) -> Self {
        let mut highlighter = Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme: Theme::default(),
            code_block_bg: None,
            background,
            theme_dir,
            color_mode,
            cache: RefCell::new(HashMap::new()),
        };
        highlighter.set_theme(theme);
        highlighter
    }

    /// Highlight with the code theme called `name` from now on. The syntax
    /// definitions, the slow part to load, are kept.
    pub fn set_theme(&mut self, name: &str) {
        self.theme = load_theme(name, self.theme_dir.as_ref());
        self.code_block_bg = match self.background {
            CodeBlockBackground::Off => None,
            CodeBlockBackground::Color(c) => Some(c),
            CodeBlockBackground::FromTheme => self
                .theme
                .settings
                .background
                .map(|c| Color::Rgb(c.r, c.g, c.b)),
        }
        .map(|c| adapt(c, self.color_mode));
        self.cache.borrow_mut().clear();
    }

    /// Background to paint behind a code block, if any.
    pub fn code_block_bg(&self) -> Option<Color> {
        self.code_block_bg
    }

    /// Highlight `code` as `language`. Result is memoized — repeat calls with
    /// the same `(code, language)` pair return cloned cached lines without
    /// re-invoking syntect.
    pub fn highlight_code(&self, code: &str, language: &str) -> Vec<Line<'static>> {
        let key = cache_key(code, language);

        if let Some(cached) = self.cache.borrow().get(&key) {
            return cached.clone();
        }

        // Replace tabs with spaces once at cache-miss time, not every render.
        let code_owned = code.replace('\t', "    ");

        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();

        for line in LinesWithEndings::from(&code_owned) {
            // A very long line, minified code or a pasted blob, is drawn
            // plain: highlighting runs regexes over the whole line, and their
            // cost on a single line of hundreds of kilobytes stalled the
            // frame. The lines after it may be colored a little off, since
            // the highlighter carries its state from line to line.
            if line.len() > MAX_HIGHLIGHTED_LINE {
                lines.push(Line::from(line.trim_end_matches(['\n', '\r']).to_string()));
                continue;
            }
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<Span> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    let color = adapt(Color::Rgb(fg.r, fg.g, fg.b), self.color_mode);
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

                    // Syntect needs the line ending to parse correctly, but a
                    // Span is one row and must not carry it: unicode-width
                    // counts '\n' as a cell, so a trailing newline makes every
                    // code line measure one wider than it draws.
                    Span::styled(
                        text.trim_end_matches(['\n', '\r']).to_string(),
                        ratatui_style,
                    )
                })
                .collect();

            lines.push(Line::from(spans));
        }

        // Bounded cache: clear when full. Simpler than LRU and adequate here
        // because highlighting is the cold path; cache hits dominate.
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, lines.clone());
        lines
    }
}

fn cache_key(code: &str, language: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    code.hash(&mut hasher);
    language.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighter() -> SyntaxHighlighter {
        SyntaxHighlighter::new(
            "base16-ocean.dark",
            None,
            CodeBlockBackground::Off,
            ColorMode::Rgb,
        )
    }

    #[test]
    fn a_very_long_line_is_drawn_plain_and_whole() {
        let long = "x".repeat(MAX_HIGHLIGHTED_LINE * 4);
        let code = format!("let a = 1;\n{long}\nlet b = 2;");
        let lines = highlighter().highlight_code(&code, "rust");

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1].spans.len(), 1);
        assert_eq!(lines[1].spans[0].content, long);
        // The ordinary lines around it are still highlighted.
        assert!(lines[0].spans.len() > 1);
    }
}
