//! The outline as a popup over the whole document.
//!
//! The content always shows the whole document. Moving through the open popup
//! scrolls it to each heading, a little below the top so what comes before
//! stays in view, its section highlighted; Enter keeps it there, the
//! highlight lasting a moment longer, and Esc goes back to where it was.

use super::*;
use crate::parser::content::parse_content;
use crate::parser::output::Block as ContentBlock;
use std::time::{Duration, Instant};

/// Where to scroll the whole document once it is laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PopupScroll {
    /// To this heading, by its index in the document, while choosing.
    Heading(usize),
    /// To this heading, chosen: it is highlighted for a moment.
    Jump(usize),
    /// Back to this row.
    Row(usize),
}

/// How long the section jumped to stays highlighted.
const JUMP_HIGHLIGHT: Duration = Duration::from_millis(600);

impl App {
    /// Whether the outline is showing as a popup over the content.
    pub fn is_outline_popup_open(&self) -> bool {
        self.outline_popup && self.show_outline
    }

    /// Open the popup on the heading at the top of the screen, leaving the
    /// document where it is.
    pub(super) fn open_outline_popup(&mut self) {
        self.popup_return_row = self.content_scroll;
        let current = self.heading_at_row(self.content_scroll);
        self.show_outline = true;
        self.focus = Focus::Outline;
        if let Some(index) = current {
            self.select_by_heading_index(index);
        }
        // Selecting where the document already is moves nothing
        self.sync_previous_selection();
    }

    /// Close the popup and show the whole document again, at the selected
    /// heading when `jump`, else back where it was.
    pub(super) fn close_outline_popup(&mut self, jump: bool) {
        let scroll = if jump {
            self.selected_heading_index()
                .map_or(PopupScroll::Row(0), PopupScroll::Jump)
        } else {
            PopupScroll::Row(self.popup_return_row)
        };
        self.clear_outline_search();
        // The whole document is the first entry in popup mode
        self.select_outline_index(0);
        self.pending_popup_scroll = Some(scroll);
        self.show_outline = false;
        self.focus = Focus::Content;
    }

    /// Scroll to the heading selected in the open popup. Once it has closed
    /// the scroll it asked for stands.
    pub(super) fn scroll_to_popup_selection(&mut self) {
        if self.is_outline_popup_open() {
            self.pending_popup_scroll = Some(
                self.selected_heading_index()
                    .map_or(PopupScroll::Row(0), PopupScroll::Heading),
            );
        }
    }

    /// Scroll as asked, now the whole document is laid out.
    pub(crate) fn apply_popup_scroll(&mut self) {
        let Some(scroll) = self.pending_popup_scroll.take() else {
            return;
        };
        let row = match scroll {
            PopupScroll::Heading(index) | PopupScroll::Jump(index) => {
                let rows = self.section_rows(index);
                let until =
                    matches!(scroll, PopupScroll::Jump(_)).then(|| Instant::now() + JUMP_HIGHLIGHT);
                let start = rows.start;
                self.section_highlight = Some((rows, until));
                // A quarter of the screen above the heading shows what leads to it
                let context = usize::from(self.content_viewport_height / 4);
                start.saturating_sub(context)
            }
            PopupScroll::Row(row) => {
                self.section_highlight = None;
                row
            }
        };
        self.content_scroll = row.min(self.max_content_scroll());
    }

    /// How long the jumped-to section stays highlighted, zero once it is due
    /// to go, `None` when no highlight is timed.
    pub fn section_highlight_expires_in(&self) -> Option<Duration> {
        let (_, until) = self.section_highlight.as_ref()?;
        until.map(|until| until.saturating_duration_since(Instant::now()))
    }

    /// The rows to highlight, if the highlight has not run out.
    pub(crate) fn section_highlight_rows(&mut self) -> Option<std::ops::Range<usize>> {
        if self.section_highlight_expires_in() == Some(Duration::ZERO) {
            self.section_highlight = None;
        }
        self.section_highlight
            .as_ref()
            .map(|(rows, _)| rows.clone())
    }

    /// The row each heading starts on, in document order.
    fn heading_starts(&self) -> Vec<usize> {
        let view = &self.content_view;
        parse_content(&self.compute_displayed_section(), 0)
            .iter()
            .zip(&view.block_lines)
            .filter(|(block, _)| matches!(block, ContentBlock::Heading { .. }))
            .filter_map(|(_, &line)| view.line_rows.get(line).copied())
            .collect()
    }

    /// The rows of a heading's section: from its heading to the next heading
    /// at its level or above, its subsections included.
    fn section_rows(&self, index: usize) -> std::ops::Range<usize> {
        let starts = self.heading_starts();
        let Some(&start) = starts.get(index) else {
            return 0..0;
        };
        let headings = &self.document.headings;
        let level = headings.get(index).map_or(1, |heading| heading.level);
        let end = (index + 1..starts.len())
            .find(|&next| {
                headings
                    .get(next)
                    .is_some_and(|heading| heading.level <= level)
            })
            .map_or(self.content_view.rows.len(), |next| starts[next]);
        start..end
    }

    /// The heading whose section the given row of the whole document is in.
    fn heading_at_row(&self, row: usize) -> Option<usize> {
        self.heading_starts()
            .iter()
            .rposition(|&start| start <= row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::Action;

    fn popup_app() -> App {
        let mut config = Config::default();
        config.ui.outline_popup = true;
        App::new(
            crate::parser::parse_markdown("# One\n\none\n\n# Two\n\ntwo\n\n# Three\n\nthree\n"),
            "t.md".into(),
            PathBuf::from("t.md"),
            config,
            ColorMode::Rgb,
            false,
        )
    }

    /// The popup starts closed, on the whole document, keys on the content.
    #[test]
    fn popup_starts_closed_on_the_whole_document() {
        let app = popup_app();
        assert!(!app.is_outline_popup_open());
        assert_eq!(app.focus, Focus::Content);
        assert_eq!(app.selected_heading_text(), Some(DOCUMENT_OVERVIEW));
    }

    /// Enter goes back to the whole document, to scroll to the chosen heading.
    #[test]
    fn enter_shows_the_whole_document_at_the_heading() {
        let mut app = popup_app();
        app.execute_action(Action::ToggleOutline);
        assert!(app.is_outline_popup_open());
        assert_eq!(app.focus, Focus::Outline);

        app.execute_action(Action::Next);
        app.execute_action(Action::Next);
        assert_eq!(app.selected_heading_text(), Some("Two"));
        app.execute_action(Action::ToggleExpand);

        assert!(!app.is_outline_popup_open());
        assert_eq!(app.focus, Focus::Content);
        assert_eq!(app.selected_heading_text(), Some(DOCUMENT_OVERVIEW));
        assert_eq!(app.pending_popup_scroll, Some(PopupScroll::Jump(1)));
    }

    /// Esc goes back to the whole document where it was.
    #[test]
    fn escape_returns_to_where_it_was() {
        let mut app = popup_app();
        app.content_scroll = 3;
        app.execute_action(Action::ToggleOutline);
        app.execute_action(Action::Next);
        app.execute_action(Action::ExitMode);
        assert!(!app.is_outline_popup_open());
        assert_eq!(app.selected_heading_text(), Some(DOCUMENT_OVERVIEW));
        assert_eq!(app.pending_popup_scroll, Some(PopupScroll::Row(3)));
    }

    /// Tab, which moves to the outline in the sidebar, opens and closes the
    /// popup, as the footer says.
    #[test]
    fn tab_opens_and_closes_the_popup() {
        let mut app = popup_app();
        app.execute_action(Action::ToggleFocus);
        assert!(app.is_outline_popup_open());
        assert_eq!(app.focus, Focus::Outline);
        app.execute_action(Action::ToggleFocus);
        assert!(!app.is_outline_popup_open());
        assert_eq!(app.focus, Focus::Content);
    }

    /// The sidebar is untouched: Enter still folds there.
    #[test]
    fn sidebar_enter_still_folds() {
        let mut app = App::new(
            crate::parser::parse_markdown("# One\n\n## Sub\n\ntext\n"),
            "t.md".into(),
            PathBuf::from("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        let before = app.outline_items.len();
        app.execute_action(Action::ToggleExpand);
        assert!(app.show_outline);
        assert_ne!(app.outline_items.len(), before);
    }
}
