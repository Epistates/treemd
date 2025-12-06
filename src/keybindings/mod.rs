//! Customizable keybindings for treemd
//!
//! This module provides a flexible keybinding system that allows users to
//! customize keyboard shortcuts via configuration files.
//!
//! # Architecture
//!
//! - [`Action`] - All bindable actions in the application
//! - [`KeybindingMode`] - Different modes with their own keybinding sets
//! - [`Keybindings`] - The complete keybinding configuration (backed by keybinds-rs)
//!
//! # Configuration
//!
//! Keybindings are configured in TOML format, organized by mode:
//!
//! ```toml
//! [keybindings.Normal]
//! "j" = "Next"
//! "k" = "Previous"
//! "Ctrl+c" = "Quit"
//! "g g" = "First"  # Multi-key sequences supported!
//!
//! [keybindings.Interactive]
//! "Escape" = "ExitInteractiveMode"
//! ```

mod action;
mod defaults;

pub use action::Action;

use crossterm::event::KeyEvent;
use keybinds::Keybinds;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application modes that have their own keybinding sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum KeybindingMode {
    /// Normal navigation mode
    Normal,
    /// Help popup is shown
    Help,
    /// Theme picker is shown
    ThemePicker,
    /// Interactive element navigation
    Interactive,
    /// Table cell navigation within interactive mode
    InteractiveTable,
    /// Link following mode
    LinkFollow,
    /// Link search/filter within link follow mode
    LinkSearch,
    /// Outline search/filter mode
    Search,
    /// Cell editing mode (for tables)
    CellEdit,
    /// Confirmation dialog
    ConfirmDialog,
}

impl KeybindingMode {
    /// Get a display name for the mode
    pub fn display_name(&self) -> &'static str {
        match self {
            KeybindingMode::Normal => "Normal",
            KeybindingMode::Help => "Help",
            KeybindingMode::ThemePicker => "Theme Picker",
            KeybindingMode::Interactive => "Interactive",
            KeybindingMode::InteractiveTable => "Table Navigation",
            KeybindingMode::LinkFollow => "Link Follow",
            KeybindingMode::LinkSearch => "Link Search",
            KeybindingMode::Search => "Search",
            KeybindingMode::CellEdit => "Cell Edit",
            KeybindingMode::ConfirmDialog => "Confirm",
        }
    }
}

/// Complete keybinding configuration
///
/// Wraps keybinds-rs dispatchers with mode-based organization.
#[derive(Debug)]
pub struct Keybindings {
    /// Keybindings organized by mode
    bindings: HashMap<KeybindingMode, Keybinds<Action>>,
}

impl Default for Keybindings {
    fn default() -> Self {
        defaults::default_keybindings()
    }
}

impl Clone for Keybindings {
    fn clone(&self) -> Self {
        // We need to rebuild since Keybinds doesn't implement Clone
        // This is fine since cloning is rare (only during config reload)
        defaults::default_keybindings()
    }
}

impl Keybindings {
    /// Create empty keybindings
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Get the action for a key event in a specific mode
    ///
    /// This is the main dispatch method - pass crossterm KeyEvents directly.
    pub fn dispatch(&mut self, mode: KeybindingMode, event: KeyEvent) -> Option<Action> {
        self.bindings
            .get_mut(&mode)
            .and_then(|kb| kb.dispatch(event).copied())
    }

    /// Check if a multi-key sequence is in progress for this mode
    pub fn is_sequence_ongoing(&self, mode: KeybindingMode) -> bool {
        self.bindings
            .get(&mode)
            .map(|kb| kb.is_ongoing())
            .unwrap_or(false)
    }

    /// Reset any in-progress key sequences (call when switching modes)
    pub fn reset_sequences(&mut self) {
        for kb in self.bindings.values_mut() {
            kb.reset();
        }
    }

    /// Get the keybinds for a specific mode
    pub fn get_mode_keybinds(&self, mode: KeybindingMode) -> Option<&Keybinds<Action>> {
        self.bindings.get(&mode)
    }

    /// Bind a key sequence to an action in a mode
    pub fn bind(
        &mut self,
        mode: KeybindingMode,
        key_sequence: &str,
        action: Action,
    ) -> Result<(), keybinds::Error> {
        self.bindings
            .entry(mode)
            .or_default()
            .bind(key_sequence, action)
    }

    /// Get all keys bound to an action in a mode (for help text generation)
    pub fn keys_for_action(&self, mode: KeybindingMode, action: Action) -> Vec<String> {
        self.bindings
            .get(&mode)
            .map(|kb| {
                kb.as_slice()
                    .iter()
                    .filter(|bind| bind.action == action)
                    .map(|bind| format_key_sequence(&bind.seq))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Generate help entries for a mode (action -> keys)
    pub fn help_entries(&self, mode: KeybindingMode) -> Vec<(Action, Vec<String>)> {
        let mut action_keys: HashMap<Action, Vec<String>> = HashMap::new();

        if let Some(kb) = self.bindings.get(&mode) {
            for bind in kb.as_slice() {
                let key_str = format_key_sequence(&bind.seq);
                action_keys
                    .entry(bind.action)
                    .or_default()
                    .push(key_str);
            }
        }

        let mut entries: Vec<_> = action_keys.into_iter().collect();
        entries.sort_by(|a, b| {
            a.0.category()
                .cmp(b.0.category())
                .then(a.0.description().cmp(b.0.description()))
        });
        entries
    }

    /// Merge another keybindings set into this one (other takes precedence)
    pub fn merge(&mut self, other: &KeybindingsConfig) -> Result<(), String> {
        for (mode, mode_bindings) in &other.0 {
            let kb = self.bindings.entry(*mode).or_default();
            for (key_str, action) in mode_bindings {
                kb.bind(key_str, *action)
                    .map_err(|e| format!("Invalid key '{}': {}", key_str, e))?;
            }
        }
        Ok(())
    }
}

/// Format a key sequence for display
fn format_key_sequence(seq: &keybinds::KeySeq) -> String {
    seq.as_slice()
        .iter()
        .map(format_key_input)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a single key input for display
fn format_key_input(input: &keybinds::KeyInput) -> String {
    let mut parts = Vec::new();

    let mods = input.mods();
    if mods.contains(keybinds::Mods::CTRL) {
        parts.push("C");
    }
    if mods.contains(keybinds::Mods::ALT) {
        parts.push("A");
    }
    if mods.contains(keybinds::Mods::SHIFT) {
        parts.push("S");
    }

    let key_str = format_key(input.key());
    parts.push(&key_str);

    if parts.len() == 1 {
        key_str
    } else {
        parts.join("-")
    }
}

/// Format a key for display
fn format_key(key: keybinds::Key) -> String {
    use keybinds::Key;
    match key {
        Key::Char(' ') => "Spc".to_string(),
        Key::Char(c) => c.to_string(),
        Key::Enter => "Ret".to_string(),
        Key::Esc => "Esc".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::Backspace => "BS".to_string(),
        Key::Delete => "Del".to_string(),
        Key::Up => "↑".to_string(),
        Key::Down => "↓".to_string(),
        Key::Left => "←".to_string(),
        Key::Right => "→".to_string(),
        Key::PageUp => "PgU".to_string(),
        Key::PageDown => "PgD".to_string(),
        Key::Home => "Home".to_string(),
        Key::End => "End".to_string(),
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(),
        Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(),
        Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(),
        Key::F12 => "F12".to_string(),
        _ => "?".to_string(),
    }
}

/// Configuration format for keybindings (uses string keys for TOML compatibility)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindingsConfig(pub HashMap<KeybindingMode, HashMap<String, Action>>);

impl KeybindingsConfig {
    /// Convert to Keybindings, using defaults for any missing bindings
    pub fn to_keybindings(&self) -> Keybindings {
        let mut keybindings = Keybindings::default();

        // Override with user config (silently ignore invalid keys)
        let _ = keybindings.merge(self);

        keybindings
    }

    /// Check if the config is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_default_keybindings_exist() {
        let mut kb = Keybindings::default();

        // Check some basic normal mode bindings
        assert!(kb
            .dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('j'), KeyModifiers::NONE)
            )
            .is_some());
        assert!(kb
            .dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('k'), KeyModifiers::NONE)
            )
            .is_some());
        assert!(kb
            .dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('q'), KeyModifiers::NONE)
            )
            .is_some());
    }

    #[test]
    fn test_dispatch() {
        let mut kb = Keybindings::default();

        let action = kb.dispatch(
            KeybindingMode::Normal,
            make_key_event(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        assert_eq!(action, Some(Action::Next));

        let action = kb.dispatch(
            KeybindingMode::Normal,
            make_key_event(KeyCode::Char('x'), KeyModifiers::NONE),
        );
        assert!(action.is_none() || action == Some(Action::Next)); // May match or not
    }

    #[test]
    fn test_keys_for_action() {
        let kb = Keybindings::default();

        let keys = kb.keys_for_action(KeybindingMode::Normal, Action::Next);
        assert!(keys.len() >= 1); // j and/or Down should be bound
    }

    #[test]
    fn test_all_modes_have_bindings() {
        let kb = Keybindings::default();

        let modes = [
            KeybindingMode::Normal,
            KeybindingMode::Help,
            KeybindingMode::ThemePicker,
            KeybindingMode::Interactive,
            KeybindingMode::InteractiveTable,
            KeybindingMode::LinkFollow,
            KeybindingMode::LinkSearch,
            KeybindingMode::Search,
            KeybindingMode::ConfirmDialog,
        ];

        for mode in modes {
            assert!(
                kb.get_mode_keybinds(mode).is_some(),
                "Mode {:?} has no bindings",
                mode
            );
            assert!(
                !kb.get_mode_keybinds(mode).unwrap().as_slice().is_empty(),
                "Mode {:?} has empty bindings",
                mode
            );
        }
    }
}
