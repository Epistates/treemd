//! Link detection and parsing from markdown content.
//!
//! This module provides functionality to extract and parse various types of links
//! from markdown documents, including relative file links, anchor links, wikilinks,
//! and external URLs.
//!
//! All parsing is delegated to `turbovault-parser` for unified, code-block-aware
//! link extraction.

use std::path::PathBuf;
use turbovault_parser::LinkType;

/// Represents a link found in markdown content.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// Display text of the link
    pub text: String,
    /// The target this link points to
    pub target: LinkTarget,
    /// Byte offset in the source content where the link starts
    pub offset: usize,
}

/// The different types of link targets supported.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTarget {
    /// Internal anchor link to a heading in the current document (e.g., `#installation`)
    Anchor(String),

    /// Relative file path, optionally with an anchor (e.g., `./docs/api.md#usage`)
    RelativeFile {
        path: PathBuf,
        anchor: Option<String>,
    },

    /// Wikilink format used in Obsidian and other PKM tools (e.g., `[[filename]]`)
    WikiLink {
        target: String,
        alias: Option<String>,
    },

    /// External URL (e.g., `https://example.com`)
    External(String),
}

impl LinkTarget {
    /// Get a string representation of the link target for display/search
    pub fn as_str(&self) -> String {
        match self {
            LinkTarget::Anchor(a) => format!("#{}", a),
            LinkTarget::RelativeFile { path, anchor } => {
                if let Some(a) = anchor {
                    format!("{}#{}", path.display(), a)
                } else {
                    path.display().to_string()
                }
            }
            LinkTarget::WikiLink { target, alias } => {
                if let Some(a) = alias {
                    format!("[[{}|{}]]", target, a)
                } else {
                    format!("[[{}]]", target)
                }
            }
            LinkTarget::External(url) => url.clone(),
        }
    }
}

impl Link {
    /// Create a new link.
    pub fn new(text: String, target: LinkTarget, offset: usize) -> Self {
        Self {
            text,
            target,
            offset,
        }
    }
}

/// Extract all links from markdown content.
///
/// This function uses turbovault-parser to extract all link types with
/// proper code-block awareness. Links inside fenced code blocks or
/// inline code are correctly excluded.
///
/// Supported link types:
/// - Standard markdown links: `[text](url)`
/// - Wikilinks: `[[target]]` or `[[target|alias]]`
/// - Anchor links: `[text](#section)`
/// - External links: `[text](https://...)`
///
/// # Arguments
///
/// * `content` - The markdown content to parse
///
/// # Returns
///
/// A vector of `Link` structs representing all links found in the content.
pub fn extract_links(content: &str) -> Vec<Link> {
    let mut links = Vec::new();

    // Extract standard markdown links via turbovault-parser
    for md_link in turbovault_parser::parse_markdown_links(content) {
        let text = md_link
            .display_text
            .clone()
            .unwrap_or_else(|| md_link.target.clone());
        let target = convert_link_type(&md_link.type_, &md_link.target);

        links.push(Link::new(text, target, md_link.position.offset));
    }

    // Extract wikilinks via turbovault-parser
    for wikilink in turbovault_parser::parse_wikilinks(content) {
        let target = wikilink.target.clone();
        let alias = wikilink.display_text.clone();
        let display_text = alias.clone().unwrap_or_else(|| target.clone());

        links.push(Link::new(
            display_text,
            LinkTarget::WikiLink { target, alias },
            wikilink.position.offset,
        ));
    }

    // Sort by offset for consistent ordering
    links.sort_by_key(|l| l.offset);

    links
}

/// Convert turbovault LinkType to treemd LinkTarget.
fn convert_link_type(link_type: &LinkType, target: &str) -> LinkTarget {
    match link_type {
        LinkType::Anchor => {
            // Pure anchor: #section
            let anchor = target.strip_prefix('#').unwrap_or(target);
            LinkTarget::Anchor(anchor.to_string())
        }
        LinkType::ExternalLink => LinkTarget::External(target.to_string()),
        LinkType::HeadingRef => {
            // File with anchor: file.md#section
            if let Some((path, anchor)) = target.split_once('#') {
                LinkTarget::RelativeFile {
                    path: PathBuf::from(path),
                    anchor: Some(anchor.to_string()),
                }
            } else {
                // Shouldn't happen for HeadingRef, but handle gracefully
                LinkTarget::RelativeFile {
                    path: PathBuf::from(target),
                    anchor: None,
                }
            }
        }
        LinkType::MarkdownLink => {
            // Relative file without anchor
            LinkTarget::RelativeFile {
                path: PathBuf::from(target),
                anchor: None,
            }
        }
        LinkType::WikiLink | LinkType::Embed | LinkType::BlockRef => {
            // These shouldn't come through parse_markdown_links, but handle anyway
            LinkTarget::WikiLink {
                target: target.to_string(),
                alias: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_anchor_link() {
        let md = "See [Installation](#installation) for details.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "Installation");
        assert_eq!(
            links[0].target,
            LinkTarget::Anchor("installation".to_string())
        );
    }

    #[test]
    fn test_extract_relative_file_link() {
        let md = "Check [API docs](./docs/api.md) for more.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "API docs");
        match &links[0].target {
            LinkTarget::RelativeFile { path, anchor } => {
                assert_eq!(path, &PathBuf::from("./docs/api.md"));
                assert_eq!(anchor, &None);
            }
            _ => panic!("Expected RelativeFile link"),
        }
    }

    #[test]
    fn test_extract_relative_file_link_with_anchor() {
        let md = "See [usage guide](../guide.md#usage) here.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "usage guide");
        match &links[0].target {
            LinkTarget::RelativeFile { path, anchor } => {
                assert_eq!(path, &PathBuf::from("../guide.md"));
                assert_eq!(anchor, &Some("usage".to_string()));
            }
            _ => panic!("Expected RelativeFile link"),
        }
    }

    #[test]
    fn test_extract_external_link() {
        let md = "Visit [GitHub](https://github.com) now.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "GitHub");
        assert_eq!(
            links[0].target,
            LinkTarget::External("https://github.com".to_string())
        );
    }

    #[test]
    fn test_extract_wikilink_simple() {
        let md = "See [[README]] for info.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "README");
        match &links[0].target {
            LinkTarget::WikiLink { target, alias } => {
                assert_eq!(target, "README");
                assert_eq!(alias, &None);
            }
            _ => panic!("Expected WikiLink"),
        }
    }

    #[test]
    fn test_extract_wikilink_with_alias() {
        let md = "Check [[README.md|readme file]] here.";
        let links = extract_links(md);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].text, "readme file");
        match &links[0].target {
            LinkTarget::WikiLink { target, alias } => {
                assert_eq!(target, "README.md");
                assert_eq!(alias, &Some("readme file".to_string()));
            }
            _ => panic!("Expected WikiLink"),
        }
    }

    #[test]
    fn test_extract_multiple_links() {
        let md = r#"
# Documentation

See [Installation](#installation) first.
Then check [API docs](./api.md) and [[contributing]].
Visit [GitHub](https://github.com/user/repo) for source.
"#;
        let links = extract_links(md);

        assert_eq!(links.len(), 4);

        // Links should be sorted by offset
        assert_eq!(links[0].text, "Installation");
        assert!(matches!(links[0].target, LinkTarget::Anchor(_)));

        assert_eq!(links[1].text, "API docs");
        assert!(matches!(links[1].target, LinkTarget::RelativeFile { .. }));

        assert_eq!(links[2].text, "contributing");
        assert!(matches!(links[2].target, LinkTarget::WikiLink { .. }));

        assert_eq!(links[3].text, "GitHub");
        assert!(matches!(links[3].target, LinkTarget::External(_)));
    }

    #[test]
    fn test_empty_content() {
        let md = "";
        let links = extract_links(md);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_no_links() {
        let md = "This is just plain text with no links.";
        let links = extract_links(md);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_malformed_wikilink() {
        let md = "This has [[incomplete wikilink";
        let links = extract_links(md);
        assert_eq!(links.len(), 0); // Should not extract malformed links
    }

    #[test]
    fn test_wikilinks_excluded_from_code_blocks() {
        // Wikilinks inside code blocks should NOT be extracted
        let md = r#"
# Test Document

[[Valid Link]] outside code block.

```rust
let x = "[[Fake Inside Code]]";
```

[[Another Valid]] after code block.
"#;
        let links = extract_links(md);

        // Should only find the 2 valid wikilinks, NOT the one inside the code block
        let wikilink_count = links
            .iter()
            .filter(|l| matches!(l.target, LinkTarget::WikiLink { .. }))
            .count();

        assert_eq!(
            wikilink_count, 2,
            "Should find exactly 2 wikilinks (not the one in code block)"
        );

        // Verify the correct wikilinks were found
        let wikilink_targets: Vec<_> = links
            .iter()
            .filter_map(|l| match &l.target {
                LinkTarget::WikiLink { target, .. } => Some(target.as_str()),
                _ => None,
            })
            .collect();

        assert!(wikilink_targets.contains(&"Valid Link"));
        assert!(wikilink_targets.contains(&"Another Valid"));
        assert!(!wikilink_targets.contains(&"Fake Inside Code"));
    }

    #[test]
    fn test_wikilinks_excluded_from_inline_code() {
        // Wikilinks inside inline code should also be excluded
        let md = "This is `[[not a link]]` but [[this is]] a link.";
        let links = extract_links(md);

        let wikilink_count = links
            .iter()
            .filter(|l| matches!(l.target, LinkTarget::WikiLink { .. }))
            .count();

        assert_eq!(
            wikilink_count, 1,
            "Should find exactly 1 wikilink (not the one in inline code)"
        );
    }

    #[test]
    fn test_markdown_links_excluded_from_code_blocks() {
        // Standard markdown links inside code blocks should also be excluded
        let md = r#"
[Valid](https://example.com) outside.

```markdown
[Fake](https://fake.com) inside code
```

[Also Valid](./file.md) after.
"#;
        let links = extract_links(md);

        // Should only find 2 links, not the one in code block
        assert_eq!(links.len(), 2);
        assert!(matches!(links[0].target, LinkTarget::External(_)));
        assert!(matches!(links[1].target, LinkTarget::RelativeFile { .. }));
    }

    #[test]
    fn test_link_types_correctly_classified() {
        let md = r#"
[anchor](#section)
[external](https://example.com)
[file](./docs/api.md)
[file with anchor](./docs/api.md#usage)
[[wikilink]]
"#;
        let links = extract_links(md);

        assert_eq!(links.len(), 5);

        // Verify each link type is correctly classified
        assert!(
            matches!(&links[0].target, LinkTarget::Anchor(a) if a == "section"),
            "Expected Anchor"
        );
        assert!(
            matches!(&links[1].target, LinkTarget::External(u) if u == "https://example.com"),
            "Expected External"
        );
        assert!(
            matches!(&links[2].target, LinkTarget::RelativeFile { path, anchor: None } if path == &PathBuf::from("./docs/api.md")),
            "Expected RelativeFile without anchor"
        );
        assert!(
            matches!(&links[3].target, LinkTarget::RelativeFile { path, anchor: Some(a) } if path == &PathBuf::from("./docs/api.md") && a == "usage"),
            "Expected RelativeFile with anchor"
        );
        assert!(
            matches!(&links[4].target, LinkTarget::WikiLink { target, .. } if target == "wikilink"),
            "Expected WikiLink"
        );
    }
}
