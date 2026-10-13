use crate::keybindings::{Keybindings, KeybindingsConfig};
use crate::tui::theme::ThemeName;
use opensesame::EditorConfig;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub path: Option<PathBuf>,

    /// Problems found loading the file, such as a key or action name that
    /// does not exist. The TUI shows them, since anything printed before it
    /// starts is gone behind the alternate screen.
    #[serde(skip)]
    pub warnings: Vec<String>,

    /// The settings as loaded, which [`save`](Self::save) compares against to
    /// write only what changed.
    #[serde(skip)]
    baseline: Option<toml::Table>,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub terminal: TerminalConfig,

    #[serde(default)]
    pub theme: CustomThemeConfig,

    #[serde(default)]
    pub keybindings: KeybindingsConfig,

    /// Editor configuration for external file editing
    #[serde(default)]
    pub editor: EditorConfig,

    /// Image display configuration
    #[serde(default)]
    pub images: ImageConfig,

    /// Content filtering options
    #[serde(default)]
    pub content: ContentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_code_theme")]
    pub code_theme: String,

    #[serde(default = "default_outline_width")]
    pub outline_width: u16,

    /// Tree rendering style: "compact" (default, gapless) or "spaced"
    #[serde(default = "default_tree_style")]
    pub tree_style: String,

    /// Show heading level markers (e.g. ##, ###) in the outline sidebar (default: true)
    #[serde(default = "default_outline_heading_markers")]
    pub outline_heading_markers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    #[serde(default = "default_color_mode")]
    pub color_mode: String,

    #[serde(default)]
    pub warned_terminal_app: bool,
}

/// Image display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Whether to render images in the TUI (default: true)
    /// When disabled, images are skipped entirely
    #[serde(default = "default_images_enabled")]
    pub enabled: bool,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            enabled: default_images_enabled(),
        }
    }
}

fn default_images_enabled() -> bool {
    true
}

/// Content filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentConfig {
    /// Hide YAML frontmatter (---\n...\n---) at document start (default: true)
    #[serde(default = "default_hide_frontmatter")]
    pub hide_frontmatter: bool,

    /// Hide LaTeX math expressions ($...$, $$...$$, \begin{...}) (default: true)
    #[serde(default = "default_hide_latex")]
    pub hide_latex: bool,

    /// Aggressive LaTeX filtering: strip ALL lines starting with backslash (default: false)
    /// Enable this if standard filtering misses some LaTeX commands
    #[serde(default = "default_latex_aggressive")]
    pub latex_aggressive: bool,

    /// How to mark the edges of a fenced code block (default: full)
    #[serde(default)]
    pub code_fences: CodeFences,
}

/// How fenced code blocks are delimited in the rendered content pane.
///
/// The fence rows are decoration, not content, so they can be dropped
/// entirely. They are removed rather than hidden, so they take no vertical
/// space either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodeFences {
    /// Both fence rows, as written in the source: ```lang and ```
    #[default]
    Full,
    /// A dim language label above the block, and no closing row.
    ///
    /// Keeps the language visible without boxing the block in. The label sits
    /// on its own row rather than on the first line of code, so selecting the
    /// block with the mouse does not pick it up.
    Label,
    /// No fence rows at all.
    None,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            hide_frontmatter: default_hide_frontmatter(),
            hide_latex: default_hide_latex(),
            latex_aggressive: default_latex_aggressive(),
            code_fences: CodeFences::default(),
        }
    }
}

fn default_hide_frontmatter() -> bool {
    true
}

fn default_hide_latex() -> bool {
    true
}

fn default_latex_aggressive() -> bool {
    true
}

/// Custom theme color overrides
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CustomThemeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_1: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_2: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_3: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_4: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_5: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_focused: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_unfocused: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_bar_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_bar_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_code_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_code_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_bullet: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockquote_border: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockquote_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_fence: Option<ColorValue>,
    /// Background for the whole code block area.
    ///
    /// Unlike the other keys here this one is not a field on `Theme`: it
    /// belongs to the *code* theme, so when unset it falls back to the syntect
    /// theme's own background and stays consistent with the token colors.
    /// Resolved in `SyntaxHighlighter`, not `Theme::with_custom_colors`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_block_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_bar_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrollbar_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_indicator_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_indicator_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_selected_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_selected_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_border: Option<ColorValue>,
    // Search highlighting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_match_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_match_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_current_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_current_fg: Option<ColorValue>,
    // Footer keybinding hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_key_bg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_key_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_desc_fg: Option<ColorValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_bg: Option<ColorValue>,
}

/// Color value that can be specified in multiple formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorValue {
    /// Named color (e.g., "Red", "Cyan", "White")
    Named(String),
    /// RGB color { rgb = [r, g, b] }
    Rgb { rgb: [u8; 3] },
    /// Indexed color { indexed = 235 }
    Indexed { indexed: u8 },
}

impl ColorValue {
    /// Convert to ratatui Color
    pub fn to_color(&self) -> Option<Color> {
        match self {
            ColorValue::Named(name) => match name.to_lowercase().as_str() {
                "black" => Some(Color::Black),
                "red" => Some(Color::Red),
                "green" => Some(Color::Green),
                "yellow" => Some(Color::Yellow),
                "blue" => Some(Color::Blue),
                "magenta" => Some(Color::Magenta),
                "cyan" => Some(Color::Cyan),
                "gray" | "grey" => Some(Color::Gray),
                "darkgray" | "darkgrey" => Some(Color::DarkGray),
                "lightred" => Some(Color::LightRed),
                "lightgreen" => Some(Color::LightGreen),
                "lightyellow" => Some(Color::LightYellow),
                "lightblue" => Some(Color::LightBlue),
                "lightmagenta" => Some(Color::LightMagenta),
                "lightcyan" => Some(Color::LightCyan),
                "white" => Some(Color::White),
                _ => None,
            },
            ColorValue::Rgb { rgb } => Some(Color::Rgb(rgb[0], rgb[1], rgb[2])),
            ColorValue::Indexed { indexed } => Some(Color::Indexed(*indexed)),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            code_theme: default_code_theme(),
            outline_width: default_outline_width(),
            tree_style: default_tree_style(),
            outline_heading_markers: default_outline_heading_markers(),
        }
    }
}

fn default_tree_style() -> String {
    "compact".to_string()
}

fn default_outline_heading_markers() -> bool {
    true
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            color_mode: default_color_mode(),
            warned_terminal_app: false,
        }
    }
}

fn default_theme() -> String {
    "OceanDark".to_string()
}

fn default_code_theme() -> String {
    "base16-ocean.dark".to_string()
}

fn default_outline_width() -> u16 {
    30
}

fn default_color_mode() -> String {
    "auto".to_string()
}

impl Config {
    /// Get the XDG-style config file path (~/.config/treemd/config.toml)
    /// This is preferred on macOS for CLI tools and cross-platform dotfiles
    #[cfg(target_os = "macos")]
    /// `$XDG_CONFIG_HOME/treemd/config.toml` when that is set to an absolute
    /// path, as the XDG spec has it, else `~/.config/treemd/config.toml`.
    fn xdg_config_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| dirs::home_dir().map(|home| home.join(".config")))?;
        Some(base.join("treemd").join("config.toml"))
    }

    /// Get the platform-specific config file path
    /// - macOS: ~/Library/Application Support/treemd/config.toml
    /// - Linux: ~/.config/treemd/config.toml
    /// - Windows: %APPDATA%/treemd/config.toml
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("treemd").join("config.toml"))
    }

    /// Resolve the config file path
    /// On macOS, checks ~/.config/treemd first, then falls back to ~/Library/Application Support
    fn resolve_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            if let Some(xdg_path) = Self::xdg_config_path()
                && xdg_path.exists()
            {
                return Some(xdg_path);
            }
            Self::config_path()
        }

        #[cfg(not(target_os = "macos"))]
        Self::config_path()
    }

    /// Load the configuration file, falling back to `Default` on error.
    ///
    /// The path is kept, so a later [`save`](Self::save) writes there, only
    /// when there is nothing at it yet or it was read and parsed. A file that
    /// could not be used is still the user's: saving defaults over it the
    /// first time they changed the theme lost every setting in it over one
    /// typo, a Latin-1 comment, a UTF-16 file or a permission bit.
    ///
    /// `[keybindings]` is read entry by entry: a key that does not parse or
    /// an action that does not exist is skipped with a warning. Read as part
    /// of the whole file, one such entry failed the parse, so every setting
    /// fell back to its default over a single misspelled action name.
    fn load_from_path(path: &Path) -> Self {
        let unusable = |e: &dyn std::fmt::Display| {
            let warning = format!(
                "could not use config {}: {} (using defaults; \
                 settings changed this session will not be saved over it)",
                path.display(),
                e
            );
            eprintln!("warning: {warning}");
            Self {
                warnings: vec![warning],
                ..Self::default()
            }
        };

        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut config = Self::default();
                config.baseline = config.settings_table().ok();
                config.path = Some(path.to_path_buf());
                return config;
            }
            Err(e) => return unusable(&e),
        };

        let mut table = match content.parse::<toml::Table>() {
            Ok(table) => table,
            Err(e) => return unusable(&e),
        };
        let keybindings = table.remove("keybindings");
        let mut config = match toml::Value::Table(table).try_into::<Self>() {
            Ok(config) => config,
            Err(e) => return unusable(&e),
        };

        let mut warnings = Vec::new();
        if let Some(keybindings) = keybindings {
            config.keybindings = KeybindingsConfig::from_toml_lenient(keybindings, &mut warnings);
        }
        for warning in &warnings {
            eprintln!("warning: {}: {warning}", path.display());
        }
        config.warnings = warnings;
        config.baseline = config.settings_table().ok();
        config.path = Some(path.to_path_buf());
        config
    }

    /// The settings as a TOML table, without the keybindings, which a save
    /// never rewrites.
    fn settings_table(&self) -> Result<toml::Table, toml::ser::Error> {
        let mut table = toml::Table::try_from(self)?;
        table.remove("keybindings");
        Ok(table)
    }

    /// Resolve and load the configuration file, falling back to `Default` if any step fails.
    pub fn load() -> Self {
        Self::resolve_config_path()
            .map(|path| Self::load_from_path(&path))
            .unwrap_or_default()
    }

    /// Save config to file.
    ///
    /// The user's file is edited in place, writing only the settings that
    /// changed since it was loaded or last saved. Serializing the whole
    /// config over it dropped every comment and unknown key, and wrote out
    /// every default the user had left unset, which then stopped following
    /// the defaults. With no file yet, only settings that differ from the
    /// defaults are written. A file that can no longer be read or parsed is
    /// left alone.
    pub fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let path = self
            .path
            .clone()
            .ok_or("config not saved: no config directory, or the config file could not be used")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut document = match fs::read_to_string(&path) {
            Ok(text) => text.parse::<toml_edit::DocumentMut>()?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => toml_edit::DocumentMut::new(),
            Err(e) => return Err(e.into()),
        };
        let current = self.settings_table()?;
        let baseline = match self.baseline.take() {
            Some(baseline) => baseline,
            None => Self::default().settings_table()?,
        };
        let applied = apply_changes(document.as_table_mut(), &baseline, &current);
        self.baseline = Some(baseline);
        applied?;
        crate::fsutil::write_atomic(&path, document.to_string().as_bytes())?;

        // What is on disk now is what the next save compares against, so
        // changing a setting back to its loaded value is saved too.
        self.baseline = Some(current);
        Ok(())
    }

    /// Parse theme name from string
    pub fn theme_name(&self) -> ThemeName {
        match self.ui.theme.as_str() {
            "OceanDark" => ThemeName::OceanDark,
            "Nord" => ThemeName::Nord,
            "Dracula" => ThemeName::Dracula,
            "Solarized" => ThemeName::Solarized,
            "Monokai" => ThemeName::Monokai,
            "Gruvbox" => ThemeName::Gruvbox,
            "TokyoNight" => ThemeName::TokyoNight,
            "CatppuccinMocha" => ThemeName::CatppuccinMocha,
            _ => ThemeName::OceanDark, // Default fallback
        }
    }

    /// Update theme and save config
    pub fn set_theme(&mut self, theme: ThemeName) -> Result<(), Box<dyn std::error::Error>> {
        self.ui.theme = match theme {
            ThemeName::OceanDark => "OceanDark",
            ThemeName::Nord => "Nord",
            ThemeName::Dracula => "Dracula",
            ThemeName::Solarized => "Solarized",
            ThemeName::Monokai => "Monokai",
            ThemeName::Gruvbox => "Gruvbox",
            ThemeName::TokyoNight => "TokyoNight",
            ThemeName::CatppuccinMocha => "CatppuccinMocha",
        }
        .to_string();

        self.save()
    }

    /// Update outline width and save config
    pub fn set_outline_width(&mut self, width: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.ui.outline_width = width;
        self.save()
    }

    /// Mark that we've warned the user about Terminal.app
    pub fn set_warned_terminal_app(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.warned_terminal_app = true;
        self.save()
    }

    /// Get keybindings with user customizations applied
    pub fn keybindings(&self) -> Keybindings {
        self.keybindings.to_keybindings()
    }

    /// Check if compact (gapless) tree style is enabled
    pub fn is_compact_tree(&self) -> bool {
        self.ui.tree_style == "compact"
    }

    /// Get the path of the directory that contains the user's sublime color schemes
    /// (used for syntax highlighting in code blocks)
    pub fn code_theme_dir_path(&self) -> Option<PathBuf> {
        self.path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|parent| parent.join("code-themes"))
    }
}

/// Write into `document` every value of `new` that differs from `baseline`,
/// recursing into tables, and leave everything else as it is.
fn apply_changes(
    document: &mut dyn toml_edit::TableLike,
    baseline: &toml::Table,
    new: &toml::Table,
) -> Result<(), Box<dyn std::error::Error>> {
    for (key, value) in new {
        let before = baseline.get(key);
        if before == Some(value) {
            continue;
        }
        match value {
            toml::Value::Table(table) => {
                let empty = toml::Table::new();
                let before = match before {
                    Some(toml::Value::Table(before)) => before,
                    _ => &empty,
                };
                // `ui = { theme = "Nord" }` is a table too. Taken for a
                // missing one, it was replaced and lost its other keys.
                if !document
                    .get(key)
                    .is_some_and(toml_edit::Item::is_table_like)
                {
                    document.insert(key, toml_edit::Item::Table(toml_edit::Table::new()));
                }
                let section = document
                    .get_mut(key)
                    .and_then(toml_edit::Item::as_table_like_mut)
                    .ok_or_else(|| format!("config key `{key}` is not a table"))?;
                apply_changes(section, before, table)?;
            }
            other => {
                let mut value = other.to_string().parse::<toml_edit::Value>()?;
                // Keep the spacing and any comment after the old value.
                if let Some(old) = document.get(key).and_then(toml_edit::Item::as_value) {
                    *value.decor_mut() = old.decor().clone();
                }
                document.insert(key, toml_edit::value(value));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- ColorValue::to_color ----------

    #[test]
    fn color_named_known_values() {
        assert_eq!(ColorValue::Named("red".into()).to_color(), Some(Color::Red));
        assert_eq!(ColorValue::Named("RED".into()).to_color(), Some(Color::Red));
        assert_eq!(
            ColorValue::Named("Gray".into()).to_color(),
            Some(Color::Gray)
        );
        assert_eq!(
            ColorValue::Named("grey".into()).to_color(),
            Some(Color::Gray)
        );
        assert_eq!(
            ColorValue::Named("LightCyan".into()).to_color(),
            Some(Color::LightCyan)
        );
    }

    #[test]
    fn color_named_unknown_returns_none() {
        assert_eq!(ColorValue::Named("chartreuse".into()).to_color(), None);
        assert_eq!(ColorValue::Named("".into()).to_color(), None);
    }

    #[test]
    fn color_rgb_and_indexed() {
        assert_eq!(
            ColorValue::Rgb { rgb: [10, 20, 30] }.to_color(),
            Some(Color::Rgb(10, 20, 30))
        );
        assert_eq!(
            ColorValue::Indexed { indexed: 235 }.to_color(),
            Some(Color::Indexed(235))
        );
    }

    // ---------- Defaults ----------

    #[test]
    fn config_default_is_sane() {
        let c = Config::default();
        assert_eq!(c.ui.theme, "OceanDark");
        assert_eq!(c.ui.code_theme, "base16-ocean.dark");
        assert_eq!(c.ui.outline_width, 30);
        assert_eq!(c.ui.tree_style, "compact");
        assert!(c.ui.outline_heading_markers);
        assert_eq!(c.terminal.color_mode, "auto");
        assert!(!c.terminal.warned_terminal_app);
        assert!(c.images.enabled);
        assert!(c.content.hide_frontmatter);
        assert!(c.content.hide_latex);
        assert!(c.path.is_none());
    }

    #[test]
    fn is_compact_tree_reflects_style() {
        let mut c = Config::default();
        assert!(c.is_compact_tree());
        c.ui.tree_style = "spaced".to_string();
        assert!(!c.is_compact_tree());
    }

    #[test]
    fn theme_name_known_values() {
        let mut c = Config::default();
        for (raw, expected) in [
            ("OceanDark", ThemeName::OceanDark),
            ("Nord", ThemeName::Nord),
            ("Dracula", ThemeName::Dracula),
            ("Solarized", ThemeName::Solarized),
            ("Monokai", ThemeName::Monokai),
            ("Gruvbox", ThemeName::Gruvbox),
            ("TokyoNight", ThemeName::TokyoNight),
            ("CatppuccinMocha", ThemeName::CatppuccinMocha),
        ] {
            c.ui.theme = raw.into();
            assert_eq!(c.theme_name(), expected, "theme={raw}");
        }
    }

    #[test]
    fn theme_name_unknown_falls_back_to_oceandark() {
        let mut c = Config::default();
        c.ui.theme = "Nonexistent".into();
        assert_eq!(c.theme_name(), ThemeName::OceanDark);
    }

    // ---------- TOML round-trip ----------

    #[test]
    fn config_round_trips_through_toml() {
        let mut c = Config::default();
        c.ui.theme = "Nord".into();
        c.ui.outline_width = 42;
        c.theme.heading_1 = Some(ColorValue::Named("Cyan".into()));
        c.theme.background = Some(ColorValue::Rgb { rgb: [1, 2, 3] });
        c.theme.foreground = Some(ColorValue::Indexed { indexed: 7 });

        let s = toml::to_string_pretty(&c).expect("serialize");
        let parsed: Config = toml::from_str(&s).expect("parse back");

        assert_eq!(parsed.ui.theme, "Nord");
        assert_eq!(parsed.ui.outline_width, 42);
        assert!(matches!(
            parsed.theme.heading_1,
            Some(ColorValue::Named(ref n)) if n == "Cyan"
        ));
        assert!(matches!(
            parsed.theme.background,
            Some(ColorValue::Rgb { rgb: [1, 2, 3] })
        ));
        assert!(matches!(
            parsed.theme.foreground,
            Some(ColorValue::Indexed { indexed: 7 })
        ));
    }

    #[test]
    fn config_partial_toml_uses_defaults_for_missing_fields() {
        // Only set ui.theme — everything else should fall back to Default::default().
        let s = "[ui]\ntheme = \"Dracula\"\n";
        let c: Config = toml::from_str(s).expect("parse");
        assert_eq!(c.ui.theme, "Dracula");
        assert_eq!(c.ui.outline_width, 30); // default
        assert_eq!(c.terminal.color_mode, "auto"); // default
        assert!(c.content.hide_frontmatter); // default
    }

    #[test]
    fn config_code_fences_defaults_to_full_and_parses_each_mode() {
        let c: Config = toml::from_str("[ui]\ntheme = \"Nord\"\n").expect("parse");
        assert_eq!(c.content.code_fences, CodeFences::Full);

        for (value, want) in [
            ("full", CodeFences::Full),
            ("label", CodeFences::Label),
            ("none", CodeFences::None),
        ] {
            let s = format!("[content]\ncode_fences = \"{value}\"\n");
            let c: Config = toml::from_str(&s).unwrap_or_else(|e| panic!("{value}: {e}"));
            assert_eq!(c.content.code_fences, want, "for {value}");
        }
    }

    #[test]
    fn config_code_block_bg_accepts_a_color_or_none() {
        // Unset means "fall back to the code theme's background".
        let c: Config = toml::from_str("[ui]\ntheme = \"Nord\"\n").expect("parse");
        assert!(c.theme.code_block_bg.is_none());

        let s = "[theme]\ncode_block_bg = { rgb = [40, 40, 50] }\n";
        let c: Config = toml::from_str(s).expect("rgb");
        assert!(matches!(
            c.theme.code_block_bg,
            Some(ColorValue::Rgb { rgb: [40, 40, 50] })
        ));

        // "none" is how the background gets turned off entirely.
        let s = "[theme]\ncode_block_bg = \"none\"\n";
        let c: Config = toml::from_str(s).expect("none");
        assert!(matches!(
            c.theme.code_block_bg,
            Some(ColorValue::Named(ref n)) if n == "none"
        ));
    }

    #[test]
    fn config_color_value_untagged_parses_three_forms() {
        // Named
        let s = r#"[theme]
heading_1 = "Red"
"#;
        let c: Config = toml::from_str(s).expect("named");
        assert!(matches!(c.theme.heading_1, Some(ColorValue::Named(_))));

        // Indexed
        let s = r#"[theme]
heading_1 = { indexed = 200 }
"#;
        let c: Config = toml::from_str(s).expect("indexed");
        assert!(matches!(
            c.theme.heading_1,
            Some(ColorValue::Indexed { indexed: 200 })
        ));

        // RGB
        let s = r#"[theme]
heading_1 = { rgb = [10, 20, 30] }
"#;
        let c: Config = toml::from_str(s).expect("rgb");
        assert!(matches!(
            c.theme.heading_1,
            Some(ColorValue::Rgb { rgb: [10, 20, 30] })
        ));
    }

    // ---------- load_from_path & save round-trip ----------

    #[test]
    fn load_from_path_missing_file_returns_default() {
        let p = std::env::temp_dir().join("treemd-nonexistent-xyz-987654.toml");
        assert!(!p.exists());
        let c = Config::load_from_path(&p);
        // Compare to Default by checking a couple of fields.
        assert_eq!(c.ui.theme, Config::default().ui.theme);
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut c = Config::default();
        c.ui.theme = "Gruvbox".into();
        c.ui.outline_width = 55;
        c.path = Some(path.clone());
        c.save().expect("save");

        let loaded = Config::load_from_path(&path);
        assert_eq!(loaded.ui.theme, "Gruvbox");
        assert_eq!(loaded.ui.outline_width, 55);
    }

    const USER_CONFIG: &str = r#"# my settings
[ui]
theme = "Gruvbox" # the good one
outline_width = 40
something_new = "kept"

[keybindings.Normal]
"j" = "Next"
"x" = "NoSuchAction"
"Ctrl+Nope" = "Quit"
"#;

    /// One misspelled action used to fail the whole file, so every setting,
    /// the theme included, fell back to its default. Each bad binding is a
    /// warning now, and everything else still applies.
    #[test]
    fn a_bad_keybinding_is_skipped_with_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, USER_CONFIG).unwrap();

        let c = Config::load_from_path(&path);
        assert_eq!(c.ui.theme, "Gruvbox");
        assert_eq!(c.ui.outline_width, 40);
        assert_eq!(c.warnings.len(), 2, "{:?}", c.warnings);
        assert!(c.warnings.iter().any(|w| w.contains("NoSuchAction")));
        assert!(c.warnings.iter().any(|w| w.contains("Ctrl+Nope")));
        let normal = &c.keybindings.0[&crate::keybindings::KeybindingMode::Normal];
        assert_eq!(normal.get("j"), Some(&crate::keybindings::Action::Next));
        assert!(c.path.is_some(), "a usable config can still be saved");
    }

    /// A save rewrote the whole file from the struct, which dropped comments,
    /// unknown keys, and any binding it could not read. Only what changed is
    /// written now.
    #[test]
    fn a_save_changes_only_the_setting_that_changed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, USER_CONFIG).unwrap();

        let mut c = Config::load_from_path(&path);
        c.set_theme(ThemeName::Nord).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, USER_CONFIG.replace(r#""Gruvbox""#, r#""Nord""#));

        // And the result loads back with the new value.
        assert_eq!(Config::load_from_path(&path).ui.theme, "Nord");
    }

    /// A file that could not be read (here not UTF-8, as a Latin-1 comment
    /// or a UTF-16 file is) was treated as no file yet and replaced with
    /// defaults on the first save.
    #[test]
    fn an_unreadable_config_is_never_saved_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let bytes = b"# caf\xe9\n[ui]\ntheme = \"Nord\"\n";
        std::fs::write(&path, bytes).unwrap();

        let mut c = Config::load_from_path(&path);
        assert_eq!(c.warnings.len(), 1, "{:?}", c.warnings);
        assert!(c.set_outline_width(40).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    /// The comparison point for a save was fixed at load, so changing a
    /// setting and then changing it back left the changed value on disk.
    #[test]
    fn changing_a_setting_back_is_saved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[ui]\ntheme = \"Nord\"\n").unwrap();

        let mut c = Config::load_from_path(&path);
        c.set_outline_width(40).unwrap();
        c.set_outline_width(30).unwrap();
        assert_eq!(Config::load_from_path(&path).ui.outline_width, 30);
    }

    /// `ui = { ... }` was taken for a missing table and replaced.
    #[test]
    fn an_inline_table_keeps_its_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "ui = { theme = \"Nord\", outline_width = 30 }\n").unwrap();

        let mut c = Config::load_from_path(&path);
        c.set_outline_width(40).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "ui = { theme = \"Nord\", outline_width = 40 }\n"
        );
    }

    /// With no file yet, the first save wrote every default, pinning them.
    #[test]
    fn a_first_save_writes_only_what_changed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("treemd").join("config.toml");

        let mut c = Config::load_from_path(&path);
        c.set_outline_width(40).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "[ui]\noutline_width = 40\n"
        );
    }

    #[test]
    fn load_from_path_invalid_toml_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is not valid = = = toml [[[").unwrap();

        // Suppress the eprintln in the test output by just checking the result.
        let mut c = Config::load_from_path(&path);
        assert_eq!(c.ui.theme, Config::default().ui.theme);

        // Changing a setting must not save defaults over the user's file.
        assert!(c.set_theme(ThemeName::Nord).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "this is not valid = = = toml [[["
        );
    }

    // ---------- code_theme_dir_path ----------

    #[test]
    fn code_theme_dir_path_no_path_returns_none() {
        let c = Config::default();
        assert!(c.code_theme_dir_path().is_none());
    }

    #[test]
    fn code_theme_dir_path_uses_config_parent() {
        let c = Config {
            path: Some(PathBuf::from("/etc/treemd/config.toml")),
            ..Default::default()
        };
        assert_eq!(
            c.code_theme_dir_path(),
            Some(PathBuf::from("/etc/treemd/code-themes"))
        );
    }
}
