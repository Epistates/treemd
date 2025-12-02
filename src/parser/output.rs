//! JSON output types for nested, markdown-intelligent structure

use serde::{Deserialize, Serialize};

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

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Block {
    Paragraph {
        content: String,
        inline: Vec<InlineElement>,
    },
    Code {
        language: Option<String>,
        content: String,
        #[serde(rename = "startLine")]
        start_line: usize,
        #[serde(rename = "endLine")]
        end_line: usize,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    Blockquote {
        content: String,
        blocks: Vec<Block>,
    },
    Table {
        headers: Vec<String>,
        alignments: Vec<Alignment>,
        rows: Vec<Vec<String>>,
    },
    Image {
        alt: String,
        src: String,
        title: Option<String>,
    },
    #[serde(rename = "horizontal_rule")]
    HorizontalRule,
    Details {
        summary: String,
        content: String,
        blocks: Vec<Block>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    /// For task lists: true/false/null
    pub checked: Option<bool>,
    /// Raw content
    pub content: String,
    /// Parsed inline elements
    pub inline: Vec<InlineElement>,
    /// Nested blocks (e.g., code blocks inside list items)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
}

/// Inline formatting elements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InlineElement {
    Text {
        value: String,
    },
    Strong {
        value: String,
    },
    Emphasis {
        value: String,
    },
    Code {
        value: String,
    },
    Link {
        text: String,
        url: String,
        title: Option<String>,
    },
    Image {
        alt: String,
        src: String,
        title: Option<String>,
    },
    Strikethrough {
        value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    Left,
    Center,
    Right,
    None,
}
