//! Interactive mode: selecting, activating and copying elements.

use super::*;

impl App {
    /// Index interactive elements, passing the mermaid placeholder-rows cache when available.
    ///
    /// Centralises the cfg-gated map extraction so callers don't repeat the pattern.
    pub(crate) fn index_interactive_elements(&mut self, blocks: &[crate::parser::output::Block]) {
        #[cfg(all(feature = "mermaid", unix))]
        let rows = self.mermaid_placeholder_rows.clone();
        #[cfg(not(all(feature = "mermaid", unix)))]
        let rows: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
        self.interactive_state.index_elements(blocks, &rows);
    }

    /// Select the first or the last interactive element, and show it.
    pub(super) fn select_interactive_end(&mut self, last: bool) {
        let Some(final_index) = self.interactive_state.elements.len().checked_sub(1) else {
            return;
        };
        self.interactive_state.current_index = Some(if last { final_index } else { 0 });
        self.scroll_to_interactive_element(self.content_viewport_height);
    }

    /// Select the interactive element nearest the top of the view: the first
    /// one drawn on or below the top row, or failing that the last one.
    ///
    /// Elements are compared by the rows they are drawn on. Their counted
    /// lines know nothing of wrapping, and compared with the scroll row,
    /// which does, below a few wrapped paragraphs `i` picked an element
    /// pages further down and scrolled away from what was on screen.
    pub(super) fn select_element_at_scroll(&mut self) {
        let top = self.content_scroll;
        let starts: Option<Vec<usize>> = self
            .interactive_state
            .elements
            .iter()
            .map(|element| self.element_rows(element).map(|(start, _)| start))
            .collect();
        let Some(starts) = starts else {
            // Not laid out yet; the counted lines are the best there is.
            self.interactive_state.enter_at_scroll_position(top);
            return;
        };
        self.interactive_state.current_index = starts
            .iter()
            .position(|&start| start >= top)
            .or_else(|| starts.len().checked_sub(1));
        self.scroll_to_interactive_element(self.content_viewport_height);
    }

    /// Scroll page down in interactive mode (bypasses focus check)
    pub fn scroll_page_down_interactive(&mut self) {
        let page = usize::from(self.content_viewport_height.saturating_sub(2).max(1));
        let new_scroll = self.content_scroll.saturating_add(page);
        self.content_scroll = new_scroll.min(self.max_content_scroll());
        self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);
    }

    /// Scroll page up in interactive mode (bypasses focus check)
    pub fn scroll_page_up_interactive(&mut self) {
        let page = usize::from(self.content_viewport_height.saturating_sub(2).max(1));
        self.content_scroll = self.content_scroll.saturating_sub(page);
        self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);
    }

    /// Auto-scroll to keep the selected interactive element in view.
    ///
    /// Only asks for it: the rows the element is drawn on are known once the
    /// frame has laid the section out, so the scroll is applied there, by
    /// `apply_element_scroll`. Computing it here used the previous
    /// frame's layout, which is wrong right after anything that changes it,
    /// such as expanding a `<details>`.
    pub fn scroll_to_interactive_element(&mut self, _viewport_height: u16) {
        self.pending_element_scroll = true;
    }

    /// Scroll the selected element into view, if a scroll was asked for,
    /// against the layout of the frame being drawn.
    pub(crate) fn apply_element_scroll(&mut self) {
        if !std::mem::take(&mut self.pending_element_scroll) {
            return;
        }
        let Some((start, end)) = self.current_element_rows() else {
            return;
        };
        let viewport = usize::from(self.content_viewport_height.max(1));
        let scroll = self.content_scroll;

        // Add margin for smoother scrolling - trigger before element goes completely off-screen
        let margin = 2.min(viewport / 4);

        // Element is above viewport (or too close to top margin) - scroll up
        if start < scroll + margin {
            self.content_scroll = start.saturating_sub(margin);
        }
        // Element end is below viewport (or within bottom margin) - scroll down
        else if end + margin > scroll + viewport {
            // Position so the element's end is near the bottom, but never
            // past its start: a tall element shows its top.
            let to_end = (end + margin).saturating_sub(viewport);
            self.content_scroll = to_end.min(start.saturating_sub(margin));
        }
        self.content_scroll = self.content_scroll.min(self.max_content_scroll());
    }

    /// The screen rows the given element is drawn on, in the current layout.
    pub(crate) fn element_rows(
        &self,
        element: &crate::tui::interactive::InteractiveElement,
    ) -> Option<(usize, usize)> {
        let counted = *self
            .interactive_state
            .block_lines
            .get(element.id.block_idx)?;
        self.content_view
            .element_rows(element.id.block_idx, counted, element.line_range)
    }

    /// The screen rows of the selected element, in the current layout.
    pub(crate) fn current_element_rows(&self) -> Option<(usize, usize)> {
        self.element_rows(self.interactive_state.current_element()?)
    }

    /// Get the source line number (1-indexed) for the current interactive element.
    ///
    /// Computes the source line by adding the element's rendered line offset
    /// to the section's starting line in the source file.
    pub fn interactive_element_source_line(&self) -> Option<u32> {
        let (element_line, _) = self.interactive_state.current_element_line_range()?;
        u32::try_from(self.displayed_text_first_line() + element_line).ok()
    }

    pub fn copy_content(&mut self) {
        // In interactive mode `y` copies the selected element, as the help
        // and the status line say. It copied the whole section.
        if self.mode == AppMode::Interactive
            && let Some((text, what)) = self.current_element_text()
        {
            self.status_message = Some(match self.copy_to_clipboard(&text) {
                Ok(()) => format!("✓ {what} copied to clipboard"),
                Err(e) => format!("✗ {e}"),
            });
            return;
        }
        let content = self.current_section_content();
        if let Some(clipboard) = &mut self.clipboard {
            match clipboard.set_text(content) {
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
    }

    pub fn copy_anchor(&mut self) {
        // Copy the anchor link for the currently selected heading, in the
        // configured slug style. GitHub's numbers repeated headings, so the
        // slug depends on the headings before it.
        if let Some(index) = self.selected_heading_index() {
            let anchor = self
                .document
                .heading_slugs(self.config.content.slug_style)
                .swap_remove(index);
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

    /// Enter interactive mode - build element index and enter mode
    pub fn enter_interactive_mode(&mut self) {
        // Exit raw source view if active (interactive elements aren't visible in raw mode)
        if self.show_raw_source {
            self.show_raw_source = false;
        }

        // Get current section content to index
        let content = self.displayed_section_content().to_string();

        // Parse content into blocks
        use crate::parser::content::parse_content;
        let blocks = parse_content(&content, 0);

        // Index interactive elements
        self.index_interactive_elements(&blocks);
        self.populate_image_cache();

        // Enter interactive mode at current scroll position (preserve user's view)
        self.select_element_at_scroll();
        self.mode = AppMode::Interactive;

        // Only scroll if the selected element is not fully visible
        self.scroll_to_interactive_element(self.content_viewport_height);

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
                if self.images_enabled {
                    // Open image modal to view the image fullscreen
                    self.open_image_modal(src);
                    self.status_message = Some(format!("📸 Viewing: {} (Esc:Close)", alt));
                } else {
                    self.status_message =
                        Some("Images disabled (use --images or config to enable)".to_string());
                }
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
    pub fn reindex_interactive_elements(&mut self) {
        let content = self.displayed_section_content().to_string();

        use crate::parser::content::parse_content;
        let blocks = parse_content(&content, 0);
        self.index_interactive_elements(&blocks);
        self.populate_image_cache();
    }

    /// Follow a link from interactive mode
    pub(super) fn follow_link_from_interactive(
        &mut self,
        link: &crate::parser::Link,
    ) -> Result<(), String> {
        use crate::parser::LinkTarget;

        match &link.target {
            LinkTarget::Anchor(anchor) => {
                // Jump to heading in current document
                self.jump_to_anchor(anchor)?;
                self.exit_interactive_mode();
                self.status_message = Some(format!("✓ Jumped to #{}", anchor));
                Ok(())
            }
            LinkTarget::RelativeFile { path, anchor } => {
                if self.resolve_relative_file_link(path, anchor)? {
                    self.exit_interactive_mode();
                }
                Ok(())
            }
            LinkTarget::WikiLink { target, .. } => {
                // Try to find and load the wikilinked file
                self.load_wikilink(target)?;
                // Only exit interactive mode if we're not prompting for file creation
                if self.mode != AppMode::ConfirmFileCreate {
                    self.exit_interactive_mode();
                }
                Ok(())
            }
            LinkTarget::External(url) => {
                if !is_openable_url(url) {
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

    /// The selected interactive element as text to copy, and what it is: a
    /// link's target, a code block's code, a table as markdown, an image's
    /// source, a task's text, a details block's summary.
    pub(super) fn current_element_text(&self) -> Option<(String, &'static str)> {
        use crate::tui::interactive::ElementType;

        let element = self.interactive_state.current_element()?;
        Some(match &element.element_type {
            ElementType::Link { link, .. } => (link.target.as_str(), "Link"),
            ElementType::CodeBlock { content, .. } => (content.clone(), "Code"),
            ElementType::Table { .. } => (self.table_as_markdown()?, "Table"),
            ElementType::Image { src, .. } => (src.clone(), "Image source"),
            ElementType::Checkbox { content, .. } => (content.clone(), "Task"),
            ElementType::Details { summary, .. } => (summary.clone(), "Summary"),
        })
    }

    /// The selected table as markdown.
    pub(super) fn table_as_markdown(&self) -> Option<String> {
        let (headers, rows) = self.get_current_table_data()?;
        let row = |cells: &[String]| format!("| {} |\n", cells.join(" | "));
        let mut table = row(&headers);
        table.push_str(&row(&vec!["---".to_string(); headers.len()]));
        for cells in &rows {
            table.push_str(&row(cells));
        }
        Some(table)
    }

    /// Copy text to clipboard
    pub(super) fn copy_to_clipboard(&mut self, text: &str) -> Result<(), String> {
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
    pub(super) fn get_current_table_data(&self) -> Option<(Vec<String>, Vec<Vec<String>>)> {
        if let Some(element) = self.interactive_state.current_element()
            && let crate::tui::interactive::ElementType::Table { block_idx, .. } =
                &element.element_type
        {
            // Parse current section to get table data
            let content = self.compute_displayed_section();

            use crate::parser::content::parse_content;
            let blocks = parse_content(&content, 0);

            if let Some(crate::parser::output::Block::Table { headers, rows, .. }) =
                blocks.get(*block_idx)
            {
                return Some((headers.clone(), rows.clone()));
            }
        }
        None
    }

    /// Copy table cell to clipboard
    pub fn copy_table_cell(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data()
            && let Some(cell) = self.interactive_state.get_table_cell(&headers, &rows)
        {
            self.copy_to_clipboard(&cell)?;
            self.status_message = Some(format!("✓ Cell copied: {}", cell));
            return Ok(());
        }
        Err("No cell selected".to_string())
    }

    /// Copy table row to clipboard (tab-separated)
    pub fn copy_table_row(&mut self) -> Result<(), String> {
        if let Some((headers, rows)) = self.get_current_table_data()
            && let Some(row) = self.interactive_state.get_table_row(&headers, &rows)
        {
            let row_text = row.join("\t");
            self.copy_to_clipboard(&row_text)?;
            self.status_message = Some("✓ Row copied (tab-separated)".to_string());
            return Ok(());
        }
        Err("No row selected".to_string())
    }

    /// Copy entire table as markdown
    pub fn copy_table_markdown(&mut self) -> Result<(), String> {
        let table = self.table_as_markdown().ok_or("No table data available")?;
        self.copy_to_clipboard(&table)?;
        self.status_message = Some("✓ Table copied as markdown".to_string());
        Ok(())
    }
}
