use crate::config::Config;
use crate::keybindings::{Action, Keybindings, KeybindingMode};
use crate::parser::{Document, HeadingNode, Link, extract_links};
use crate::tui::help_text;
use crate::tui::interactive::InteractiveState;
use crate::tui::syntax::SyntaxHighlighter;
use crate::tui::terminal_compat::ColorMode;
use crate::tui::theme::{Theme, ThemeName};
use crossterm::event::KeyModifiers;
use ratatui::widgets::{ListState, ScrollbarState};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Special marker for the document overview entry (shows entire file content)
pub const DOCUMENT_OVERVIEW: &str = "(Document)";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Outline,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Interactive,
    LinkFollow,
    Search,
    ThemePicker,
    Help,
    CellEdit,
    ConfirmFileCreate,
}

pub struct App {
    pub document: Document,
    pub filename: String,
    pub tree: Vec<HeadingNode>,
    pub outline_state: ListState,
    pub outline_scroll_state: ScrollbarState,
    pub focus: Focus,
    pub outline_items: Vec<OutlineItem>,
    pub content_scroll: u16,
    pub content_scroll_state: ScrollbarState,
    pub content_height: u16,
    pub show_help: bool,
    pub help_scroll: u16,
    pub show_search: bool,
    pub search_query: String,
    pub highlighter: SyntaxHighlighter,
    pub show_outline: bool,
    pub outline_width: u16,                // Percentage: 20, 30, or 40
    pub bookmark_position: Option<String>, // Bookmarked heading text (was: outline position)
    collapsed_headings: HashSet<String>,   // Track which headings are collapsed by text
    pub current_theme: ThemeName,
    pub theme: Theme,
    pub show_theme_picker: bool,
    pub theme_picker_selected: usize,
    previous_selection: Option<String>, // Track previous selection to detect changes

    // Link following state
    pub mode: AppMode,
    pub current_file_path: PathBuf, // Path to current file for resolving relative links
    pub links_in_view: Vec<Link>,   // Links in currently displayed content
    pub filtered_link_indices: Vec<usize>, // Indices into links_in_view after filtering
    pub selected_link_idx: Option<usize>, // Currently selected index in filtered list
    pub link_search_query: String,  // Search query for filtering links
    pub link_search_active: bool,   // Whether search input is active
    pub file_history: Vec<FileState>, // Back navigation stack
    pub file_future: Vec<FileState>, // Forward navigation stack (for undo back)
    pub status_message: Option<String>, // Temporary status message to display
    pub status_message_time: Option<Instant>, // When the status message was set

    // Interactive element navigation
    pub interactive_state: InteractiveState,

    // Cell editing state
    pub cell_edit_value: String, // Current value being edited
    pub cell_edit_row: usize,    // Row being edited
    pub cell_edit_col: usize,    // Column being edited

    // Persistent clipboard for Linux X11 compatibility
    // On Linux, the clipboard instance must stay alive to serve paste requests
    clipboard: Option<arboard::Clipboard>,

    // Configuration persistence
    config: Config,
    color_mode: ColorMode,

    // Keybindings
    pub keybindings: Keybindings,

    // Pending file to open in external editor (set by link following, consumed by main loop)
    pub pending_editor_file: Option<PathBuf>,

    // Raw source view toggle
    pub show_raw_source: bool,

    // Pending file creation (for confirm dialog)
    pub pending_file_create: Option<PathBuf>,
    pub pending_file_create_message: Option<String>,
}

/// Saved state for file navigation history
#[derive(Debug, Clone)]
pub struct FileState {
    pub path: PathBuf,
    pub document: Document,
    pub filename: String,
    pub selected_heading: Option<String>,
    pub content_scroll: u16,
    pub outline_state_selected: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct OutlineItem {
    pub level: usize,
    pub text: String,
    pub expanded: bool,
    pub has_children: bool, // Track if this heading has children in the tree
}

impl App {
    pub fn new(
        document: Document,
        filename: String,
        file_path: PathBuf,
        config: Config,
        color_mode: ColorMode,
    ) -> Self {
        let tree = document.build_tree();
        let collapsed_headings = HashSet::new();
        let mut outline_items = Self::flatten_tree(&tree, &collapsed_headings);

        // Add document overview entry if there's preamble content or no headings
        let has_preamble = Self::has_preamble_content(&document);
        if has_preamble || document.headings.is_empty() {
            outline_items.insert(
                0,
                OutlineItem {
                    level: 0, // Level 0 for document overview (renders without # prefix)
                    text: DOCUMENT_OVERVIEW.to_string(),
                    expanded: true,
                    has_children: !outline_items.is_empty(),
                },
            );
        }

        let mut outline_state = ListState::default();
        if !outline_items.is_empty() {
            outline_state.select(Some(0));
        }

        let content_lines = document.content.lines().count();

        // Load theme from config, apply color mode, then apply custom colors
        let current_theme = config.theme_name();
        let theme = Theme::from_name(current_theme)
            .with_color_mode(color_mode, current_theme)
            .with_custom_colors(&config.theme, color_mode);

        // Load outline width from config
        let outline_width = config.ui.outline_width;

        // Load keybindings from config
        let keybindings = config.keybindings();

        Self {
            document,
            filename,
            tree,
            outline_state,
            outline_scroll_state: ScrollbarState::new(outline_items.len()),
            focus: Focus::Outline,
            outline_items,
            content_scroll: 0,
            content_scroll_state: ScrollbarState::new(content_lines),
            content_height: content_lines as u16,
            show_help: false,
            help_scroll: 0,
            show_search: false,
            search_query: String::new(),
            highlighter: SyntaxHighlighter::new(),
            show_outline: true,
            outline_width,
            bookmark_position: None,
            collapsed_headings,
            current_theme,
            theme,
            show_theme_picker: false,
            theme_picker_selected: 0,
            previous_selection: None,

            // Link following state
            mode: AppMode::Normal,
            current_file_path: file_path,
            links_in_view: Vec::new(),
            filtered_link_indices: Vec::new(),
            selected_link_idx: None,
            link_search_query: String::new(),
            link_search_active: false,
            file_history: Vec::new(),
            file_future: Vec::new(),
            status_message: None,
            status_message_time: None,

            // Interactive element navigation
            interactive_state: InteractiveState::new(),

            // Cell editing state
            cell_edit_value: String::new(),
            cell_edit_row: 0,
            cell_edit_col: 0,

            // Initialize persistent clipboard (None if unavailable)
            clipboard: arboard::Clipboard::new().ok(),

            // Configuration persistence
            config,
            color_mode,

            // Keybindings
            keybindings,

            // Pending editor file
            pending_editor_file: None,

            // Raw source view (off by default)
            show_raw_source: false,

            // Pending file creation (for confirm dialog)
            pending_file_create: None,
            pending_file_create_message: None,
        }
    }

    /// Get the current keybinding mode based on app state
    pub fn current_keybinding_mode(&self) -> KeybindingMode {
        // Help takes highest precedence
        if self.show_help {
            return KeybindingMode::Help;
        }

        // Theme picker
        if self.show_theme_picker {
            return KeybindingMode::ThemePicker;
        }

        // Mode-specific
        match self.mode {
            AppMode::Normal => {
                if self.show_search {
                    KeybindingMode::Search
                } else {
                    KeybindingMode::Normal
                }
            }
            AppMode::Interactive => {
                if self.interactive_state.is_in_table_mode() {
                    KeybindingMode::InteractiveTable
                } else {
                    KeybindingMode::Interactive
                }
            }
            AppMode::LinkFollow => {
                if self.link_search_active {
                    KeybindingMode::LinkSearch
                } else {
                    KeybindingMode::LinkFollow
                }
            }
            AppMode::Search => KeybindingMode::Search,
            AppMode::ThemePicker => KeybindingMode::ThemePicker,
            AppMode::Help => KeybindingMode::Help,
            AppMode::CellEdit => KeybindingMode::CellEdit,
            AppMode::ConfirmFileCreate => KeybindingMode::ConfirmDialog,
        }
    }

    /// Look up the action for a key event in the current mode
    pub fn get_action_for_key(
        &self,
        code: crossterm::event::KeyCode,
        modifiers: KeyModifiers,
    ) -> Option<Action> {
        let mode = self.current_keybinding_mode();
        self.keybindings.get_action(mode, code, modifiers)
    }

    /// Toggle between raw source view and rendered markdown view
    pub fn toggle_raw_source(&mut self) {
        self.show_raw_source = !self.show_raw_source;
        let msg = if self.show_raw_source {
            "Raw source view enabled"
        } else {
            "Rendered view enabled"
        };
        self.set_status_message(msg);
    }

    /// Set a status message with automatic timeout tracking
    pub fn set_status_message(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_message_time = Some(Instant::now());
    }

    /// Clear status message if it has expired (default 1 second timeout)
    pub fn clear_expired_status_message(&mut self) {
        const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(1);

        if let Some(time) = self.status_message_time {
            if time.elapsed() >= STATUS_MESSAGE_TIMEOUT {
                self.status_message = None;
                self.status_message_time = None;
            }
        }
    }

    /// Check if the document has non-whitespace content before the first heading
    fn has_preamble_content(document: &Document) -> bool {
        if document.headings.is_empty() {
            // No headings at all - entire document is preamble
            return !document.content.trim().is_empty();
        }

        // Check if there's content before the first heading
        let first_heading_offset = document.headings[0].offset;
        if first_heading_offset == 0 {
            return false;
        }

        // Check if there's non-whitespace content before the first heading
        let preamble = &document.content[..first_heading_offset];
        !preamble.trim().is_empty()
    }

    /// Rebuild outline items from the tree, optionally adding document overview
    fn rebuild_outline_items(&mut self) {
        self.outline_items = Self::flatten_tree(&self.tree, &self.collapsed_headings);

        // Add document overview entry if there's preamble content or no headings
        let has_preamble = Self::has_preamble_content(&self.document);
        if has_preamble || self.document.headings.is_empty() {
            self.outline_items.insert(
                0,
                OutlineItem {
                    level: 0,
                    text: DOCUMENT_OVERVIEW.to_string(),
                    expanded: true,
                    has_children: !self.outline_items.is_empty(), // Has children if there are other items
                },
            );
        }
    }

    fn flatten_tree(
        tree: &[HeadingNode],
        collapsed_headings: &HashSet<String>,
    ) -> Vec<OutlineItem> {
        let mut items = Vec::new();

        fn flatten_recursive(
            node: &HeadingNode,
            items: &mut Vec<OutlineItem>,
            collapsed_headings: &HashSet<String>,
        ) {
            let is_collapsed = collapsed_headings.contains(&node.heading.text);
            let expanded = !is_collapsed;
            let has_children = !node.children.is_empty();

            items.push(OutlineItem {
                level: node.heading.level,
                text: node.heading.text.clone(),
                expanded,
                has_children,
            });

            // Only show children if this node is expanded
            if expanded {
                for child in &node.children {
                    flatten_recursive(child, items, collapsed_headings);
                }
            }
        }

        for node in tree {
            flatten_recursive(node, &mut items, collapsed_headings);
        }

        items
    }

    /// Select a heading by its text. Returns true if found and selected.
    fn select_by_text(&mut self, text: &str) -> bool {
        for (idx, item) in self.outline_items.iter().enumerate() {
            if item.text == text {
                self.outline_state.select(Some(idx));
                self.outline_scroll_state = self.outline_scroll_state.position(idx);
                return true;
            }
        }
        false
    }

    /// Update content height based on current selection and reset scroll if selection changed
    pub fn update_content_metrics(&mut self) {
        let current_selection = self.selected_heading_text().map(|s| s.to_string());

        // Check if selection changed
        if current_selection != self.previous_selection {
            // Reset content scroll when selection changes
            self.content_scroll = 0;
            self.previous_selection = current_selection.clone();
        }

        // Update content height based on current section
        let content_text = if let Some(heading_text) = &current_selection {
            if let Some(_heading) = self.document.find_heading(heading_text) {
                // Use extract_section_content to get the actual displayed content
                self.document
                    .extract_section(heading_text)
                    .unwrap_or_else(|| self.document.content.clone())
            } else {
                self.document.content.clone()
            }
        } else {
            self.document.content.clone()
        };

        let content_lines = content_text.lines().count();
        self.content_height = content_lines as u16;
        self.content_scroll_state =
            ScrollbarState::new(content_lines).position(self.content_scroll as usize);
    }

    pub fn next(&mut self) {
        if self.focus == Focus::Outline {
            let i = match self.outline_state.selected() {
                Some(i) => {
                    if i >= self.outline_items.len().saturating_sub(1) {
                        i
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.outline_state.select(Some(i));
            self.outline_scroll_state = self.outline_scroll_state.position(i);
        } else {
            // Scroll content
            let new_scroll = self.content_scroll.saturating_add(1);
            if new_scroll < self.content_height {
                self.content_scroll = new_scroll;
                self.content_scroll_state = self.content_scroll_state.position(new_scroll as usize);
            }
        }
    }

    pub fn previous(&mut self) {
        if self.focus == Focus::Outline {
            let i = match self.outline_state.selected() {
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            self.outline_state.select(Some(i));
            self.outline_scroll_state = self.outline_scroll_state.position(i);
        } else {
            // Scroll content
            self.content_scroll = self.content_scroll.saturating_sub(1);
            self.content_scroll_state = self
                .content_scroll_state
                .position(self.content_scroll as usize);
        }
    }

    pub fn first(&mut self) {
        if self.focus == Focus::Outline && !self.outline_items.is_empty() {
            self.outline_state.select(Some(0));
            self.outline_scroll_state = self.outline_scroll_state.position(0);
        } else {
            self.content_scroll = 0;
            self.content_scroll_state = self.content_scroll_state.position(0);
        }
    }

    pub fn last(&mut self) {
        if self.focus == Focus::Outline && !self.outline_items.is_empty() {
            let last = self.outline_items.len() - 1;
            self.outline_state.select(Some(last));
            self.outline_scroll_state = self.outline_scroll_state.position(last);
        } else {
            let last = self.content_height.saturating_sub(1);
            self.content_scroll = last;
            self.content_scroll_state = self.content_scroll_state.position(last as usize);
        }
    }

    pub fn jump_to_parent(&mut self) {
        if self.focus == Focus::Outline {
            if let Some(current_idx) = self.outline_state.selected() {
                if current_idx < self.outline_items.len() {
                    let current_level = self.outline_items[current_idx].level;

                    // Search backwards for a heading with lower level (parent)
                    for i in (0..current_idx).rev() {
                        if self.outline_items[i].level < current_level {
                            self.outline_state.select(Some(i));
                            self.outline_scroll_state = self.outline_scroll_state.position(i);
                            return;
                        }
                    }

                    // If no parent found, stay at current position
                    // (we're already at a top-level heading or first item)
                }
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.help_scroll = 0; // Reset scroll when opening help
        }
    }

    pub fn scroll_help_down(&mut self) {
        let new_scroll = self.help_scroll.saturating_add(1);
        // Compute help text length dynamically from keybindings
        let help_lines = help_text::build_dynamic_help_text(&self.theme, &self.keybindings);
        let max_scroll = help_lines.len() as u16;
        if new_scroll < max_scroll {
            self.help_scroll = new_scroll;
        }
    }

    pub fn scroll_help_up(&mut self) {
        self.help_scroll = self.help_scroll.saturating_sub(1);
    }

    pub fn toggle_search(&mut self) {
        self.show_search = !self.show_search;
        if !self.show_search {
            self.search_query.clear();
        }
    }

    /// Maximum search query length to prevent performance issues
    const MAX_SEARCH_LEN: usize = 256;

    pub fn search_input(&mut self, c: char) {
        // Limit search query length
        if self.search_query.len() >= Self::MAX_SEARCH_LEN {
            return;
        }

        // Filter control characters (except common ones)
        if c.is_control() && c != '\t' {
            return;
        }

        self.search_query.push(c);
        self.filter_outline();
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.filter_outline();
    }

    fn filter_outline(&mut self) {
        // Save current selection text
        let current_selection = self.selected_heading_text().map(|s| s.to_string());

        if self.search_query.is_empty() {
            // Reset to full tree with overview entry
            self.rebuild_outline_items();
        } else {
            // Filter by search query, but always include overview entry if applicable
            let query_lower = self.search_query.to_lowercase();
            let has_preamble = Self::has_preamble_content(&self.document);

            self.outline_items = Self::flatten_tree(&self.tree, &self.collapsed_headings)
                .into_iter()
                .filter(|item| item.text.to_lowercase().contains(&query_lower))
                .collect();

            // Add overview entry if it matches the search or if document has preamble
            if (has_preamble || self.document.headings.is_empty())
                && DOCUMENT_OVERVIEW.to_lowercase().contains(&query_lower)
            {
                self.outline_items.insert(
                    0,
                    OutlineItem {
                        level: 0,
                        text: DOCUMENT_OVERVIEW.to_string(),
                        expanded: true,
                        has_children: !self.tree.is_empty(),
                    },
                );
            }
        }

        // Try to restore previous selection, otherwise select first item
        if !self.outline_items.is_empty() {
            let restored = if let Some(text) = current_selection {
                self.select_by_text(&text)
            } else {
                false
            };

            if !restored {
                self.outline_state.select(Some(0));
                self.outline_scroll_state =
                    ScrollbarState::new(self.outline_items.len()).position(0);
            }
        }
    }

    pub fn scroll_page_down(&mut self) {
        if self.focus == Focus::Content {
            let new_scroll = self.content_scroll.saturating_add(10);
            self.content_scroll = new_scroll.min(self.content_height.saturating_sub(1));
            self.content_scroll_state = self
                .content_scroll_state
                .position(self.content_scroll as usize);
        }
    }

    pub fn scroll_page_up(&mut self) {
        if self.focus == Focus::Content {
            self.content_scroll = self.content_scroll.saturating_sub(10);
            self.content_scroll_state = self
                .content_scroll_state
                .position(self.content_scroll as usize);
        }
    }

    /// Scroll page down in interactive mode (bypasses focus check)
    pub fn scroll_page_down_interactive(&mut self) {
        let new_scroll = self.content_scroll.saturating_add(10);
        self.content_scroll = new_scroll.min(self.content_height.saturating_sub(1));
        self.content_scroll_state = self
            .content_scroll_state
            .position(self.content_scroll as usize);
    }

    /// Scroll page up in interactive mode (bypasses focus check)
    pub fn scroll_page_up_interactive(&mut self) {
        self.content_scroll = self.content_scroll.saturating_sub(10);
        self.content_scroll_state = self
            .content_scroll_state
            .position(self.content_scroll as usize);
    }

    /// Auto-scroll to keep the selected interactive element in view
    /// viewport_height: height of the visible content area (in lines)
    pub fn scroll_to_interactive_element(&mut self, viewport_height: u16) {
        if let Some((start_line, end_line)) = self.interactive_state.current_element_line_range() {
            let start = start_line as u16;
            let end = end_line as u16;
            let scroll = self.content_scroll;
            let viewport_end = scroll + viewport_height;

            // Element is above viewport - scroll up to show it
            if start < scroll {
                self.content_scroll = start;
            }
            // Element is below viewport - scroll down to show it
            else if end > viewport_end {
                // Try to position element at top of viewport
                self.content_scroll =
                    start.min(self.content_height.saturating_sub(viewport_height));
            }
            // Element partially visible at bottom - ensure fully visible
            else if start >= scroll && end > viewport_end {
                self.content_scroll = end.saturating_sub(viewport_height);
            }

            // Update scrollbar state
            self.content_scroll_state = self
                .content_scroll_state
                .position(self.content_scroll as usize);
        }
    }

    pub fn toggle_expand(&mut self) {
        if self.focus == Focus::Outline {
            if let Some(i) = self.outline_state.selected() {
                if i < self.outline_items.len() && self.outline_items[i].has_children {
                    let heading_text = self.outline_items[i].text.clone();

                    // Toggle the collapsed state
                    if self.collapsed_headings.contains(&heading_text) {
                        self.collapsed_headings.remove(&heading_text);
                    } else {
                        self.collapsed_headings.insert(heading_text.clone());
                    }

                    // Rebuild the flattened list with overview entry
                    self.rebuild_outline_items();

                    // Restore selection by text (not by index)
                    if !self.select_by_text(&heading_text) {
                        // If heading not found (shouldn't happen), clamp to valid index
                        let safe_idx = i.min(self.outline_items.len().saturating_sub(1));
                        self.outline_state.select(Some(safe_idx));
                        self.outline_scroll_state =
                            ScrollbarState::new(self.outline_items.len()).position(safe_idx);
                    }
                }
            }
        }
    }

    pub fn expand(&mut self) {
        if self.focus == Focus::Outline {
            if let Some(i) = self.outline_state.selected() {
                if i < self.outline_items.len() && self.outline_items[i].has_children {
                    let heading_text = self.outline_items[i].text.clone();

                    // Remove from collapsed set to expand
                    self.collapsed_headings.remove(&heading_text);

                    // Rebuild the flattened list with overview entry
                    self.rebuild_outline_items();

                    // Restore selection by text (not by index)
                    if !self.select_by_text(&heading_text) {
                        // If heading not found (shouldn't happen), clamp to valid index
                        let safe_idx = i.min(self.outline_items.len().saturating_sub(1));
                        self.outline_state.select(Some(safe_idx));
                        self.outline_scroll_state =
                            ScrollbarState::new(self.outline_items.len()).position(safe_idx);
                    }
                }
            }
        }
    }

    pub fn collapse(&mut self) {
        if self.focus == Focus::Outline {
            if let Some(i) = self.outline_state.selected() {
                if i < self.outline_items.len() {
                    let current_level = self.outline_items[i].level;
                    let current_text = self.outline_items[i].text.clone();

                    // If current heading has children, collapse it
                    if self.outline_items[i].has_children {
                        self.collapsed_headings.insert(current_text.clone());

                        // Rebuild the flattened list with overview entry
                        self.rebuild_outline_items();

                        // Restore selection by text
                        if !self.select_by_text(&current_text) {
                            let safe_idx = i.min(self.outline_items.len().saturating_sub(1));
                            self.outline_state.select(Some(safe_idx));
                            self.outline_scroll_state =
                                ScrollbarState::new(self.outline_items.len()).position(safe_idx);
                        }
                    } else {
                        // If no children, find parent and collapse it
                        // Look backwards for first heading with lower level
                        let mut parent_text: Option<String> = None;
                        for idx in (0..i).rev() {
                            if self.outline_items[idx].level < current_level {
                                // Found parent
                                parent_text = Some(self.outline_items[idx].text.clone());
                                break;
                            }
                        }

                        if let Some(parent) = parent_text {
                            // Collapse the parent
                            self.collapsed_headings.insert(parent.clone());

                            // Rebuild and move selection to parent
                            self.rebuild_outline_items();

                            // Select the parent by text
                            if !self.select_by_text(&parent) {
                                // Fallback: select first item if parent not found
                                if !self.outline_items.is_empty() {
                                    self.outline_state.select(Some(0));
                                    self.outline_scroll_state =
                                        ScrollbarState::new(self.outline_items.len()).position(0);
                                }
                            }
                        }
                        // No parent found, do nothing
                    }
                }
            }
        }
    }

    pub fn toggle_focus(&mut self) {
        if self.show_outline {
            self.focus = match self.focus {
                Focus::Outline => Focus::Content,
                Focus::Content => Focus::Outline,
            };
        }
    }

    pub fn toggle_outline(&mut self) {
        self.show_outline = !self.show_outline;
        if !self.show_outline {
            // When hiding outline, switch focus to content
            self.focus = Focus::Content;
        } else {
            // When showing outline, switch focus back to outline
            self.focus = Focus::Outline;
        }
    }

    pub fn cycle_outline_width(&mut self, increase: bool) {
        if increase {
            self.outline_width = match self.outline_width {
                20 => 30,
                30 => 40,
                _ => 40,
            };
        } else {
            self.outline_width = match self.outline_width {
                40 => 30,
                30 => 20,
                _ => 20,
            };
        }

        // Save to config (silently ignore errors)
        let _ = self.config.set_outline_width(self.outline_width);
    }

    pub fn jump_to_heading(&mut self, index: usize) {
        if index < self.outline_items.len() {
            self.outline_state.select(Some(index));
            self.outline_scroll_state = self.outline_scroll_state.position(index);
        }
    }

    pub fn set_bookmark(&mut self) {
        // Store bookmark as heading text instead of index
        self.bookmark_position = self.selected_heading_text().map(|s| s.to_string());
    }

    pub fn jump_to_bookmark(&mut self) {
        // Jump to bookmark by finding the heading text
        if let Some(text) = self.bookmark_position.clone() {
            self.select_by_text(&text);
        }
    }

    pub fn selected_heading_text(&self) -> Option<&str> {
        self.outline_state
            .selected()
            .and_then(|i| self.outline_items.get(i))
            .map(|item| item.text.as_str())
    }

    pub fn toggle_theme_picker(&mut self) {
        self.show_theme_picker = !self.show_theme_picker;
        if self.show_theme_picker {
            // Set selected to current theme when opening
            self.theme_picker_selected = match self.current_theme {
                ThemeName::OceanDark => 0,
                ThemeName::Nord => 1,
                ThemeName::Dracula => 2,
                ThemeName::Solarized => 3,
                ThemeName::Monokai => 4,
                ThemeName::Gruvbox => 5,
                ThemeName::TokyoNight => 6,
                ThemeName::CatppuccinMocha => 7,
            };
        }
    }

    pub fn theme_picker_next(&mut self) {
        if self.theme_picker_selected < 7 {
            self.theme_picker_selected += 1;
        }
    }

    pub fn theme_picker_previous(&mut self) {
        if self.theme_picker_selected > 0 {
            self.theme_picker_selected -= 1;
        }
    }

    pub fn apply_selected_theme(&mut self) {
        let new_theme = match self.theme_picker_selected {
            0 => ThemeName::OceanDark,
            1 => ThemeName::Nord,
            2 => ThemeName::Dracula,
            3 => ThemeName::Solarized,
            4 => ThemeName::Monokai,
            5 => ThemeName::Gruvbox,
            6 => ThemeName::TokyoNight,
            7 => ThemeName::CatppuccinMocha,
            _ => ThemeName::OceanDark,
        };

        self.current_theme = new_theme;
        // Apply color mode when setting theme (also apply custom colors from config)
        self.theme = Theme::from_name(new_theme)
            .with_color_mode(self.color_mode, new_theme)
            .with_custom_colors(&self.config.theme, self.color_mode);
        self.show_theme_picker = false;

        // Save to config (silently ignore errors)
        let _ = self.config.set_theme(new_theme);
    }

    pub fn copy_content(&mut self) {
        // Copy the currently selected section's content
        if let Some(heading_text) = self.selected_heading_text() {
            if let Some(section) = self.document.extract_section(heading_text) {
                // Use persistent clipboard for Linux X11 compatibility
                if let Some(clipboard) = &mut self.clipboard {
                    match clipboard.set_text(section) {
                        Ok(_) => {
                            self.status_message = Some("✓ Section copied to clipboard".to_string());
                        }
                        Err(e) => {
                            self.status_message = Some(format!("✗ Clipboard error: {}", e));
                        }
                    }
                } else {
                    self.status_message = Some("✗ Clipboard not available".to_string());
                }
            } else {
                self.status_message = Some("✗ Could not extract section".to_string());
            }
        } else {
            self.status_message = Some("✗ No heading selected".to_string());
        }
    }

    pub fn copy_anchor(&mut self) {
        // Copy the anchor link for the currently selected heading
        if let Some(heading_text) = self.selected_heading_text() {
            // Convert heading to anchor format (lowercase, replace spaces with dashes)
            let anchor = Self::heading_to_anchor(heading_text);
            let anchor_link = format!("#{}", anchor);

            // Use persistent clipboard for Linux X11 compatibility
            if let Some(clipboard) = &mut self.clipboard {
                match clipboard.set_text(anchor_link) {
                    Ok(_) => {
                        self.status_message = Some(format!("✓ Anchor link copied: #{}", anchor));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("✗ Clipboard error: {}", e));
                    }
                }
            } else {
                self.status_message = Some("✗ Clipboard not available".to_string());
            }
        } else {
            self.status_message = Some("✗ No heading selected".to_string());
        }
    }

    /// Convert heading text to anchor format (lowercase, replace spaces with dashes)
    fn heading_to_anchor(heading: &str) -> String {
        heading
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
            .replace(' ', "-")
    }

    /// Enter link follow mode - extract links from current section and highlight them
    pub fn enter_link_follow_mode(&mut self) {
        // Extract content for current section
        let content = if let Some(heading_text) = self.selected_heading_text() {
            self.document
                .extract_section(heading_text)
                .unwrap_or_else(|| self.document.content.clone())
        } else {
            self.document.content.clone()
        };

        // Extract all links from the content
        self.links_in_view = extract_links(&content);

        // Initialize filtered indices to show all links
        self.filtered_link_indices = (0..self.links_in_view.len()).collect();
        self.link_search_query.clear();
        self.link_search_active = false;

        // Always enter mode, even if no links (so user sees "no links" message)
        self.mode = AppMode::LinkFollow;

        // Select first link if any exist
        if !self.filtered_link_indices.is_empty() {
            self.selected_link_idx = Some(0);
        } else {
            self.selected_link_idx = None;
        }
    }

    /// Exit link follow mode and return to normal mode
    pub fn exit_link_follow_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.links_in_view.clear();
        self.filtered_link_indices.clear();
        self.selected_link_idx = None;
        self.link_search_query.clear();
        self.link_search_active = false;
        // Don't clear status message here - let it display for a moment
    }

    /// Start link search mode
    pub fn start_link_search(&mut self) {
        if self.mode == AppMode::LinkFollow {
            self.link_search_active = true;
        }
    }

    /// Stop link search mode (but keep the filter)
    pub fn stop_link_search(&mut self) {
        self.link_search_active = false;
    }

    /// Clear link search and show all links
    pub fn clear_link_search(&mut self) {
        self.link_search_query.clear();
        self.link_search_active = false;
        self.update_link_filter();
    }

    /// Add a character to the link search query
    pub fn link_search_push(&mut self, c: char) {
        self.link_search_query.push(c);
        self.update_link_filter();
    }

    /// Remove the last character from the link search query
    pub fn link_search_pop(&mut self) {
        self.link_search_query.pop();
        self.update_link_filter();
    }

    /// Update the filtered link indices based on the search query
    fn update_link_filter(&mut self) {
        let query = self.link_search_query.to_lowercase();

        if query.is_empty() {
            // Show all links when no search query
            self.filtered_link_indices = (0..self.links_in_view.len()).collect();
        } else {
            // Filter links by text or URL containing the query
            self.filtered_link_indices = self
                .links_in_view
                .iter()
                .enumerate()
                .filter(|(_, link)| {
                    link.text.to_lowercase().contains(&query)
                        || link.target.as_str().to_lowercase().contains(&query)
                })
                .map(|(idx, _)| idx)
                .collect();
        }

        // Update selection to stay within filtered results
        if self.filtered_link_indices.is_empty() {
            self.selected_link_idx = None;
        } else if let Some(idx) = self.selected_link_idx {
            if idx >= self.filtered_link_indices.len() {
                self.selected_link_idx = Some(0);
            }
        } else {
            self.selected_link_idx = Some(0);
        }
    }

    /// Cycle to the next link (Tab in link follow mode)
    pub fn next_link(&mut self) {
        if self.mode == AppMode::LinkFollow && !self.filtered_link_indices.is_empty() {
            self.selected_link_idx = Some(match self.selected_link_idx {
                Some(idx) => {
                    if idx >= self.filtered_link_indices.len() - 1 {
                        0 // Wrap to first
                    } else {
                        idx + 1
                    }
                }
                None => 0,
            });
        }
    }

    /// Cycle to the previous link (Shift+Tab in link follow mode)
    pub fn previous_link(&mut self) {
        if self.mode == AppMode::LinkFollow && !self.filtered_link_indices.is_empty() {
            self.selected_link_idx = Some(match self.selected_link_idx {
                Some(idx) => {
                    if idx == 0 {
                        self.filtered_link_indices.len() - 1 // Wrap to last
                    } else {
                        idx - 1
                    }
                }
                None => 0,
            });
        }
    }

    /// Jump to parent heading while staying in link follow mode
    pub fn jump_to_parent_links(&mut self) {
        if self.mode == AppMode::LinkFollow {
            // First, jump to parent in outline
            if let Some(current_idx) = self.outline_state.selected() {
                if current_idx < self.outline_items.len() {
                    let current_level = self.outline_items[current_idx].level;

                    // Search backwards for a heading with lower level (parent)
                    for i in (0..current_idx).rev() {
                        if self.outline_items[i].level < current_level {
                            // Jump to parent in outline
                            self.outline_state.select(Some(i));
                            self.outline_scroll_state = self.outline_scroll_state.position(i);

                            // Now extract links from parent's content
                            let content = if let Some(heading_text) = self.selected_heading_text() {
                                self.document
                                    .extract_section(heading_text)
                                    .unwrap_or_else(|| self.document.content.clone())
                            } else {
                                self.document.content.clone()
                            };
                            self.links_in_view = extract_links(&content);

                            // Reset link selection
                            if !self.links_in_view.is_empty() {
                                self.selected_link_idx = Some(0);
                                self.status_message = Some(format!(
                                    "✓ Jumped to parent ({} links found)",
                                    self.links_in_view.len()
                                ));
                            } else {
                                self.selected_link_idx = None;
                                self.status_message = Some("⚠ Parent has no links".to_string());
                            }

                            return;
                        }
                    }

                    // If no parent found (already at top-level)
                    self.status_message = Some("⚠ Already at top-level heading".to_string());
                }
            }
        }
    }

    /// Get the currently selected link (from filtered list)
    pub fn get_selected_link(&self) -> Option<&Link> {
        self.selected_link_idx
            .and_then(|idx| self.filtered_link_indices.get(idx))
            .and_then(|&real_idx| self.links_in_view.get(real_idx))
    }

    /// Follow the currently selected link
    pub fn follow_selected_link(&mut self) -> Result<(), String> {
        let link = match self.get_selected_link() {
            Some(link) => link.clone(),
            None => return Err("No link selected".to_string()),
        };

        match link.target {
            crate::parser::LinkTarget::Anchor(anchor) => {
                // Jump to heading in current document
                self.jump_to_anchor(&anchor)?;
                self.status_message = Some(format!("✓ Jumped to #{}", anchor));
                self.exit_link_follow_mode();
                Ok(())
            }
            crate::parser::LinkTarget::RelativeFile { path, anchor } => {
                // Check if the file has a markdown extension
                let has_md_extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        let ext_lower = ext.to_lowercase();
                        ext_lower == "md" || ext_lower == "markdown" || ext_lower == "mdown"
                    })
                    .unwrap_or(false);

                let current_dir = self
                    .current_file_path
                    .parent()
                    .ok_or("Cannot determine current directory")?;

                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");

                if has_md_extension {
                    // Explicit markdown extension - load in treemd
                    self.load_file(&path, anchor.as_deref())?;
                    self.status_message = Some(format!("✓ Opened {}", filename));
                    self.exit_link_follow_mode();
                } else {
                    // No markdown extension - could be:
                    // 1. A markdown file without extension (common in wikis)
                    // 2. A non-markdown file to open in editor

                    // First, try with .md extension (wiki-style links)
                    let md_path = PathBuf::from(format!("{}.md", path.display()));
                    let absolute_md_path = current_dir.join(&md_path);

                    if absolute_md_path.exists() && !absolute_md_path.is_symlink() {
                        // Found markdown file with .md extension
                        self.load_file(&md_path, anchor.as_deref())?;
                        self.status_message = Some(format!("✓ Opened {}.md", filename));
                        self.exit_link_follow_mode();
                    } else {
                        // Try the path as-is
                        let absolute_path = current_dir.join(&path);

                        if absolute_path.exists() && !absolute_path.is_symlink() {
                            // File exists - open in editor (non-markdown)
                            self.pending_editor_file = Some(absolute_path);
                            self.exit_link_follow_mode();
                        } else {
                            // File doesn't exist - prompt to create markdown file
                            let relative_path = if path.extension().is_none() {
                                PathBuf::from(format!("{}.md", path.display()))
                            } else {
                                path.clone()
                            };
                            self.load_file(&relative_path, anchor.as_deref())?;
                            self.status_message =
                                Some(format!("✓ Opened {}", relative_path.display()));
                            self.exit_link_follow_mode();
                        }
                    }
                }
                Ok(())
            }
            crate::parser::LinkTarget::WikiLink { target, .. } => {
                // Try to find and load the wikilinked file
                self.load_wikilink(&target)?;
                self.status_message = Some(format!("✓ Opened [[{}]]", target));
                self.exit_link_follow_mode();
                Ok(())
            }
            crate::parser::LinkTarget::External(url) => {
                // Try to open in default browser
                let open_result = open::that(&url);

                // Also copy to clipboard as backup
                let mut clipboard_success = false;
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    clipboard_success = clipboard.set_text(url.clone()).is_ok();
                }

                // Set status message
                self.status_message = match (open_result, clipboard_success) {
                    (Ok(_), true) => Some(format!(
                        "✓ Opened {} in browser (also copied to clipboard)",
                        url
                    )),
                    (Ok(_), false) => Some(format!("✓ Opened {} in browser", url)),
                    (Err(_), true) => Some(format!(
                        "⚠ Could not open browser, URL copied to clipboard: {}",
                        url
                    )),
                    (Err(_), false) => Some(format!("✗ Failed to open URL: {}", url)),
                };

                self.exit_link_follow_mode();
                Ok(())
            }
        }
    }

    /// Jump to a heading by anchor name
    fn jump_to_anchor(&mut self, anchor: &str) -> Result<(), String> {
        // Find heading that matches the anchor
        for (idx, item) in self.outline_items.iter().enumerate() {
            let item_anchor = Self::heading_to_anchor(&item.text);
            if item_anchor == anchor {
                self.outline_state.select(Some(idx));
                self.outline_scroll_state = self.outline_scroll_state.position(idx);
                return Ok(());
            }
        }
        Err(format!("Heading '{}' not found", anchor))
    }

    /// Load a file by relative path
    ///
    /// Security: Validates path to prevent directory traversal attacks.
    /// Files must be within the current file's directory or its subdirectories.
    fn load_file(&mut self, relative_path: &PathBuf, anchor: Option<&str>) -> Result<(), String> {
        // Reject absolute paths
        if relative_path.is_absolute() {
            return Err("Absolute paths are not allowed for security reasons".to_string());
        }

        // Reject paths containing .. components (path traversal)
        if relative_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("Path traversal (..) is not allowed for security reasons".to_string());
        }

        // Resolve path relative to current file
        let current_dir = self
            .current_file_path
            .parent()
            .ok_or("Cannot determine current directory")?;
        let absolute_path = current_dir.join(relative_path);

        // Verify the resolved path is within allowed boundaries
        // (defense in depth - even though we rejected .., canonicalize to be sure)
        if let (Ok(canonical_path), Ok(canonical_base)) =
            (absolute_path.canonicalize(), current_dir.canonicalize())
        {
            if !canonical_path.starts_with(&canonical_base) {
                return Err("Path escapes document directory boundary".to_string());
            }
        }

        // Check for symlink (prevent symlink attacks)
        if absolute_path.is_symlink() {
            return Err("Symlinks are not allowed for security reasons".to_string());
        }

        // Check if file exists - if not, prompt to create it
        if !absolute_path.exists() {
            self.pending_file_create = Some(absolute_path.clone());
            self.pending_file_create_message = Some(format!(
                "File '{}' does not exist. Create it?",
                relative_path.display()
            ));
            self.mode = AppMode::ConfirmFileCreate;
            return Ok(()); // Not an error - we're asking user to confirm
        }

        // Parse the new file
        let new_document = crate::parser::parse_file(&absolute_path)
            .map_err(|e| format!("Failed to load file: {}", e))?;

        let new_filename = absolute_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Save current state to history
        self.save_to_history();

        // Load new document
        self.load_document(new_document, new_filename, absolute_path);

        // Jump to anchor if specified
        if let Some(anchor_name) = anchor {
            let _ = self.jump_to_anchor(anchor_name);
        }

        Ok(())
    }

    /// Find and load a wikilinked file
    ///
    /// Supports formats:
    /// - `[[filename]]` - load file (tries .md, .markdown extensions)
    /// - `[[filename#anchor]]` - load file and jump to anchor
    /// - `[[#anchor]]` - jump to anchor in current document
    ///
    /// Security: WikiLinks must be simple filenames without path separators
    /// to prevent directory traversal attacks.
    fn load_wikilink(&mut self, target: &str) -> Result<(), String> {
        // Handle anchor-only wikilinks (e.g., [[#section]])
        if let Some(anchor) = target.strip_prefix('#') {
            // Jump to heading in current document
            self.select_by_text(anchor);
            return Ok(());
        }

        // Split target into file and optional anchor (e.g., "file#section" -> ("file", Some("section")))
        let (file_target, anchor) = if let Some((file, anchor)) = target.split_once('#') {
            (file, Some(anchor))
        } else {
            (target, None)
        };

        // Security: Reject wikilinks containing path separators or traversal
        if file_target.contains('/') || file_target.contains('\\') || file_target.contains("..") {
            return Err("WikiLinks cannot contain path separators".to_string());
        }

        // Security: Reject absolute paths (Windows drive letters)
        #[cfg(windows)]
        if file_target.len() >= 2 && file_target.chars().nth(1) == Some(':') {
            return Err("WikiLinks cannot be absolute paths".to_string());
        }

        // Try to find the file relative to current directory
        let current_dir = self
            .current_file_path
            .parent()
            .ok_or("Cannot determine current directory")?;

        // Try various extensions
        let candidates = vec![
            format!("{}.md", file_target),
            format!("{}.markdown", file_target),
            file_target.to_string(),
        ];

        for candidate in &candidates {
            let path = current_dir.join(candidate);
            // Check for symlinks
            if path.is_symlink() {
                continue; // Skip symlinks for security
            }
            if path.exists() {
                return self.load_file(&PathBuf::from(candidate), anchor);
            }
        }

        // File not found - prompt to create it (default to .md extension)
        let default_filename = format!("{}.md", file_target);
        let new_path = current_dir.join(&default_filename);
        self.pending_file_create = Some(new_path);
        self.pending_file_create_message = Some(format!(
            "Wikilink '[[{}]]' not found. Create '{}'?",
            target, default_filename
        ));
        self.mode = AppMode::ConfirmFileCreate;
        Ok(()) // Not an error - we're asking user to confirm
    }

    /// Save current state to history before navigating away
    fn save_to_history(&mut self) {
        let state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_history.push(state);

        // Clear forward history when navigating to a new file
        self.file_future.clear();
    }

    /// Load a new document and update all related state
    fn load_document(&mut self, document: Document, filename: String, path: PathBuf) {
        self.document = document;
        self.filename = filename;
        self.current_file_path = path;

        // Rebuild tree and outline (with overview entry if applicable)
        self.tree = self.document.build_tree();
        self.rebuild_outline_items();

        // Reset selection to first item
        let mut outline_state = ListState::default();
        if !self.outline_items.is_empty() {
            outline_state.select(Some(0));
        }
        self.outline_state = outline_state;
        self.outline_scroll_state = ScrollbarState::new(self.outline_items.len());

        // Reset content scroll
        self.content_scroll = 0;
        let content_lines = self.document.content.lines().count();
        self.content_height = content_lines as u16;
        self.content_scroll_state = ScrollbarState::new(content_lines);

        // Clear previous selection tracking
        self.previous_selection = None;
    }

    /// Navigate back in file history
    pub fn go_back(&mut self) -> Result<(), String> {
        let previous_state = self
            .file_history
            .pop()
            .ok_or("No previous file in history")?;

        // Save current state to future stack
        let current_state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_future.push(current_state);

        // Restore previous state
        self.restore_file_state(previous_state);

        Ok(())
    }

    /// Navigate forward in file history
    pub fn go_forward(&mut self) -> Result<(), String> {
        let next_state = self.file_future.pop().ok_or("No next file in history")?;

        // Save current state to history stack
        let current_state = FileState {
            path: self.current_file_path.clone(),
            document: self.document.clone(),
            filename: self.filename.clone(),
            selected_heading: self.selected_heading_text().map(|s| s.to_string()),
            content_scroll: self.content_scroll,
            outline_state_selected: self.outline_state.selected(),
        };
        self.file_history.push(current_state);

        // Restore next state
        self.restore_file_state(next_state);

        Ok(())
    }

    /// Restore a file state from history
    fn restore_file_state(&mut self, state: FileState) {
        self.load_document(state.document, state.filename, state.path);

        // Restore selection and scroll position
        if let Some(selected_idx) = state.outline_state_selected {
            if selected_idx < self.outline_items.len() {
                self.outline_state.select(Some(selected_idx));
                self.outline_scroll_state = self.outline_scroll_state.position(selected_idx);
            }
        }

        self.content_scroll = state.content_scroll;
        self.content_scroll_state = self
            .content_scroll_state
            .position(state.content_scroll as usize);
    }

    /// Reload current file from disk (used after external editing)
    pub fn reload_current_file(&mut self) -> Result<(), String> {
        // Save current state to restore after reload
        let current_selection = self.selected_heading_text().map(|s| s.to_string());
        let current_scroll = self.content_scroll;

        // Reload the file
        let content = std::fs::read_to_string(&self.current_file_path)
            .map_err(|e| format!("Failed to reload file: {}", e))?;

        let document = crate::parser::parse_markdown(&content);
        let filename = self
            .current_file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        self.load_document(document, filename, self.current_file_path.clone());

        // Try to restore selection if the heading still exists
        if let Some(heading) = current_selection {
            self.select_by_text(&heading);
        }

        // Restore scroll position (may be adjusted if content changed)
        if current_scroll < self.content_height {
            self.content_scroll = current_scroll;
            self.content_scroll_state = self.content_scroll_state.position(current_scroll as usize);
        }

        Ok(())
    }

    /// Enter interactive mode - build element index and enter mode
    pub fn enter_interactive_mode(&mut self) {
        // Exit raw source view if active (interactive elements aren't visible in raw mode)
        if self.show_raw_source {
            self.show_raw_source = false;
        }

        // Get current section content to index
        let content = if let Some(selected) = self.selected_heading_text() {
            self.document
                .extract_section(selected)
                .unwrap_or_else(|| self.document.content.clone())
        } else {
            self.document.content.clone()
        };

        // Parse content into blocks
        use crate::parser::content::parse_content;
        let blocks = parse_content(&content, 0);

        // Index interactive elements
        self.interactive_state.index_elements(&blocks);

        // Enter interactive mode at current scroll position (preserve user's view)
        self.interactive_state
            .enter_at_scroll_position(self.content_scroll as usize);
        self.mode = AppMode::Interactive;

        // Only scroll if the selected element is not fully visible
        self.scroll_to_interactive_element(20);

        // Set status message
        if self.interactive_state.elements.is_empty() {
            self.status_message = Some("⚠ No interactive elements in this section".to_string());
        } else {
            self.status_message = Some(format!(
                "✓ Interactive mode: {} elements found (Tab to cycle)",
                self.interactive_state.elements.len()
            ));
        }
    }

    /// Exit interactive mode and return to normal
    pub fn exit_interactive_mode(&mut self) {
        self.interactive_state.exit();
        self.mode = AppMode::Normal;
        self.status_message = None;
    }

    /// Confirm file creation and open the new file
    pub fn confirm_file_create(&mut self) -> Result<(), String> {
        if let Some(path) = self.pending_file_create.take() {
            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }
            }

            // Create the file with default content
            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("untitled");
            let default_content = format!("# {}\n\n", filename);

            std::fs::write(&path, &default_content)
                .map_err(|e| format!("Failed to create file: {}", e))?;

            // Load the new file
            let relative_path = path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.clone());

            self.pending_file_create_message = None;
            self.mode = AppMode::Normal;

            // Load the newly created file
            self.load_file(&relative_path, None)?;
            self.status_message = Some(format!("✓ Created and opened {}", relative_path.display()));
            self.exit_link_follow_mode();
        }
        Ok(())
    }

    /// Cancel file creation and return to previous mode
    pub fn cancel_file_create(&mut self) {
        self.pending_file_create = None;
        self.pending_file_create_message = None;
        self.mode = AppMode::Normal;
        self.status_message = Some("File creation cancelled".to_string());
    }

    /// Get the currently selected interactive element
    pub fn get_selected_interactive_element(
        &self,
    ) -> Option<&crate::tui::interactive::InteractiveElement> {
        self.interactive_state.current_element()
    }

    /// Activate the currently selected interactive element
    pub fn activate_interactive_element(&mut self) -> Result<(), String> {
        use crate::tui::interactive::ElementType;

        let element = match self.interactive_state.current_element() {
            Some(elem) => elem.clone(),
            None => return Err("No element selected".to_string()),
        };

        match &element.element_type {
            ElementType::Details { .. } => {
                // Toggle details expansion
                self.interactive_state.toggle_details(element.id);

                // Re-index elements since expanded state changed content
                self.reindex_interactive_elements();

                self.status_message = Some("✓ Toggled details".to_string());
                Ok(())
            }
            ElementType::Checkbox {
                checked,
                block_idx,
                item_idx,
                ..
            } => {
                // Toggle checkbox and save to file
                self.toggle_checkbox_and_save(*block_idx, *item_idx, *checked)?;
                Ok(())
            }
            ElementType::Link { link, .. } => {
                // Follow link using existing link follow logic
                self.follow_link_from_interactive(&link.clone())?;
                Ok(())
            }
            ElementType::CodeBlock { content, .. } => {
                // Copy code to clipboard
                self.copy_to_clipboard(content)?;
                self.status_message = Some("✓ Code copied to clipboard".to_string());
                Ok(())
            }
            ElementType::Image { src, alt, .. } => {
                // Copy image path to clipboard
                self.copy_to_clipboard(src)?;
                self.status_message = Some(format!("✓ Image path copied: {}", alt));
                Ok(())
            }
            ElementType::Table { rows, cols, .. } => {
                // Enter table navigation mode
                self.interactive_state.enter_table_mode()?;
                self.status_message =
                    Some(self.interactive_state.table_status_text(rows + 1, *cols));
                Ok(())
            }
        }
    }

    /// Re-index interactive elements after state changes
    fn reindex_interactive_elements(&mut self) {
        let content = if let Some(selected) = self.selected_heading_text() {
            self.document
                .extract_section(selected)
                .unwrap_or_else(|| self.document.content.clone())
        } else {
            self.document.content.clone()
        };

        use crate::parser::content::parse_content;
        let blocks = parse_content(&content, 0);
        self.interactive_state.index_elements(&blocks);
    }

    /// Toggle a checkbox and save changes to the file
    fn toggle_checkbox_and_save(
        &mut self,
        block_idx: usize,
        item_idx: usize,
        checked: bool,
    ) -> Result<(), String> {
        // Get the checkbox content text to use as identifier
        let checkbox_content = {
            let content = if let Some(selected) = self.selected_heading_text() {
                self.document
                    .extract_section(selected)
                    .unwrap_or_else(|| self.document.content.clone())
            } else {
                self.document.content.clone()
            };

            use crate::parser::content::parse_content;
            let blocks = parse_content(&content, 0);

            if let Some(crate::parser::output::Block::List { items, .. }) = blocks.get(block_idx) {
                items.get(item_idx).map(|item| item.content.clone())
            } else {
                None
            }
        };

        let checkbox_content =
            checkbox_content.ok_or_else(|| "Could not find checkbox content".to_string())?;

        // Read the current file
        let file_content = std::fs::read_to_string(&self.current_file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Find and toggle the checkbox in the file content
        let new_content =
            self.toggle_checkbox_by_content(&file_content, &checkbox_content, checked)?;

        // Atomic write: write to temp file, then rename (prevents data corruption)
        use std::io::Write;
        let parent_dir = self
            .current_file_path
            .parent()
            .ok_or("Cannot determine parent directory")?;

        let mut temp_file = tempfile::NamedTempFile::new_in(parent_dir)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        temp_file
            .write_all(new_content.as_bytes())
            .map_err(|e| format!("Failed to write temp file: {}", e))?;

        temp_file
            .flush()
            .map_err(|e| format!("Failed to flush temp file: {}", e))?;

        // Atomic rename (same filesystem guarantees atomicity)
        temp_file
            .persist(&self.current_file_path)
            .map_err(|e| format!("Failed to save file: {}", e))?;

        // Reload the document
        self.reload_current_file()?;

        // Re-index interactive elements
        self.reindex_interactive_elements();

        let new_state = if checked { "unchecked" } else { "checked" };
        self.status_message = Some(format!("✓ Checkbox {} and saved", new_state));

        Ok(())
    }

    /// Toggle a checkbox in markdown content by matching the content text
    fn toggle_checkbox_by_content(
        &self,
        file_content: &str,
        checkbox_text: &str,
        current_checked: bool,
    ) -> Result<String, String> {
        let lines: Vec<&str> = file_content.lines().collect();
        let mut result = Vec::new();
        let mut found = false;

        // Clean the checkbox text to match (remove any checkbox markers if present)
        let clean_text = checkbox_text
            .trim_start()
            .trim_start_matches("[x]")
            .trim_start_matches("[X]")
            .trim_start_matches("[ ]")
            .trim();

        for line in lines {
            let trimmed = line.trim_start();

            // Check if this is a checkbox line
            if (trimmed.starts_with("- [ ]")
                || trimmed.starts_with("- [x]")
                || trimmed.starts_with("- [X]"))
                && !found
            {
                // Extract the text after the checkbox marker
                let line_text = trimmed
                    .trim_start_matches("- [ ]")
                    .trim_start_matches("- [x]")
                    .trim_start_matches("- [X]")
                    .trim();

                // Check if this matches our target checkbox
                if line_text == clean_text {
                    // Toggle the checkbox
                    let new_line = if current_checked {
                        // Change [x] or [X] to [ ]
                        line.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1)
                    } else {
                        // Change [ ] to [x]
                        line.replacen("[ ]", "[x]", 1)
                    };
                    result.push(new_line);
                    found = true;
                } else {
                    result.push(line.to_string());
                }
            } else {
                result.push(line.to_string());
            }
        }

        if !found {
            return Err(format!("Checkbox not found in file: '{}'", clean_text));
        }

        Ok(result.join("\n") + "\n")
    }

    /// Follow a link from interactive mode
    fn follow_link_from_interactive(&mut self, link: &crate::parser::Link) -> Result<(), String> {
        use crate::parser::LinkTarget;

        match &link.target {
            LinkTarget::Anchor(anchor) => {
                // Jump to heading in current document
                self.select_by_text(anchor);
                self.exit_interactive_mode();
                Ok(())
            }
            LinkTarget::RelativeFile { path, anchor } => {
                // Check if the file has a markdown extension
                let has_md_extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        let ext_lower = ext.to_lowercase();
                        ext_lower == "md" || ext_lower == "markdown" || ext_lower == "mdown"
                    })
                    .unwrap_or(false);

                let current_dir = self
                    .current_file_path
                    .parent()
                    .ok_or("Cannot determine current directory")?;

                if has_md_extension {
                    // Explicit markdown extension - load in treemd
                    self.load_file(path, anchor.as_deref())?;
                    self.exit_interactive_mode();
                } else {
                    // No markdown extension - could be:
                    // 1. A markdown file without extension (common in wikis)
                    // 2. A non-markdown file to open in editor

                    // First, try with .md extension (wiki-style links)
                    let md_path = PathBuf::from(format!("{}.md", path.display()));
                    let absolute_md_path = current_dir.join(&md_path);

                    if absolute_md_path.exists() && !absolute_md_path.is_symlink() {
                        // Found markdown file with .md extension
                        self.load_file(&md_path, anchor.as_deref())?;
                        self.exit_interactive_mode();
                    } else {
                        // Try the path as-is
                        let absolute_path = current_dir.join(path);

                        if absolute_path.exists() && !absolute_path.is_symlink() {
                            // File exists - open in editor (non-markdown)
                            self.pending_editor_file = Some(absolute_path);
                            self.exit_interactive_mode();
                        } else {
                            // File doesn't exist - prompt to create markdown file
                            let relative_path = if path.extension().is_none() {
                                PathBuf::from(format!("{}.md", path.display()))
                            } else {
                                path.clone()
                            };
                            self.load_file(&relative_path, anchor.as_deref())?;
                            self.exit_interactive_mode();
                        }
                    }
                }
                Ok(())
            }
            LinkTarget::WikiLink { target, .. } => {
                // Try to find and load the wikilinked file
                self.load_wikilink(target)?;
                self.exit_interactive_mode();
                Ok(())
            }
            LinkTarget::External(url) => {
                // Security: Validate URL scheme (only http/https allowed)
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(
                        "Unsafe URL scheme. Only http:// and https:// URLs are allowed."
                            .to_string(),
                    );
                }

                // Use the `open` crate for safe URL opening (no shell injection)
                open::that(url).map_err(|e| format!("Failed to open URL: {}", e))?;

                self.status_message = Some(format!("✓ Opened {}", url));
                Ok(())
            }
        }
    }

    /// Copy text to clipboard
    fn copy_to_clipboard(&mut self, text: &str) -> Result<(), String> {
        if let Some(clipboard) = &mut self.clipboard {
            clipboard
                .set_text(text.to_string())
                .map_err(|e| format!("Clipboard error: {}", e))?;
            Ok(())
        } else {
            Err("Clipboard not available".to_string())
        }
    }

    /// Get table data for current interactive element
    fn get_current_table_data(&self) -> Option<(Vec<String>, Vec<Vec<String>>)> {
        if let Some(element) = self.interactive_state.current_element() {
            if let crate::tui::interactive::ElementType::Table { block_idx, .. } =
                &element.element_type
            {
                // Parse current section to get table data
                let content = if let Some(selected) = self.selected_heading_text() {
                    self.document
                        .extract_section(selected)
                        .unwrap_or_else(|| self.document.content.clone())
                } else {
                    self.document.content.clone()
                };

                use crate::parser::content::parse_content;
                let blocks = parse_content(&content, 0);

                if let Some(crate::parser::output::Block::Table { headers, rows, .. }) =
                    blocks.get(*block_idx)
                {
                    return Some((headers.clone(), rows.clone()));
                }
            }
        }
        None
    }

    /// Copy table cell to clipboard
    pub fn copy_table_cell(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data() {
            if let Some(cell) = self.interactive_state.get_table_cell(&headers, &rows) {
                self.copy_to_clipboard(&cell)?;
                self.status_message = Some(format!("✓ Cell copied: {}", cell));
                return Ok(());
            }
        }
        Err("No cell selected".to_string())
    }

    /// Copy table row to clipboard (tab-separated)
    pub fn copy_table_row(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data() {
            if let Some(row) = self.interactive_state.get_table_row(&headers, &rows) {
                let row_text = row.join("\t");
                self.copy_to_clipboard(&row_text)?;
                self.status_message = Some("✓ Row copied (tab-separated)".to_string());
                return Ok(());
            }
        }
        Err("No row selected".to_string())
    }

    /// Copy entire table as markdown
    pub fn copy_table_markdown(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data() {
            let mut table_md = String::new();

            // Header row
            table_md.push_str("| ");
            table_md.push_str(&headers.join(" | "));
            table_md.push_str(" |\n");

            // Separator row
            table_md.push_str("| ");
            table_md.push_str(&vec!["---"; headers.len()].join(" | "));
            table_md.push_str(" |\n");

            // Data rows
            for row in &rows {
                table_md.push_str("| ");
                table_md.push_str(&row.join(" | "));
                table_md.push_str(" |\n");
            }

            self.copy_to_clipboard(&table_md)?;
            self.status_message = Some("✓ Table copied as markdown".to_string());
            Ok(())
        } else {
            Err("No table data available".to_string())
        }
    }

    /// Enter cell edit mode for the currently selected table cell
    pub fn enter_cell_edit_mode(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data() {
            if let Some((row, col)) = self.interactive_state.get_table_position() {
                // Get current cell value
                let cell_value = if row == 0 {
                    // Header row
                    headers.get(col).cloned().unwrap_or_default()
                } else {
                    // Data row
                    rows.get(row - 1)
                        .and_then(|r| r.get(col))
                        .cloned()
                        .unwrap_or_default()
                };

                self.cell_edit_value = cell_value;
                self.cell_edit_row = row;
                self.cell_edit_col = col;
                self.mode = AppMode::CellEdit;
                return Ok(());
            }
        }
        Err("No cell selected for editing".to_string())
    }

    /// Sanitize table cell content to prevent markdown injection
    fn sanitize_table_cell(value: &str) -> String {
        value
            .replace('|', "\\|") // Escape pipe characters (table delimiters)
            .replace(['\n', '\r'], " ") // Replace newlines and carriage returns
    }

    /// Save the edited cell value back to the file
    pub fn save_edited_cell(&mut self) -> Result<(), String> {
        use std::fs;
        use std::io::Write;

        // Sanitize the cell value to prevent table structure corruption
        let sanitized_value = Self::sanitize_table_cell(&self.cell_edit_value);

        // Read the current file
        let file_content = fs::read_to_string(&self.current_file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Find and replace the table cell in the markdown
        let new_content = self.replace_table_cell_in_markdown(
            &file_content,
            self.cell_edit_row,
            self.cell_edit_col,
            &sanitized_value,
        )?;

        // Atomic write: write to temp file, then rename (prevents data corruption)
        let parent_dir = self
            .current_file_path
            .parent()
            .ok_or("Cannot determine parent directory")?;

        let mut temp_file = tempfile::NamedTempFile::new_in(parent_dir)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        temp_file
            .write_all(new_content.as_bytes())
            .map_err(|e| format!("Failed to write temp file: {}", e))?;

        temp_file
            .flush()
            .map_err(|e| format!("Failed to flush temp file: {}", e))?;

        // Atomic rename
        temp_file
            .persist(&self.current_file_path)
            .map_err(|e| format!("Failed to save file: {}", e))?;

        // Reload the document
        let updated_document = crate::parser::parse_file(&self.current_file_path)
            .map_err(|e| format!("Failed to reload document: {}", e))?;

        self.document = updated_document;
        self.status_message = Some("✓ Cell updated".to_string());
        Ok(())
    }

    /// Replace a specific table cell in markdown content
    fn replace_table_cell_in_markdown(
        &self,
        content: &str,
        row: usize,
        col: usize,
        new_value: &str,
    ) -> Result<String, String> {
        use crate::parser::content::parse_content;

        // Get the current section content to find the right table
        let section_content = if let Some(heading_text) = self.selected_heading_text() {
            self.document
                .extract_section(heading_text)
                .unwrap_or_else(|| self.document.content.clone())
        } else {
            self.document.content.clone()
        };

        // Parse to find the table block
        let blocks = parse_content(&section_content, 0);

        // Find the block index of the current table element
        if let Some(element) = self.interactive_state.current_element() {
            let block_idx = element.id.block_idx;

            if let Some(crate::parser::output::Block::Table { .. }) = blocks.get(block_idx) {
                // Find this table in the full file content
                return self.replace_table_cell_in_file(content, row, col, new_value);
            }
        }

        Err("Could not locate table in file".to_string())
    }

    /// Find and replace a cell in the first table found in the current section
    fn replace_table_cell_in_file(
        &self,
        content: &str,
        row: usize,
        col: usize,
        new_value: &str,
    ) -> Result<String, String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut in_table = false;
        let mut table_row_idx = 0;
        let mut found_table = false;

        for line in lines {
            let trimmed = line.trim();

            // Detect table start (line starting with |)
            if trimmed.starts_with('|') && trimmed.ends_with('|') {
                if !in_table {
                    in_table = true;
                    found_table = true;
                    table_row_idx = 0;
                }

                // Skip separator rows (| --- | --- |)
                if trimmed.contains("---") {
                    result.push(line.to_string());
                    continue;
                }

                // This is a data row in the table
                if table_row_idx == row {
                    // Replace this row's cell
                    let new_line = self.replace_cell_in_row(line, col, new_value);
                    result.push(new_line);
                    in_table = false; // Stop after modifying the target row
                } else {
                    result.push(line.to_string());
                }

                table_row_idx += 1;
            } else {
                if in_table {
                    in_table = false;
                }
                result.push(line.to_string());
            }
        }

        if found_table {
            Ok(result.join("\n"))
        } else {
            Err("Table not found in file".to_string())
        }
    }

    /// Replace a specific cell in a table row line
    fn replace_cell_in_row(&self, line: &str, col: usize, new_value: &str) -> String {
        // Split by | and reconstruct
        let parts: Vec<&str> = line.split('|').collect();

        // Table format: | cell0 | cell1 | cell2 |
        // After split: ["", " cell0 ", " cell1 ", " cell2 ", ""]
        let mut new_parts = Vec::new();

        for (i, part) in parts.iter().enumerate() {
            if i == 0 || i == parts.len() - 1 {
                // Keep empty parts at start/end
                new_parts.push(part.to_string());
            } else if i - 1 == col {
                // This is the cell to replace (accounting for leading empty part)
                new_parts.push(format!(" {} ", new_value));
            } else {
                new_parts.push(part.to_string());
            }
        }

        new_parts.join("|")
    }
}
