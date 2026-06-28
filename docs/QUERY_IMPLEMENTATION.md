# treemd Query Language - Implementation Architecture

## Overview

This document outlines the implementation architecture for treemd's jq-like query language.

## Module Structure

```
src/
├── query/                      # NEW: Query language implementation
│   ├── mod.rs                  # Public API
│   ├── lexer.rs                # Tokenizer (~300 lines)
│   ├── parser.rs               # Query parser (~500 lines)
│   ├── ast.rs                  # AST types (~200 lines)
│   ├── eval.rs                 # Query evaluator (~600 lines)
│   ├── value.rs                # Runtime values (~150 lines)
│   ├── builtins/               # Built-in functions
│   │   ├── mod.rs              # Function registry
│   │   ├── string.rs           # String functions (~150 lines)
│   │   ├── collection.rs       # Collection functions (~200 lines)
│   │   ├── navigation.rs       # Tree navigation (~150 lines)
│   │   └── aggregation.rs      # Stats/counting (~100 lines)
│   ├── error.rs                # Error types & formatting (~200 lines)
│   └── output.rs               # Output formatting (~150 lines)
├── cli/
│   ├── commands.rs             # Add -q/--query flag
│   └── query_repl.rs           # Optional: Interactive REPL
├── parser/
│   ├── mod.rs                  # Add full document parsing
│   ├── document.rs             # Enhanced with content blocks
│   └── frontmatter.rs          # NEW: YAML front matter parsing
└── main.rs                     # Integrate query execution
```

## Key Types

### AST (`src/query/ast.rs`)

```rust
/// A complete query expression
#[derive(Debug, Clone)]
pub struct Query {
    pub expressions: Vec<PipedExpr>,
}

/// Expressions connected by pipes
#[derive(Debug, Clone)]
pub struct PipedExpr {
    pub stages: Vec<Expr>,
}

/// Single expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// Identity selector: `.`
    Identity,

    /// Element selector: `.h2`, `.code`, `.link`
    Element {
        kind: ElementKind,
        filters: Vec<Filter>,
        index: Option<IndexOp>,
    },

    /// Property access: `.text`, `.level`
    Property(String),

    /// Function call: `count`, `select(...)`, `contains(...)`
    Function {
        name: String,
        args: Vec<Expr>,
    },

    /// Object construction: `{title: .h1.text}`
    Object(Vec<(String, Expr)>),

    /// Array construction: `[.h2[].text]`
    Array(Vec<Expr>),

    /// Conditional: `if ... then ... else ... end`
    Conditional {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    /// Multiple outputs: `.h1, .h2`
    Multiple(Vec<Expr>),

    /// Hierarchy: `.h1 > .h2` or `.h1 >> .h2`
    Hierarchy {
        parent: Box<Expr>,
        child: Box<Expr>,
        direct: bool, // true for >, false for >>
    },

    /// Literal values
    Literal(Literal),

    /// Binary operations: `==`, `!=`, `>`, `<`, `and`, `or`
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operations: `not`
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum ElementKind {
    Heading(Option<u8>),  // None = any level, Some(n) = specific level
    Code,
    Link,
    Image,
    Table,
    List,
    Blockquote,
    Paragraph,
    FrontMatter,
}

#[derive(Debug, Clone)]
pub enum Filter {
    /// Text filter: `[text]` or `["text"]`
    Text { pattern: String, exact: bool },

    /// Regex filter: `[/pattern/]`
    Regex(String),

    /// Type filter for links: `[anchor]`, `[external]`
    Type(String),
}

#[derive(Debug, Clone)]
pub enum IndexOp {
    /// Single index: `[0]`, `[-1]`
    Single(i64),

    /// Slice: `[0:3]`, `[:3]`, `[2:]`
    Slice { start: Option<i64>, end: Option<i64> },

    /// Iterate: `[]`
    Iterate,
}

#[derive(Debug, Clone)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Add, Sub, Mul, Div,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not,
    Neg,
}
```

### Runtime Values (`src/query/value.rs`)

```rust
/// Runtime value during query evaluation
#[derive(Debug, Clone)]
pub enum Value {
    /// Null/empty
    Null,

    /// Boolean
    Bool(bool),

    /// Number
    Number(f64),

    /// String
    String(String),

    /// Array of values
    Array(Vec<Value>),

    /// Object/map
    Object(IndexMap<String, Value>),

    /// Heading reference
    Heading(HeadingValue),

    /// Code block reference
    Code(CodeValue),

    /// Link reference
    Link(LinkValue),

    /// Image reference
    Image(ImageValue),

    /// Table reference
    Table(TableValue),

    /// List reference
    List(ListValue),

    /// Full document reference
    Document(DocumentValue),
}

#[derive(Debug, Clone)]
pub struct HeadingValue {
    pub level: u8,
    pub text: String,
    pub offset: usize,
    pub line: usize,
    pub content: String,
    pub raw_md: String,
    pub children: Vec<HeadingValue>,
    pub parent_idx: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct CodeValue {
    pub language: Option<String>,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct LinkValue {
    pub text: String,
    pub url: String,
    pub link_type: LinkType,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub enum LinkType {
    Anchor,
    Relative,
    WikiLink,
    External,
}

// ... similar for Image, Table, List
```

### Evaluator (`src/query/eval.rs`)

```rust
pub struct Evaluator<'a> {
    document: &'a Document,
    context: EvalContext,
}

pub struct EvalContext {
    /// Current value being processed
    current: Value,

    /// Document-level data (headings, blocks, etc.)
    doc_data: DocumentData,

    /// Variable bindings
    variables: HashMap<String, Value>,
}

impl<'a> Evaluator<'a> {
    pub fn new(document: &'a Document) -> Self { ... }

    pub fn eval(&mut self, query: &Query) -> Result<Vec<Value>, EvalError> {
        let mut results = vec![Value::Document(self.document_value())];

        for piped in &query.expressions {
            results = self.eval_piped(piped, results)?;
        }

        Ok(results)
    }

    fn eval_piped(&mut self, piped: &PipedExpr, inputs: Vec<Value>) -> Result<Vec<Value>, EvalError> {
        let mut current = inputs;

        for stage in &piped.stages {
            let mut next = Vec::new();
            for input in current {
                self.context.current = input;
                next.extend(self.eval_expr(stage)?);
            }
            current = next;
        }

        Ok(current)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Vec<Value>, EvalError> {
        match expr {
            Expr::Identity => Ok(vec![self.context.current.clone()]),

            Expr::Element { kind, filters, index } => {
                self.eval_element(kind, filters, index)
            }

            Expr::Property(name) => {
                self.eval_property(name)
            }

            Expr::Function { name, args } => {
                self.eval_function(name, args)
            }

            // ... other cases
        }
    }

    fn eval_element(&mut self, kind: &ElementKind, filters: &[Filter], index: &Option<IndexOp>) -> Result<Vec<Value>, EvalError> {
        let elements = match kind {
            ElementKind::Heading(level) => self.get_headings(*level),
            ElementKind::Code => self.get_code_blocks(),
            ElementKind::Link => self.get_links(),
            // ...
        };

        let filtered = self.apply_filters(elements, filters)?;
        self.apply_index(filtered, index)
    }

    // ... implementation details
}
```

## Error Handling (`src/query/error.rs`)

```rust
#[derive(Debug)]
pub struct QueryError {
    pub kind: QueryErrorKind,
    pub span: Span,
    pub source: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug)]
pub enum QueryErrorKind {
    // Lexer errors
    UnexpectedChar(char),
    UnterminatedString,
    UnterminatedRegex,

    // Parser errors
    UnexpectedToken { expected: Vec<TokenKind>, found: TokenKind },
    InvalidHeadingLevel(u8),
    InvalidElementType(String),

    // Evaluation errors
    TypeError { expected: &'static str, found: &'static str },
    PropertyNotFound { property: String, on_type: &'static str },
    UnknownFunction(String),
    NoMatch { selector: String, available: Vec<String> },
    IndexOutOfBounds { index: i64, length: usize },
}

impl QueryError {
    pub fn format(&self) -> String {
        // Rich error formatting with:
        // - Source location
        // - Underlined problem area
        // - Contextual suggestions
        // - Similar names for typos
    }
}
```

## CLI Integration (`src/cli/commands.rs`)

```rust
#[derive(Parser, Debug)]
pub struct Cli {
    // ... existing fields ...

    /// Query expression for selecting/filtering document elements
    ///
    /// Uses a jq-like syntax for navigating and extracting markdown structure.
    /// See `treemd --query-help` for full documentation.
    ///
    /// Examples:
    ///   -q '.h2'                    # All h2 headings
    ///   -q '.code[rust]'            # Rust code blocks
    ///   -q '.h1[Install] | content' # Installation section
    ///   -q '.link | url'            # All link URLs
    #[arg(short = 'q', long = "query", value_name = "EXPR")]
    pub query: Option<String>,

    /// Show query language help and examples
    #[arg(long = "query-help")]
    pub query_help: bool,

    /// Start interactive query REPL
    #[arg(long = "repl")]
    pub repl: bool,
}
```

## Output Formatting (`src/query/output.rs`)

```rust
pub enum OutputFormat {
    /// Plain text, one result per line
    Plain,
    /// JSON array/object
    Json,
    /// Line-delimited JSON
    JsonLines,
    /// Raw markdown
    Markdown,
    /// Tree structure
    Tree,
}

pub fn format_output(values: &[Value], format: OutputFormat) -> String {
    match format {
        OutputFormat::Plain => format_plain(values),
        OutputFormat::Json => format_json(values),
        OutputFormat::JsonLines => format_json_lines(values),
        OutputFormat::Markdown => format_markdown(values),
        OutputFormat::Tree => format_tree(values),
    }
}
```

## Dependencies

```toml
[dependencies]
# Existing
pulldown-cmark = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# New for query language
logos = "0.14"           # Fast lexer generator
indexmap = "2.0"         # Ordered map for objects
regex = "1.10"           # Regex support
yaml-rust2 = "0.8"       # Front matter parsing (replaces unmaintained yaml-rust)

# Optional
rustyline = "14.0"       # REPL support (optional feature)
```

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    // Unit tests for each component
    mod lexer_tests { ... }
    mod parser_tests { ... }
    mod eval_tests { ... }

    // Integration tests with real markdown
    #[test]
    fn test_heading_selection() {
        let doc = parse_markdown("# Title\n## Section\n### Sub");
        let result = eval_query(&doc, ".h2");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "Section");
    }

    #[test]
    fn test_code_extraction() {
        let doc = parse_markdown("```rust\nfn main() {}\n```");
        let result = eval_query(&doc, ".code[rust] | text");
        assert_eq!(result[0], "fn main() {}");
    }

    // Snapshot tests for complex queries
    #[test]
    fn test_complex_query_output() {
        let doc = parse_file("tests/fixtures/complex.md");
        let result = eval_query(&doc, ".h1[Features] > .h2 | {text, level}");
        insta::assert_json_snapshot!(result);
    }
}
```

## Performance Considerations

1. **Lazy evaluation**: Don't parse entire document for simple queries
2. **Index caching**: Cache heading tree, code blocks, links on first access
3. **Early termination**: `first`, `nth(0)` should stop after finding match
4. **Streaming output**: For large results, stream JSON instead of buffering

## Implementation Order

### Phase 1: MVP (~2-3 days)
1. Lexer with basic tokens
2. Parser for element selectors
3. Basic evaluator
4. Properties: `text`, `level`, `lang`, `url`
5. Functions: `count`, `select`, `contains`
6. CLI integration with `-q` flag

### Phase 2: Core Features (~2-3 days)
1. Indexing and slicing
2. Pipes
3. Hierarchy navigation (`>`, `>>`)
4. More filters: `startswith`, `endswith`, `matches`
5. JSON output formatting

### Phase 3: Advanced (~3-4 days)
1. Construction syntax (`{}`, `[]`)
2. Navigation functions (`parent`, `children`, etc.)
3. String functions
4. Aggregations
5. Front matter support

### Phase 4: Polish (~2-3 days)
1. Rich error messages
2. REPL mode
3. Shell completions for query syntax
4. Documentation and examples
5. Performance optimization
