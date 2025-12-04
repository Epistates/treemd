//! Runtime values for query evaluation.
//!
//! The value system is designed to be extensible while maintaining
//! type safety and efficient operations.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Runtime value during query evaluation.
///
/// Values are the currency of the query language - every expression
/// produces and consumes values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null/empty value
    Null,

    /// Boolean
    Bool(bool),

    /// Number (always f64 for simplicity, like JSON)
    Number(f64),

    /// String
    String(String),

    /// Array of values
    Array(Vec<Value>),

    /// Object/map with ordered keys
    Object(IndexMap<String, Value>),

    /// Heading element
    Heading(HeadingValue),

    /// Code block element
    Code(CodeValue),

    /// Link element
    Link(LinkValue),

    /// Image element
    Image(ImageValue),

    /// Table element
    Table(TableValue),

    /// List element
    List(ListValue),

    /// Blockquote element
    Blockquote(BlockquoteValue),

    /// Paragraph element
    Paragraph(ParagraphValue),

    /// Full document reference
    Document(DocumentValue),

    /// Front matter (YAML)
    FrontMatter(IndexMap<String, Value>),
}

impl Value {
    /// Get the kind/type of this value as a string.
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Null => ValueKind::Null,
            Value::Bool(_) => ValueKind::Bool,
            Value::Number(_) => ValueKind::Number,
            Value::String(_) => ValueKind::String,
            Value::Array(_) => ValueKind::Array,
            Value::Object(_) => ValueKind::Object,
            Value::Heading(_) => ValueKind::Heading,
            Value::Code(_) => ValueKind::Code,
            Value::Link(_) => ValueKind::Link,
            Value::Image(_) => ValueKind::Image,
            Value::Table(_) => ValueKind::Table,
            Value::List(_) => ValueKind::List,
            Value::Blockquote(_) => ValueKind::Blockquote,
            Value::Paragraph(_) => ValueKind::Paragraph,
            Value::Document(_) => ValueKind::Document,
            Value::FrontMatter(_) => ValueKind::FrontMatter,
        }
    }

    /// Check if this value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            _ => true, // Element types are always truthy
        }
    }

    /// Try to get this value as a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get this value as a number.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get this value as a bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get this value as an array.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get this value as an object.
    pub fn as_object(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Get a property from this value by name.
    ///
    /// This is the core property access mechanism used by `.property` syntax.
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match self {
            Value::Object(obj) => obj.get(name).cloned(),
            Value::Heading(h) => h.get_property(name),
            Value::Code(c) => c.get_property(name),
            Value::Link(l) => l.get_property(name),
            Value::Image(i) => i.get_property(name),
            Value::Table(t) => t.get_property(name),
            Value::List(l) => l.get_property(name),
            Value::Document(d) => d.get_property(name),
            Value::FrontMatter(fm) => fm.get(name).cloned(),
            _ => None,
        }
    }

    /// Get the "text" representation of this value.
    ///
    /// Used by the `text` function and for plain output.
    pub fn to_text(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    (*n as i64).to_string()
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Array(a) => a.iter().map(|v| v.to_text()).collect::<Vec<_>>().join("\n"),
            Value::Object(o) => serde_json::to_string(o).unwrap_or_default(),
            Value::Heading(h) => h.text.clone(),
            Value::Code(c) => c.content.clone(),
            Value::Link(l) => l.text.clone(),
            Value::Image(i) => i.alt.clone(),
            Value::Table(t) => format!("Table({}x{})", t.headers.len(), t.rows.len()),
            Value::List(l) => l
                .items
                .iter()
                .map(|i| i.content.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            Value::Blockquote(b) => b.content.clone(),
            Value::Paragraph(p) => p.content.clone(),
            Value::Document(d) => d.content.clone(),
            Value::FrontMatter(fm) => serde_json::to_string(fm).unwrap_or_default(),
        }
    }

    /// Get the length of this value (for arrays, strings, objects).
    pub fn len(&self) -> Option<usize> {
        match self {
            Value::String(s) => Some(s.len()),
            Value::Array(a) => Some(a.len()),
            Value::Object(o) => Some(o.len()),
            Value::Table(t) => Some(t.rows.len()),
            Value::List(l) => Some(l.items.len()),
            _ => None,
        }
    }

    /// Check if this value is empty.
    pub fn is_empty(&self) -> bool {
        self.len().map(|l| l == 0).unwrap_or(false)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Number(n as f64)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(o: Option<T>) -> Self {
        match o {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

/// Value type enumeration for type checking and error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
    Heading,
    Code,
    Link,
    Image,
    Table,
    List,
    Blockquote,
    Paragraph,
    Document,
    FrontMatter,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ValueKind::Null => "null",
            ValueKind::Bool => "boolean",
            ValueKind::Number => "number",
            ValueKind::String => "string",
            ValueKind::Array => "array",
            ValueKind::Object => "object",
            ValueKind::Heading => "heading",
            ValueKind::Code => "code",
            ValueKind::Link => "link",
            ValueKind::Image => "image",
            ValueKind::Table => "table",
            ValueKind::List => "list",
            ValueKind::Blockquote => "blockquote",
            ValueKind::Paragraph => "paragraph",
            ValueKind::Document => "document",
            ValueKind::FrontMatter => "frontmatter",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// Element Value Types
// ============================================================================

/// Heading element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingValue {
    pub level: u8,
    pub text: String,
    pub offset: usize,
    pub line: usize,
    /// Content under this heading (excluding subheadings)
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    /// Raw markdown of the entire section
    #[serde(skip)]
    pub raw_md: String,
    /// Index in the flat headings list (for navigation)
    #[serde(skip)]
    pub index: usize,
}

impl HeadingValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "level" => Some(Value::Number(self.level as f64)),
            "text" => Some(Value::String(self.text.clone())),
            "offset" => Some(Value::Number(self.offset as f64)),
            "line" => Some(Value::Number(self.line as f64)),
            "content" => Some(Value::String(self.content.clone())),
            "md" | "markdown" => Some(Value::String(self.raw_md.clone())),
            "slug" => Some(Value::String(slugify(&self.text))),
            _ => None,
        }
    }
}

/// Code block element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl CodeValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "lang" | "language" => self
                .language
                .clone()
                .map(Value::String)
                .or(Some(Value::Null)),
            "text" | "content" => Some(Value::String(self.content.clone())),
            "start_line" => Some(Value::Number(self.start_line as f64)),
            "end_line" => Some(Value::Number(self.end_line as f64)),
            "lines" => Some(Value::Number(self.content.lines().count() as f64)),
            _ => None,
        }
    }
}

/// Link element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkValue {
    pub text: String,
    pub url: String,
    #[serde(rename = "type")]
    pub link_type: LinkType,
    pub offset: usize,
}

impl LinkValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "text" => Some(Value::String(self.text.clone())),
            "url" => Some(Value::String(self.url.clone())),
            "type" => Some(Value::String(self.link_type.as_str().to_string())),
            "offset" => Some(Value::Number(self.offset as f64)),
            _ => None,
        }
    }
}

/// Link type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Anchor,
    Relative,
    WikiLink,
    External,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::Anchor => "anchor",
            LinkType::Relative => "relative",
            LinkType::WikiLink => "wikilink",
            LinkType::External => "external",
        }
    }
}

/// Image element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageValue {
    pub alt: String,
    pub src: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl ImageValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "alt" | "text" => Some(Value::String(self.alt.clone())),
            "src" | "url" => Some(Value::String(self.src.clone())),
            "title" => self.title.clone().map(Value::String).or(Some(Value::Null)),
            _ => None,
        }
    }
}

/// Table element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableValue {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub alignments: Vec<String>,
}

impl TableValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "headers" => Some(Value::Array(
                self.headers
                    .iter()
                    .map(|h| Value::String(h.clone()))
                    .collect(),
            )),
            "rows" => Some(Value::Array(
                self.rows
                    .iter()
                    .map(|row| Value::Array(row.iter().map(|c| Value::String(c.clone())).collect()))
                    .collect(),
            )),
            "cols" | "columns" => Some(Value::Number(self.headers.len() as f64)),
            "alignments" => Some(Value::Array(
                self.alignments
                    .iter()
                    .map(|a| Value::String(a.clone()))
                    .collect(),
            )),
            _ => None,
        }
    }
}

/// List element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListValue {
    pub ordered: bool,
    pub items: Vec<ListItemValue>,
}

impl ListValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "ordered" => Some(Value::Bool(self.ordered)),
            "items" => Some(Value::Array(
                self.items
                    .iter()
                    .map(|i| Value::String(i.content.clone()))
                    .collect(),
            )),
            "length" | "count" => Some(Value::Number(self.items.len() as f64)),
            _ => None,
        }
    }
}

/// List item value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItemValue {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
}

/// Blockquote element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockquoteValue {
    pub content: String,
}

/// Paragraph element value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphValue {
    pub content: String,
}

/// Document value (root).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentValue {
    pub content: String,
    pub heading_count: usize,
    pub word_count: usize,
}

impl DocumentValue {
    pub fn get_property(&self, name: &str) -> Option<Value> {
        match name {
            "content" | "text" => Some(Value::String(self.content.clone())),
            "heading_count" | "headings" => Some(Value::Number(self.heading_count as f64)),
            "word_count" | "words" => Some(Value::Number(self.word_count as f64)),
            _ => None,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate URL-friendly slug from text.
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '.' || c == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_kind() {
        assert_eq!(Value::Null.kind(), ValueKind::Null);
        assert_eq!(Value::Bool(true).kind(), ValueKind::Bool);
        assert_eq!(Value::Number(42.0).kind(), ValueKind::Number);
        assert_eq!(Value::String("test".into()).kind(), ValueKind::String);
    }

    #[test]
    fn test_value_truthy() {
        assert!(!Value::Null.is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::String("".into()).is_truthy());
        assert!(Value::String("hello".into()).is_truthy());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Getting Started!"), "getting-started");
        assert_eq!(slugify("API v2.0"), "api-v2-0");
    }
}
