//! Abstract Syntax Tree types for the query language.
//!
//! The AST represents the parsed structure of a query expression.

use std::fmt;

/// A complete query consisting of one or more piped expressions.
#[derive(Debug, Clone)]
pub struct Query {
    /// The expressions connected by commas (multiple outputs)
    pub expressions: Vec<PipedExpr>,
}

impl Query {
    pub fn new(expressions: Vec<PipedExpr>) -> Self {
        Self { expressions }
    }
}

/// Expressions connected by pipes (`|`).
#[derive(Debug, Clone)]
pub struct PipedExpr {
    /// Pipeline stages executed left-to-right
    pub stages: Vec<Expr>,
}

impl PipedExpr {
    pub fn new(stages: Vec<Expr>) -> Self {
        Self { stages }
    }

    pub fn single(expr: Expr) -> Self {
        Self { stages: vec![expr] }
    }
}

/// A single expression in the query language.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Identity selector: `.`
    Identity,

    /// Element selector: `.h2`, `.code`, `.link`
    Element {
        kind: ElementKind,
        filters: Vec<Filter>,
        index: Option<IndexOp>,
        span: Span,
    },

    /// Property access: `.text`, `.level`
    Property { name: String, span: Span },

    /// Function call: `count`, `select(...)`, `contains(...)`
    Function {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },

    /// Object construction: `{title: .h1.text}`
    Object {
        pairs: Vec<(String, Expr)>,
        span: Span,
    },

    /// Array construction: `[.h2[].text]`
    Array { elements: Vec<Expr>, span: Span },

    /// Conditional: `if ... then ... else ... end`
    Conditional {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },

    /// Hierarchy: `.h1 > .h2` (direct child) or `.h1 >> .h2` (descendant)
    Hierarchy {
        parent: Box<Expr>,
        child: Box<Expr>,
        direct: bool,
        span: Span,
    },

    /// Literal value
    Literal { value: Literal, span: Span },

    /// Binary operation: `==`, `!=`, `>`, `<`, `and`, `or`, `+`, `-`, etc.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },

    /// Unary operation: `not`, `-`
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },

    /// Parenthesized expression for grouping
    Group { expr: Box<Expr>, span: Span },
}

impl Expr {
    /// Get the span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::Identity => Span::new(0, 1),
            Expr::Element { span, .. } => *span,
            Expr::Property { span, .. } => *span,
            Expr::Function { span, .. } => *span,
            Expr::Object { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Conditional { span, .. } => *span,
            Expr::Hierarchy { span, .. } => *span,
            Expr::Literal { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Group { span, .. } => *span,
        }
    }
}

/// Element type for selectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementKind {
    /// Any heading: `.h`
    Heading(Option<u8>),
    /// Code block: `.code`
    Code,
    /// Link: `.link`
    Link,
    /// Image: `.img`
    Image,
    /// Table: `.table`
    Table,
    /// List: `.list`
    List,
    /// Blockquote: `.blockquote`
    Blockquote,
    /// Paragraph: `.para`
    Paragraph,
    /// Front matter: `.frontmatter`
    FrontMatter,
}

impl ElementKind {
    /// Parse an element kind from a string.
    /// Supports multiple aliases for discoverability and convenience.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // Headings - multiple conventions
            "h" | "heading" | "headings" | "header" | "headers" => Some(ElementKind::Heading(None)),
            "h1" => Some(ElementKind::Heading(Some(1))),
            "h2" => Some(ElementKind::Heading(Some(2))),
            "h3" => Some(ElementKind::Heading(Some(3))),
            "h4" => Some(ElementKind::Heading(Some(4))),
            "h5" => Some(ElementKind::Heading(Some(5))),
            "h6" => Some(ElementKind::Heading(Some(6))),

            // Code blocks
            "code" | "codeblock" | "codeblocks" | "pre" => Some(ElementKind::Code),

            // Links - HTML-like and plural
            "link" | "links" | "a" | "anchor" => Some(ElementKind::Link),

            // Images
            "img" | "image" | "images" => Some(ElementKind::Image),

            // Tables
            "table" | "tables" => Some(ElementKind::Table),

            // Lists
            "list" | "lists" | "ul" | "ol" => Some(ElementKind::List),

            // Blockquotes
            "blockquote" | "blockquotes" | "quote" | "quotes" | "bq" => {
                Some(ElementKind::Blockquote)
            }

            // Paragraphs
            "para" | "paragraph" | "paragraphs" | "p" => Some(ElementKind::Paragraph),

            // Front matter
            "frontmatter" | "fm" | "meta" | "yaml" => Some(ElementKind::FrontMatter),

            _ => None,
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ElementKind::Heading(None) => "h",
            ElementKind::Heading(Some(1)) => "h1",
            ElementKind::Heading(Some(2)) => "h2",
            ElementKind::Heading(Some(3)) => "h3",
            ElementKind::Heading(Some(4)) => "h4",
            ElementKind::Heading(Some(5)) => "h5",
            ElementKind::Heading(Some(6)) => "h6",
            ElementKind::Heading(Some(_)) => "h",
            ElementKind::Code => "code",
            ElementKind::Link => "link",
            ElementKind::Image => "img",
            ElementKind::Table => "table",
            ElementKind::List => "list",
            ElementKind::Blockquote => "blockquote",
            ElementKind::Paragraph => "para",
            ElementKind::FrontMatter => "frontmatter",
        }
    }
}

impl fmt::Display for ElementKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Filter for element selection.
#[derive(Debug, Clone)]
pub enum Filter {
    /// Text filter: `[text]` or `["exact text"]`
    Text {
        pattern: String,
        exact: bool,
        span: Span,
    },

    /// Regex filter: `[/pattern/]`
    Regex { pattern: String, span: Span },

    /// Type filter: `[anchor]`, `[external]` for links
    Type { type_name: String, span: Span },
}

/// Index operation for element access.
#[derive(Debug, Clone)]
pub enum IndexOp {
    /// Single index: `[0]`, `[-1]`
    Single(i64),

    /// Slice: `[0:3]`, `[:3]`, `[2:]`
    Slice {
        start: Option<i64>,
        end: Option<i64>,
    },

    /// Iterate (no index): `[]`
    Iterate,
}

/// Literal values.
#[derive(Debug, Clone)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Number(n) => write!(f, "{}", n),
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::Null => write!(f, "null"),
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // String
    Concat,

    // Null coalescing
    Alt, // //
}

impl BinaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "and",
            BinaryOp::Or => "or",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Concat => "+",
            BinaryOp::Alt => "//",
        }
    }

    /// Get operator precedence (higher = binds tighter).
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::Ne => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Alt => 5,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Concat => 6,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 7,
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl UnaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnaryOp::Not => "not",
            UnaryOp::Neg => "-",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Source location span.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_kind_from_str() {
        assert_eq!(
            ElementKind::from_str("h2"),
            Some(ElementKind::Heading(Some(2)))
        );
        assert_eq!(ElementKind::from_str("code"), Some(ElementKind::Code));
        assert_eq!(ElementKind::from_str("link"), Some(ElementKind::Link));
        assert_eq!(ElementKind::from_str("unknown"), None);
    }

    #[test]
    fn test_binary_op_precedence() {
        assert!(BinaryOp::Mul.precedence() > BinaryOp::Add.precedence());
        assert!(BinaryOp::And.precedence() > BinaryOp::Or.precedence());
        assert!(BinaryOp::Eq.precedence() > BinaryOp::And.precedence());
    }
}
