use supports_color::{Stream, on};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorMode {
    Rgb,        // True color (16M colors)
    Indexed256, // 256-color palette
}

#[derive(Debug)]
pub struct TerminalCapabilities {
    pub supports_rgb: bool,
    pub is_terminal_app: bool,
    pub macos_version: Option<u32>,
    pub recommended_color_mode: ColorMode,
    pub should_warn: bool,
}

impl TerminalCapabilities {
    /// Detect terminal capabilities and recommend appropriate color mode
    pub fn detect() -> Self {
        let is_terminal_app = std::env::var("TERM_PROGRAM")
            .map(|v| v == "Apple_Terminal")
            .unwrap_or(false);

        // Detect RGB/truecolor support using multiple methods for robustness.
        // The supports_color crate can miss truecolor in some terminals,
        // so we check environment variables first per termstandard/colors recommendations.
        let supports_rgb = Self::detect_truecolor_support();

        let macos_version = Self::detect_macos_version();

        // Determine if we should warn and which color mode to use
        let (should_warn, recommended_color_mode) = if is_terminal_app {
            match macos_version {
                Some(version) if version >= 26 => {
                    // macOS 26+ (Tahoe and later) - Terminal.app works well
                    (false, ColorMode::Rgb)
                }
                Some(_) | None => {
                    // macOS < 26 (Sequoia and earlier) or unknown - use fallback
                    (true, ColorMode::Indexed256)
                }
            }
        } else {
            // Not Terminal.app - trust the terminal's capabilities
            let mode = if supports_rgb {
                ColorMode::Rgb
            } else {
                ColorMode::Indexed256
            };
            (false, mode)
        };

        Self {
            supports_rgb,
            is_terminal_app,
            macos_version,
            recommended_color_mode,
            should_warn,
        }
    }

    /// Detect macOS Darwin version (e.g., 24 for Sequoia, 26 for Tahoe)
    fn detect_macos_version() -> Option<u32> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            // Run `uname -r` to get Darwin version (e.g., "24.6.0" for Sequoia)
            let output = Command::new("uname").arg("-r").output().ok()?;

            let version_str = String::from_utf8(output.stdout).ok()?;
            let major_version = version_str.split('.').next()?.parse::<u32>().ok()?;

            Some(major_version)
        }

        #[cfg(not(target_os = "macos"))]
        None
    }

    /// Detect truecolor (24-bit RGB) support using multiple methods.
    ///
    /// Per termstandard/colors recommendations, we check in this order:
    /// 1. COLORTERM env var for "truecolor" or "24bit" (most reliable)
    /// 2. TERM env var for known truecolor terminals or suffixes
    /// 3. Fall back to supports_color crate detection
    fn detect_truecolor_support() -> bool {
        // Method 1: Check COLORTERM environment variable (primary standard)
        // VTE, Konsole, iTerm2, Kitty, Alacritty all set this
        if let Ok(colorterm) = std::env::var("COLORTERM")
            && (colorterm == "truecolor" || colorterm == "24bit")
        {
            return true;
        }

        // Method 2: Check TERM for known truecolor-capable terminals or suffixes
        if let Ok(term) = std::env::var("TERM") {
            let term_lower = term.to_lowercase();
            // Check for explicit truecolor/direct suffixes
            if term_lower.ends_with("-truecolor")
                || term_lower.ends_with("-direct")
                || term_lower.ends_with("direct")
            {
                return true;
            }
            // Check for known truecolor-capable terminal types
            if term_lower.contains("kitty")
                || term_lower.contains("alacritty")
                || term_lower.contains("wezterm")
            {
                return true;
            }
        }

        // Method 3: Check TERM_PROGRAM for known truecolor apps
        // (iTerm2 is already handled by supports_color, but be explicit)
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            let prog_lower = term_program.to_lowercase();
            if prog_lower.contains("iterm")
                || prog_lower.contains("kitty")
                || prog_lower.contains("alacritty")
                || prog_lower.contains("wezterm")
                || prog_lower.contains("hyper")
                || prog_lower.contains("vscode")
            {
                return true;
            }
        }

        // Method 4: Fall back to supports_color crate detection
        on(Stream::Stdout)
            .map(|level| level.has_16m)
            .unwrap_or(false)
    }

    /// Get a user-friendly warning message
    pub fn warning_message(&self) -> Option<String> {
        if !self.should_warn {
            return None;
        }

        Some(format!(
            "⚠️  Terminal Compatibility Notice\n\n\
             Apple Terminal.app on macOS {} has limited RGB color support.\n\
             Switching to 256-color mode for better compatibility.\n\n\
             For the best experience, consider using:\n\
             • iTerm2 (https://iterm2.com/)\n\
             • Kitty (https://sw.kovidgoyal.net/kitty/)\n\
             • Alacritty (https://alacritty.org/)\n\n\
             Press any key to continue...",
            self.macos_version
                .map(|v| format!("Sequoia (Darwin {})", v))
                .unwrap_or_else(|| "< 26".to_string())
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_detection() {
        let caps = TerminalCapabilities::detect();
        // Just ensure it doesn't panic
        println!("Detected capabilities: {:?}", caps);
    }

    #[test]
    fn test_truecolor_detection_uses_env_vars() {
        // This test verifies that the detection method runs without panicking.
        // Full testing of env var logic would require mocking, but we verify
        // the detection integrates properly with the capabilities struct.
        let caps = TerminalCapabilities::detect();

        // The detection should return a valid color mode regardless of environment
        assert!(
            caps.recommended_color_mode == ColorMode::Rgb
                || caps.recommended_color_mode == ColorMode::Indexed256
        );
    }

    #[test]
    fn test_color_mode_enum() {
        // Verify ColorMode variants are distinct
        assert_ne!(ColorMode::Rgb, ColorMode::Indexed256);

        // Verify Copy trait works
        let mode = ColorMode::Rgb;
        let mode_copy = mode;
        assert_eq!(mode, mode_copy);
    }
}
