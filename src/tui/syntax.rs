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

const DEFAULT_CODE_THEME: &str = "base16-ocean.dark";

/// Soft cap on cached entries before the cache resets. Each entry is a small
/// `Vec<Line>` so 256 covers virtually any document while bounding memory.
const CACHE_LIMIT: usize = 256;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
    /// Cached highlight results keyed by `hash((content, language))`.
    /// `RefCell` because highlight_code takes `&self` and is called from render.
    cache: RefCell<HashMap<u64, Vec<Line<'static>>>>,
}

impl SyntaxHighlighter {
    pub fn new(theme: &str, theme_dir: Option<PathBuf>) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
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

        let theme = theme_set
            .themes
            .get(theme)
            .or_else(|| {
                if theme != DEFAULT_CODE_THEME {
                    eprintln!(
                        "warning: code theme '{}' not found, using '{}'",
                        theme, DEFAULT_CODE_THEME
                    );
                }
                theme_set.themes.get(DEFAULT_CODE_THEME)
            })
            .cloned()
            .expect("syntect default themes must contain base16-ocean.dark");

        Self {
            syntax_set,
            theme,
            cache: RefCell::new(HashMap::new()),
        }
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
