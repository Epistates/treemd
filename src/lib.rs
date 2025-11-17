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
//! - Interactive TUI with dual-pane interface
//! - Syntax-highlighted code blocks (50+ languages)
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
pub mod config;

/// Parser module for markdown documents.
///
/// Provides functions to parse markdown files and content into structured documents.
pub mod parser;

/// TUI module for interactive terminal interface.
///
/// Provides the App and UI rendering functionality for building interactive
/// markdown viewers.
pub mod tui;

// Re-export commonly used types for convenience
pub use config::Config;
pub use parser::{Document, Heading, HeadingNode, parse_file, parse_markdown};
pub use tui::App;
