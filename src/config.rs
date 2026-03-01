use crate::keybindings::{Keybindings, KeybindingsConfig};
use crate::tui::theme::ThemeName;
use opensesame::EditorConfig;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
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
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("treemd").join("config.toml"))
    }

    /// Load config from file, or return default if file doesn't exist
    /// On macOS, checks ~/.config/treemd first, then falls back to ~/Library/Application Support
    pub fn load() -> Self {
        #[cfg(target_os = "macos")]
        {
            // Prefer XDG-style path on macOS for CLI tools
            if let Some(xdg_path) = Self::xdg_config_path()
                && let Ok(contents) = fs::read_to_string(&xdg_path)
                && let Ok(config) = toml::from_str(&contents)
            {
                return config;
            }
        }

        // Fall back to platform-specific path
        Self::config_path()
            .and_then(|path| {
                fs::read_to_string(&path)
                    .ok()
                    .and_then(|contents| toml::from_str(&contents).ok())
            })
            .unwrap_or_default()
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;

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
}
