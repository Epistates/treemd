//! Error types and formatting for the query language.
//!
//! Provides rich, Rust-quality error messages with source spans,
//! suggestions, and contextual help.

use super::ast::Span;
use super::lexer::TokenKind;
use std::fmt;

/// Query error with source location and suggestions.
#[derive(Debug)]
pub struct QueryError(pub Box<QueryErrorInner>);

#[derive(Debug)]
pub struct QueryErrorInner {
    pub kind: QueryErrorKind,
    pub span: Span,
    pub source: String,
    pub suggestions: Vec<String>,
    pub help: Option<String>,
    pub note: Option<String>,
}

impl QueryError {
    pub fn new(kind: QueryErrorKind, span: Span, source: String) -> Self {
        Self(Box::new(QueryErrorInner {
            kind,
            span,
            source,
            suggestions: Vec::new(),
            help: None,
            note: None,
        }))
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.0.suggestions = suggestions;
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.0.help = Some(help.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.0.note = Some(note.into());
        self
    }

    /// Format the error for display.
    pub fn format(&self) -> String {
        let inner = &self.0;
        let mut output = String::new();

        // Error header
        output.push_str(&format!("error: {}\n", inner.kind));

        // Source snippet with span indicator
        if !inner.source.is_empty() {
            output.push_str("  --> query\n");
            output.push_str("  |\n");

            // Show the line containing the error
            let line = inner.source.lines().next().unwrap_or(&inner.source);
            output.push_str(&format!("1 | {}\n", line));

            // Underline the error span
            let start = inner.span.start.min(line.len());
            let end = inner.span.end.min(line.len()).max(start + 1);
            let padding = " ".repeat(start + 4); // "1 | " = 4 chars
            let underline = "^".repeat(end - start);
            output.push_str(&format!(
                "{}{}  {}\n",
                padding,
                underline,
                inner.kind.short_message()
            ));
        }

        // Suggestions
        if !inner.suggestions.is_empty() {
            output.push_str("  |\n");
            output.push_str(&format!(
                "  = help: did you mean {}?\n",
                inner
                    .suggestions
                    .iter()
                    .map(|s| format!("'{}'", s))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ));
        }

        // Help message
        if let Some(ref help) = inner.help {
            output.push_str(&format!("  = help: {}\n", help));
        }

        // Note
        if let Some(ref note) = inner.note {
            output.push_str(&format!("  = note: {}\n", note));
        }

        output
    }
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl std::error::Error for QueryError {}

/// Query error kinds.
#[derive(Debug)]
pub enum QueryErrorKind {
    // Lexer errors
    UnexpectedChar(char),
    UnterminatedString,
    UnterminatedRegex,
    InvalidEscape(char),

    // Parser errors
    UnexpectedToken {
        expected: Vec<&'static str>,
        found: TokenKind,
    },
    UnexpectedEof {
        expected: Vec<&'static str>,
    },
    InvalidHeadingLevel(u8),
    InvalidElementType(String),
    InvalidFilter(String),
    MissingColon,
    MissingClosingBracket,
    MissingClosingParen,
    MissingClosingBrace,
    MissingThen,
    MissingEnd,

    // Evaluation errors
    TypeError {
        expected: &'static str,
        found: String,
    },
    PropertyNotFound {
        property: String,
        on_type: String,
    },
    UnknownFunction(String),
    UnknownElement(String),
    InvalidArity {
        function: String,
        expected: String,
        found: usize,
    },
    NoMatch {
        selector: String,
        available: Vec<String>,
    },
    IndexOutOfBounds {
        index: i64,
        length: usize,
    },
    InvalidRegex {
        pattern: String,
        error: String,
    },
    DivisionByZero,
}

impl QueryErrorKind {
    /// Get a short message for inline display.
    pub fn short_message(&self) -> &'static str {
        match self {
            QueryErrorKind::UnexpectedChar(_) => "unexpected character",
            QueryErrorKind::UnterminatedString => "string not closed",
            QueryErrorKind::UnterminatedRegex => "regex not closed",
            QueryErrorKind::InvalidEscape(_) => "invalid escape",
            QueryErrorKind::UnexpectedToken { .. } => "unexpected token",
            QueryErrorKind::UnexpectedEof { .. } => "unexpected end",
            QueryErrorKind::InvalidHeadingLevel(_) => "invalid level",
            QueryErrorKind::InvalidElementType(_) => "unknown element",
            QueryErrorKind::InvalidFilter(_) => "invalid filter",
            QueryErrorKind::MissingColon => "expected ':'",
            QueryErrorKind::MissingClosingBracket => "expected ']'",
            QueryErrorKind::MissingClosingParen => "expected ')'",
            QueryErrorKind::MissingClosingBrace => "expected '}'",
            QueryErrorKind::MissingThen => "expected 'then'",
            QueryErrorKind::MissingEnd => "expected 'end'",
            QueryErrorKind::TypeError { .. } => "type error",
            QueryErrorKind::PropertyNotFound { .. } => "no such property",
            QueryErrorKind::UnknownFunction(_) => "unknown function",
            QueryErrorKind::UnknownElement(_) => "unknown element",
            QueryErrorKind::InvalidArity { .. } => "wrong argument count",
            QueryErrorKind::NoMatch { .. } => "no match",
            QueryErrorKind::IndexOutOfBounds { .. } => "index out of bounds",
            QueryErrorKind::InvalidRegex { .. } => "invalid regex",
            QueryErrorKind::DivisionByZero => "division by zero",
        }
    }
}

impl fmt::Display for QueryErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryErrorKind::UnexpectedChar(c) => {
                write!(f, "Unexpected character '{}'", c)
            }
            QueryErrorKind::UnterminatedString => {
                write!(f, "Unterminated string literal")
            }
            QueryErrorKind::UnterminatedRegex => {
                write!(f, "Unterminated regex pattern")
            }
            QueryErrorKind::InvalidEscape(c) => {
                write!(f, "Invalid escape sequence '\\{}'", c)
            }
            QueryErrorKind::UnexpectedToken { expected, found } => {
                if expected.len() == 1 {
                    write!(f, "Expected {}, found {}", expected[0], found.name())
                } else {
                    write!(
                        f,
                        "Expected one of {}, found {}",
                        expected.join(", "),
                        found.name()
                    )
                }
            }
            QueryErrorKind::UnexpectedEof { expected } => {
                if expected.len() == 1 {
                    write!(f, "Unexpected end of input, expected {}", expected[0])
                } else {
                    write!(
                        f,
                        "Unexpected end of input, expected one of {}",
                        expected.join(", ")
                    )
                }
            }
            QueryErrorKind::InvalidHeadingLevel(level) => {
                write!(
                    f,
                    "Invalid heading level '{}' (must be 1-6, or use 'h' for any)",
                    level
                )
            }
            QueryErrorKind::InvalidElementType(name) => {
                write!(f, "Unknown element type '{}'", name)
            }
            QueryErrorKind::InvalidFilter(msg) => {
                write!(f, "Invalid filter: {}", msg)
            }
            QueryErrorKind::MissingColon => {
                write!(f, "Expected ':' in object literal")
            }
            QueryErrorKind::MissingClosingBracket => {
                write!(f, "Missing closing ']'")
            }
            QueryErrorKind::MissingClosingParen => {
                write!(f, "Missing closing ')'")
            }
            QueryErrorKind::MissingClosingBrace => {
                write!(f, "Missing closing '}}'")
            }
            QueryErrorKind::MissingThen => {
                write!(f, "Expected 'then' after condition")
            }
            QueryErrorKind::MissingEnd => {
                write!(f, "Expected 'end' to close conditional")
            }
            QueryErrorKind::TypeError { expected, found } => {
                write!(f, "Type error: expected {}, found {}", expected, found)
            }
            QueryErrorKind::PropertyNotFound { property, on_type } => {
                write!(f, "Property '{}' not found on {}", property, on_type)
            }
            QueryErrorKind::UnknownFunction(name) => {
                write!(f, "Unknown function '{}'", name)
            }
            QueryErrorKind::UnknownElement(name) => {
                write!(f, "Unknown element selector '{}'", name)
            }
            QueryErrorKind::InvalidArity {
                function,
                expected,
                found,
            } => {
                write!(
                    f,
                    "Function '{}' expects {} arguments, got {}",
                    function, expected, found
                )
            }
            QueryErrorKind::NoMatch {
                selector,
                available,
            } => {
                let available_str = if available.is_empty() {
                    "none available".to_string()
                } else {
                    available.join(", ")
                };
                write!(
                    f,
                    "No elements match '{}' (available: {})",
                    selector, available_str
                )
            }
            QueryErrorKind::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds (length: {})", index, length)
            }
            QueryErrorKind::InvalidRegex { pattern, error } => {
                write!(f, "Invalid regex '{}': {}", pattern, error)
            }
            QueryErrorKind::DivisionByZero => {
                write!(f, "Division by zero")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_formatting() {
        let error = QueryError::new(
            QueryErrorKind::InvalidElementType("h99".to_string()),
            Span::new(1, 4),
            ".h99".to_string(),
        )
        .with_suggestions(vec!["h1".to_string(), "h2".to_string()])
        .with_help("heading levels must be 1-6");

        let formatted = error.format();
        assert!(formatted.contains("error:"));
        assert!(formatted.contains("h99"));
        assert!(formatted.contains("h1"));
        assert!(formatted.contains("heading levels"));
    }
}
