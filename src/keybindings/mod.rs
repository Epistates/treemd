//! Customizable keybindings for treemd
//!
//! This module provides a flexible keybinding system that allows users to
//! customize keyboard shortcuts via configuration files.
//!
//! # Architecture
//!
//! - [`Action`] - All bindable actions in the application
//! - [`KeybindingMode`] - Different modes with their own keybinding sets
//! - [`KeyBinding`] - A key code with modifiers
//! - [`Keybindings`] - The complete keybinding configuration
//!
//! # Configuration
//!
//! Keybindings are configured in TOML format, organized by mode:
//!
//! ```toml
//! [keybindings.Normal]
//! "j" = "Next"
//! "k" = "Previous"
//! "ctrl-c" = "Quit"
//!
//! [keybindings.Interactive]
//! "esc" = "ExitInteractiveMode"
//! ```

mod action;
mod defaults;
mod parse;

pub use action::Action;
pub use parse::{format_key, format_key_compact, parse_key, KeyBinding};

use crossterm::event::{KeyCode, KeyModifiers};
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
#[derive(Debug, Clone)]
pub struct Keybindings {
    /// Keybindings organized by mode
    bindings: HashMap<KeybindingMode, HashMap<KeyBinding, Action>>,
}

impl Default for Keybindings {
    fn default() -> Self {
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

    /// Get the action for a key in a specific mode
    pub fn get_action(
        &self,
        mode: KeybindingMode,
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<Action> {
        let binding = KeyBinding::new(code, modifiers);
        self.bindings
            .get(&mode)
            .and_then(|mode_bindings| mode_bindings.get(&binding))
            .copied()
    }

    /// Get all bindings for a mode
    pub fn get_mode_bindings(&self, mode: KeybindingMode) -> Option<&HashMap<KeyBinding, Action>> {
        self.bindings.get(&mode)
    }

    /// Set a keybinding
    pub fn set(&mut self, mode: KeybindingMode, binding: KeyBinding, action: Action) {
        self.bindings
            .entry(mode)
            .or_default()
            .insert(binding, action);
    }

    /// Remove a keybinding
    pub fn remove(&mut self, mode: KeybindingMode, binding: &KeyBinding) -> Option<Action> {
        self.bindings
            .get_mut(&mode)
            .and_then(|mode_bindings| mode_bindings.remove(binding))
    }

    /// Get all keys bound to an action in a mode
    pub fn keys_for_action(&self, mode: KeybindingMode, action: Action) -> Vec<&KeyBinding> {
        self.bindings
            .get(&mode)
            .map(|mode_bindings| {
                mode_bindings
                    .iter()
                    .filter(|&(_, &a)| a == action)
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Generate help entries for a mode (action -> keys)
    pub fn help_entries(&self, mode: KeybindingMode) -> Vec<(Action, Vec<String>)> {
        let mut action_keys: HashMap<Action, Vec<String>> = HashMap::new();

        if let Some(mode_bindings) = self.bindings.get(&mode) {
            for (binding, action) in mode_bindings {
                action_keys
                    .entry(*action)
                    .or_default()
                    .push(format_key_compact(binding));
            }
        }

        let mut entries: Vec<_> = action_keys.into_iter().collect();
        entries.sort_by(|a, b| a.0.category().cmp(b.0.category()).then(a.0.description().cmp(b.0.description())));
        entries
    }

    /// Merge another keybindings set into this one (other takes precedence)
    pub fn merge(&mut self, other: &Keybindings) {
        for (mode, other_bindings) in &other.bindings {
            let mode_bindings = self.bindings.entry(*mode).or_default();
            for (binding, action) in other_bindings {
                mode_bindings.insert(binding.clone(), *action);
            }
        }
    }

    /// Create from a config map (string keys)
    pub fn from_config(
        config: &HashMap<KeybindingMode, HashMap<String, Action>>,
    ) -> Result<Self, String> {
        let mut keybindings = Self::new();

        for (mode, mode_config) in config {
            for (key_str, action) in mode_config {
                let binding = parse_key(key_str)?;
                keybindings.set(*mode, binding, *action);
            }
        }

        Ok(keybindings)
    }
}

/// Configuration format for keybindings (uses string keys for TOML compatibility)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindingsConfig(pub HashMap<KeybindingMode, HashMap<String, Action>>);

impl KeybindingsConfig {
    /// Convert to Keybindings, using defaults for any missing bindings
    pub fn to_keybindings(&self) -> Keybindings {
        let mut keybindings = Keybindings::default();

        // Override with user config
        if let Ok(user_bindings) = Keybindings::from_config(&self.0) {
            keybindings.merge(&user_bindings);
        }

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

    #[test]
    fn test_default_keybindings_exist() {
        let kb = Keybindings::default();

        // Check some basic normal mode bindings
        assert!(kb
            .get_action(KeybindingMode::Normal, KeyCode::Char('j'), KeyModifiers::NONE)
            .is_some());
        assert!(kb
            .get_action(KeybindingMode::Normal, KeyCode::Char('k'), KeyModifiers::NONE)
            .is_some());
        assert!(kb
            .get_action(KeybindingMode::Normal, KeyCode::Char('q'), KeyModifiers::NONE)
            .is_some());
    }

    #[test]
    fn test_get_action() {
        let mut kb = Keybindings::new();
        kb.set(
            KeybindingMode::Normal,
            KeyBinding::key(KeyCode::Char('j')),
            Action::Next,
        );

        assert_eq!(
            kb.get_action(KeybindingMode::Normal, KeyCode::Char('j'), KeyModifiers::NONE),
            Some(Action::Next)
        );
        assert_eq!(
            kb.get_action(KeybindingMode::Normal, KeyCode::Char('x'), KeyModifiers::NONE),
            None
        );
    }

    #[test]
    fn test_keys_for_action() {
        let mut kb = Keybindings::new();
        kb.set(
            KeybindingMode::Normal,
            KeyBinding::key(KeyCode::Char('j')),
            Action::Next,
        );
        kb.set(
            KeybindingMode::Normal,
            KeyBinding::key(KeyCode::Down),
            Action::Next,
        );

        let keys = kb.keys_for_action(KeybindingMode::Normal, Action::Next);
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_merge() {
        let mut kb1 = Keybindings::new();
        kb1.set(
            KeybindingMode::Normal,
            KeyBinding::key(KeyCode::Char('j')),
            Action::Next,
        );

        let mut kb2 = Keybindings::new();
        kb2.set(
            KeybindingMode::Normal,
            KeyBinding::key(KeyCode::Char('j')),
            Action::Previous, // Override
        );
        kb2.set(
            KeybindingMode::Normal,
            KeyBinding::key(KeyCode::Char('x')),
            Action::Quit,
        );

        kb1.merge(&kb2);

        // j should be overridden
        assert_eq!(
            kb1.get_action(KeybindingMode::Normal, KeyCode::Char('j'), KeyModifiers::NONE),
            Some(Action::Previous)
        );
        // x should be added
        assert_eq!(
            kb1.get_action(KeybindingMode::Normal, KeyCode::Char('x'), KeyModifiers::NONE),
            Some(Action::Quit)
        );
    }
}
