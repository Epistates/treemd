//! Element extractors for the query language.
//!
//! This module provides the pluggable extraction system for different
//! markdown element types. Custom extractors can be registered to
//! support new element types (e.g., MDX components, custom blocks).
//!
//! # Example: Custom Extractor
//!
//! ```rust
//! use treemd::query::{Registry, Value, ExtractorFn};
//! use std::sync::Arc;
//!
//! // Extract custom admonition blocks (e.g., :::note, :::warning)
//! let admonition_extractor: ExtractorFn = Arc::new(|doc, _ctx| {
//!     let mut results = Vec::new();
//!     // Parse custom syntax from doc.content and extract values
//!     // For now, return empty (placeholder for custom parsing logic)
//!     Ok(results)
//! });
//!
//! let mut registry = Registry::with_builtins();
//! registry.register_extractor("admonition", admonition_extractor);
//! ```

// This module is a placeholder for future custom extractors.
// The built-in extractors are currently implemented in eval.rs.
