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
/// Represents a single heading with its level (1-6) and text content.
#[derive(Debug, Clone, Serialize)]
pub struct Heading {
    /// Heading level (1 for #, 2 for ##, etc.)
    pub level: usize,
    /// Heading text content
    pub text: String,
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

    /// Extract the content of a section by heading text
    pub fn extract_section(&self, heading_text: &str) -> Option<String> {
        // Find the heading
        let heading = self.find_heading(heading_text)?;

        // Build the search pattern
        let search = format!("{} {}", "#".repeat(heading.level), heading.text);

        // Find the start of the section
        let start = self.content.find(&search)?;

        // Find the end (next heading of same or higher level, or end of document)
        // But skip headings inside code blocks
        let after = &self.content[start..];

        let mut end = after.len();
        let mut in_code_block = false;

        for (i, line) in after.lines().enumerate() {
            if i == 0 {
                continue; // Skip the heading line itself
            }

            // Track code block boundaries
            if line.trim_start().starts_with("```") {
                in_code_block = !in_code_block;
            }

            // Only check for headings when not in a code block
            if !in_code_block {
                if let Some(level) = get_heading_level(line) {
                    if level <= heading.level {
                        // Found next heading at same or higher level
                        end = line.as_ptr() as usize - after.as_ptr() as usize;
                        break;
                    }
                }
            }
        }

        Some(after[..end].to_string())
    }
}

fn get_heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let mut level = 0;

    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else if ch.is_whitespace() {
            if level > 0 {
                return Some(level);
            }
            return None;
        } else {
            break;
        }
    }

    None
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
    pub fn render_box_tree(&self, prefix: &str, is_last: bool) -> String {
        let mut result = String::new();

        let connector = if is_last { "└─ " } else { "├─ " };
        let marker = "#".repeat(self.heading.level);
        result.push_str(&format!(
            "{}{}{} {}\n",
            prefix, connector, marker, self.heading.text
        ));

        let child_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });

        for (i, child) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;
            result.push_str(&child.render_box_tree(&child_prefix, is_last_child));
        }

        result
    }
}
