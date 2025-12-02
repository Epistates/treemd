//! Dynamic layout builder for flexible UI composition
//!
//! Provides a builder pattern for creating layouts that can show/hide sections
//! dynamically based on application state.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::collections::HashMap;

/// A dynamic layout that maps section IDs to their rendered areas
pub struct DynamicLayout {
    areas: HashMap<&'static str, Rect>,
}

impl DynamicLayout {
    /// Start building a vertical layout
    pub fn vertical(area: Rect) -> DynamicLayoutBuilder {
        DynamicLayoutBuilder::new(area, Direction::Vertical)
    }

    /// Start building a horizontal layout
    #[allow(dead_code)]
    pub fn horizontal(area: Rect) -> DynamicLayoutBuilder {
        DynamicLayoutBuilder::new(area, Direction::Horizontal)
    }

    /// Get the area for a section by ID
    pub fn get(&self, id: &str) -> Option<Rect> {
        self.areas.get(id).copied()
    }

    /// Get the area for a section, panicking if not found (use when section is always visible)
    pub fn require(&self, id: &str) -> Rect {
        self.areas
            .get(id)
            .copied()
            .unwrap_or_else(|| panic!("Required layout section '{}' not found", id))
    }
}

/// Builder for creating dynamic layouts
pub struct DynamicLayoutBuilder {
    area: Rect,
    direction: Direction,
    sections: Vec<LayoutSection>,
}

struct LayoutSection {
    id: &'static str,
    constraint: Constraint,
    visible: bool,
}

impl DynamicLayoutBuilder {
    fn new(area: Rect, direction: Direction) -> Self {
        Self {
            area,
            direction,
            sections: Vec::new(),
        }
    }

    /// Add a section that is always visible
    pub fn section(mut self, id: &'static str, constraint: Constraint) -> Self {
        self.sections.push(LayoutSection {
            id,
            constraint,
            visible: true,
        });
        self
    }

    /// Add a section that is conditionally visible
    pub fn section_if(mut self, visible: bool, id: &'static str, constraint: Constraint) -> Self {
        self.sections.push(LayoutSection {
            id,
            constraint,
            visible,
        });
        self
    }

    /// Build the final layout
    pub fn build(self) -> DynamicLayout {
        // Build constraints only for visible sections
        let constraints: Vec<Constraint> = self
            .sections
            .iter()
            .filter(|s| s.visible)
            .map(|s| s.constraint)
            .collect();

        let chunks = Layout::default()
            .direction(self.direction)
            .constraints(constraints)
            .split(self.area);

        // Map visible sections to their rects
        let mut areas = HashMap::new();
        let mut chunk_idx = 0;

        for section in &self.sections {
            if section.visible {
                areas.insert(section.id, chunks[chunk_idx]);
                chunk_idx += 1;
            }
        }

        DynamicLayout { areas }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_sections_visible() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = DynamicLayout::vertical(area)
            .section("header", Constraint::Length(2))
            .section("content", Constraint::Min(0))
            .section("footer", Constraint::Length(1))
            .build();

        assert!(layout.get("header").is_some());
        assert!(layout.get("content").is_some());
        assert!(layout.get("footer").is_some());
    }

    #[test]
    fn test_conditional_section_hidden() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = DynamicLayout::vertical(area)
            .section("header", Constraint::Length(2))
            .section_if(false, "search", Constraint::Length(3))
            .section("content", Constraint::Min(0))
            .build();

        assert!(layout.get("header").is_some());
        assert!(layout.get("search").is_none());
        assert!(layout.get("content").is_some());
    }

    #[test]
    fn test_conditional_section_visible() {
        let area = Rect::new(0, 0, 100, 50);
        let layout = DynamicLayout::vertical(area)
            .section("header", Constraint::Length(2))
            .section_if(true, "search", Constraint::Length(3))
            .section("content", Constraint::Min(0))
            .build();

        assert!(layout.get("header").is_some());
        assert!(layout.get("search").is_some());
        assert!(layout.get("content").is_some());
    }
}
