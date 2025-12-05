//! Key string parsing and formatting
//!
//! This module handles conversion between human-readable key strings
//! (like "ctrl-c", "alt-enter", "shift-tab") and crossterm KeyEvent types.

use crossterm::event::{KeyCode, KeyModifiers};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::hash::Hash;

/// A key binding representing a key code with modifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Create a key binding with no modifiers
    pub fn key(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Create a key binding with Ctrl modifier
    pub fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    /// Create a key binding with Alt modifier
    pub fn alt(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::ALT,
        }
    }

    /// Create a key binding with Shift modifier
    pub fn shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    /// Check if this binding matches a key event
    pub fn matches(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        self.code == code && self.modifiers == modifiers
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_key(self))
    }
}

impl Serialize for KeyBinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format_key(self))
    }
}

impl<'de> Deserialize<'de> for KeyBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_key(&s).map_err(serde::de::Error::custom)
    }
}

/// Parse a key string into a KeyBinding
///
/// Supported formats:
/// - Single characters: "j", "k", "?", "/"
/// - Special keys: "enter", "esc", "tab", "space", "backspace", "delete"
/// - Arrow keys: "up", "down", "left", "right"
/// - Page keys: "pageup", "pagedown", "home", "end"
/// - Function keys: "f1", "f2", ..., "f12"
/// - With modifiers: "ctrl-c", "alt-enter", "shift-tab", "ctrl-alt-delete"
///
/// Modifiers are case-insensitive and can be combined.
pub fn parse_key(s: &str) -> Result<KeyBinding, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty key string".to_string());
    }

    let lower = s.to_lowercase();
    let mut modifiers = KeyModifiers::NONE;
    let mut remaining = lower.as_str();

    // Extract modifiers
    loop {
        if let Some(rest) = remaining.strip_prefix("ctrl-") {
            modifiers.insert(KeyModifiers::CONTROL);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("control-") {
            modifiers.insert(KeyModifiers::CONTROL);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("alt-") {
            modifiers.insert(KeyModifiers::ALT);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("meta-") {
            modifiers.insert(KeyModifiers::ALT);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("shift-") {
            modifiers.insert(KeyModifiers::SHIFT);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("c-") {
            modifiers.insert(KeyModifiers::CONTROL);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("a-") {
            modifiers.insert(KeyModifiers::ALT);
            remaining = rest;
        } else if let Some(rest) = remaining.strip_prefix("s-") {
            modifiers.insert(KeyModifiers::SHIFT);
            remaining = rest;
        } else {
            break;
        }
    }

    // Parse the key code
    let code = parse_key_code(remaining)?;

    Ok(KeyBinding { code, modifiers })
}

/// Parse a key code string (without modifiers)
fn parse_key_code(s: &str) -> Result<KeyCode, String> {
    match s {
        // Special keys
        "enter" | "return" | "cr" => Ok(KeyCode::Enter),
        "esc" | "escape" => Ok(KeyCode::Esc),
        "tab" => Ok(KeyCode::Tab),
        "backtab" | "btab" => Ok(KeyCode::BackTab),
        "backspace" | "bs" => Ok(KeyCode::Backspace),
        "delete" | "del" => Ok(KeyCode::Delete),
        "insert" | "ins" => Ok(KeyCode::Insert),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" | "pgup" | "page_up" => Ok(KeyCode::PageUp),
        "pagedown" | "pgdn" | "pgdown" | "page_down" => Ok(KeyCode::PageDown),

        // Arrow keys
        "up" | "uparrow" => Ok(KeyCode::Up),
        "down" | "downarrow" => Ok(KeyCode::Down),
        "left" | "leftarrow" => Ok(KeyCode::Left),
        "right" | "rightarrow" => Ok(KeyCode::Right),

        // Special characters with names
        "space" | "spc" => Ok(KeyCode::Char(' ')),
        "lt" | "less" => Ok(KeyCode::Char('<')),
        "gt" | "greater" => Ok(KeyCode::Char('>')),
        "bar" | "pipe" => Ok(KeyCode::Char('|')),
        "bslash" | "backslash" => Ok(KeyCode::Char('\\')),

        // Function keys
        s if s.starts_with('f') && s.len() >= 2 => {
            let num_str = &s[1..];
            let num: u8 = num_str
                .parse()
                .map_err(|_| format!("Invalid function key: {}", s))?;
            if !(1..=24).contains(&num) {
                return Err(format!("Function key out of range (1-24): {}", s));
            }
            Ok(KeyCode::F(num))
        }

        // Single character
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            Ok(KeyCode::Char(c))
        }

        // Unknown
        _ => Err(format!("Unknown key: {}", s)),
    }
}

/// Format a KeyBinding into a human-readable string
pub fn format_key(binding: &KeyBinding) -> String {
    let mut parts = Vec::new();

    // Add modifiers
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if binding.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }

    // Add key
    let key_str = format_key_code(&binding.code);
    parts.push(key_str);

    parts.join("-")
}

/// Format a KeyCode into a human-readable string
fn format_key_code(code: &KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => {
            if c.is_uppercase() || !c.is_alphabetic() {
                c.to_string()
            } else {
                c.to_uppercase().to_string()
            }
        }
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "BackTab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Null => "Null".to_string(),
        _ => "?".to_string(),
    }
}

/// Format a KeyBinding for display in help text (compact form)
pub fn format_key_compact(binding: &KeyBinding) -> String {
    let mut parts = Vec::new();

    // Add modifiers (compact)
    if binding.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("C");
    }
    if binding.modifiers.contains(KeyModifiers::ALT) {
        parts.push("A");
    }
    if binding.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("S");
    }

    // Add key
    let key_str = match &binding.code {
        KeyCode::Char(' ') => "Spc".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Ret".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "S-Tab".to_string(),
        KeyCode::Backspace => "BS".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::PageUp => "PgU".to_string(),
        KeyCode::PageDown => "PgD".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => format_key_code(&binding.code),
    };

    if parts.is_empty() {
        key_str
    } else {
        parts.push(&key_str);
        parts.join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_keys() {
        assert_eq!(
            parse_key("j").unwrap(),
            KeyBinding::key(KeyCode::Char('j'))
        );
        assert_eq!(
            parse_key("?").unwrap(),
            KeyBinding::key(KeyCode::Char('?'))
        );
        assert_eq!(parse_key("enter").unwrap(), KeyBinding::key(KeyCode::Enter));
        assert_eq!(parse_key("esc").unwrap(), KeyBinding::key(KeyCode::Esc));
        assert_eq!(parse_key("tab").unwrap(), KeyBinding::key(KeyCode::Tab));
        assert_eq!(parse_key("space").unwrap(), KeyBinding::key(KeyCode::Char(' ')));
    }

    #[test]
    fn test_parse_with_modifiers() {
        assert_eq!(
            parse_key("ctrl-c").unwrap(),
            KeyBinding::ctrl(KeyCode::Char('c'))
        );
        assert_eq!(
            parse_key("alt-enter").unwrap(),
            KeyBinding::alt(KeyCode::Enter)
        );
        assert_eq!(
            parse_key("shift-tab").unwrap(),
            KeyBinding::shift(KeyCode::Tab)
        );
    }

    #[test]
    fn test_parse_combined_modifiers() {
        let binding = parse_key("ctrl-alt-delete").unwrap();
        assert_eq!(binding.code, KeyCode::Delete);
        assert!(binding.modifiers.contains(KeyModifiers::CONTROL));
        assert!(binding.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn test_parse_function_keys() {
        assert_eq!(parse_key("f1").unwrap(), KeyBinding::key(KeyCode::F(1)));
        assert_eq!(parse_key("f12").unwrap(), KeyBinding::key(KeyCode::F(12)));
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(parse_key("up").unwrap(), KeyBinding::key(KeyCode::Up));
        assert_eq!(parse_key("down").unwrap(), KeyBinding::key(KeyCode::Down));
        assert_eq!(parse_key("left").unwrap(), KeyBinding::key(KeyCode::Left));
        assert_eq!(parse_key("right").unwrap(), KeyBinding::key(KeyCode::Right));
    }

    #[test]
    fn test_format_roundtrip() {
        let keys = vec!["j", "ctrl-c", "alt-enter", "shift-tab", "f1", "space"];
        for key_str in keys {
            let binding = parse_key(key_str).unwrap();
            let formatted = format_key(&binding).to_lowercase().replace('-', "-");
            // Parse again and compare
            let reparsed = parse_key(&formatted).unwrap();
            assert_eq!(binding, reparsed, "Roundtrip failed for: {}", key_str);
        }
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(parse_key("CTRL-C").unwrap(), parse_key("ctrl-c").unwrap());
        assert_eq!(parse_key("Alt-Enter").unwrap(), parse_key("alt-enter").unwrap());
    }

    #[test]
    fn test_alternative_names() {
        assert_eq!(parse_key("return").unwrap(), parse_key("enter").unwrap());
        assert_eq!(parse_key("escape").unwrap(), parse_key("esc").unwrap());
        assert_eq!(parse_key("bs").unwrap(), parse_key("backspace").unwrap());
        assert_eq!(parse_key("del").unwrap(), parse_key("delete").unwrap());
        assert_eq!(parse_key("pgup").unwrap(), parse_key("pageup").unwrap());
    }

    #[test]
    fn test_short_modifier_names() {
        assert_eq!(parse_key("c-c").unwrap(), parse_key("ctrl-c").unwrap());
        assert_eq!(parse_key("a-x").unwrap(), parse_key("alt-x").unwrap());
        assert_eq!(parse_key("s-tab").unwrap(), parse_key("shift-tab").unwrap());
    }
}
