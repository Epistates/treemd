//! JSON output types for nested, markdown-intelligent structure
//!
//! Block-level content types are re-exported from turbovault-parser for
//! unified parsing with OFM (Obsidian Flavored Markdown) support and
//! proper code block awareness.

use serde::{Deserialize, Serialize};

// Re-export content block types from turbovault-parser
// These provide code-block-aware parsing and OFM support
pub use turbovault_parser::{
    ContentBlock as Block,
    InlineElement,
    ListItem,
    TableAlignment as Alignment,
};

/// Root document structure with metadata and nested sections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOutput {
    pub document: DocumentRoot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRoot {
    pub metadata: DocumentMetadata,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub source: Option<String>,
    #[serde(rename = "headingCount")]
    pub heading_count: usize,
    #[serde(rename = "maxDepth")]
    pub max_depth: usize,
    #[serde(rename = "wordCount")]
    pub word_count: usize,
}

/// A section with nested children based on heading hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Unique identifier (slugified heading)
    pub id: String,
    /// Heading level (1-6)
    pub level: usize,
    /// Heading text
    pub title: String,
    /// URL-friendly slug
    pub slug: String,
    /// Position in document
    pub position: Position,
    /// Parsed content
    pub content: Content,
    /// Child sections (nested headings)
    pub children: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Line number in source file (1-indexed)
    pub line: usize,
    /// Character offset from start (0-indexed)
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    /// Raw markdown content
    pub raw: String,
    /// Parsed content blocks
    pub blocks: Vec<Block>,
}
