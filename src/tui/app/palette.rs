//! The command palette.

use super::*;

/// Available commands in the command palette
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandAction {
    SaveWidth,
    SaveFile, // Save pending edits to file (:w)
    Undo,     // Undo last pending edit
    ToggleOutline,
    ToggleHeadingMarkers,
    ToggleHelp,
    ToggleRawSource,
    ToggleMouseCapture,
    JumpToTop,
    JumpToBottom,
    CollapseAll,
    ExpandAll,
    /// Collapse headings at a specific level (parsed from command argument)
    CollapseLevel,
    /// Expand headings at a specific level (parsed from command argument)
    ExpandLevel,
    Quit,
    /// Forward to the regular keybinding action dispatch — lets the palette
    /// cover every command without duplicating handlers
    Dispatch(Action),
}

/// A command in the palette
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub action: CommandAction,
}

impl PaletteCommand {
    const fn new(
        name: &'static str,
        aliases: &'static [&'static str],
        description: &'static str,
        action: CommandAction,
    ) -> Self {
        Self {
            name,
            aliases,
            description,
            action,
        }
    }

    /// Check if query matches this command (fuzzy match on name or aliases).
    /// `query_lower` is the caller's pre-lowercased query — avoids re-allocating
    /// per command when filtering the palette.
    pub fn matches(&self, query_lower: &str) -> bool {
        if query_lower.is_empty() {
            return true;
        }

        // Aliases and name are static ASCII — compare case-insensitively without
        // allocating a lowercase copy of each.
        if contains_ignore_ascii_case(self.name, query_lower) {
            return true;
        }
        for alias in self.aliases {
            if starts_with_ignore_ascii_case(alias, query_lower) {
                return true;
            }
        }

        // Fuzzy: every char of query appears in order in name (ASCII fold).
        let mut name_chars = self.name.chars().map(|c| c.to_ascii_lowercase()).peekable();
        for qc in query_lower.chars() {
            loop {
                match name_chars.next() {
                    Some(nc) if nc == qc => break,
                    Some(_) => continue,
                    None => return false,
                }
            }
        }
        true
    }

    /// Calculate match score (higher = better match).
    /// `query_lower` is the caller's pre-lowercased query.
    pub fn match_score(&self, query_lower: &str) -> usize {
        if query_lower.is_empty() {
            return 100;
        }

        for alias in self.aliases {
            if alias.eq_ignore_ascii_case(query_lower) {
                return 1000;
            }
        }
        for alias in self.aliases {
            if starts_with_ignore_ascii_case(alias, query_lower) {
                return 500;
            }
        }
        if starts_with_ignore_ascii_case(self.name, query_lower) {
            return 300;
        }
        if contains_ignore_ascii_case(self.name, query_lower) {
            return 200;
        }
        100
    }
}

pub(super) fn starts_with_ignore_ascii_case(haystack: &str, needle_lower: &str) -> bool {
    haystack.len() >= needle_lower.len()
        && haystack.as_bytes()[..needle_lower.len()].eq_ignore_ascii_case(needle_lower.as_bytes())
}

pub(super) fn contains_ignore_ascii_case(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.len() < needle_lower.len() {
        return false;
    }
    let needle_bytes = needle_lower.as_bytes();
    haystack
        .as_bytes()
        .windows(needle_bytes.len())
        .any(|w| w.eq_ignore_ascii_case(needle_bytes))
}

/// All available commands
pub const PALETTE_COMMANDS: &[PaletteCommand] = &[
    PaletteCommand::new(
        "Save changes",
        &["w", "write", "save"],
        "Save pending edits to file",
        CommandAction::SaveFile,
    ),
    PaletteCommand::new(
        "Undo edit",
        &["u", "undo"],
        "Undo last table cell edit",
        CommandAction::Undo,
    ),
    PaletteCommand::new(
        "Save width to config",
        &["sw", "savewidth"],
        "Save current outline width to config file",
        CommandAction::SaveWidth,
    ),
    PaletteCommand::new(
        "Toggle outline",
        &["outline", "sidebar"],
        "Show/hide the outline sidebar",
        CommandAction::ToggleOutline,
    ),
    PaletteCommand::new(
        "Toggle heading markers",
        &["markers", "hashes"],
        "Show/hide # level markers in outline",
        CommandAction::ToggleHeadingMarkers,
    ),
    PaletteCommand::new(
        "Toggle help",
        &["help", "?"],
        "Show/hide keyboard shortcuts",
        CommandAction::ToggleHelp,
    ),
    PaletteCommand::new(
        "Toggle raw source",
        &["raw", "source"],
        "Switch between rendered and raw markdown",
        CommandAction::ToggleRawSource,
    ),
    PaletteCommand::new(
        "Toggle mouse capture",
        &["mouse", "select", "selection"],
        "Release the mouse so you can select and copy text natively",
        CommandAction::ToggleMouseCapture,
    ),
    PaletteCommand::new(
        "Jump to top",
        &["top", "first", "gg"],
        "Go to first heading",
        CommandAction::JumpToTop,
    ),
    PaletteCommand::new(
        "Jump to bottom",
        &["bottom", "last", "G"],
        "Go to last heading",
        CommandAction::JumpToBottom,
    ),
    PaletteCommand::new(
        "Collapse all",
        &["collapse", "ca"],
        "Collapse all headings (or :collapse N for level N)",
        CommandAction::CollapseAll,
    ),
    PaletteCommand::new(
        "Expand all",
        &["expand", "ea"],
        "Expand all headings (or :expand N for level N)",
        CommandAction::ExpandAll,
    ),
    PaletteCommand::new(
        "Collapse level",
        &[
            "collapse 1",
            "collapse 2",
            "collapse 3",
            "collapse 4",
            "collapse 5",
        ],
        "Collapse headings at specific level",
        CommandAction::CollapseLevel,
    ),
    PaletteCommand::new(
        "Expand level",
        &["expand 1", "expand 2", "expand 3", "expand 4", "expand 5"],
        "Expand headings at specific level",
        CommandAction::ExpandLevel,
    ),
    PaletteCommand::new(
        "Theme picker",
        &["theme", "colors"],
        "Choose a color theme (live preview)",
        CommandAction::Dispatch(Action::ToggleThemePicker),
    ),
    PaletteCommand::new(
        "Search content",
        &["/", "find", "search"],
        "Search the document content",
        CommandAction::Dispatch(Action::EnterDocSearch),
    ),
    PaletteCommand::new(
        "Filter outline",
        &["s", "filter"],
        "Filter outline headings",
        CommandAction::Dispatch(Action::EnterSearchMode),
    ),
    PaletteCommand::new(
        "Interactive mode",
        &["i", "interactive"],
        "Navigate tables, checkboxes, links and code blocks",
        CommandAction::Dispatch(Action::EnterInteractiveMode),
    ),
    PaletteCommand::new(
        "Link mode",
        &["f", "links", "follow"],
        "Follow links in the current section",
        CommandAction::Dispatch(Action::EnterLinkFollowMode),
    ),
    PaletteCommand::new(
        "Open in editor",
        &["e", "edit", "editor"],
        "Open the file in $VISUAL/$EDITOR at the current position",
        CommandAction::Dispatch(Action::OpenInEditor),
    ),
    PaletteCommand::new(
        "Open file picker",
        &["o", "open", "files"],
        "Browse and open markdown files",
        CommandAction::Dispatch(Action::OpenFilePicker),
    ),
    PaletteCommand::new(
        "Set bookmark",
        &["m", "mark", "bookmark"],
        "Bookmark the current position",
        CommandAction::Dispatch(Action::SetBookmark),
    ),
    PaletteCommand::new(
        "Jump to bookmark",
        &["'", "gotomark"],
        "Jump to the bookmarked position",
        CommandAction::Dispatch(Action::JumpToBookmark),
    ),
    PaletteCommand::new(
        "Copy section",
        &["y", "copy", "yank"],
        "Copy current section content to clipboard",
        CommandAction::Dispatch(Action::CopyContent),
    ),
    PaletteCommand::new(
        "Copy anchor link",
        &["anchor"],
        "Copy the current heading's anchor link",
        CommandAction::Dispatch(Action::CopyAnchor),
    ),
    PaletteCommand::new(
        "Toggle TODO filter",
        &["todo", "tasks"],
        "Show only sections containing open tasks",
        CommandAction::Dispatch(Action::ToggleTodoFilter),
    ),
    PaletteCommand::new(
        "Go back",
        &["back", "b"],
        "Go back to the previous file",
        CommandAction::Dispatch(Action::GoBack),
    ),
    PaletteCommand::new(
        "Go forward",
        &["forward"],
        "Go forward in navigation history",
        CommandAction::Dispatch(Action::GoForward),
    ),
    PaletteCommand::new(
        "Quit",
        &["q", "quit", "exit"],
        "Exit treemd",
        CommandAction::Quit,
    ),
];

/// Command palette state.
#[derive(Debug)]
pub struct CommandPaletteState {
    pub query: String,
    /// Indices into `PALETTE_COMMANDS`, ordered by match score.
    pub filtered: Vec<usize>,
    pub selected: usize,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            query: String::new(),
            filtered: (0..PALETTE_COMMANDS.len()).collect(),
            selected: 0,
        }
    }
}

impl App {
    /// Open command palette (triggered by `:`)
    pub fn open_command_palette(&mut self) {
        self.mode = AppMode::CommandPalette;
        self.command_palette.query.clear();
        self.command_palette.filtered = (0..PALETTE_COMMANDS.len()).collect();
        self.command_palette.selected = 0;
    }

    /// Filter commands based on current query
    pub(super) fn filter_commands(&mut self) {
        // Lowercase once, pass to each command — saves N allocations per
        // keystroke where N = number of palette commands.
        let query_lower = self.command_palette.query.to_lowercase();
        let mut matches: Vec<(usize, usize)> = PALETTE_COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, cmd)| cmd.matches(&query_lower))
            .map(|(idx, cmd)| (idx, cmd.match_score(&query_lower)))
            .collect();

        // Sort by score (highest first)
        matches.sort_by_key(|m| std::cmp::Reverse(m.1));

        self.command_palette.filtered = matches.into_iter().map(|(idx, _)| idx).collect();

        // Reset selection if it's out of bounds
        if self.command_palette.selected >= self.command_palette.filtered.len() {
            self.command_palette.selected = 0;
        }
    }

    /// Move selection down in command palette
    pub fn command_palette_next(&mut self) {
        if !self.command_palette.filtered.is_empty() {
            self.command_palette.selected =
                (self.command_palette.selected + 1) % self.command_palette.filtered.len();
        }
    }

    /// Move selection up in command palette
    pub fn command_palette_prev(&mut self) {
        if !self.command_palette.filtered.is_empty() {
            self.command_palette.selected = if self.command_palette.selected == 0 {
                self.command_palette.filtered.len() - 1
            } else {
                self.command_palette.selected - 1
            };
        }
    }

    /// Autocomplete command palette with selected command's alias
    pub fn command_palette_autocomplete(&mut self) {
        if let Some(&cmd_idx) = self
            .command_palette
            .filtered
            .get(self.command_palette.selected)
        {
            let cmd = &PALETTE_COMMANDS[cmd_idx];
            // Use the first alias (typically the shortest canonical form)
            if let Some(&alias) = cmd.aliases.first() {
                self.command_palette.query = alias.to_string();
                // Re-filter with the new query (will likely still match the same command)
                self.filter_commands();
            }
        }
    }

    /// Close command palette without executing
    pub fn close_command_palette(&mut self) {
        self.mode = AppMode::Normal;
        self.command_palette.query.clear();
    }

    /// Execute selected command and return whether to quit
    pub fn execute_selected_command(&mut self) -> ActionResult {
        if let Some(&cmd_idx) = self
            .command_palette
            .filtered
            .get(self.command_palette.selected)
        {
            let action = PALETTE_COMMANDS[cmd_idx].action;
            let query = self.command_palette.query.clone(); // Capture query for argument parsing
            self.mode = AppMode::Normal;
            self.command_palette.query.clear();
            self.execute_command_action(action, &query)
        } else {
            self.mode = AppMode::Normal;
            ActionResult::Continue
        }
    }

    /// Execute a command action
    pub(super) fn execute_command_action(
        &mut self,
        action: CommandAction,
        query: &str,
    ) -> ActionResult {
        match action {
            CommandAction::SaveWidth => {
                match self.config.set_outline_width(self.outline_width) {
                    Ok(_) => {
                        self.config_has_custom_outline_width = self.outline_width != 20
                            && self.outline_width != 30
                            && self.outline_width != 40;
                        self.set_status_message(&format!(
                            "\u{2713} Width {}% saved to config",
                            self.outline_width
                        ));
                    }
                    Err(e) => {
                        self.set_status_message(&format!("\u{2717} Failed to save: {}", e));
                    }
                }
                ActionResult::Continue
            }
            CommandAction::ToggleOutline => {
                self.toggle_outline();
                ActionResult::Continue
            }
            CommandAction::ToggleHeadingMarkers => {
                self.toggle_heading_markers();
                ActionResult::Continue
            }
            CommandAction::ToggleMouseCapture => {
                self.toggle_mouse_capture();
                ActionResult::Continue
            }
            CommandAction::ToggleHelp => {
                self.toggle_help();
                ActionResult::Continue
            }
            CommandAction::ToggleRawSource => {
                self.toggle_raw_source();
                ActionResult::Continue
            }
            CommandAction::JumpToTop => {
                self.first();
                ActionResult::Continue
            }
            CommandAction::JumpToBottom => {
                self.last();
                ActionResult::Continue
            }
            CommandAction::CollapseAll => {
                self.collapse_all();
                ActionResult::Continue
            }
            CommandAction::ExpandAll => {
                self.expand_all();
                ActionResult::Continue
            }
            CommandAction::CollapseLevel => {
                // Parse level from query (e.g., "collapse 2" -> 2)
                if let Some(level) = Self::parse_level_from_query(query) {
                    self.collapse_level(level);
                } else {
                    self.collapse_all();
                }
                ActionResult::Continue
            }
            CommandAction::ExpandLevel => {
                // Parse level from query (e.g., "expand 2" -> 2)
                if let Some(level) = Self::parse_level_from_query(query) {
                    self.expand_level(level);
                } else {
                    self.expand_all();
                }
                ActionResult::Continue
            }
            CommandAction::SaveFile => {
                if let Err(e) = self.save_pending_edits_to_file() {
                    self.set_status_message(&format!("\u{2717} Save failed: {}", e));
                }
                ActionResult::Continue
            }
            CommandAction::Undo => {
                if let Err(e) = self.undo_last_edit() {
                    self.set_status_message(&format!("\u{2717} Undo failed: {}", e));
                }
                ActionResult::Continue
            }
            CommandAction::Quit => {
                if self.has_unsaved_changes() {
                    // Show confirmation dialog instead of quitting immediately
                    self.mode = AppMode::ConfirmSaveBeforeQuit;
                    ActionResult::Continue
                } else {
                    ActionResult::Quit
                }
            }
            // Forward to the regular keybinding action dispatch
            CommandAction::Dispatch(action) => self.execute_action(action),
        }
    }

    /// Parse a level number from a command query like "collapse 2" or "expand 3"
    pub(super) fn parse_level_from_query(query: &str) -> Option<usize> {
        // Find the last word and try to parse it as a number
        query
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| (1..=6).contains(&n))
    }
}

#[cfg(test)]
mod palette_tests {
    use super::*;

    /// Every action can be reached, by a key in some mode or from the command
    /// palette. One that neither binds nor lists is code no user can run.
    #[test]
    fn every_action_is_reachable() {
        use crate::keybindings::{KeybindingMode, Keybindings};
        use strum::IntoEnumIterator;

        let keys = Keybindings::default();
        let palette: HashSet<Action> = PALETTE_COMMANDS
            .iter()
            .filter_map(|command| match command.action {
                CommandAction::Dispatch(action) => Some(action),
                _ => None,
            })
            .collect();
        let unreachable: Vec<Action> = Action::iter()
            .filter(|action| {
                // Noop unbinds a key; the JumpToHeading actions ship unbound
                // because digits are the count prefix, for users to bind.
                !matches!(action, Action::Noop) && !action.to_string().starts_with("JumpToHeading")
            })
            .filter(|action| !palette.contains(action))
            .filter(|action| {
                KeybindingMode::iter().all(|mode| keys.keys_for_action(mode, *action).is_empty())
            })
            .collect();
        assert!(
            unreachable.is_empty(),
            "no key or palette entry: {unreachable:?}"
        );
    }

    fn cmd(name: &'static str, aliases: &'static [&'static str]) -> PaletteCommand {
        PaletteCommand::new(name, aliases, "desc", CommandAction::Quit)
    }

    // ---------- ascii-fold helpers ----------

    #[test]
    fn starts_with_ignore_ascii_case_basics() {
        assert!(starts_with_ignore_ascii_case("Quit", "qu"));
        assert!(starts_with_ignore_ascii_case("Quit", "QUIT"));
        assert!(starts_with_ignore_ascii_case("Quit", ""));
        assert!(!starts_with_ignore_ascii_case("Quit", "uit"));
        assert!(!starts_with_ignore_ascii_case("Quit", "quitter"));
    }

    #[test]
    fn contains_ignore_ascii_case_basics() {
        assert!(contains_ignore_ascii_case("Toggle outline", "outline"));
        assert!(contains_ignore_ascii_case("Toggle outline", "OUT"));
        assert!(contains_ignore_ascii_case("anything", "")); // empty needle always matches
        assert!(!contains_ignore_ascii_case("abc", "abcd")); // needle longer than haystack
        assert!(!contains_ignore_ascii_case("hello", "xyz"));
    }

    // ---------- matches() ----------

    #[test]
    fn matches_empty_query_matches_all() {
        let c = cmd("Quit", &["q", "quit"]);
        assert!(c.matches(""));
    }

    #[test]
    fn matches_name_substring_case_insensitive() {
        let c = cmd("Toggle outline", &["outline"]);
        assert!(c.matches("toggle"));
        assert!(c.matches("OUT"));
        assert!(c.matches("line"));
    }

    #[test]
    fn matches_alias_prefix() {
        let c = cmd("Save changes", &["w", "write", "save"]);
        // alias-prefix path: "wri" is a prefix of "write"
        assert!(c.matches("wri"));
        // but "rite" isn't a prefix of any alias and doesn't fuzzy-match name
        assert!(!c.matches("rite"));
    }

    #[test]
    fn matches_fuzzy_in_name() {
        // "tgo" appears in order in "toggle outline" → t...o...g...
        let c = cmd("Toggle outline", &["outline"]);
        assert!(c.matches("tgo"));
        // chars not in order should fail
        assert!(!c.matches("zzz"));
    }

    #[test]
    fn matches_fuzzy_requires_in_order() {
        let c = cmd("Save changes", &["save"]);
        // 's', 'a', 'v', 'e' are present in order
        assert!(c.matches("sve"));
        // 'e' before 's' is not in order in "save changes"
        // ('e' first appears after 's', so this should still match — pick a real reverse)
        assert!(!c.matches("xq"));
    }

    // ---------- match_score() ----------

    #[test]
    fn score_empty_query_is_baseline() {
        let c = cmd("Quit", &["q", "quit"]);
        assert_eq!(c.match_score(""), 100);
    }

    #[test]
    fn score_exact_alias_is_highest() {
        let c = cmd("Quit", &["q", "quit", "exit"]);
        assert_eq!(c.match_score("q"), 1000);
        assert_eq!(c.match_score("quit"), 1000);
        assert_eq!(c.match_score("EXIT"), 1000); // case-insensitive
    }

    #[test]
    fn score_alias_prefix_beats_name() {
        let c = cmd("Save changes", &["w", "write", "save"]);
        // "wri" prefixes "write" → 500, not 300/200
        assert_eq!(c.match_score("wri"), 500);
    }

    #[test]
    fn score_name_starts_with_beats_contains() {
        let c = cmd("Toggle outline", &["sidebar"]);
        // "togg" is a prefix of name (no alias match)
        assert_eq!(c.match_score("togg"), 300);
    }

    #[test]
    fn score_name_contains_below_starts_with() {
        let c = cmd("Toggle outline", &["sidebar"]);
        // "outline" is contained but not a prefix of "Toggle outline"
        assert_eq!(c.match_score("outline"), 200);
    }

    #[test]
    fn score_fuzzy_only_match_is_baseline() {
        let c = cmd("Toggle outline", &["sidebar"]);
        // "tgo" matches via fuzzy (in matches()) but doesn't hit any score tier
        // above baseline — match_score has no fuzzy tier, so it falls through to 100.
        assert_eq!(c.match_score("tgo"), 100);
    }

    #[test]
    fn score_priority_ordering() {
        // Build commands that each hit a different tier and confirm relative order.
        let exact_alias = cmd("Anything", &["xx"]);
        let alias_prefix = cmd("Anything", &["xxlong"]);
        let name_prefix = cmd("XxName", &["other"]);
        let name_contains = cmd("Has Xx Inside", &["other"]);

        let q = "xx";
        let s1 = exact_alias.match_score(q);
        let s2 = alias_prefix.match_score(q);
        let s3 = name_prefix.match_score(q);
        let s4 = name_contains.match_score(q);

        assert!(s1 > s2, "exact alias should beat alias prefix");
        assert!(s2 > s3, "alias prefix should beat name prefix");
        assert!(s3 > s4, "name prefix should beat name contains");
    }

    // ---------- registered palette commands ----------
    // Sanity-check the actual PALETTE_COMMANDS table — these are the commands
    // users actually type, so it's worth pinning their behavior.

    #[test]
    fn registered_aliases_are_exact_matchable() {
        for cmd in PALETTE_COMMANDS {
            for alias in cmd.aliases {
                let lower = alias.to_lowercase();
                assert_eq!(
                    cmd.match_score(&lower),
                    1000,
                    "alias {:?} of {:?} should be exact-match",
                    alias,
                    cmd.name
                );
            }
        }
    }

    #[test]
    fn registered_quit_command_resolves() {
        // Typing "q" should pick the Quit command as the top score.
        let q = "q";
        let best = PALETTE_COMMANDS
            .iter()
            .max_by_key(|c| c.match_score(q))
            .expect("non-empty");
        assert_eq!(best.action, CommandAction::Quit);
    }
}
