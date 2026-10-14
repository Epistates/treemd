//! # treemd
//!
//! A markdown navigator library with tree-based structural navigation and syntax highlighting.
//!
//! This library provides tools for parsing markdown documents, extracting their heading structure,
//! and building hierarchical trees. It's designed to power both interactive TUI applications and
//! programmatic markdown analysis.
//!
//! ## Features
//!
//! - Parse markdown and extract heading hierarchy
//! - Build tree structures from flat heading lists
//! - Filter and search headings
//! - Extract sections by heading name
//! - Query documents with a jq-like language ([`query`])
//!
//! ## Stability
//!
//! [`parser`] and [`query`] are the library. The `config`, `input`,
//! `keybindings` and `tui` modules, and the `App` and `Config` re-exports,
//! are public so the `treemd` binary can use them; they are hidden from
//! these docs and may change in any release.
//!
//! ## Example
//!
//! ```rust
//! use treemd::{parse_markdown, Document};
//!
//! let markdown = r#"
//! # Introduction
//! Some content here.
//!
//! ## Background
//! More details.
//!
//! ## Methodology
//! Research approach.
//! "#;
//!
//! let doc = parse_markdown(markdown);
//! println!("Found {} headings", doc.headings.len());
//!
//! // Filter headings by text
//! let filtered = doc.filter_headings("method");
//! for heading in filtered {
//!     println!("{} {}", "#".repeat(heading.level), heading.text);
//! }
//!
//! // Build a tree structure
//! let tree = doc.build_tree();
//! for node in &tree {
//!     println!("{}", node.render_box_tree("", true));
//! }
//! ```

/// Configuration module for persisting user preferences.
///
/// Provides configuration management for theme choices, UI settings, and terminal preferences.
#[doc(hidden)]
pub mod config;

/// Input handling module for stdin and file sources.
///
/// Provides robust input reading, format detection, and tree output parsing.
#[doc(hidden)]
pub mod input;

/// Parser module for markdown documents.
///
/// Provides functions to parse markdown files and content into structured documents.
pub mod parser;

/// TUI module for interactive terminal interface.
///
/// Provides the App and UI rendering functionality for building interactive
/// markdown viewers.
#[doc(hidden)]
pub mod tui;

/// Query language module for jq-like markdown querying.
///
/// Provides a powerful query language for navigating and extracting
/// markdown structure, similar to jq for JSON.
///
/// ## Example
///
/// ```rust
/// use treemd::{parse_markdown, query};
///
/// let doc = parse_markdown("# Hello\n## World");
/// let results = query::execute(&doc, ".h2 | .text").unwrap();
/// assert_eq!(results.len(), 1);
/// ```
pub mod query;

/// Keybindings module for customizable keyboard shortcuts.
///
/// Provides a flexible keybinding system that allows users to customize
/// keyboard shortcuts via configuration files.
#[doc(hidden)]
pub mod keybindings;

mod fsutil;

// Re-export commonly used types for convenience
#[doc(hidden)]
pub use config::Config;
pub use parser::{Document, Heading, HeadingNode, parse_file, parse_markdown};
#[doc(hidden)]
pub use tui::App;
