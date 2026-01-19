//! Document model for markdown files.
//!
//! This module defines the core data structures for representing
//! markdown documents and their heading hierarchy.

use indextree::{Arena, NodeId};
use serde::Serialize;

/// A markdown document with its content and structure.
///
/// Contains the original markdown content and a list of extracted headings.
#[derive(Debug, Clone)]
pub struct Document {
    pub content: String,
    pub headings: Vec<Heading>,
}

/// A heading in a markdown document.
///
/// Represents a single heading with its level (1-6), text content, and byte position.
#[derive(Debug, Clone, Serialize)]
pub struct Heading {
    /// Heading level (1 for #, 2 for ##, etc.)
    pub level: usize,
    /// Heading text content (stripped of inline markdown formatting)
    pub text: String,
    /// Byte offset where the heading starts in the source document
    #[serde(skip_serializing)]
    pub offset: usize,
}

/// A node in the heading tree.
///
/// Represents a heading and its child headings in a hierarchical structure.
#[derive(Debug, Clone)]
pub struct HeadingNode {
    pub heading: Heading,
    pub children: Vec<HeadingNode>,
}

impl Document {
    pub fn new(content: String, headings: Vec<Heading>) -> Self {
        Self { content, headings }
    }

    /// Build a hierarchical tree from flat heading list
    pub fn build_tree(&self) -> Vec<HeadingNode> {
        let mut arena = Arena::new();
        let mut stack: Vec<(usize, NodeId)> = Vec::new();
        let mut roots = Vec::new();

        for heading in &self.headings {
            let node_id = arena.new_node(heading.clone());

            // Pop stack until we find a parent (heading with level < current)
            while let Some(&(parent_level, _)) = stack.last() {
                if parent_level < heading.level {
                    break;
                }
                stack.pop();
            }

            // Attach to parent or mark as root
            if let Some(&(_, parent_id)) = stack.last() {
                parent_id.append(node_id, &mut arena);
            } else {
                roots.push(node_id);
            }

            stack.push((heading.level, node_id));
        }

        // Convert arena to tree structure
        roots
            .into_iter()
            .map(|root_id| build_heading_node(root_id, &arena))
            .collect()
    }

    /// Get headings at a specific level
    pub fn headings_at_level(&self, level: usize) -> Vec<&Heading> {
        self.headings.iter().filter(|h| h.level == level).collect()
    }

    /// Find heading by text (case-insensitive)
    pub fn find_heading(&self, text: &str) -> Option<&Heading> {
        let search = text.to_lowercase();
        self.headings
            .iter()
            .find(|h| h.text.to_lowercase() == search)
    }

    /// Get all headings matching a filter
    pub fn filter_headings(&self, filter: &str) -> Vec<&Heading> {
        let search = filter.to_lowercase();
        self.headings
            .iter()
            .filter(|h| h.text.to_lowercase().contains(&search))
            .collect()
    }

    /// Extract the content of a section by heading text.
    ///
    /// Uses stored byte offsets for fast, accurate extraction without string searching.
    pub fn extract_section(&self, heading_text: &str) -> Option<String> {
        // Find the heading (O(n) scan of headings list)
        let heading_idx = self
            .headings
            .iter()
            .position(|h| h.text.to_lowercase() == heading_text.to_lowercase())?;

        let heading = &self.headings[heading_idx];

        // Start from the heading's stored byte offset
        let start = heading.offset;

        // Find content start (skip the heading line itself)
        let after_heading = &self.content[start..];
        let content_start = after_heading
            .find('\n')
            .map(|i| start + i + 1)
            .unwrap_or(start);

        // Find end: next heading at same or higher level
        let end = self
            .headings
            .iter()
            .skip(heading_idx + 1)
            .find(|h| h.level <= heading.level)
            .map(|h| h.offset)
            .unwrap_or(self.content.len());

        // Extract section content
        Some(self.content[content_start..end].trim().to_string())
    }
}

fn build_heading_node(node_id: NodeId, arena: &Arena<Heading>) -> HeadingNode {
    let heading = arena[node_id].get().clone();
    let children = node_id
        .children(arena)
        .map(|child_id| build_heading_node(child_id, arena))
        .collect();

    HeadingNode { heading, children }
}

impl HeadingNode {
    /// Render as tree with box-drawing characters
    /// If compact is true, uses gapless box characters without trailing spaces
    pub fn render_box_tree(&self, prefix: &str, is_last: bool) -> String {
        self.render_box_tree_styled(prefix, is_last, false)
    }

    /// Render as tree with box-drawing characters, with optional compact style
    pub fn render_box_tree_styled(&self, prefix: &str, is_last: bool, compact: bool) -> String {
        let mut result = String::new();

        let (connector, space, continuation) = if compact {
            // Compact/gapless style: no trailing space after connector
            if is_last {
                ("└──", "", "   ")
            } else {
                ("├──", "", "│  ")
            }
        } else {
            // Spaced style (default): space after connector for readability
            if is_last {
                ("└─ ", "", "    ")
            } else {
                ("├─ ", "", "│   ")
            }
        };

        let marker = "#".repeat(self.heading.level);
        result.push_str(&format!(
            "{}{}{}{} {}\n",
            prefix, connector, space, marker, self.heading.text
        ));

        let child_prefix = format!("{}{}", prefix, continuation);

        for (i, child) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;
            result.push_str(&child.render_box_tree_styled(&child_prefix, is_last_child, compact));
        }

        result
    }
}
