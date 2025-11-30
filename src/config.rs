use crate::tui::theme::ThemeName;
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_outline_width")]
    pub outline_width: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    #[serde(default = "default_color_mode")]
    pub color_mode: String,

    #[serde(default)]
    pub warned_terminal_app: bool,
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
        }
    }
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
    /// Get the config file path (platform-specific)
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("treemd").join("config.toml"))
    }

    /// Load config from file, or return default if file doesn't exist
    pub fn load() -> Self {
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
}
