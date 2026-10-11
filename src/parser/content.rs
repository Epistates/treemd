//! Content parsing for markdown sections
//!
//! This module wraps turbovault-parser's block parsing functionality,
//! providing code-block-aware OFM (Obsidian Flavored Markdown) parsing.
//!
//! ## Key Features
//! - Proper code block exclusion (wikilinks/tags inside code blocks are ignored)
//! - Wikilink preprocessing: `[[target]]` → `[target](wikilink:target)`
//! - Links with spaces: `[text](url with spaces)` → `[text](<url with spaces>)`
//! - Details block extraction: `<details><summary>` HTML blocks
//! - Full GFM support: tables, strikethrough, task lists

use super::output::{Block, InlineElement, ListItem};

/// The inline elements of a list item's own text, without its nested blocks.
///
/// turbovault-parser gives an item's `inline` its own elements first, then
/// everything gathered from `blocks`: a paragraph's inline run, and each
/// nested item's `inline`, which is already that same collection applied to
/// it. That is what lets a link in a nested item be found from the top-level
/// item, but drawing the whole run on the item's row repeats every nested
/// item's text there. This takes back off what the blocks contributed, the
/// inverse of the parser's `collect_inline_elements`.
pub fn own_inline(item: &ListItem) -> &[InlineElement] {
    fn contributed(blocks: &[Block]) -> usize {
        blocks
            .iter()
            .map(|block| match block {
                Block::Paragraph { inline, .. } => inline.len(),
                Block::List { items, .. } => items.iter().map(|i| i.inline.len()).sum(),
                Block::Blockquote { blocks, .. } | Block::Details { blocks, .. } => {
                    contributed(blocks)
                }
                _ => 0,
            })
            .sum()
    }

    let own = item.inline.len().saturating_sub(contributed(&item.blocks));
    &item.inline[..own]
}

/// Parse markdown content into structured blocks.
///
/// This is the main entry point for content parsing. It delegates to
/// turbovault-parser which provides proper code block awareness and OFM support.
///
/// # Arguments
///
/// * `markdown` - The markdown content to parse
/// * `start_line` - Starting line number for position tracking
///
/// # Returns
///
/// A vector of parsed content blocks.
pub fn parse_content(markdown: &str, start_line: usize) -> Vec<Block> {
    turbovault_parser::parse_blocks_from_line(markdown, start_line)
}

/// Generate URL-friendly slug from heading text.
///
/// Converts heading text to lowercase and replaces non-alphanumeric
/// characters with hyphens.
///
/// # Examples
///
/// ```
/// use treemd::parser::content::slugify;
///
/// assert_eq!(slugify("Hello World"), "hello-world");
/// assert_eq!(slugify("API Reference"), "api-reference");
/// ```
pub fn slugify(text: &str) -> String {
    turbovault_parser::slugify(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_paragraph() {
        let markdown = "This is a simple paragraph.";
        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], Block::Paragraph { .. }));
        if let Block::Paragraph { content, .. } = &blocks[0] {
            assert_eq!(content, "This is a simple paragraph.");
        }
    }

    #[test]
    fn test_parse_heading() {
        let markdown = "# Hello World";
        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 1);
        if let Block::Heading {
            level,
            content,
            anchor,
            ..
        } = &blocks[0]
        {
            assert_eq!(*level, 1);
            assert_eq!(content, "Hello World");
            assert_eq!(anchor.as_deref(), Some("hello-world"));
        } else {
            panic!("Expected Heading block");
        }
    }

    #[test]
    fn test_parse_code_block() {
        let markdown = "```rust\nfn main() {}\n```";
        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 1);
        if let Block::Code {
            language, content, ..
        } = &blocks[0]
        {
            assert_eq!(language.as_deref(), Some("rust"));
            assert_eq!(content, "fn main() {}");
        } else {
            panic!("Expected Code block");
        }
    }

    #[test]
    fn test_wikilinks_rendered_as_links() {
        let markdown = "Here is a [[wikilink]] and [[target|alias]] test.";
        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 1);
        if let Block::Paragraph { inline, .. } = &blocks[0] {
            use turbovault_parser::InlineElement;

            // Find the link elements
            let links: Vec<_> = inline
                .iter()
                .filter_map(|e| {
                    if let InlineElement::Link { text, url, .. } = e {
                        Some((text.clone(), url.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            assert_eq!(links.len(), 2, "Should have 2 wikilinks");
            assert_eq!(links[0].0, "wikilink");
            assert_eq!(links[0].1, "wikilink:wikilink");
            assert_eq!(links[1].0, "alias");
            assert_eq!(links[1].1, "wikilink:target");
        } else {
            panic!("Expected Paragraph block");
        }
    }

    #[test]
    fn test_code_block_excludes_wikilinks() {
        // This is the key test - wikilinks inside code blocks should NOT be parsed
        let markdown = r#"
Normal [[Valid Link]] here.

```rust
// Code block
let link = "[[Fake Link Inside Code]]";
```

Also [[Another Valid]]
"#;
        let blocks = parse_content(markdown, 0);

        // Count wikilinks by looking for wikilink: prefix in links
        let mut wikilink_count = 0;
        for block in &blocks {
            if let Block::Paragraph { inline, .. } = block {
                use turbovault_parser::InlineElement;
                for elem in inline {
                    if let InlineElement::Link { url, .. } = elem
                        && url.starts_with("wikilink:")
                    {
                        wikilink_count += 1;
                    }
                }
            }
        }

        // Should only find the 2 valid links, NOT the one inside the code block
        assert_eq!(
            wikilink_count, 2,
            "Should find exactly 2 wikilinks (not the one in code block)"
        );
    }

    #[test]
    fn test_list_with_code_block() {
        let markdown = r#"1. Test1:
   ```
   test1
   ```

2. Test2:
   test2"#;

        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 1);

        if let Block::List { ordered, items } = &blocks[0] {
            assert!(ordered, "Should be an ordered list");
            assert_eq!(items.len(), 2, "Should have 2 items");

            // First item: "Test1:" with a code block
            assert_eq!(items[0].content, "Test1:");
            assert_eq!(
                items[0].blocks.len(),
                1,
                "First item should have 1 nested block"
            );
            if let Block::Code { content, .. } = &items[0].blocks[0] {
                assert_eq!(content, "test1");
            } else {
                panic!("Expected Code block in first item");
            }

            // Second item: "Test2: test2" with no nested blocks
            assert!(items[1].content.contains("Test2:"));
            assert!(items[1].content.contains("test2"));
            assert!(
                items[1].blocks.is_empty(),
                "Second item should have no nested blocks"
            );
        } else {
            panic!("Expected List block");
        }
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("API Reference"), "api-reference");
        assert_eq!(slugify("1. Getting Started"), "1-getting-started");
    }

    #[test]
    fn test_details_block_with_table() {
        let markdown = r#"<details>
<summary><strong>Navigation</strong></summary>

| Key | Action |
|-----|--------|
| `j` / `k` | Move down/up |
| `g` | Jump to top |

</details>"#;

        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 1, "Should have 1 details block");

        if let Block::Details {
            summary,
            blocks: nested,
            ..
        } = &blocks[0]
        {
            assert_eq!(summary, "<strong>Navigation</strong>");
            assert!(
                !nested.is_empty(),
                "Details should have nested blocks (table)"
            );

            // Check that we have a table inside
            let has_table = nested.iter().any(|b| matches!(b, Block::Table { .. }));
            assert!(has_table, "Details should contain a table");
        } else {
            panic!("Expected Details block, got {:?}", blocks[0]);
        }
    }

    #[test]
    fn test_heading_inline_code_not_leaked_to_paragraph() {
        // Regression test for: inline code in headings was leaking into the
        // next paragraph's content and inline element buffer.
        let markdown = "### Convert `foo` to `bar`\n\nReplace `baz` with a redirect:";
        let blocks = parse_content(markdown, 0);

        assert_eq!(blocks.len(), 2);

        // Heading must contain the Code inline elements
        if let Block::Heading { inline, .. } = &blocks[0] {
            let code_values: Vec<&str> = inline
                .iter()
                .filter_map(|e| {
                    if let turbovault_parser::InlineElement::Code { value } = e {
                        Some(value.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(
                code_values,
                vec!["foo", "bar"],
                "heading inline should contain both Code elements"
            );
        } else {
            panic!("Expected Heading block, got {:?}", blocks[0]);
        }

        // Paragraph must NOT start with code leaked from the heading
        if let Block::Paragraph { content, inline } = &blocks[1] {
            assert!(
                content.starts_with("Replace"),
                "paragraph content should start with 'Replace', got: {content:?}"
            );
            if let Some(first) = inline.first() {
                assert!(
                    matches!(first, turbovault_parser::InlineElement::Text { .. }),
                    "paragraph should start with Text, not leaked Code: {first:?}"
                );
            }
        } else {
            panic!("Expected Paragraph block, got {:?}", blocks[1]);
        }
    }

    /// `own_inline` undoes the parser's collection of nested inline elements
    /// into an item's run, and nothing else. If the parser changes how it
    /// collects them, this is what fails, rather than the renderer quietly
    /// drawing a child's text on its parent's row.
    #[test]
    fn own_inline_is_the_item_without_its_nested_blocks() {
        fn text(elements: &[InlineElement]) -> String {
            elements
                .iter()
                .map(|e| match e {
                    InlineElement::Text { value }
                    | InlineElement::Strong { value }
                    | InlineElement::Emphasis { value }
                    | InlineElement::Code { value } => value.clone(),
                    InlineElement::Link { text, .. } => text.clone(),
                    _ => String::new(),
                })
                .collect()
        }
        fn first_item(md: &str) -> ListItem {
            match parse_content(md, 0).into_iter().next() {
                Some(Block::List { mut items, .. }) => items.remove(0),
                other => panic!("expected a list, got {other:?}"),
            }
        }

        let item = first_item("- **outer** [a](#a)\n  - inner [b](#b)\n    - deep\n");
        assert_eq!(text(own_inline(&item)), "outer a");
        // The run as a whole still carries the nested links, which is what
        // interactive mode indexes them from.
        assert!(text(&item.inline).contains('b'));

        let item = first_item("- a\n  - b\n\n  para under a\n");
        assert_eq!(text(own_inline(&item)), "a");

        let item = first_item("- [ ] todo [x](#x)\n\n  more [y](#y)\n");
        assert_eq!(text(own_inline(&item)), "todo x");

        let item = first_item("- plain\n");
        assert_eq!(own_inline(&item), item.inline.as_slice());
    }
}
