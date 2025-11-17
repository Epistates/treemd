use crate::tui::theme::ThemeName;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub terminal: TerminalConfig,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
            terminal: TerminalConfig::default(),
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
