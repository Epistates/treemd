//! Outline search and in-document search.

use super::*;

/// A match found during search
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Line number (0-indexed)
    pub line: usize,
    /// Start column (byte offset in line)
    pub col_start: usize,
    /// Length of match in bytes
    pub len: usize,
}

/// In-document search state (`/` then `n`/`N`).
#[derive(Debug, Default)]
pub struct DocSearchState {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current_idx: Option<usize>,
    /// Whether the input prompt is active (cursor visible).
    pub active: bool,
    /// True when search was opened from interactive mode (so it returns there).
    pub from_interactive: bool,
    /// If the current match falls inside a link, this is the index into
    /// `links_in_view`.
    pub selected_link_idx: Option<usize>,
}

impl App {
    /// With an outline search applied, select the next match, or the
    /// previous when not `forward`, wrapping around. Whether it did.
    pub(super) fn cycle_search_results(&mut self, forward: bool) -> bool {
        let len = self.outline_items.len();
        if self.outline_search != OutlineSearch::Applied || len == 0 {
            return false;
        }
        let current = self.outline_state.selected().unwrap_or(0);
        let step = if forward { 1 } else { len - 1 };
        self.select_outline_index((current + step) % len);
        true
    }

    pub fn toggle_search(&mut self) {
        match self.outline_search {
            // If actively typing in search, toggle it off (clear and hide)
            OutlineSearch::Typing => {
                self.outline_search = OutlineSearch::Off;
                self.search_query.clear();
                self.filter_outline();
            }
            // In accepted search state (showing filtered results) - start fresh search
            OutlineSearch::Applied => {
                self.search_query.clear();
                self.filter_outline(); // Restore full outline for new search
                self.outline_search = OutlineSearch::Typing;
            }
            // Enter search mode from normal state
            OutlineSearch::Off => {
                self.outline_search = OutlineSearch::Typing;
                self.search_query.clear();
            }
        }
    }

    /// Toggle between outline search and document search, preserving the query.
    /// After search is accepted (Enter pressed), Tab cycles through matches instead.
    pub fn toggle_search_mode(&mut self) {
        if self.outline_search.is_on() {
            // Currently in outline search -> switch to doc search
            let query = self.search_query.clone();
            let was_active = self.outline_search == OutlineSearch::Typing;
            self.outline_search = OutlineSearch::Off;
            self.search_query.clear();
            self.filter_outline(); // Reset outline filter

            // Enter doc search with the same query
            self.mode = AppMode::DocSearch;
            self.doc_search.active = was_active; // Preserve active state
            self.doc_search.query = query;
            self.doc_search.matches.clear();
            self.doc_search.current_idx = None;
            self.update_doc_search_matches();
        } else if self.mode == AppMode::DocSearch {
            if self.doc_search.active {
                // Still typing -> switch to outline search
                let query = self.doc_search.query.clone();
                self.mode = AppMode::Normal;
                self.doc_search.active = false;
                self.doc_search.query.clear();
                self.doc_search.matches.clear();
                self.doc_search.current_idx = None;

                // Enter outline search with the same query
                self.outline_search = OutlineSearch::Typing; // Keep input active
                self.search_query = query;
                self.filter_outline();
            } else {
                // Search accepted (after Enter) -> cycle through matches
                self.next_doc_match();
            }
        }
    }

    pub fn filter_outline(&mut self) {
        // Save current selection
        let current_idx = self.selected_heading_index();
        let current_selection = self.selected_heading_text().map(|s| s.to_string());

        if self.search_query.is_empty() {
            // Reset to full tree with overview entry
            self.rebuild_outline_items();
        } else {
            // Filter by search query, but always include overview entry if applicable
            let query_lower = self.search_query.to_lowercase();
            let has_preamble = Self::has_preamble_content(&self.document);

            // Every heading, collapsed parents or not: a match under a
            // collapsed heading was not found at all.
            self.outline_items = Self::flatten_tree(&self.tree, &HashSet::new(), &[])
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
                        heading_index: None,
                    },
                );
            }
        }

        // Try to restore previous selection, otherwise select first item
        if !self.outline_items.is_empty() {
            let restored = current_idx.is_some_and(|i| self.select_by_heading_index(i))
                || current_selection.is_some_and(|t| self.select_by_text(&t));

            if !restored {
                self.outline_state.select(Some(0));
                self.outline_scroll_state =
                    ScrollbarState::new(self.outline_items.len()).position(0);
            }
        }
    }

    /// Enter document search mode (activated by / when content is focused or in interactive mode)
    /// If in accepted outline search state, re-enter outline search input instead
    /// If already in accepted doc search state, re-enter doc search input
    pub fn enter_doc_search(&mut self) {
        // If in accepted outline search state (locked-in filter), re-enter outline search input
        if self.outline_search == OutlineSearch::Applied {
            // Re-activate outline search input (keep existing query for editing)
            self.outline_search = OutlineSearch::Typing;
            return;
        }

        // If already in accepted doc search state, re-enter input mode (keep existing query)
        if self.mode == AppMode::DocSearch && !self.doc_search.active {
            self.doc_search.active = true;
            return;
        }

        // Remember if we came from interactive mode to restore it later
        self.doc_search.from_interactive = self.mode == AppMode::Interactive;
        self.mode = AppMode::DocSearch;
        self.doc_search.active = true;
        self.doc_search.query.clear();
        self.doc_search.matches.clear();
        self.doc_search.current_idx = None;
    }

    /// Update search matches based on current query (supports fuzzy and exact matching)
    pub fn update_doc_search_matches(&mut self) {
        self.doc_search.matches.clear();

        if self.doc_search.query.is_empty() {
            self.doc_search.current_idx = None;
            return;
        }

        // Search the lines the content pane draws, the same ones the
        // highlighter marks, so the match count and the highlighted current
        // match agree, and a match's line is a place on screen. Searching a
        // plain-text conversion of the source instead numbered lines that
        // were never drawn, so `n` scrolled to the wrong place.
        let view = std::sync::Arc::clone(&self.content_view);
        // Find all exact substring matches (case-insensitive), as offsets
        // into the line itself, the same matcher the highlighter uses
        for (line_num, line) in view.line_text.iter().enumerate() {
            for (start, end) in crate::tui::ui::util::find_ignore_case(line, &self.doc_search.query)
            {
                self.doc_search.matches.push(SearchMatch {
                    line: line_num,
                    col_start: start,
                    len: end - start,
                });
            }
        }

        // Select first match if any exist
        self.doc_search.current_idx = if self.doc_search.matches.is_empty() {
            None
        } else {
            Some(0)
        };

        // Scroll to current match
        self.scroll_to_doc_search_match();
    }

    /// Scroll to the current search match and detect if it's inside a link
    pub(super) fn scroll_to_doc_search_match(&mut self) {
        // Reset link selection
        self.doc_search.selected_link_idx = None;

        if let Some(idx) = self.doc_search.current_idx
            && let Some(m) = self.doc_search.matches.get(idx)
        {
            // The row the match is drawn on, inside its line if that wraps.
            let match_line = self.content_view.row_of_offset(m.line, m.col_start);

            // Scroll to bring match line into view (center it if possible)
            let half_viewport = usize::from(self.content_viewport_height / 2);
            self.content_scroll = match_line.saturating_sub(half_viewport);
            self.content_scroll = self.content_scroll.min(self.max_content_scroll());
            self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);

            // Check if this match is inside a link
            self.detect_link_at_search_match(m.line, m.col_start, m.len);
        }
    }

    /// Detect if a search match position overlaps with a link and select it
    pub(super) fn detect_link_at_search_match(
        &mut self,
        match_line: usize,
        match_col: usize,
        match_len: usize,
    ) {
        use crate::parser::links::extract_links;

        // Links of the section, for following one from the match.
        let content = self.current_section_content();
        self.links_in_view = extract_links(&content);
        self.link_picker.filtered_indices = (0..self.links_in_view.len()).collect();

        // A link is at the match if its text, as drawn, overlaps the match in
        // the line the match is on.
        let Some(line) = self.content_view.line_text.get(match_line) else {
            return;
        };
        let match_end = match_col + match_len;
        let found = self.links_in_view.iter().position(|link| {
            !link.text.is_empty()
                && line
                    .match_indices(link.text.as_str())
                    .any(|(at, text)| at < match_end && at + text.len() > match_col)
        });
        if let Some(idx) = found {
            self.doc_search.selected_link_idx = Some(idx);
            self.link_picker.selected = Some(idx); // Also set link mode selection
        }
    }

    /// Accept search and exit search input mode (keep matches for n/N navigation)
    pub fn accept_doc_search(&mut self) {
        self.doc_search.active = false;
        // Keep mode as DocSearch for n/N navigation
        // If no matches, show status message
        if self.doc_search.matches.is_empty() && !self.doc_search.query.is_empty() {
            self.status_message = Some(format!("Pattern not found: {}", self.doc_search.query));
        }
    }

    /// Cancel search and return to previous mode (interactive or normal)
    pub fn cancel_doc_search(&mut self) {
        // Restore interactive mode if that's where we came from
        if self.doc_search.from_interactive {
            self.mode = AppMode::Interactive;
        } else {
            self.mode = AppMode::Normal;
        }
        self.doc_search.active = false;
        self.doc_search.from_interactive = false;
        self.doc_search.query.clear();
        self.doc_search.matches.clear();
        self.doc_search.current_idx = None;
        self.doc_search.selected_link_idx = None;
        // Sync to prevent update_content_metrics() from resetting scroll
        self.sync_previous_selection();
    }

    /// Clear search highlighting and return to previous mode (interactive or normal)
    pub fn clear_doc_search(&mut self) {
        // Restore interactive mode if that's where we came from
        if self.doc_search.from_interactive {
            self.mode = AppMode::Interactive;
        } else {
            self.mode = AppMode::Normal;
        }
        self.doc_search.from_interactive = false;
        self.doc_search.query.clear();
        self.doc_search.matches.clear();
        self.doc_search.current_idx = None;
        self.doc_search.selected_link_idx = None;
        // Sync to prevent update_content_metrics() from resetting scroll
        self.sync_previous_selection();
    }

    /// Navigate to next search match
    /// Handles both doc search matches and accepted outline search navigation
    pub fn next_doc_match(&mut self) {
        // With an outline search applied, cycle through its matches
        if self.cycle_search_results(true) {
            return;
        }

        if self.doc_search.matches.is_empty() {
            return;
        }

        self.doc_search.current_idx = Some(match self.doc_search.current_idx {
            Some(idx) => (idx + 1) % self.doc_search.matches.len(),
            None => 0,
        });

        self.scroll_to_doc_search_match();
    }

    /// Navigate to previous search match
    /// Handles both doc search matches and accepted outline search navigation
    pub fn prev_doc_match(&mut self) {
        // With an outline search applied, cycle through its matches
        if self.cycle_search_results(false) {
            return;
        }

        if self.doc_search.matches.is_empty() {
            return;
        }

        let len = self.doc_search.matches.len();
        self.doc_search.current_idx = Some(match self.doc_search.current_idx {
            Some(idx) => (idx + len - 1) % len,
            None => len - 1,
        });

        self.scroll_to_doc_search_match();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(
            crate::parser::parse_markdown("# Alpha\n## Beta\n## Alphabet\n## Gamma\n"),
            "t.md".into(),
            PathBuf::from("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        )
    }

    fn outline(app: &App) -> Vec<&str> {
        app.outline_items.iter().map(|i| i.text.as_str()).collect()
    }

    /// The outline search moves from typing to applied and back to off,
    /// one state at a time, and the keys follow it.
    #[test]
    fn the_outline_search_moves_through_its_states() {
        let mut app = app();
        assert_eq!(app.outline_search, OutlineSearch::Off);

        app.execute_action(Action::EnterSearchMode);
        assert_eq!(app.outline_search, OutlineSearch::Typing);
        assert_eq!(app.current_keybinding_mode(), KeybindingMode::Search);
        for c in "alph".chars() {
            assert!(app.apply_text_input_edit(TextInputEdit::Insert(c)));
        }
        assert_eq!(outline(&app), ["Alpha", "Alphabet"]);

        app.execute_action(Action::ConfirmAction);
        assert_eq!(app.outline_search, OutlineSearch::Applied);
        assert_eq!(app.current_keybinding_mode(), KeybindingMode::Normal);
        // Typed text no longer goes to the query.
        assert!(!app.apply_text_input_edit(TextInputEdit::Insert('x')));

        // Tab cycles through the matches, wrapping.
        app.select_outline_index(0);
        app.toggle_focus();
        assert_eq!(app.selected_heading_text(), Some("Alphabet"));
        app.toggle_focus();
        assert_eq!(app.selected_heading_text(), Some("Alpha"));

        app.execute_action(Action::ExitMode);
        assert_eq!(app.outline_search, OutlineSearch::Off);
        assert_eq!(outline(&app), ["Alpha", "Beta", "Alphabet", "Gamma"]);
    }

    /// Typing stopped at the search's length limit, but a paste did not,
    /// so a query of any length could be pasted and re-run on every key.
    #[test]
    fn a_paste_is_held_to_the_fields_limit() {
        let mut app = app();
        app.execute_action(Action::EnterSearchMode);
        assert!(app.apply_text_input_paste(&"a".repeat(1000)));
        assert_eq!(app.search_query.len(), 256);

        // Nothing is typed into a search while help is in front of it.
        app.toggle_help();
        assert_eq!(app.text_input(), None);
        assert!(!app.apply_text_input_paste("b"));
    }
}
