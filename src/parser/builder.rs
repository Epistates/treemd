//! Build nested JSON output from document structure

use super::content::{parse_content, slugify};
use super::document::{Document, HeadingNode};
use super::output::*;
use super::utils::get_heading_level;
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
    let sections = tree
        .iter()
        .map(|node| build_section(node, &doc.content))
        .collect();

    DocumentOutput {
        document: DocumentRoot { metadata, sections },
    }
}

fn build_section(node: &HeadingNode, full_content: &str) -> Section {
    let heading = &node.heading;

    // Extract content for this section
    let (raw_content, offset, line) = extract_section_content(heading, full_content);

    // Parse content into blocks
    let blocks = parse_content(&raw_content, line);

    // Build child sections
    let children = node
        .children
        .iter()
        .map(|child| build_section(child, full_content))
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

fn extract_section_content(
    heading: &super::document::Heading,
    full_content: &str,
) -> (String, usize, usize) {
    // Use stored byte offset for direct access
    let offset = heading.offset;

    // Calculate line number from byte offset
    let line = full_content[..offset].lines().count() + 1;

    // Find content start (skip the heading line itself)
    let after_heading = &full_content[offset..];
    let content_start = after_heading.find('\n').map(|i| i + 1).unwrap_or(0);
    let section_content = &after_heading[content_start..];

    // Find next heading (any level, since children are extracted separately)
    let end = find_next_heading(section_content);

    (
        section_content[..end].trim().to_string(),
        offset + content_start,
        line + 1,
    )
}

fn find_next_heading(content: &str) -> usize {
    let mut in_code_block = false;
    let mut pos = 0;

    for line in content.lines() {
        // Track code block fences
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
        }

        // Check for heading only if not in code block
        if !in_code_block && get_heading_level(line).is_some() {
            // For nested JSON output, stop at ANY heading (child sections are
            // extracted separately and included in the children array)
            return pos;
        }

        pos += line.len() + 1; // +1 for newline
    }

    content.len()
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

    // ---------- find_next_heading (code-block awareness) ----------

    #[test]
    fn find_next_heading_stops_at_first_heading() {
        let content = "para one\nmore text\n## Subsection\nbody\n";
        let end = find_next_heading(content);
        assert_eq!(&content[..end], "para one\nmore text\n");
    }

    #[test]
    fn find_next_heading_no_heading_returns_full_len() {
        let content = "just a paragraph\nwith two lines\n";
        assert_eq!(find_next_heading(content), content.len());
    }

    #[test]
    fn find_next_heading_ignores_heading_inside_code_block() {
        // The `# fake heading` inside ``` must NOT terminate the section.
        let content = "intro\n\n```rust\n# fake heading\nlet x = 1;\n```\n\n## real heading\n";
        let end = find_next_heading(content);
        // Should land at the real heading's line, not the fake one.
        assert!(content[..end].contains("# fake heading"));
        assert!(!content[..end].contains("## real heading"));
    }

    #[test]
    fn find_next_heading_ignores_indented_code_fences() {
        // turbovault treats `  ```` as a code fence (trim_start before checking).
        let content = "intro\n  ```\n# fake\n  ```\n## real\n";
        let end = find_next_heading(content);
        assert!(content[..end].contains("# fake"));
        assert!(!content[..end].contains("## real"));
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
