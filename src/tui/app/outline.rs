//! The outline: building it from the heading tree, moving through it, collapsing, and bookmarks.

use super::*;

impl App {
    /// Check if the document has non-whitespace content before the first heading
    pub(super) fn has_preamble_content(document: &Document) -> bool {
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

    /// Check if a heading's section contains open todos (- [ ])
    pub(super) fn heading_has_open_todos(&self, heading_index: usize) -> bool {
        if let Some(content) = self.document.extract_section_at_index(heading_index) {
            // Check for unchecked todo pattern: - [ ] or * [ ]
            content.contains("- [ ]") || content.contains("* [ ]")
        } else {
            false
        }
    }

    /// Check if a heading or any of its descendants have open todos
    pub(super) fn heading_tree_has_open_todos(&self, node: &HeadingNode) -> bool {
        // Check this heading's direct content
        if self.heading_has_open_todos(node.index) {
            return true;
        }
        // Recursively check children
        for child in &node.children {
            if self.heading_tree_has_open_todos(child) {
                return true;
            }
        }
        false
    }

    /// Build a set of heading texts that have open todos (directly or in descendants)
    pub(super) fn headings_with_open_todos(&self) -> HashSet<usize> {
        let mut result = HashSet::new();
        for node in &self.tree {
            self.collect_headings_with_todos(node, &mut result);
        }
        result
    }

    /// Recursively collect headings that should be shown when filtering by todos
    pub(super) fn collect_headings_with_todos(
        &self,
        node: &HeadingNode,
        result: &mut HashSet<usize>,
    ) {
        if self.heading_tree_has_open_todos(node) {
            // This heading or a descendant has todos, include it
            result.insert(node.index);
            // Also recursively add children that have todos
            for child in &node.children {
                self.collect_headings_with_todos(child, result);
            }
        }
    }

    /// Rebuild outline items from the tree, optionally adding document overview
    pub(super) fn rebuild_outline_items(&mut self) {
        let mut items = Self::flatten_tree(
            &self.tree,
            &self.collapsed_headings,
            &heading_keys(&self.document),
        );

        // Apply todo filter if enabled
        if self.filter_by_todos {
            let headings_with_todos = self.headings_with_open_todos();
            items.retain(|item| {
                item.heading_index
                    .is_some_and(|index| headings_with_todos.contains(&index))
            });
        }

        self.outline_items = items;

        // Add document overview entry if there's preamble content or no headings
        // When filtering by todos, only show overview if it has todos
        let has_preamble = Self::has_preamble_content(&self.document);
        let preamble_has_todos = self.filter_by_todos
            && self
                .document
                .content
                .split_once("\n#")
                .is_some_and(|(preamble, _)| {
                    preamble.contains("- [ ]") || preamble.contains("* [ ]")
                });

        let show_preamble = (!self.filter_by_todos
            && (has_preamble || self.document.headings.is_empty()))
            || (self.filter_by_todos && preamble_has_todos);

        if show_preamble {
            self.outline_items.insert(
                0,
                OutlineItem {
                    level: 0,
                    text: DOCUMENT_OVERVIEW.to_string(),
                    expanded: true,
                    has_children: !self.outline_items.is_empty(),
                    heading_index: None,
                },
            );
        }
    }

    pub(super) fn flatten_tree(
        tree: &[HeadingNode],
        collapsed_headings: &HashSet<HeadingKey>,
        keys: &[HeadingKey],
    ) -> Vec<OutlineItem> {
        let mut items = Vec::new();

        fn flatten_recursive(
            node: &HeadingNode,
            items: &mut Vec<OutlineItem>,
            collapsed_headings: &HashSet<HeadingKey>,
            keys: &[HeadingKey],
        ) {
            let is_collapsed = keys
                .get(node.index)
                .is_some_and(|key| collapsed_headings.contains(key));
            let expanded = !is_collapsed;
            let has_children = !node.children.is_empty();

            items.push(OutlineItem {
                level: node.heading.level,
                text: node.heading.text.clone(),
                expanded,
                has_children,
                heading_index: Some(node.index),
            });

            // Only show children if this node is expanded
            if expanded {
                for child in &node.children {
                    flatten_recursive(child, items, collapsed_headings, keys);
                }
            }
        }

        for node in tree {
            flatten_recursive(node, &mut items, collapsed_headings, keys);
        }

        items
    }

    /// Select an outline item by index, updating both selection and scroll state.
    pub(super) fn select_outline_index(&mut self, idx: usize) {
        self.outline_state.select(Some(idx));
        self.outline_scroll_state = self.outline_scroll_state.position(idx);
    }

    /// Select a heading by its text. Returns true if found and selected.
    pub(super) fn select_by_text(&mut self, text: &str) -> bool {
        for (idx, item) in self.outline_items.iter().enumerate() {
            if item.text == text {
                self.select_outline_index(idx);
                return true;
            }
        }
        false
    }

    pub(super) fn select_by_heading_index(&mut self, heading_index: usize) -> bool {
        for (idx, item) in self.outline_items.iter().enumerate() {
            if item.heading_index == Some(heading_index) {
                self.select_outline_index(idx);
                return true;
            }
        }
        false
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
            self.select_outline_index(i);
        } else {
            // Scroll content - stop when last line is at viewport bottom
            self.scroll_content_down();
        }
    }

    pub fn previous(&mut self) {
        if self.focus == Focus::Outline {
            let i = match self.outline_state.selected() {
                Some(i) => i.saturating_sub(1),
                None => 0,
            };
            self.select_outline_index(i);
        } else {
            // Scroll content
            self.scroll_content_up();
        }
    }

    pub fn first(&mut self) {
        if self.mode == AppMode::FilePicker {
            let total = self.file_picker_item_count();
            if total > 0 {
                self.file_picker.selected = Some(0);
            }
        } else if self.focus == Focus::Outline && !self.outline_items.is_empty() {
            self.select_outline_index(0);
        } else {
            self.content_scroll = 0;
            self.content_scroll_state = self.content_scroll_state.position(0);
        }
    }

    pub fn last(&mut self) {
        if self.mode == AppMode::FilePicker {
            let total = self.file_picker_item_count();
            if total > 0 {
                self.file_picker.selected = Some(total - 1);
            }
        } else if self.focus == Focus::Outline && !self.outline_items.is_empty() {
            let last = self.outline_items.len() - 1;
            self.select_outline_index(last);
        } else {
            // Scroll to show the last line at the bottom of the viewport
            let max_scroll = self.max_content_scroll();
            self.content_scroll = max_scroll;
            self.content_scroll_state = self.content_scroll_state.position(max_scroll);
        }
    }

    pub fn jump_to_parent(&mut self) {
        // Works in both Outline and Content focus
        if let Some(current_idx) = self.outline_state.selected()
            && current_idx < self.outline_items.len()
        {
            let current_level = self.outline_items[current_idx].level;

            // Search backwards for a heading with lower level (parent)
            for i in (0..current_idx).rev() {
                if self.outline_items[i].level < current_level {
                    self.select_outline_index(i);
                    return;
                }
            }

            // If no parent found, stay at current position
            // (we're already at a top-level heading or first item)
        }
    }

    /// Move the outline selection by `rows`, clamped to the outline. With the
    /// outline focused, PageUp and PageDown did nothing at all.
    pub(super) fn page_outline(&mut self, rows: isize) {
        let Some(last) = self.outline_items.len().checked_sub(1) else {
            return;
        };
        let current = self.outline_state.selected().unwrap_or(0);
        self.select_outline_index(current.saturating_add_signed(rows).min(last));
    }

    pub fn toggle_expand(&mut self) {
        if self.focus == Focus::Outline
            && let Some(i) = self.outline_state.selected()
            && i < self.outline_items.len()
            && self.outline_items[i].has_children
        {
            let heading_text = self.outline_items[i].text.clone();
            let heading_idx = self.outline_items[i].heading_index;
            let Some(key) = heading_idx.and_then(|index| self.heading_key(index)) else {
                return;
            };

            // Toggle the collapsed state
            if !self.collapsed_headings.remove(&key) {
                self.collapsed_headings.insert(key);
            }

            // Rebuild the flattened list with overview entry
            self.rebuild_outline_items();

            let restored = heading_idx.is_some_and(|i| self.select_by_heading_index(i))
                || self.select_by_text(&heading_text);
            if !restored {
                let safe_idx = i.min(self.outline_items.len().saturating_sub(1));
                self.outline_state.select(Some(safe_idx));
                self.outline_scroll_state =
                    ScrollbarState::new(self.outline_items.len()).position(safe_idx);
            }
        }
    }

    pub fn expand(&mut self) {
        if self.focus == Focus::Outline
            && let Some(i) = self.outline_state.selected()
            && i < self.outline_items.len()
            && self.outline_items[i].has_children
        {
            let heading_text = self.outline_items[i].text.clone();
            let heading_idx = self.outline_items[i].heading_index;

            // Remove from collapsed set to expand
            if let Some(key) = heading_idx.and_then(|index| self.heading_key(index)) {
                self.collapsed_headings.remove(&key);
            }

            // Rebuild the flattened list with overview entry
            self.rebuild_outline_items();

            let restored = heading_idx.is_some_and(|i| self.select_by_heading_index(i))
                || self.select_by_text(&heading_text);
            if !restored {
                // If heading not found (shouldn't happen), clamp to valid index
                let safe_idx = i.min(self.outline_items.len().saturating_sub(1));
                self.outline_state.select(Some(safe_idx));
                self.outline_scroll_state =
                    ScrollbarState::new(self.outline_items.len()).position(safe_idx);
            }
        }
    }

    pub fn collapse(&mut self) {
        if self.focus == Focus::Outline
            && let Some(i) = self.outline_state.selected()
            && i < self.outline_items.len()
        {
            let current_level = self.outline_items[i].level;
            let current_text = self.outline_items[i].text.clone();
            let current_heading_idx = self.outline_items[i].heading_index;

            // If current heading has children, collapse it
            if self.outline_items[i].has_children {
                if let Some(key) = current_heading_idx.and_then(|index| self.heading_key(index)) {
                    self.collapsed_headings.insert(key);
                }

                // Rebuild the flattened list with overview entry
                self.rebuild_outline_items();

                let restored = current_heading_idx.is_some_and(|i| self.select_by_heading_index(i))
                    || self.select_by_text(&current_text);
                if !restored {
                    let safe_idx = i.min(self.outline_items.len().saturating_sub(1));
                    self.outline_state.select(Some(safe_idx));
                    self.outline_scroll_state =
                        ScrollbarState::new(self.outline_items.len()).position(safe_idx);
                }
            } else {
                // If no children, find parent and collapse it
                // Look backwards for first heading with lower level
                let mut parent_text: Option<String> = None;
                let mut parent_heading_idx: Option<usize> = None;
                for idx in (0..i).rev() {
                    if self.outline_items[idx].level < current_level {
                        // Found parent
                        parent_text = Some(self.outline_items[idx].text.clone());
                        parent_heading_idx = self.outline_items[idx].heading_index;
                        break;
                    }
                }

                if let Some(parent) = parent_text {
                    // Collapse the parent
                    if let Some(key) = parent_heading_idx.and_then(|index| self.heading_key(index))
                    {
                        self.collapsed_headings.insert(key);
                    }

                    // Rebuild and move selection to parent
                    self.rebuild_outline_items();

                    let restored = parent_heading_idx
                        .is_some_and(|i| self.select_by_heading_index(i))
                        || self.select_by_text(&parent);
                    if !restored && !self.outline_items.is_empty() {
                        self.outline_state.select(Some(0));
                        self.outline_scroll_state =
                            ScrollbarState::new(self.outline_items.len()).position(0);
                    }
                }
                // No parent found, do nothing
            }
        }
    }

    /// Collapse all headings that have children
    pub fn collapse_all(&mut self) {
        // Collect all headings that have children
        let keys = heading_keys(&self.document);
        let headings_to_collapse: Vec<usize> = self
            .tree
            .iter()
            .flat_map(Self::collect_collapsible_headings)
            .collect();

        for index in headings_to_collapse {
            if let Some(key) = keys.get(index) {
                self.collapsed_headings.insert(key.clone());
            }
        }

        // Rebuild outline and preserve selection
        let selected_offset = self.selected_heading_index();
        let selected_text = self.selected_heading_text().map(|s| s.to_string());
        self.rebuild_outline_items();

        // Try to restore selection, or select first item
        let restored = selected_offset.is_some_and(|o| self.select_by_heading_index(o))
            || selected_text.is_some_and(|t| self.select_by_text(&t));
        if !restored && !self.outline_items.is_empty() {
            self.outline_state.select(Some(0));
            self.outline_scroll_state = ScrollbarState::new(self.outline_items.len()).position(0);
        }

        let count = self.collapsed_headings.len();
        self.set_status_message(&format!("Collapsed {} headings", count));
    }

    /// Recursively collect all headings that have children
    pub(super) fn collect_collapsible_headings(node: &HeadingNode) -> Vec<usize> {
        let mut result = Vec::new();
        if !node.children.is_empty() {
            result.push(node.index);
            for child in &node.children {
                result.extend(Self::collect_collapsible_headings(child));
            }
        }
        result
    }

    /// Expand all headings
    pub fn expand_all(&mut self) {
        let count = self.collapsed_headings.len();
        self.collapsed_headings.clear();

        // Rebuild outline and preserve selection
        let selected_offset = self.selected_heading_index();
        let selected_text = self.selected_heading_text().map(|s| s.to_string());
        self.rebuild_outline_items();

        if !(selected_offset.is_some_and(|o| self.select_by_heading_index(o)))
            && let Some(text) = selected_text
        {
            self.select_by_text(&text);
        }

        self.set_status_message(&format!("Expanded {} headings", count));
    }

    /// Collapse all headings at a specific level (1-6)
    pub fn collapse_level(&mut self, level: usize) {
        // Collect all headings at the target level that have children
        let keys = heading_keys(&self.document);
        let headings_to_collapse: Vec<usize> = self
            .tree
            .iter()
            .flat_map(|node| Self::collect_headings_at_level_with_children(node, level))
            .collect();

        let count = headings_to_collapse.len();
        for index in headings_to_collapse {
            if let Some(key) = keys.get(index) {
                self.collapsed_headings.insert(key.clone());
            }
        }

        // Rebuild outline and preserve selection
        let selected_offset = self.selected_heading_index();
        let selected_text = self.selected_heading_text().map(|s| s.to_string());
        self.rebuild_outline_items();

        let restored = selected_offset.is_some_and(|o| self.select_by_heading_index(o))
            || selected_text.is_some_and(|t| self.select_by_text(&t));
        if !restored && !self.outline_items.is_empty() {
            self.outline_state.select(Some(0));
            self.outline_scroll_state = ScrollbarState::new(self.outline_items.len()).position(0);
        }

        self.set_status_message(&format!("Collapsed {} h{} headings", count, level));
    }

    /// Recursively collect headings at a specific level that have children
    pub(super) fn collect_headings_at_level_with_children(
        node: &HeadingNode,
        target_level: usize,
    ) -> Vec<usize> {
        let mut result = Vec::new();

        if node.heading.level == target_level && !node.children.is_empty() {
            result.push(node.index);
        }

        // Always recurse to find nested headings at the target level
        for child in &node.children {
            result.extend(Self::collect_headings_at_level_with_children(
                child,
                target_level,
            ));
        }

        result
    }

    /// Expand all headings at a specific level (1-6)
    pub fn expand_level(&mut self, level: usize) {
        let mut count = 0;

        // Find all collapsed headings at the specified level and expand them
        let keys = heading_keys(&self.document);
        let headings_at_level: Vec<usize> = self
            .tree
            .iter()
            .flat_map(|node| self.collect_headings_at_level(node, level))
            .collect();

        for index in headings_at_level {
            if keys
                .get(index)
                .is_some_and(|key| self.collapsed_headings.remove(key))
            {
                count += 1;
            }
        }

        // Rebuild outline and preserve selection
        let selected_offset = self.selected_heading_index();
        let selected_text = self.selected_heading_text().map(|s| s.to_string());
        self.rebuild_outline_items();

        if !(selected_offset.is_some_and(|o| self.select_by_heading_index(o)))
            && let Some(text) = selected_text
        {
            self.select_by_text(&text);
        }

        self.set_status_message(&format!("Expanded {} h{} headings", count, level));
    }

    /// Collect heading texts at a specific level
    pub(super) fn collect_headings_at_level(
        &self,
        node: &HeadingNode,
        target_level: usize,
    ) -> Vec<usize> {
        let mut result = Vec::new();

        if node.heading.level == target_level {
            result.push(node.index);
        }

        for child in &node.children {
            result.extend(self.collect_headings_at_level(child, target_level));
        }

        result
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

    /// Toggle heading level markers (#, ##, ###) in the outline sidebar
    pub fn toggle_heading_markers(&mut self) {
        self.show_heading_markers = !self.show_heading_markers;
        let state = if self.show_heading_markers {
            "ON"
        } else {
            "OFF"
        };
        self.set_status_message(&format!("Heading markers: {}", state));
    }

    /// Toggle filtering outline by open todos
    pub fn toggle_todo_filter(&mut self) {
        self.filter_by_todos = !self.filter_by_todos;

        // Rebuild outline with/without filter
        let selected_offset = self.selected_heading_index();
        let selected_text = self.selected_heading_text().map(|s| s.to_string());
        self.rebuild_outline_items();

        // Try to restore selection
        let restored = selected_offset.is_some_and(|o| self.select_by_heading_index(o))
            || selected_text.is_some_and(|t| self.select_by_text(&t));
        if !restored && !self.outline_items.is_empty() {
            self.outline_state.select(Some(0));
            self.outline_scroll_state = ScrollbarState::new(self.outline_items.len()).position(0);
        }

        // Set status message
        if self.filter_by_todos {
            let count = self.outline_items.len();
            self.set_status_message(&format!(
                "Todo filter ON: {} heading{} with open todos",
                count,
                if count == 1 { "" } else { "s" }
            ));
        } else {
            self.set_status_message("Todo filter OFF: showing all headings");
        }
    }

    /// Cycle outline width between 20%, 30%, and 40%.
    ///
    /// Behavior depends on user's config:
    /// - **New users** (default or standard width in config): Changes are persisted
    ///   to config file for a seamless experience.
    /// - **Power users** (custom width like 25% in config): Changes are session-only
    ///   to protect their carefully crafted config from accidental overwrites.
    ///   They can explicitly save with `S` key.
    ///
    /// This respects the principle that user config should always take precedence.
    pub fn cycle_outline_width(&mut self, increase: bool) {
        if increase {
            self.outline_width = match self.outline_width {
                20 => 30,
                30 => 40,
                40 => 20, // Wrap around
                // For custom widths, snap to nearest standard value going up
                w if w < 25 => 30,
                w if w < 35 => 40,
                _ => 20,
            };
        } else {
            self.outline_width = match self.outline_width {
                40 => 30,
                30 => 20,
                20 => 40, // Wrap around
                // For custom widths, snap to nearest standard value going down
                w if w > 35 => 30,
                w if w > 25 => 20,
                _ => 40,
            };
        }

        // Decide whether to persist based on user's config type
        if self.config_has_custom_outline_width {
            // Power user: protect their custom config value, offer explicit save
            self.set_status_message(&format!("Width: {}% | :w to save", self.outline_width));
        } else {
            // New user or standard config: safe to persist for better UX
            let _ = self.config.set_outline_width(self.outline_width);
            self.set_status_message(&format!("Width: {}%", self.outline_width));
        }
    }

    /// Confirm and save outline width to config file.
    pub fn confirm_save_outline_width(&mut self) {
        match self.config.set_outline_width(self.outline_width) {
            Ok(_) => {
                // Update the flag since user explicitly chose to save
                self.config_has_custom_outline_width = self.outline_width != 20
                    && self.outline_width != 30
                    && self.outline_width != 40;
                self.set_status_message(&format!(
                    "✓ Width {}% saved to config",
                    self.outline_width
                ));
            }
            Err(e) => {
                self.set_status_message(&format!("✗ Failed to save: {}", e));
            }
        }
        self.mode = AppMode::Normal;
    }

    /// Cancel the save width confirmation modal.
    pub fn cancel_save_width_confirmation(&mut self) {
        self.mode = AppMode::Normal;
        self.set_status_message("Save cancelled");
    }

    pub fn jump_to_heading(&mut self, index: usize) {
        if index < self.outline_items.len() {
            self.select_outline_index(index);
        }
    }

    pub fn set_bookmark(&mut self) {
        self.bookmark_position = match self.selected_heading_index() {
            Some(index) => self.heading_key(index),
            // The document overview, which no heading shares a key with.
            None => self
                .selected_heading_text()
                .map(|text| (text.to_string(), 0)),
        };
    }

    pub fn jump_to_bookmark(&mut self) {
        let Some(key) = self.bookmark_position.clone() else {
            return;
        };
        match heading_keys(&self.document).iter().position(|k| *k == key) {
            Some(index) => {
                self.reveal_heading(index);
                self.select_by_heading_index(index);
            }
            None => {
                self.select_by_text(&key.0);
            }
        }
    }

    /// Whether `item` is the bookmarked heading.
    pub(crate) fn is_bookmarked(&self, item: &OutlineItem) -> bool {
        let Some(key) = &self.bookmark_position else {
            return false;
        };
        if key.0 != item.text {
            return false;
        }
        match item.heading_index {
            Some(index) => self.heading_key(index).as_ref() == Some(key),
            None => true,
        }
    }

    /// The [`HeadingKey`] of heading `index`.
    pub(super) fn heading_key(&self, index: usize) -> Option<HeadingKey> {
        let text = &self.document.headings.get(index)?.text;
        let before = self.document.headings[..index]
            .iter()
            .filter(|heading| heading.text == *text)
            .count();
        Some((text.clone(), before))
    }

    /// Expand every collapsed ancestor of heading `index`, so its outline row
    /// exists to be selected.
    pub(super) fn reveal_heading(&mut self, index: usize) {
        let Some(level) = self.document.headings.get(index).map(|h| h.level) else {
            return;
        };
        let keys = heading_keys(&self.document);
        let mut deepest = level;
        let mut changed = false;
        for ancestor in (0..index).rev() {
            let ancestor_level = self.document.headings[ancestor].level;
            if ancestor_level < deepest {
                deepest = ancestor_level;
                changed |= self.collapsed_headings.remove(&keys[ancestor]);
            }
        }
        if changed {
            self.rebuild_outline_items();
        }
    }

    pub fn selected_heading_text(&self) -> Option<&str> {
        self.outline_state
            .selected()
            .and_then(|i| self.outline_items.get(i))
            .map(|item| item.text.as_str())
    }

    pub fn selected_heading_index(&self) -> Option<usize> {
        self.outline_state
            .selected()
            .and_then(|i| self.outline_items.get(i))
            .and_then(|item| item.heading_index)
    }
}

#[cfg(test)]
mod outline_integration_tests {
    use super::*;
    use crate::Document;
    use crate::parser::parse_markdown;
    use std::collections::HashSet;

    fn make_doc(content: &str) -> Document {
        parse_markdown(content)
    }

    #[test]
    fn duplicate_headings_resolve_to_correct_content() {
        let content = "\
# Chapter 1
chapter 1 body

## Details
details for chapter 1

# Chapter 2
chapter 2 body

## Details
details for chapter 2
";
        let doc = make_doc(content);
        let tree = doc.build_tree();
        let items = App::flatten_tree(&tree, &HashSet::new(), &heading_keys(&doc));

        assert_eq!(items.len(), 4);
        assert_eq!(items[1].text, "Details");
        assert_eq!(items[3].text, "Details");
        assert_ne!(items[1].heading_index, items[3].heading_index);

        let first = doc
            .extract_section_at_index(items[1].heading_index.unwrap())
            .unwrap();
        assert!(first.contains("details for chapter 1"));
        assert!(!first.contains("details for chapter 2"));

        let second = doc
            .extract_section_at_index(items[3].heading_index.unwrap())
            .unwrap();
        assert!(second.contains("details for chapter 2"));
        assert!(!second.contains("details for chapter 1"));
    }

    #[test]
    fn collapsed_parent_hides_children_but_indices_stay_correct() {
        let content = "\
# A
body a

## Sub A
sub a content

# B
body b

## Sub B
sub b content
";
        let doc = make_doc(content);
        let tree = doc.build_tree();

        let mut collapsed = HashSet::new();
        collapsed.insert(("A".to_string(), 0));
        let items = App::flatten_tree(&tree, &collapsed, &heading_keys(&doc));

        // Sub A hidden, Sub B still accessible
        assert_eq!(items.len(), 3);
        assert_eq!(items[2].text, "Sub B");

        let sub_b = doc
            .extract_section_at_index(items[2].heading_index.unwrap())
            .unwrap();
        assert!(sub_b.contains("sub b content"));
        assert!(!sub_b.contains("sub a content"));
    }
}
