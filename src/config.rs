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
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            hide_frontmatter: default_hide_frontmatter(),
            hide_latex: default_hide_latex(),
            latex_aggressive: default_latex_aggressive(),
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
        }
    }
}

fn default_tree_style() -> String {
    "compact".to_string()
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
    fn xdg_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|p| p.join(".config").join("treemd").join("config.toml"))
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
    fn load_from_path(path: &Path) -> Self {
        let Ok(content) = fs::read_to_string(path) else {
            return Self::default();
        };

        match toml::from_str::<Self>(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!(
                    "warning: failed to parse config {}: {} (using defaults)",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Resolve and load the configuration file, falling back to `Default` if any step fails.
    pub fn load() -> Self {
        Self::resolve_config_path()
            .map(|path| {
                let mut config = Self::load_from_path(&path);
                config.path = Some(path);
                config
            })
            .unwrap_or_default()
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = self
            .path
            .as_ref()
            .ok_or("Could not determine config directory")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;

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
        let dir = std::env::temp_dir().join(format!("treemd-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let mut c = Config::default();
        c.ui.theme = "Gruvbox".into();
        c.ui.outline_width = 55;
        c.path = Some(path.clone());
        c.save().expect("save");

        let loaded = Config::load_from_path(&path);
        assert_eq!(loaded.ui.theme, "Gruvbox");
        assert_eq!(loaded.ui.outline_width, 55);

        // Cleanup
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn load_from_path_invalid_toml_falls_back_to_default() {
        let dir = std::env::temp_dir().join(format!("treemd-test-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "this is not valid = = = toml [[[").unwrap();

        // Suppress the eprintln in the test output by just checking the result.
        let c = Config::load_from_path(&path);
        assert_eq!(c.ui.theme, Config::default().ui.theme);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
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
