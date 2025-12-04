//! # treemd Query Language (tql)
//!
//! A jq-like query language for navigating and extracting markdown structure.
//!
//! ## Architecture
//!
//! The query system is designed with pluggability in mind:
//!
//! - **Function Registry**: Register built-in and custom functions via traits
//! - **Element Extractors**: Pluggable extractors for different markdown elements
//! - **Output Formatters**: Extensible output rendering
//! - **Value System**: Extensible runtime value types
//!
//! ## Example
//!
//! ```ignore
//! use treemd::query::{Engine, Query};
//! use treemd::parser::Document;
//!
//! let doc = Document::parse("# Hello\n## World");
//! let engine = Engine::new(&doc);
//! let results = engine.execute(".h2 | text")?;
//! ```

mod ast;
mod error;
mod eval;
mod lexer;
mod parser;
mod registry;
mod value;

pub mod builtins;
pub mod extractors;

// Re-exports for public API
pub use ast::Span;
pub use ast::{Expr, Query};
pub use error::{QueryError, QueryErrorKind};
pub use eval::{Engine, EvalContext};
pub use registry::{ExtractorFn, Function, FunctionRegistry, Registry};
pub use value::{Value, ValueKind};

use crate::parser::Document;

/// Parse and execute a query against a document.
///
/// This is the main entry point for query execution.
///
/// # Example
///
/// ```ignore
/// let results = treemd::query::execute(&doc, ".h2 | text")?;
/// ```
pub fn execute(doc: &Document, query_str: &str) -> Result<Vec<Value>, QueryError> {
    let query = parse(query_str)?;
    let mut engine = Engine::new(doc);
    engine.execute(&query)
}

/// Parse a query string into an AST.
///
/// Useful when you want to parse once and execute multiple times.
pub fn parse(query_str: &str) -> Result<Query, QueryError> {
    let tokens = lexer::tokenize(query_str)?;
    parser::parse(&tokens, query_str)
}

/// Create a new query engine with default configuration.
pub fn engine(doc: &Document) -> Engine<'_> {
    Engine::new(doc)
}

/// Create a new query engine with a custom registry.
///
/// This allows registering custom functions and extractors.
///
/// # Example
///
/// ```ignore
/// let mut registry = Registry::default();
/// registry.register_function("custom_fn", my_function);
///
/// let engine = treemd::query::engine_with_registry(&doc, registry);
/// ```
pub fn engine_with_registry(doc: &Document, registry: Registry) -> Engine<'_> {
    Engine::with_registry(doc, registry)
}

/// Format query results for output.
pub fn format_output(values: &[Value], format: OutputFormat) -> String {
    output::format(values, format)
}

mod output;

/// Output format for query results.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Plain text, one result per line
    #[default]
    Plain,
    /// JSON array
    Json,
    /// Pretty-printed JSON
    JsonPretty,
    /// Line-delimited JSON (one JSON value per line)
    JsonLines,
    /// Raw markdown output
    Markdown,
    /// Tree structure with box-drawing
    Tree,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plain" | "text" => Ok(Self::Plain),
            "json" => Ok(Self::Json),
            "json-pretty" | "jsonpretty" => Ok(Self::JsonPretty),
            "jsonl" | "jsonlines" | "ndjson" => Ok(Self::JsonLines),
            "md" | "markdown" => Ok(Self::Markdown),
            "tree" => Ok(Self::Tree),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}
