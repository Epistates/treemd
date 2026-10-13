//! Customizable keybindings for treemd
//!
//! This module provides a flexible keybinding system that allows users to
//! customize keyboard shortcuts via configuration files.
//!
//! # Architecture
//!
//! - [`Action`](crate::keybindings::Action) - All bindable actions in the application
//! - [`KeybindingMode`](crate::keybindings::KeybindingMode) - Different modes with their own keybinding sets
//! - [`Keybindings`](crate::keybindings::Keybindings) - The complete keybinding configuration (backed by keybinds-rs)
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
use std::{collections::HashMap, str::FromStr};

/// Application modes that have their own keybinding sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::EnumIter)]
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
    /// Document content search mode
    DocSearch,
    /// Command palette mode
    CommandPalette,
    /// Cell editing mode (for tables)
    CellEdit,
    /// Confirmation dialog
    ConfirmDialog,
    /// File picker for switching between files
    FilePicker,
    /// File picker search/filter mode
    FileSearch,
    /// The full-screen image viewer
    ImageViewer,
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
            KeybindingMode::DocSearch => "Doc Search",
            KeybindingMode::CommandPalette => "Command Palette",
            KeybindingMode::CellEdit => "Cell Edit",
            KeybindingMode::ConfirmDialog => "Confirm",
            KeybindingMode::FilePicker => "File Picker",
            KeybindingMode::FileSearch => "File Search",
            KeybindingMode::ImageViewer => "Image Viewer",
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
        Self {
            bindings: self.bindings.clone(),
        }
    }
}

impl Keybindings {
    /// Create empty keybindings
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Create keybindings from config.
    /// Keys that are not defined will be bound to their default actions.
    /// Keys that are bound to Noop will be left unbound.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the keys cannot be parsed.
    fn from_config(config: &KeybindingsConfig) -> Result<Self, keybinds::Error> {
        let mut def = Keybindings::default();

        for (mode, bindings) in &mut def.bindings {
            let mut binding_vec = std::mem::take(bindings).into_vec();

            if let Some(config_bindings) = config.0.get(mode) {
                for (config_key, config_action) in config_bindings {
                    // Checked when the config was read, so a key that does
                    // not parse here is skipped rather than discarding every
                    // other custom binding with it.
                    let Ok(config_seq) = keybinds::KeySeq::from_str(config_key) else {
                        continue;
                    };

                    if let Some(existing) = binding_vec.iter_mut().find(|b| b.seq == config_seq) {
                        existing.action = *config_action;
                    } else if *config_action != Action::Noop {
                        binding_vec.push(keybinds::Keybind::new(config_seq, *config_action));
                    }
                }

                binding_vec.retain(|b| b.action != Action::Noop);
            }

            *bindings = keybinds::Keybinds::new(binding_vec);
        }

        Ok(def)
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
            for bind in kb.as_slice().iter().filter(|b| b.action != Action::Noop) {
                let key_str = format_key_sequence(&bind.seq);
                action_keys.entry(bind.action).or_default().push(key_str);
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

/// Format a key for compact display in help text
pub fn format_key_compact(key: &str) -> String {
    key.to_string()
}

/// Configuration format for keybindings (uses string keys for TOML compatibility)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindingsConfig(pub HashMap<KeybindingMode, HashMap<String, Action>>);

impl KeybindingsConfig {
    /// Read a `[keybindings]` table entry by entry, keeping every valid
    /// binding and adding a warning for each mode, key or action that is
    /// not one. Deserialized whole, one bad entry failed the entire config.
    pub fn from_toml_lenient(value: toml::Value, warnings: &mut Vec<String>) -> Self {
        let mut config = HashMap::new();
        let toml::Value::Table(modes) = value else {
            warnings.push("[keybindings] is not a table; ignoring it".to_string());
            return Self(config);
        };
        for (mode_name, bindings) in modes {
            let Ok(mode) = toml::Value::String(mode_name.clone()).try_into::<KeybindingMode>()
            else {
                warnings.push(format!("unknown keybinding mode [keybindings.{mode_name}]"));
                continue;
            };
            let toml::Value::Table(bindings) = bindings else {
                warnings.push(format!("[keybindings.{mode_name}] is not a table"));
                continue;
            };
            let entries: &mut HashMap<String, Action> = config.entry(mode).or_default();
            for (key, action) in bindings {
                if keybinds::KeySeq::from_str(&key).is_err() {
                    warnings.push(format!("[keybindings.{mode_name}] \"{key}\" is not a key"));
                    continue;
                }
                match action.clone().try_into::<Action>() {
                    Ok(action) => {
                        entries.insert(key, action);
                    }
                    Err(_) => warnings.push(format!(
                        "[keybindings.{mode_name}] \"{key}\" = {action}: no such action"
                    )),
                }
            }
        }
        Self(config)
    }

    /// Convert to Keybindings, using defaults for any missing bindings
    pub fn to_keybindings(&self) -> Keybindings {
        // Falls back to pure defaults if any key is invalid
        Keybindings::from_config(self).unwrap_or_default()
    }

    /// Check if the config is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key bound twice in one mode does whichever binding the dispatcher
    /// happens to find first, and the other is silently dead.
    #[test]
    fn no_key_is_bound_twice_in_a_mode() {
        let kb = Keybindings::default();
        for (mode, binds) in &kb.bindings {
            let binds = binds.as_slice();
            for (i, a) in binds.iter().enumerate() {
                for b in &binds[i + 1..] {
                    assert!(
                        a.seq != b.seq,
                        "{mode:?}: {} is bound to both {:?} and {:?}",
                        format_key_sequence(&a.seq),
                        a.action,
                        b.action
                    );
                }
            }
        }
    }
    /// Raw mode makes Ctrl+C an ordinary key. Bound in Normal mode only, it
    /// did nothing anywhere else.
    #[test]
    fn ctrl_c_is_bound_in_every_mode() {
        use strum::IntoEnumIterator;

        let mut kb = Keybindings::default();
        for mode in KeybindingMode::iter() {
            let event = make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL);
            assert!(kb.dispatch(mode, event).is_some(), "{mode:?}");
        }
    }

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
        assert!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('j'), KeyModifiers::NONE)
            )
            .is_some()
        );
        assert!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('k'), KeyModifiers::NONE)
            )
            .is_some()
        );
        assert!(
            kb.dispatch(
                KeybindingMode::Normal,
                make_key_event(KeyCode::Char('q'), KeyModifiers::NONE)
            )
            .is_some()
        );
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
        assert!(!keys.is_empty()); // j and/or Down should be bound
    }

    #[test]
    fn test_user_config_overrides_defaults() {
        let mut config_map = HashMap::new();
        let mut normal_bindings = HashMap::new();
        // Rebind 'j' from Next (default) to Last
        normal_bindings.insert("j".to_string(), Action::Last);
        config_map.insert(KeybindingMode::Normal, normal_bindings);
        let config = KeybindingsConfig(config_map);

        let mut kb = config.to_keybindings();
        let action = kb.dispatch(
            KeybindingMode::Normal,
            make_key_event(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        assert_eq!(
            action,
            Some(Action::Last),
            "User binding must override default"
        );
    }

    #[test]
    fn test_noop_unbinds_key() {
        let mut config_map = HashMap::new();
        let mut normal_bindings = HashMap::new();
        // Unbind 'j' by mapping to Noop
        normal_bindings.insert("j".to_string(), Action::Noop);
        config_map.insert(KeybindingMode::Normal, normal_bindings);
        let config = KeybindingsConfig(config_map);

        let mut kb = config.to_keybindings();
        let action = kb.dispatch(
            KeybindingMode::Normal,
            make_key_event(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        assert_eq!(
            action, None,
            "Noop binding should effectively unbind the key"
        );
    }

    #[test]
    fn test_clone_preserves_user_config() {
        let mut config_map = HashMap::new();
        let mut normal_bindings = HashMap::new();
        normal_bindings.insert("j".to_string(), Action::Last);
        config_map.insert(KeybindingMode::Normal, normal_bindings);
        let config = KeybindingsConfig(config_map);

        let kb = config.to_keybindings();
        let mut cloned = kb.clone();
        let action = cloned.dispatch(
            KeybindingMode::Normal,
            make_key_event(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        assert_eq!(
            action,
            Some(Action::Last),
            "Clone must preserve user bindings"
        );
    }

    #[test]
    fn test_noop_filtered_from_help_entries() {
        let mut config_map = HashMap::new();
        let mut normal_bindings = HashMap::new();
        normal_bindings.insert("j".to_string(), Action::Noop);
        config_map.insert(KeybindingMode::Normal, normal_bindings);
        let config = KeybindingsConfig(config_map);

        let kb = config.to_keybindings();
        let entries = kb.help_entries(KeybindingMode::Normal);
        assert!(
            !entries.iter().any(|(action, _)| *action == Action::Noop),
            "Noop should not appear in help entries"
        );
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
            KeybindingMode::DocSearch,
            KeybindingMode::CommandPalette,
            KeybindingMode::ConfirmDialog,
            KeybindingMode::CellEdit,
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
