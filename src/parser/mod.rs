//! Markdown parsing and document structure extraction.
//!
//! This module provides functions to parse markdown files and extract
//! their heading structure into a hierarchical tree.
//!
//! All parsing is delegated to `turbovault-parser` for unified, code-block-aware
//! OFM (Obsidian Flavored Markdown) support.

pub mod builder;
pub mod content;
mod document;
pub mod links;
pub mod output;
pub mod utils;

pub use builder::build_json_output;
pub use document::{Document, Heading, HeadingNode};
pub use links::{Link, LinkTarget, extract_links};
pub use output::{Block, DocumentOutput, InlineElement, Section};
pub use utils::{parse_inline_html, strip_markdown_inline};

use std::path::Path;

/// Parse a markdown file and extract its structure.
///
/// # Arguments
///
/// * `path` - Path to the markdown file
///
/// # Returns
///
/// A `Document` containing the file content and extracted headings.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn parse_file(path: &Path) -> std::io::Result<Document> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_markdown(&content))
}

/// Parse markdown content and extract headings with byte offsets.
///
/// Uses turbovault-parser for unified markdown parsing with proper
/// code-block awareness and OFM support.
///
/// # Arguments
///
/// * `content` - Markdown content as a string
///
/// # Returns
///
/// A `Document` containing the content and extracted headings with byte offsets.
pub fn parse_markdown(content: &str) -> Document {
    let headings = turbovault_parser::parse_headings(content)
        .into_iter()
        .map(|h| Heading {
            level: h.level as usize,
            text: h.text,
            offset: h.position.offset,
            source_len: h.position.length,
        })
        .collect();

    Document::new(content.to_string(), headings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headings() {
        let md = r#"# Title
Some content

## Section 1
More content

### Subsection
Details

## Section 2
End"#;

        let doc = parse_markdown(md);
        assert_eq!(doc.headings.len(), 4);
        assert_eq!(doc.headings[0].level, 1);
        assert_eq!(doc.headings[0].text, "Title");
        assert_eq!(doc.headings[1].level, 2);
        assert_eq!(doc.headings[1].text, "Section 1");
    }

    #[test]
    fn test_parse_headings_with_bold() {
        let md = r#"# Title

## **Bold** Section

### Normal Subsection

#### **1. Item** with number"#;

        let doc = parse_markdown(md);
        assert_eq!(doc.headings.len(), 4);

        // Verify bold formatting is stripped from heading text
        assert_eq!(doc.headings[1].text, "Bold Section");
        assert_eq!(doc.headings[3].text, "1. Item with number");
    }

    #[test]
    fn test_headings_store_offsets() {
        let md = r#"# First
Content here

## Second
More content"#;

        let doc = parse_markdown(md);
        assert_eq!(doc.headings.len(), 2);

        // Verify offsets are stored
        assert_eq!(doc.headings[0].offset, 0); // "# First" starts at byte 0
        assert!(doc.headings[1].offset > doc.headings[0].offset);

        // Verify we can use offsets to extract
        let first_heading_text = &md[doc.headings[0].offset..doc.headings[0].offset + 7];
        assert_eq!(first_heading_text, "# First");
    }

    #[test]
    fn test_extract_section_with_bold_headings() {
        let md = r#"# Main

## **Bold** Section
Content of bold section

## Next Section
Other content"#;

        let doc = parse_markdown(md);
        let content = doc.extract_section("Bold Section").unwrap();

        // Should extract only the content between bold section and next section
        assert!(content.contains("Content of bold section"));
        assert!(!content.contains("Other content"));
        assert!(!content.contains("## Next Section"));
    }

    #[test]
    fn test_extract_section_with_numbered_bold_headings() {
        let md = r#"# Architecture

### Core Crates

#### 1. **turbocli-parser** (850 LOC)
Robustly parses POSIX-style help text.

#### 2. **turbocli-config** (400 LOC)
Multi-level configuration loading."#;

        let doc = parse_markdown(md);

        // Test extraction of heading with bold and numbers
        let content = doc.extract_section("1. turbocli-parser (850 LOC)").unwrap();
        assert!(content.contains("Robustly parses"));
        assert!(!content.contains("Multi-level configuration"));

        let content2 = doc.extract_section("2. turbocli-config (400 LOC)").unwrap();
        assert!(content2.contains("Multi-level configuration"));
        assert!(!content2.contains("Robustly parses"));
    }

    #[test]
    fn test_extract_section_at_end_of_document() {
        let md = r#"# First

## Last Section
Final content here
More lines
End of doc"#;

        let doc = parse_markdown(md);
        let content = doc.extract_section("Last Section").unwrap();

        assert!(content.contains("Final content"));
        assert!(content.contains("More lines"));
        assert!(content.contains("End of doc"));
    }
}
