//! Build nested JSON output from document structure

use super::content::{parse_content, slugify};
use super::document::{Document, HeadingNode};
use super::output::*;
use std::path::Path;

/// Build complete JSON output with nested sections and markdown intelligence
pub fn build_json_output(doc: &Document, source_path: Option<&Path>) -> DocumentOutput {
    let tree = doc.build_tree();

    // Calculate metadata
    let max_depth = calculate_max_depth(&tree);
    let word_count = count_words(&doc.content);

    let metadata = DocumentMetadata {
        source: source_path.map(|p| p.to_string_lossy().to_string()),
        heading_count: doc.headings.len(),
        max_depth,
        word_count,
    };

    // Build sections with content
    let sections = tree.iter().map(|node| build_section(node, doc)).collect();

    DocumentOutput {
        document: DocumentRoot { metadata, sections },
    }
}

fn build_section(node: &HeadingNode, doc: &Document) -> Section {
    let heading = &node.heading;

    // Extract content for this section
    let (raw_content, offset, line) = extract_section_content(doc, node.index);

    // Parse content into blocks
    let blocks = parse_content(&raw_content, line);

    // Build child sections
    let children = node
        .children
        .iter()
        .map(|child| build_section(child, doc))
        .collect();

    Section {
        id: slugify(&heading.text),
        level: heading.level,
        title: heading.text.clone(),
        slug: slugify(&heading.text),
        position: Position { line, offset },
        content: Content {
            raw: raw_content,
            blocks,
        },
        children,
    }
}

/// Extract the raw body of section `idx` for nested JSON output.
///
/// Bounds the body at the *next heading of any level* (child sections are
/// emitted separately in the `children` array), and shares `Document::body_start`
/// so CRLF, setext underlines, and EOF headings are handled correctly.
///
/// Returns `(raw_body, body_offset, body_line)`.
fn extract_section_content(doc: &Document, idx: usize) -> (String, usize, usize) {
    let offset = doc.headings[idx].offset;

    // Line number of the heading itself (1-indexed); body starts on the line
    // after the heading.
    let line = doc.content[..offset].lines().count() + 1;

    let content_start = doc.body_start(idx);
    let end = doc.section_end_any(idx);

    (
        doc.content[content_start..end].trim().to_string(),
        content_start,
        line + 1,
    )
}

fn calculate_max_depth(tree: &[HeadingNode]) -> usize {
    tree.iter()
        .map(|node| 1 + calculate_max_depth(&node.children))
        .max()
        .unwrap_or(0)
}

fn count_words(content: &str) -> usize {
    content.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_markdown;

    // ---------- section body bounding (via parse_markdown + builder) ----------

    /// Helper: build the JSON output and return the raw body of the first
    /// top-level section.
    fn first_section_raw(md: &str) -> String {
        let doc = parse_markdown(md);
        let out = build_json_output(&doc, None);
        out.document.sections[0].content.raw.clone()
    }

    #[test]
    fn section_body_stops_at_first_child_heading() {
        // The nested builder bounds a parent's raw body at the next heading of
        // any level; the child body is emitted separately.
        let md = "# Top\npara one\nmore text\n## Subsection\nbody\n";
        let raw = first_section_raw(md);
        assert!(raw.contains("para one"));
        assert!(raw.contains("more text"));
        assert!(!raw.contains("Subsection"));
        assert!(!raw.contains("body"));
    }

    #[test]
    fn section_body_no_child_heading_takes_rest_of_doc() {
        let md = "# Top\njust a paragraph\nwith two lines\n";
        let raw = first_section_raw(md);
        assert!(raw.contains("just a paragraph"));
        assert!(raw.contains("with two lines"));
    }

    #[test]
    fn section_body_keeps_heading_inside_code_block() {
        // A `# fake heading` inside a fenced block must NOT split the section —
        // turbovault's heading parser is code-block aware, so it is not a heading.
        let md = "# Top\nintro\n\n```rust\n# fake heading\nlet x = 1;\n```\n\n## real heading\n";
        let raw = first_section_raw(md);
        assert!(raw.contains("# fake heading"));
        assert!(!raw.contains("real heading"));
    }

    #[test]
    fn section_body_keeps_indented_code_fence_content() {
        let md = "# Top\nintro\n  ```\n# fake\n  ```\n## real\n";
        let raw = first_section_raw(md);
        assert!(raw.contains("# fake"));
        assert!(!raw.contains("real"));
    }

    // ---------- count_words ----------

    #[test]
    fn count_words_basic() {
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("one"), 1);
        assert_eq!(count_words("one two three"), 3);
        assert_eq!(count_words("  leading and  trailing  "), 3);
        assert_eq!(count_words("with\nnewlines\nand\ttabs"), 4);
    }

    // ---------- calculate_max_depth ----------

    #[test]
    fn calculate_max_depth_empty_is_zero() {
        assert_eq!(calculate_max_depth(&[]), 0);
    }

    #[test]
    fn calculate_max_depth_counts_deepest_branch() {
        // Build a tree manually:
        // A
        //  ├── B
        //  └── C
        //       └── D
        let tree = parse_markdown("# A\n## B\n## C\n### D\n").build_tree();
        // Depth: A(1) → C(2) → D(3) = 3.
        assert_eq!(calculate_max_depth(&tree), 3);
    }

    // ---------- build_json_output (end-to-end) ----------

    #[test]
    fn build_json_output_metadata_and_shape() {
        let md = "# Top\nintro\n\n## Sub\nsub body\nmore\n";
        let doc = parse_markdown(md);
        let out = build_json_output(&doc, None);

        assert!(out.document.metadata.source.is_none());
        assert_eq!(out.document.metadata.heading_count, 2);
        assert_eq!(out.document.metadata.max_depth, 2);
        assert!(out.document.metadata.word_count > 0);

        assert_eq!(out.document.sections.len(), 1);
        let top = &out.document.sections[0];
        assert_eq!(top.title, "Top");
        assert_eq!(top.level, 1);
        assert_eq!(top.children.len(), 1);
        assert_eq!(top.children[0].title, "Sub");
    }

    #[test]
    fn build_json_output_records_source_path() {
        let doc = parse_markdown("# X\n");
        let p = std::path::Path::new("/tmp/example.md");
        let out = build_json_output(&doc, Some(p));
        assert_eq!(
            out.document.metadata.source.as_deref(),
            Some("/tmp/example.md")
        );
    }

    #[test]
    fn build_json_output_section_raw_excludes_children() {
        // Section "A" raw content should NOT include child "A1" body —
        // children are emitted separately via the children array.
        let md = "# A\nA-body\n\n## A1\nA1-body\n";
        let doc = parse_markdown(md);
        let out = build_json_output(&doc, None);
        let a = &out.document.sections[0];
        assert!(a.content.raw.contains("A-body"));
        assert!(
            !a.content.raw.contains("A1-body"),
            "child body leaked into parent raw"
        );
        assert_eq!(a.children[0].content.raw.trim(), "A1-body");
    }

    #[test]
    fn build_json_output_slugifies_titles() {
        let doc = parse_markdown("# Hello, World!\n");
        let out = build_json_output(&doc, None);
        let s = &out.document.sections[0];
        assert_eq!(s.title, "Hello, World!");
        // Just sanity-check the slug is lowercase and has no spaces/punct.
        assert!(!s.slug.contains(' '));
        assert!(!s.slug.contains(','));
        assert_eq!(s.slug, s.id);
    }

    #[test]
    fn build_json_output_position_line_is_1_indexed() {
        // First heading on line 1 → reported line of the *content* should be > 1
        // (build_section returns line + 1 for the section content start).
        let md = "# Top\nbody\n";
        let doc = parse_markdown(md);
        let out = build_json_output(&doc, None);
        assert!(out.document.sections[0].position.line >= 2);
    }
}
