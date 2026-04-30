//! Interactive element navigation system
//!
//! Provides modal navigation through all interactive elements in markdown:
//! - Details blocks (expand/collapse)
//! - Links (follow/copy)
//! - Checkboxes (toggle/save)
//! - Code blocks (copy)
//! - Tables (navigate cells)
//! - Images (view info)

use crate::parser::output::{Block, InlineElement};
use crate::parser::{Link, LinkTarget};
use std::collections::HashMap;

// Sub-index encoding constants for nested elements within list items
// Format: item_idx * ITEM_MULTIPLIER + nested_idx * NESTED_MULTIPLIER + TYPE_OFFSET
/// Multiplier for list item index in sub_idx encoding
pub const ITEM_MULTIPLIER: usize = 10000;
/// Multiplier for nested block index within a list item
pub const NESTED_MULTIPLIER: usize = 10;
/// Offset for inline links within list items (item_idx * 1000 + inline_idx + LINK_OFFSET)
pub const LINK_OFFSET: usize = 100;
/// Multiplier for link encoding (different from nested blocks)
pub const LINK_ITEM_MULTIPLIER: usize = 1000;
/// Offset for code blocks nested in list items
pub const CODE_BLOCK_OFFSET: usize = 5000;
/// Offset for tables nested in list items
pub const TABLE_OFFSET: usize = 6000;
/// Offset for images nested in list items
pub const IMAGE_OFFSET: usize = 7000;

/// Placeholder lines reserved for block-level images in rendered output.
/// 1 label line + IMAGE_PLACEHOLDER_LINES blank lines = BLOCK_IMAGE_TOTAL_LINES.
pub const IMAGE_PLACEHOLDER_LINES: usize = 16;
pub const BLOCK_IMAGE_TOTAL_LINES: usize = 1 + IMAGE_PLACEHOLDER_LINES; // 17

/// Placeholder lines reserved for paragraphs containing inline images.
/// 1 text line + PARAGRAPH_IMAGE_PLACEHOLDER_LINES blank lines = PARAGRAPH_WITH_IMAGE_TOTAL_LINES.
pub const PARAGRAPH_IMAGE_PLACEHOLDER_LINES: usize = 13;
pub const PARAGRAPH_WITH_IMAGE_TOTAL_LINES: usize = 1 + PARAGRAPH_IMAGE_PLACEHOLDER_LINES; // 14

/// Compute the hash key for a mermaid source string (mirrors App::mermaid_source_hash).
#[cfg(all(feature = "mermaid", unix))]
fn mermaid_hash(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Estimate placeholder lines for a mermaid diagram based on its source content.
///
/// Counts significant lines (nodes, edges, labels) and multiplies by a per-line
/// row estimate. Clamped to a sensible min/max so simple diagrams don't waste
/// space and very complex ones still fit.
pub fn mermaid_placeholder_lines(source: &str) -> usize {
    let node_lines = source
        .lines()
        .filter(|l| {
            let t = l.trim();
            if t.is_empty() || t.starts_with("%%") {
                return false;
            }
            let first = t.split_whitespace().next().unwrap_or("");
            !matches!(
                first,
                "graph"
                    | "flowchart"
                    | "sequenceDiagram"
                    | "classDiagram"
                    | "stateDiagram"
                    | "stateDiagram-v2"
                    | "gantt"
                    | "pie"
                    | "gitGraph"
                    | "erDiagram"
                    | "journey"
                    | "mindmap"
                    | "timeline"
                    | "xychart-beta"
                    | "block-beta"
                    | "packet-beta"
                    | "sankey-beta"
                    | "quadrantChart"
            )
        })
        .count();
    // ~5 terminal rows per meaningful source line; min 25, max 120
    (node_lines * 5).clamp(25, 120)
}

// Sub-index encoding constants for nested elements within details blocks
/// Base offset for elements nested inside details blocks
pub const DETAILS_NESTED_BASE: usize = 100000;
/// Multiplier for nested block index within details
pub const DETAILS_NESTED_MULTIPLIER: usize = 100;

/// Interactive navigation state
#[derive(Debug, Clone)]
pub struct InteractiveState {
    /// All interactive elements in current view
    pub elements: Vec<InteractiveElement>,
    /// Current selected element index
    pub current_index: Option<usize>,
    /// Per-element state (expanded/collapsed, selected cell, etc.)
    pub element_states: HashMap<ElementId, ElementState>,
    /// Current detail navigation mode (for tables/lists)
    pub detail_mode: Option<DetailMode>,
}

/// Unique identifier for an element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId {
    /// Block index in parsed content
    pub block_idx: usize,
    /// Sub-item index for lists, cells in tables
    pub sub_idx: Option<usize>,
}

impl ElementId {
    pub fn new(block_idx: usize, sub_idx: Option<usize>) -> Self {
        Self { block_idx, sub_idx }
    }
}

/// An interactive element that can be navigated to and acted upon
#[derive(Debug, Clone)]
pub struct InteractiveElement {
    pub id: ElementId,
    pub element_type: ElementType,
    /// Line range in rendered content (for scroll-to-view)
    pub line_range: (usize, usize),
}

/// Types of interactive elements
#[derive(Debug, Clone)]
pub enum ElementType {
    Details {
        summary: String,
        block_idx: usize,
    },
    Link {
        link: Link,
        /// Position in rendered content for highlighting
        line_idx: usize,
    },
    Checkbox {
        content: String,
        checked: bool,
        /// Block index and item index within the list
        block_idx: usize,
        item_idx: usize,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
        block_idx: usize,
    },
    Table {
        rows: usize,
        cols: usize,
        block_idx: usize,
    },
    Image {
        alt: String,
        src: String,
        block_idx: usize,
    },
}

/// Per-element state
#[derive(Debug, Clone)]
pub enum ElementState {
    Details {
        expanded: bool,
    },
    Table {
        selected_row: usize,
        selected_col: usize,
    },
    List {
        selected_item: usize,
    },
}

/// Fine-grained navigation mode for complex elements
#[derive(Debug, Clone)]
pub enum DetailMode {
    Table { element_idx: usize },
    List { element_idx: usize },
}

impl InteractiveState {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            current_index: None,
            element_states: HashMap::new(),
            detail_mode: None,
        }
    }

    /// Build element index from parsed blocks
    ///
    /// WikiLinks are preprocessed into standard markdown links with `wikilink:` URL prefix,
    /// so they are detected during Block parsing along with regular links.
    pub fn index_elements(&mut self, blocks: &[Block], mermaid_rows: &std::collections::HashMap<u64, usize>) {
        self.elements.clear();
        let mut current_line = 0;

        // Resolve placeholder rows: use cached pixel-based value if available, else heuristic.
        #[cfg(all(feature = "mermaid", unix))]
        let mermaid_rows_for = |source: &str| -> usize {
            let hash = mermaid_hash(source);
            mermaid_rows.get(&hash).copied().unwrap_or_else(|| mermaid_placeholder_lines(source))
        };

        for (block_idx, block) in blocks.iter().enumerate() {
            let start_line = current_line;

            match block {
                Block::Details {
                    summary,
                    blocks: nested,
                    ..
                } => {
                    // Add details block as interactive element
                    let id = ElementId {
                        block_idx,
                        sub_idx: None,
                    };

                    let is_expanded = self.is_details_expanded(id);

                    // Count lines for this details block
                    let lines = 1 + if is_expanded {
                        count_block_lines(nested, mermaid_rows)
                    } else {
                        0
                    };

                    self.elements.push(InteractiveElement {
                        id,
                        element_type: ElementType::Details {
                            summary: summary.clone(),
                            block_idx,
                        },
                        line_range: (start_line, start_line + lines),
                    });

                    // Initialize state if not exists, replacing any stale entry
                    // from a previous section that mapped this `block_idx` to a
                    // non-Details variant (e.g., Table). Without this, a stale
                    // `Table` state silently blocks `toggle_details` since it
                    // only matches `Details {..}`.
                    if !matches!(
                        self.element_states.get(&id),
                        Some(ElementState::Details { .. })
                    ) {
                        self.element_states
                            .insert(id, ElementState::Details { expanded: false });
                    }

                    current_line += 1; // Details summary line

                    // If expanded, index nested interactive elements
                    if is_expanded {
                        for (nested_idx, nested_block) in nested.iter().enumerate() {
                            let nested_start_line = current_line;
                            let nested_base =
                                DETAILS_NESTED_BASE + nested_idx * DETAILS_NESTED_MULTIPLIER;

                            match nested_block {
                                Block::Table { headers, rows, .. } => {
                                    let nested_id = ElementId {
                                        block_idx,
                                        sub_idx: Some(nested_base + TABLE_OFFSET),
                                    };

                                    let table_lines = 3 + rows.len();

                                    self.elements.push(InteractiveElement {
                                        id: nested_id,
                                        element_type: ElementType::Table {
                                            rows: rows.len(),
                                            cols: headers.len(),
                                            block_idx,
                                        },
                                        line_range: (
                                            nested_start_line,
                                            nested_start_line + table_lines,
                                        ),
                                    });

                                    if !matches!(
                                        self.element_states.get(&nested_id),
                                        Some(ElementState::Table { .. })
                                    ) {
                                        self.element_states.insert(
                                            nested_id,
                                            ElementState::Table {
                                                selected_row: 0,
                                                selected_col: 0,
                                            },
                                        );
                                    }

                                    current_line += table_lines;
                                }
                                Block::Code {
                                    language, content, ..
                                } => {
                                    let nested_id = ElementId {
                                        block_idx,
                                        sub_idx: Some(nested_base + CODE_BLOCK_OFFSET),
                                    };

                                    #[cfg(all(feature = "mermaid", unix))]
                                    let code_lines = if language.as_deref() == Some("mermaid") {
                                        1 + mermaid_rows_for(content)
                                    } else {
                                        2 + content.lines().count()
                                    };
                                    #[cfg(not(all(feature = "mermaid", unix)))]
                                    let code_lines = 2 + content.lines().count();

                                    self.elements.push(InteractiveElement {
                                        id: nested_id,
                                        element_type: ElementType::CodeBlock {
                                            language: language.clone(),
                                            content: content.clone(),
                                            block_idx,
                                        },
                                        line_range: (
                                            nested_start_line,
                                            nested_start_line + code_lines,
                                        ),
                                    });

                                    current_line += code_lines;
                                }
                                Block::Image { alt, src, .. } => {
                                    let nested_id = ElementId {
                                        block_idx,
                                        sub_idx: Some(nested_base + IMAGE_OFFSET),
                                    };

                                    self.elements.push(InteractiveElement {
                                        id: nested_id,
                                        element_type: ElementType::Image {
                                            alt: alt.clone(),
                                            src: src.clone(),
                                            block_idx,
                                        },
                                        line_range: (nested_start_line, nested_start_line + 1),
                                    });

                                    current_line += 1;
                                }
                                Block::Paragraph { inline, .. } => {
                                    // Extract links from nested paragraphs
                                    for (inline_idx, inline_elem) in inline.iter().enumerate() {
                                        if let InlineElement::Link { text, url, .. } = inline_elem {
                                            let nested_id = ElementId {
                                                block_idx,
                                                sub_idx: Some(
                                                    nested_base + LINK_OFFSET + inline_idx,
                                                ),
                                            };

                                            let target = if let Some(wikilink_target) =
                                                url.strip_prefix("wikilink:")
                                            {
                                                LinkTarget::WikiLink {
                                                    target: wikilink_target.to_string(),
                                                    alias: if text != wikilink_target {
                                                        Some(text.clone())
                                                    } else {
                                                        None
                                                    },
                                                }
                                            } else if let Some(anchor) = url.strip_prefix('#') {
                                                LinkTarget::Anchor(anchor.to_string())
                                            } else if url.starts_with("http://")
                                                || url.starts_with("https://")
                                            {
                                                LinkTarget::External(url.clone())
                                            } else if let Some((path, anchor)) = url.split_once('#')
                                            {
                                                LinkTarget::RelativeFile {
                                                    path: path.into(),
                                                    anchor: Some(anchor.to_string()),
                                                }
                                            } else {
                                                LinkTarget::RelativeFile {
                                                    path: url.into(),
                                                    anchor: None,
                                                }
                                            };

                                            self.elements.push(InteractiveElement {
                                                id: nested_id,
                                                element_type: ElementType::Link {
                                                    link: Link::new(text.clone(), target, 0),
                                                    line_idx: nested_start_line,
                                                },
                                                line_range: (
                                                    nested_start_line,
                                                    nested_start_line + 1,
                                                ),
                                            });
                                        }
                                    }
                                    current_line += 1;
                                }
                                _ => {
                                    // Other block types - just count lines
                                    current_line += count_single_block_lines(nested_block, mermaid_rows);
                                }
                            }
                        }
                    }
                }
                Block::Paragraph { inline, .. } => {
                    // Extract links and images from inline elements
                    let mut paragraph_has_image = false;
                    for (inline_idx, inline_elem) in inline.iter().enumerate() {
                        if let InlineElement::Link { text, url, .. } = inline_elem {
                            let id = ElementId {
                                block_idx,
                                sub_idx: Some(inline_idx),
                            };

                            // Parse link target
                            let target = if let Some(wikilink_target) =
                                url.strip_prefix("wikilink:")
                            {
                                // Wikilink parsed from [[target]] or [[target|alias]] syntax
                                LinkTarget::WikiLink {
                                    target: wikilink_target.to_string(),
                                    alias: if text != wikilink_target {
                                        Some(text.clone())
                                    } else {
                                        None
                                    },
                                }
                            } else if let Some(anchor) = url.strip_prefix('#') {
                                LinkTarget::Anchor(anchor.to_string())
                            } else if url.starts_with("http://") || url.starts_with("https://") {
                                LinkTarget::External(url.clone())
                            } else if let Some((path, anchor)) = url.split_once('#') {
                                LinkTarget::RelativeFile {
                                    path: path.into(),
                                    anchor: Some(anchor.to_string()),
                                }
                            } else {
                                LinkTarget::RelativeFile {
                                    path: url.into(),
                                    anchor: None,
                                }
                            };

                            self.elements.push(InteractiveElement {
                                id,
                                element_type: ElementType::Link {
                                    link: Link::new(text.clone(), target, 0),
                                    line_idx: current_line,
                                },
                                line_range: (current_line, current_line + 1),
                            });
                        } else if let InlineElement::Image { alt, src, .. } = inline_elem {
                            paragraph_has_image = true;
                            let id = ElementId {
                                block_idx,
                                sub_idx: Some(inline_idx),
                            };

                            self.elements.push(InteractiveElement {
                                id,
                                element_type: ElementType::Image {
                                    alt: alt.clone(),
                                    src: src.clone(),
                                    block_idx,
                                },
                                line_range: (
                                    current_line,
                                    current_line + PARAGRAPH_WITH_IMAGE_TOTAL_LINES,
                                ),
                            });
                        }
                    }
                    if paragraph_has_image {
                        current_line += PARAGRAPH_WITH_IMAGE_TOTAL_LINES;
                    } else {
                        current_line += 1;
                    }
                }
                Block::List { items, .. } => {
                    // Extract checkboxes and links from list items
                    for (item_idx, item) in items.iter().enumerate() {
                        let item_start_line = current_line;

                        if let Some(checked) = item.checked {
                            let id = ElementId {
                                block_idx,
                                sub_idx: Some(item_idx),
                            };

                            self.elements.push(InteractiveElement {
                                id,
                                element_type: ElementType::Checkbox {
                                    content: item.content.clone(),
                                    checked,
                                    block_idx,
                                    item_idx,
                                },
                                line_range: (current_line, current_line + 1),
                            });
                        }

                        // Extract links from list items (e.g., TOC links)
                        // The parser provides line_offset for links inside list items
                        for (inline_idx, inline_elem) in item.inline.iter().enumerate() {
                            if let InlineElement::Link {
                                text,
                                url,
                                line_offset,
                                ..
                            } = inline_elem
                            {
                                // Use a composite sub_idx to differentiate from checkboxes
                                let id = ElementId {
                                    block_idx,
                                    sub_idx: Some(
                                        item_idx * LINK_ITEM_MULTIPLIER + inline_idx + LINK_OFFSET,
                                    ),
                                };

                                // Use parser-provided line_offset (0 for first line, 1 for second, etc.)
                                let offset = line_offset.unwrap_or(0);
                                let link_line = item_start_line + offset;

                                // Parse link target
                                let target = if let Some(wikilink_target) =
                                    url.strip_prefix("wikilink:")
                                {
                                    // Wikilink parsed from [[target]] or [[target|alias]] syntax
                                    LinkTarget::WikiLink {
                                        target: wikilink_target.to_string(),
                                        alias: if text != wikilink_target {
                                            Some(text.clone())
                                        } else {
                                            None
                                        },
                                    }
                                } else if let Some(anchor) = url.strip_prefix('#') {
                                    LinkTarget::Anchor(anchor.to_string())
                                } else if url.starts_with("http://") || url.starts_with("https://")
                                {
                                    LinkTarget::External(url.clone())
                                } else if let Some((path, anchor)) = url.split_once('#') {
                                    LinkTarget::RelativeFile {
                                        path: path.into(),
                                        anchor: Some(anchor.to_string()),
                                    }
                                } else {
                                    LinkTarget::RelativeFile {
                                        path: url.into(),
                                        anchor: None,
                                    }
                                };

                                self.elements.push(InteractiveElement {
                                    id,
                                    element_type: ElementType::Link {
                                        link: Link::new(text.clone(), target, 0),
                                        line_idx: link_line,
                                    },
                                    line_range: (link_line, link_line + 1),
                                });
                            }
                        }

                        // Account for all lines in this item (main + nested)
                        let item_line_count = item.content.lines().count().max(1);
                        current_line += item_line_count;

                        // Process nested blocks within list items (code blocks, tables, etc.)
                        for (nested_idx, nested_block) in item.blocks.iter().enumerate() {
                            let nested_start_line = current_line;
                            let nested_base =
                                item_idx * ITEM_MULTIPLIER + nested_idx * NESTED_MULTIPLIER;
                            match nested_block {
                                Block::Code {
                                    language, content, ..
                                } => {
                                    let id = ElementId {
                                        block_idx,
                                        sub_idx: Some(nested_base + CODE_BLOCK_OFFSET),
                                    };

                                    #[cfg(all(feature = "mermaid", unix))]
                                    let lines = if language.as_deref() == Some("mermaid") {
                                        1 + mermaid_rows_for(content)
                                    } else {
                                        2 + content.lines().count()
                                    };
                                    #[cfg(not(all(feature = "mermaid", unix)))]
                                    let lines = 2 + content.lines().count();

                                    self.elements.push(InteractiveElement {
                                        id,
                                        element_type: ElementType::CodeBlock {
                                            language: language.clone(),
                                            content: content.clone(),
                                            block_idx,
                                        },
                                        line_range: (nested_start_line, nested_start_line + lines),
                                    });

                                    current_line += lines;
                                }
                                Block::Table { headers, rows, .. } => {
                                    let id = ElementId {
                                        block_idx,
                                        sub_idx: Some(nested_base + TABLE_OFFSET),
                                    };

                                    let lines = 3 + rows.len();

                                    self.elements.push(InteractiveElement {
                                        id,
                                        element_type: ElementType::Table {
                                            rows: rows.len(),
                                            cols: headers.len(),
                                            block_idx,
                                        },
                                        line_range: (nested_start_line, nested_start_line + lines),
                                    });

                                    if !matches!(
                                        self.element_states.get(&id),
                                        Some(ElementState::Table { .. })
                                    ) {
                                        self.element_states.insert(
                                            id,
                                            ElementState::Table {
                                                selected_row: 0,
                                                selected_col: 0,
                                            },
                                        );
                                    }

                                    current_line += lines;
                                }
                                Block::Image { alt, src, .. } => {
                                    let id = ElementId {
                                        block_idx,
                                        sub_idx: Some(nested_base + IMAGE_OFFSET),
                                    };

                                    self.elements.push(InteractiveElement {
                                        id,
                                        element_type: ElementType::Image {
                                            alt: alt.clone(),
                                            src: src.clone(),
                                            block_idx,
                                        },
                                        line_range: (nested_start_line, nested_start_line + 1),
                                    });

                                    current_line += 1;
                                }
                                _ => {
                                    // Non-interactive nested blocks
                                    current_line += count_single_block_lines(nested_block, mermaid_rows);
                                }
                            }
                        }
                    }
                }
                Block::Code {
                    language, content, ..
                } => {
                    let id = ElementId {
                        block_idx,
                        sub_idx: None,
                    };

                    // Mermaid blocks use placeholder lines; regular code uses fences + content
                    #[cfg(all(feature = "mermaid", unix))]
                    let lines = if language.as_deref() == Some("mermaid") {
                        1 + mermaid_rows_for(content) // header + blank lines
                    } else {
                        2 + content.lines().count() // +2 for fences
                    };
                    #[cfg(not(all(feature = "mermaid", unix)))]
                    let lines = 2 + content.lines().count();

                    self.elements.push(InteractiveElement {
                        id,
                        element_type: ElementType::CodeBlock {
                            language: language.clone(),
                            content: content.clone(),
                            block_idx,
                        },
                        line_range: (current_line, current_line + lines),
                    });

                    current_line += lines;
                }
                Block::Table { headers, rows, .. } => {
                    let id = ElementId {
                        block_idx,
                        sub_idx: None,
                    };

                    let lines = 3 + rows.len(); // Top border + header + separator + rows + bottom

                    self.elements.push(InteractiveElement {
                        id,
                        element_type: ElementType::Table {
                            rows: rows.len(),
                            cols: headers.len(),
                            block_idx,
                        },
                        line_range: (current_line, current_line + lines),
                    });

                    // Initialize table state, replacing any stale wrong-variant entry.
                    if !matches!(
                        self.element_states.get(&id),
                        Some(ElementState::Table { .. })
                    ) {
                        self.element_states.insert(
                            id,
                            ElementState::Table {
                                selected_row: 0,
                                selected_col: 0,
                            },
                        );
                    }

                    current_line += lines;
                }
                Block::Image { alt, src, .. } => {
                    let id = ElementId {
                        block_idx,
                        sub_idx: None,
                    };

                    self.elements.push(InteractiveElement {
                        id,
                        element_type: ElementType::Image {
                            alt: alt.clone(),
                            src: src.clone(),
                            block_idx,
                        },
                        line_range: (current_line, current_line + BLOCK_IMAGE_TOTAL_LINES),
                    });

                    current_line += BLOCK_IMAGE_TOTAL_LINES;
                }
                _ => {
                    // Non-interactive blocks (still count lines)
                    current_line += count_single_block_lines(block, mermaid_rows);
                }
            }

            // Account for blank line added after each block in render_markdown_enhanced
            current_line += 1;
        }

        // Sort elements by line position for proper navigation order
        self.elements.sort_by_key(|e| e.line_range.0);

        // Reset selection if elements changed
        if self.current_index.is_some() {
            if self.elements.is_empty() {
                self.current_index = None;
            } else if let Some(idx) = self.current_index
                && idx >= self.elements.len()
            {
                self.current_index = Some(0);
            }
        }
    }

    /// Get the currently selected element
    pub fn current_element(&self) -> Option<&InteractiveElement> {
        self.current_index.and_then(|idx| self.elements.get(idx))
    }

    /// Get the line range of the current element for scrolling
    pub fn current_element_line_range(&self) -> Option<(usize, usize)> {
        self.current_element().map(|elem| elem.line_range)
    }

    /// Move to next element
    pub fn next(&mut self) {
        if self.elements.is_empty() {
            return;
        }

        self.current_index = Some(match self.current_index {
            Some(idx) if idx >= self.elements.len() - 1 => 0, // Wrap to first
            Some(idx) => idx + 1,
            None => 0,
        });
    }

    /// Move to previous element
    pub fn previous(&mut self) {
        if self.elements.is_empty() {
            return;
        }

        self.current_index = Some(match self.current_index {
            Some(0) | None => self.elements.len() - 1, // Wrap to last
            Some(idx) => idx - 1,
        });
    }

    /// Check if details block is expanded
    pub fn is_details_expanded(&self, id: ElementId) -> bool {
        matches!(
            self.element_states.get(&id),
            Some(ElementState::Details { expanded: true })
        )
    }

    /// Toggle details block expansion
    pub fn toggle_details(&mut self, id: ElementId) {
        if let Some(ElementState::Details { expanded }) = self.element_states.get_mut(&id) {
            *expanded = !*expanded;
        }
    }

    /// Get status bar text for current element
    pub fn status_text(&self) -> String {
        if let Some(element) = self.current_element() {
            let position = if self.elements.is_empty() {
                "0/0".to_string()
            } else {
                format!(
                    "{}/{}",
                    self.current_index.unwrap_or(0) + 1,
                    self.elements.len()
                )
            };

            match &element.element_type {
                ElementType::Details { .. } => {
                    format!(
                        "[INTERACTIVE] Details({}) | Enter:Toggle Tab:Next Esc:Exit",
                        position
                    )
                }
                ElementType::Link { .. } => {
                    format!(
                        "[INTERACTIVE] Link({}) | Enter:Follow y:Copy Tab:Next Esc:Exit",
                        position
                    )
                }
                ElementType::Checkbox { .. } => {
                    format!(
                        "[INTERACTIVE] Checkbox({}) | Space:Toggle Tab:Next Esc:Exit",
                        position
                    )
                }
                ElementType::CodeBlock { .. } => {
                    format!(
                        "[INTERACTIVE] Code({}) | y:Copy Tab:Next Esc:Exit",
                        position
                    )
                }
                ElementType::Table { .. } => {
                    format!(
                        "[INTERACTIVE] Table({}) | Enter:Navigate y:Copy Tab:Next Esc:Exit",
                        position
                    )
                }
                ElementType::Image { .. } => {
                    format!(
                        "[INTERACTIVE] Image({}) | i:Info y:Copy Tab:Next Esc:Exit",
                        position
                    )
                }
            }
        } else if self.elements.is_empty() {
            "[INTERACTIVE] No interactive elements in this section | Esc:Exit".to_string()
        } else {
            "[INTERACTIVE] Tab:Next Shift+Tab:Prev u/d:Page Esc:Exit".to_string()
        }
    }

    /// Check if an element is nested inside a details block
    fn is_nested_in_details(&self, element: &InteractiveElement) -> bool {
        element
            .id
            .sub_idx
            .map(|idx| idx >= DETAILS_NESTED_BASE)
            .unwrap_or(false)
    }

    /// Find the parent details block for a nested element
    fn find_parent_details(&self, element: &InteractiveElement) -> Option<&InteractiveElement> {
        if !self.is_nested_in_details(element) {
            return None;
        }

        // Find the details block with the same block_idx and no sub_idx
        self.elements.iter().find(|e| {
            e.id.block_idx == element.id.block_idx
                && e.id.sub_idx.is_none()
                && matches!(e.element_type, ElementType::Details { .. })
        })
    }

    /// Get a short hint about the current element type for status bar
    pub fn get_status_hint(&self) -> String {
        if let Some(element) = self.current_element() {
            // Check if nested inside a details block
            let prefix = if let Some(parent) = self.find_parent_details(element) {
                if let ElementType::Details { summary, .. } = &parent.element_type {
                    // Strip HTML tags and truncate
                    let clean_summary = summary
                        .replace("<strong>", "")
                        .replace("</strong>", "")
                        .replace("<b>", "")
                        .replace("</b>", "")
                        .replace("<em>", "")
                        .replace("</em>", "");
                    let display = if clean_summary.len() > 15 {
                        format!("{}...", &clean_summary[..12])
                    } else {
                        clean_summary
                    };
                    format!("▸{} > ", display)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let element_hint = match &element.element_type {
                ElementType::Details { summary, .. } => {
                    // Strip HTML tags and truncate for display
                    let clean_summary = summary
                        .replace("<strong>", "")
                        .replace("</strong>", "")
                        .replace("<b>", "")
                        .replace("</b>", "")
                        .replace("<em>", "")
                        .replace("</em>", "");
                    let display = if clean_summary.len() > 20 {
                        format!("{}...", &clean_summary[..17])
                    } else {
                        clean_summary
                    };
                    format!("▸ {}", display)
                }
                ElementType::Link { link, .. } => {
                    let text = if link.text.len() > 20 {
                        format!("{}...", &link.text[..17])
                    } else {
                        link.text.clone()
                    };
                    format!("Link: {}", text)
                }
                ElementType::Checkbox {
                    content, checked, ..
                } => {
                    let mark = if *checked { "☑" } else { "☐" };
                    let text = if content.len() > 15 {
                        format!("{}...", &content[..12])
                    } else {
                        content.clone()
                    };
                    format!("{} {}", mark, text)
                }
                ElementType::CodeBlock { language, .. } => {
                    let lang = language.as_deref().unwrap_or("code");
                    format!("Code: {}", lang)
                }
                ElementType::Table { rows, cols, .. } => {
                    format!("Table: {}×{}", rows, cols)
                }
                ElementType::Image { alt, .. } => {
                    let text = if alt.len() > 20 {
                        format!("{}...", &alt[..17])
                    } else {
                        alt.clone()
                    };
                    format!("Image: {}", text)
                }
            };

            format!("{}{}", prefix, element_hint)
        } else if self.elements.is_empty() {
            "No elements".to_string()
        } else {
            "Select element".to_string()
        }
    }

    /// Enter interactive mode (select first element)
    pub fn enter(&mut self) {
        if !self.elements.is_empty() {
            self.current_index = Some(0);
        }
    }

    /// Enter interactive mode at the element closest to the given scroll position
    /// This preserves the user's current view instead of jumping to the first element
    pub fn enter_at_scroll_position(&mut self, scroll_pos: usize) {
        if self.elements.is_empty() {
            self.current_index = None;
            return;
        }

        // Find the element whose start line is closest to the scroll position
        // Prefer elements that are at or just after the scroll position
        let mut best_idx = 0;
        let mut best_distance = usize::MAX;

        for (idx, element) in self.elements.iter().enumerate() {
            let (start_line, _) = element.line_range;

            // Calculate distance, preferring elements at or after scroll position
            let distance = if start_line >= scroll_pos {
                start_line - scroll_pos
            } else {
                // Element is above scroll position - add penalty to prefer visible elements
                (scroll_pos - start_line) + 1000
            };

            if distance < best_distance {
                best_distance = distance;
                best_idx = idx;
            }
        }

        self.current_index = Some(best_idx);
    }

    /// Exit interactive mode
    pub fn exit(&mut self) {
        self.current_index = None;
        self.detail_mode = None;
    }

    /// Check if in interactive mode
    pub fn is_active(&self) -> bool {
        self.current_index.is_some()
    }

    /// Enter table navigation mode
    pub fn enter_table_mode(&mut self) -> Result<(), String> {
        if let Some(idx) = self.current_index
            && let Some(element) = self.elements.get(idx)
            && matches!(element.element_type, ElementType::Table { .. })
        {
            self.detail_mode = Some(DetailMode::Table { element_idx: idx });
            return Ok(());
        }
        Err("Not on a table element".to_string())
    }

    /// Exit table navigation mode
    pub fn exit_table_mode(&mut self) {
        self.detail_mode = None;
    }

    /// Check if in table navigation mode
    pub fn is_in_table_mode(&self) -> bool {
        matches!(self.detail_mode, Some(DetailMode::Table { .. }))
    }

    /// Get table navigation status text
    pub fn table_status_text(&self, _rows: usize, _cols: usize) -> String {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table {
                selected_row,
                selected_col,
            }) = self.element_states.get(&id)
            {
                return format!(
                    "[TABLE] Cell({},{}) | hjkl:Move y:Copy Y:CopyRow r:CopyTable Esc:Exit",
                    selected_row + 1,
                    selected_col + 1
                );
            }
        }
        "[TABLE] hjkl:Move y:Copy Esc:Exit".to_string()
    }

    /// Move to next cell (right)
    pub fn table_move_right(&mut self, cols: usize) {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table {
                selected_row: _,
                selected_col,
            }) = self.element_states.get_mut(&id)
                && *selected_col < cols - 1
            {
                *selected_col += 1;
            }
        }
    }

    /// Move to previous cell (left)
    pub fn table_move_left(&mut self) {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table { selected_col, .. }) = self.element_states.get_mut(&id)
                && *selected_col > 0
            {
                *selected_col -= 1;
            }
        }
    }

    /// Move to next row (down)
    pub fn table_move_down(&mut self, rows: usize) {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table { selected_row, .. }) = self.element_states.get_mut(&id)
            {
                // rows is data row count; row 0 is header, so valid rows are 0..=rows
                if *selected_row < rows {
                    *selected_row += 1;
                }
            }
        }
    }

    /// Move to previous row (up)
    pub fn table_move_up(&mut self) {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table { selected_row, .. }) = self.element_states.get_mut(&id)
                && *selected_row > 0
            {
                *selected_row -= 1;
            }
        }
    }

    /// Get the currently selected table cell content
    pub fn get_table_cell(&self, headers: &[String], rows: &[Vec<String>]) -> Option<String> {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table {
                selected_row,
                selected_col,
            }) = self.element_states.get(&id)
            {
                if *selected_row == 0 {
                    // Header row
                    return headers.get(*selected_col).cloned();
                } else {
                    // Data row
                    let data_row = *selected_row - 1;
                    return rows
                        .get(data_row)
                        .and_then(|row| row.get(*selected_col).cloned());
                }
            }
        }
        None
    }

    /// Get the currently selected table row
    pub fn get_table_row(&self, headers: &[String], rows: &[Vec<String>]) -> Option<Vec<String>> {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table { selected_row, .. }) = self.element_states.get(&id) {
                if *selected_row == 0 {
                    // Header row
                    return Some(headers.to_vec());
                } else {
                    // Data row
                    let data_row = *selected_row - 1;
                    return rows.get(data_row).cloned();
                }
            }
        }
        None
    }

    /// Get the selected cell position (row, col)
    pub fn get_table_position(&self) -> Option<(usize, usize)> {
        if let Some(DetailMode::Table { element_idx }) = &self.detail_mode
            && let Some(element) = self.elements.get(*element_idx)
        {
            let id = element.id;
            if let Some(ElementState::Table {
                selected_row,
                selected_col,
            }) = self.element_states.get(&id)
            {
                return Some((*selected_row, *selected_col));
            }
        }
        None
    }
}

impl Default for InteractiveState {
    fn default() -> Self {
        Self::new()
    }
}

/// Count lines for nested blocks
fn count_block_lines(blocks: &[Block], mermaid_rows: &std::collections::HashMap<u64, usize>) -> usize {
    blocks.iter().map(|b| count_single_block_lines(b, mermaid_rows)).sum()
}

/// Count lines for a single block
fn count_single_block_lines(block: &Block, mermaid_rows: &std::collections::HashMap<u64, usize>) -> usize {
    match block {
        Block::Heading { .. } => 1,
        Block::Paragraph { inline, .. } => {
            let has_image = inline
                .iter()
                .any(|e| matches!(e, InlineElement::Image { .. }));
            if has_image {
                PARAGRAPH_WITH_IMAGE_TOTAL_LINES
            } else {
                1
            }
        }
        Block::Code {
            language, content, ..
        } => {
            #[cfg(all(feature = "mermaid", unix))]
            if language.as_deref() == Some("mermaid") {
                let hash = mermaid_hash(content);
                let rows = mermaid_rows.get(&hash).copied()
                    .unwrap_or_else(|| mermaid_placeholder_lines(content));
                return 1 + rows;
            }
            let _ = language;
            2 + content.lines().count()
        }
        Block::List { items, .. } => items.len(),
        Block::Blockquote { blocks, .. } => count_block_lines(blocks, mermaid_rows),
        Block::Table { rows, .. } => 3 + rows.len(),
        Block::Image { .. } => BLOCK_IMAGE_TOTAL_LINES,
        Block::HorizontalRule => 1,
        Block::Details { blocks, .. } => 1 + count_block_lines(blocks, mermaid_rows),
    }
}

#[cfg(test)]
mod interactive_tests {
    use super::*;
    use crate::parser::content::parse_content;

    #[test]
    fn test_nested_code_blocks_in_list_items() {
        // Regression test: code blocks nested inside list items should be detected
        let markdown = r#"# Test Document

1. **First step**
   ```bash
   echo "hello"
   ```

2. **Second step**
   ```python
   print("world")
   ```

| Header | Value |
|--------|-------|
| A      | 1     |
"#;

        let blocks = parse_content(markdown, 0);
        let mut state = InteractiveState::new();
        state.index_elements(&blocks, &std::collections::HashMap::new());

        // Should find: 2 nested code blocks + 1 table = 3 interactive elements
        assert_eq!(
            state.elements.len(),
            3,
            "Should find 2 code blocks and 1 table"
        );

        // Verify we have the right types
        let code_count = state
            .elements
            .iter()
            .filter(|e| matches!(e.element_type, ElementType::CodeBlock { .. }))
            .count();
        let table_count = state
            .elements
            .iter()
            .filter(|e| matches!(e.element_type, ElementType::Table { .. }))
            .count();

        assert_eq!(code_count, 2, "Should find 2 code blocks");
        assert_eq!(table_count, 1, "Should find 1 table");
    }

    #[test]
    fn test_mixed_interactive_elements() {
        let markdown = r#"# Document

A paragraph with a [link](https://example.com).

- [ ] Unchecked task
- [x] Checked task

```rust
fn main() {}
```

| Col1 | Col2 |
|------|------|
| a    | b    |
"#;

        let blocks = parse_content(markdown, 0);
        let mut state = InteractiveState::new();
        state.index_elements(&blocks, &std::collections::HashMap::new());

        // Should find: 1 link + 2 checkboxes + 1 code block + 1 table = 5 elements
        assert!(
            state.elements.len() >= 5,
            "Should find at least 5 interactive elements, found {}",
            state.elements.len()
        );
    }

    #[test]
    fn test_nested_list_item_links() {
        // Test that links from nested list items are found in interactive mode
        let markdown = r#"# Table of Contents

- [Features](#features)
  - [Interactive TUI](#interactive-tui)
  - [CLI Mode](#cli-mode)
- [Installation](#installation)
"#;

        let blocks = parse_content(markdown, 0);
        let mut state = InteractiveState::new();
        state.index_elements(&blocks, &std::collections::HashMap::new());

        // Count link elements
        let link_count = state
            .elements
            .iter()
            .filter(|e| matches!(e.element_type, ElementType::Link { .. }))
            .count();

        // Should find all 4 links: Features, Interactive TUI, CLI Mode, Installation
        assert_eq!(
            link_count, 4,
            "Should find 4 links (including nested), found {}",
            link_count
        );
    }

    #[test]
    fn reindex_replaces_stale_wrong_variant_state() {
        // Regression: when navigating between sections, element_states is keyed
        // only by ElementId (block_idx, sub_idx). A previous section's Table at
        // block_idx=N would silently block a Details inserted at the same key
        // in the new section, since `or_insert` is a no-op when the key exists.
        // toggle_details would then no-op (variant mismatch) while the activate
        // handler still reported "Toggled details".
        let mut state = InteractiveState::new();

        // Section A: a table at block_idx 0.
        let table_md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        state.index_elements(&parse_content(table_md, 0), &std::collections::HashMap::new());
        let table_id = ElementId {
            block_idx: 0,
            sub_idx: None,
        };
        assert!(matches!(
            state.element_states.get(&table_id),
            Some(ElementState::Table { .. })
        ));

        // Section B: a Details at the same block_idx.
        let details_md = "<details>\n<summary>S</summary>\n\nbody\n\n</details>\n";
        state.index_elements(&parse_content(details_md, 0), &std::collections::HashMap::new());

        assert!(
            matches!(
                state.element_states.get(&table_id),
                Some(ElementState::Details { expanded: false })
            ),
            "stale Table state should have been replaced with fresh Details, got {:?}",
            state.element_states.get(&table_id)
        );

        state.toggle_details(table_id);
        assert!(
            state.is_details_expanded(table_id),
            "toggle_details should flip the freshly-inserted Details state"
        );
    }
}
