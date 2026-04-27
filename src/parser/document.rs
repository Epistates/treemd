//! Document model for markdown files.
//!
//! This module defines the core data structures for representing
//! markdown documents and their heading hierarchy.

use serde::Serialize;

/// A markdown document with its content and structure.
///
/// Contains the original markdown content and a list of extracted headings.
#[derive(Debug, Clone)]
pub struct Document {
    pub content: String,
    pub headings: Vec<Heading>,
    /// Lowercased heading text, parallel to `headings`. Used for
    /// case-insensitive search without re-allocating per comparison.
    heading_text_lc: Vec<String>,
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
        let heading_text_lc = headings.iter().map(|h| h.text.to_lowercase()).collect();
        Self {
            content,
            headings,
            heading_text_lc,
        }
    }

    /// Build a hierarchical tree from the flat heading list.
    ///
    /// Walks the headings once with an explicit stack of `(level, &mut Vec<HeadingNode>)`
    /// pointers; child arrays are filled in place. No intermediate arena, no
    /// extra clones beyond the one Heading copy each node owns.
    pub fn build_tree(&self) -> Vec<HeadingNode> {
        // Build into raw indices first so we can mutate parent nodes safely.
        let mut roots: Vec<HeadingNode> = Vec::new();
        // `stack` stores indices describing how to navigate from the root
        // down to the current parent: each entry is the index into the
        // parent's `children` Vec. Walking the path on demand avoids
        // borrow-checker issues from holding mutable references on the stack.
        let mut path: Vec<(usize, usize)> = Vec::new(); // (level, child_idx)

        for heading in &self.headings {
            let node = HeadingNode {
                heading: heading.clone(),
                children: Vec::new(),
            };

            // Pop until current heading is deeper than top of stack.
            while let Some(&(parent_level, _)) = path.last() {
                if parent_level < heading.level {
                    break;
                }
                path.pop();
            }

            // Walk down the path to the parent's children Vec, push, and
            // record the new node's index for descendants.
            if path.is_empty() {
                roots.push(node);
                let idx = roots.len() - 1;
                path.push((heading.level, idx));
            } else {
                let mut cursor: &mut Vec<HeadingNode> = &mut roots;
                let last = path.len() - 1;
                for (i, &(_, child_idx)) in path.iter().enumerate() {
                    if i == last {
                        cursor[child_idx].children.push(node);
                        let new_idx = cursor[child_idx].children.len() - 1;
                        let parent_level = heading.level;
                        path.push((parent_level, new_idx));
                        break;
                    } else {
                        cursor = &mut cursor[child_idx].children;
                    }
                }
            }
        }

        roots
    }

    /// Get headings at a specific level
    pub fn headings_at_level(&self, level: usize) -> Vec<&Heading> {
        self.headings.iter().filter(|h| h.level == level).collect()
    }

    /// Find heading by text (case-insensitive)
    pub fn find_heading(&self, text: &str) -> Option<&Heading> {
        let search = text.to_lowercase();
        self.heading_text_lc
            .iter()
            .position(|lc| *lc == search)
            .map(|i| &self.headings[i])
    }

    /// Get all headings matching a filter
    pub fn filter_headings(&self, filter: &str) -> Vec<&Heading> {
        let search = filter.to_lowercase();
        self.headings
            .iter()
            .zip(self.heading_text_lc.iter())
            .filter(|(_, lc)| lc.contains(&search))
            .map(|(h, _)| h)
            .collect()
    }

    /// Extract the content of a section by heading text.
    ///
    /// Uses stored byte offsets for fast, accurate extraction without string searching.
    pub fn extract_section(&self, heading_text: &str) -> Option<String> {
        // Find the heading (O(n) scan of headings list)
        let search = heading_text.to_lowercase();
        let heading_idx = self.heading_text_lc.iter().position(|lc| *lc == search)?;

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
