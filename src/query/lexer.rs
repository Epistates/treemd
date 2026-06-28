//! Lexer/tokenizer for the query language.
//!
//! Converts a query string into a stream of tokens for parsing.

use super::ast::Span;
use super::error::{QueryError, QueryErrorKind};

/// Token types for the query language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Punctuation
    Dot,      // .
    Pipe,     // |
    Comma,    // ,
    Colon,    // :
    LBracket, // [
    RBracket, // ]
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    Gt,       // >
    GtGt,     // >>
    Question, // ?

    // Operators
    Eq,         // ==
    Ne,         // !=
    Lt,         // <
    Le,         // <=
    Ge,         // >=
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    SlashSlash, // //

    // Keywords
    And,
    Or,
    Not,
    If,
    Then,
    Elif,
    Else,
    End,
    True,
    False,
    Null,

    // Literals
    String(String),
    Number(f64),

    // Identifiers
    Ident(String),

    // End of input
    Eof,
}

impl TokenKind {
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::Dot => "'.'",
            TokenKind::Pipe => "'|'",
            TokenKind::Comma => "','",
            TokenKind::Colon => "':'",
            TokenKind::LBracket => "'['",
            TokenKind::RBracket => "']'",
            TokenKind::LParen => "'('",
            TokenKind::RParen => "')'",
            TokenKind::LBrace => "'{'",
            TokenKind::RBrace => "'}'",
            TokenKind::Gt => "'>'",
            TokenKind::GtGt => "'>>'",
            TokenKind::Question => "'?'",
            TokenKind::Eq => "'=='",
            TokenKind::Ne => "'!='",
            TokenKind::Lt => "'<'",
            TokenKind::Le => "'<='",
            TokenKind::Ge => "'>='",
            TokenKind::Plus => "'+'",
            TokenKind::Minus => "'-'",
            TokenKind::Star => "'*'",
            TokenKind::Slash => "'/'",
            TokenKind::Percent => "'%'",
            TokenKind::SlashSlash => "'//'",
            TokenKind::And => "'and'",
            TokenKind::Or => "'or'",
            TokenKind::Not => "'not'",
            TokenKind::If => "'if'",
            TokenKind::Then => "'then'",
            TokenKind::Elif => "'elif'",
            TokenKind::Else => "'else'",
            TokenKind::End => "'end'",
            TokenKind::True => "'true'",
            TokenKind::False => "'false'",
            TokenKind::Null => "'null'",
            TokenKind::String(_) => "string",
            TokenKind::Number(_) => "number",
            TokenKind::Ident(_) => "identifier",
            TokenKind::Eof => "end of input",
        }
    }
}

/// A token with its source location.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Lexer state.
struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            pos: 0,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((pos, _)) = result {
            self.pos = pos + 1;
        }
        result
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self, quote: char, start: usize) -> Result<Token, QueryError> {
        let mut value = String::new();

        loop {
            match self.advance() {
                Some((_, c)) if c == quote => {
                    return Ok(Token::new(
                        TokenKind::String(value),
                        Span::new(start, self.pos),
                    ));
                }
                Some((_, '\\')) => {
                    // Escape sequences
                    match self.advance() {
                        Some((_, 'n')) => value.push('\n'),
                        Some((_, 'r')) => value.push('\r'),
                        Some((_, 't')) => value.push('\t'),
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, c)) if c == quote => value.push(c),
                        Some((pos, c)) => {
                            return Err(QueryError::new(
                                QueryErrorKind::InvalidEscape(c),
                                Span::new(pos, pos + 1),
                                self.input.to_string(),
                            ));
                        }
                        None => {
                            return Err(QueryError::new(
                                QueryErrorKind::UnterminatedString,
                                Span::new(start, self.pos),
                                self.input.to_string(),
                            ));
                        }
                    }
                }
                Some((_, c)) => value.push(c),
                None => {
                    return Err(QueryError::new(
                        QueryErrorKind::UnterminatedString,
                        Span::new(start, self.pos),
                        self.input.to_string(),
                    ));
                }
            }
        }
    }

    fn read_number(&mut self, start: usize, first_char: char) -> Result<Token, QueryError> {
        let mut num_str = String::new();
        num_str.push(first_char);

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '-' || c == '+' {
                // Handle signs only after e/E
                if (c == '-' || c == '+') && !num_str.ends_with('e') && !num_str.ends_with('E') {
                    break;
                }
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Reject malformed numbers (e.g. `1.2.3`, `1e`) rather than silently
        // coercing them to 0.0.
        match num_str.parse::<f64>() {
            Ok(value) => Ok(Token::new(
                TokenKind::Number(value),
                Span::new(start, self.pos),
            )),
            Err(_) => Err(QueryError::new(
                QueryErrorKind::InvalidNumber(num_str),
                Span::new(start, self.pos),
                self.input.to_string(),
            )),
        }
    }

    fn read_identifier(&mut self, start: usize, first_char: char) -> Token {
        let mut ident = String::new();
        ident.push(first_char);

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match ident.as_str() {
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "elif" => TokenKind::Elif,
            "else" => TokenKind::Else,
            "end" => TokenKind::End,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(ident),
        };

        Token::new(kind, Span::new(start, self.pos))
    }

    fn next_token(&mut self, prev: Option<&TokenKind>) -> Result<Token, QueryError> {
        self.skip_whitespace();

        let Some((start, c)) = self.advance() else {
            return Ok(Token::new(
                TokenKind::Eof,
                Span::new(self.input.len(), self.input.len()),
            ));
        };

        let token = match c {
            '.' => Token::new(TokenKind::Dot, Span::new(start, self.pos)),
            '|' => Token::new(TokenKind::Pipe, Span::new(start, self.pos)),
            ',' => Token::new(TokenKind::Comma, Span::new(start, self.pos)),
            ':' => Token::new(TokenKind::Colon, Span::new(start, self.pos)),
            '[' => Token::new(TokenKind::LBracket, Span::new(start, self.pos)),
            ']' => Token::new(TokenKind::RBracket, Span::new(start, self.pos)),
            '(' => Token::new(TokenKind::LParen, Span::new(start, self.pos)),
            ')' => Token::new(TokenKind::RParen, Span::new(start, self.pos)),
            '{' => Token::new(TokenKind::LBrace, Span::new(start, self.pos)),
            '}' => Token::new(TokenKind::RBrace, Span::new(start, self.pos)),
            '?' => Token::new(TokenKind::Question, Span::new(start, self.pos)),
            '+' => Token::new(TokenKind::Plus, Span::new(start, self.pos)),
            '*' => Token::new(TokenKind::Star, Span::new(start, self.pos)),
            '%' => Token::new(TokenKind::Percent, Span::new(start, self.pos)),

            '-' => {
                // A leading `-` is a negative numeric literal only in prefix
                // position — at the start of input or right after an operator,
                // an opening delimiter, `,`/`:`/`|`, or a keyword. In infix
                // position (after a value, identifier, `)`, `]`, or number) it
                // is the subtraction operator, so `5-3` lexes as `5 - 3`.
                if prev_allows_prefix_minus(prev)
                    && let Some(c) = self.peek()
                    && c.is_ascii_digit()
                {
                    return self.read_number(start, '-');
                }
                Token::new(TokenKind::Minus, Span::new(start, self.pos))
            }

            '>' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token::new(TokenKind::GtGt, Span::new(start, self.pos))
                } else if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Ge, Span::new(start, self.pos))
                } else {
                    Token::new(TokenKind::Gt, Span::new(start, self.pos))
                }
            }

            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Le, Span::new(start, self.pos))
                } else {
                    Token::new(TokenKind::Lt, Span::new(start, self.pos))
                }
            }

            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Eq, Span::new(start, self.pos))
                } else {
                    return Err(QueryError::new(
                        QueryErrorKind::UnexpectedChar('='),
                        Span::new(start, self.pos),
                        self.input.to_string(),
                    )
                    .with_help("Use '==' for equality comparison"));
                }
            }

            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::new(TokenKind::Ne, Span::new(start, self.pos))
                } else {
                    return Err(QueryError::new(
                        QueryErrorKind::UnexpectedChar('!'),
                        Span::new(start, self.pos),
                        self.input.to_string(),
                    )
                    .with_help("Use 'not' for negation or '!=' for inequality"));
                }
            }

            '/' => {
                if self.peek() == Some('/') {
                    self.advance();
                    Token::new(TokenKind::SlashSlash, Span::new(start, self.pos))
                } else {
                    // Could be division or start of regex
                    // For now, treat as division; parser will handle regex context
                    Token::new(TokenKind::Slash, Span::new(start, self.pos))
                }
            }

            '"' => self.read_string('"', start)?,
            '\'' => self.read_string('\'', start)?,

            c if c.is_ascii_digit() => return self.read_number(start, c),

            c if c.is_alphabetic() || c == '_' => self.read_identifier(start, c),

            c => {
                return Err(QueryError::new(
                    QueryErrorKind::UnexpectedChar(c),
                    Span::new(start, self.pos),
                    self.input.to_string(),
                ));
            }
        };

        Ok(token)
    }
}

/// Whether a `-` following the token `prev` should be lexed as the sign of a
/// negative numeric literal (prefix position) rather than the subtraction
/// operator (infix position).
///
/// Prefix position is: start of input, or directly after an operator, an
/// opening delimiter, `,`/`:`/`|`, or a keyword. After a value-producing token
/// (number, string, identifier, `)`, `]`) a `-` is subtraction.
fn prev_allows_prefix_minus(prev: Option<&TokenKind>) -> bool {
    match prev {
        // Start of input.
        None => true,
        Some(kind) => matches!(
            kind,
            // Open delimiters
            TokenKind::LBracket
                | TokenKind::LParen
                | TokenKind::LBrace
                // Separators / pipe
                | TokenKind::Comma
                | TokenKind::Colon
                | TokenKind::Pipe
                // Comparison / arithmetic operators
                | TokenKind::Eq
                | TokenKind::Ne
                | TokenKind::Lt
                | TokenKind::Le
                | TokenKind::Gt
                | TokenKind::GtGt
                | TokenKind::Ge
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::SlashSlash
                | TokenKind::Percent
                // Keywords that introduce an expression
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::If
                | TokenKind::Then
                | TokenKind::Elif
                | TokenKind::Else
        ),
    }
}

/// Tokenize a query string.
pub fn tokenize(input: &str) -> Result<Vec<Token>, QueryError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    let mut prev_kind: Option<TokenKind> = None;

    loop {
        let token = lexer.next_token(prev_kind.as_ref())?;
        let is_eof = matches!(token.kind, TokenKind::Eof);
        prev_kind = Some(token.kind.clone());
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize_kinds(input: &str) -> Vec<TokenKind> {
        tokenize(input)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_simple_tokens() {
        assert_eq!(
            tokenize_kinds(".h2"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h2".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_pipe() {
        assert_eq!(
            tokenize_kinds(".h2 | text"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h2".into()),
                TokenKind::Pipe,
                TokenKind::Ident("text".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_filter_syntax() {
        assert_eq!(
            tokenize_kinds(".h[Features]"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h".into()),
                TokenKind::LBracket,
                TokenKind::Ident("Features".into()),
                TokenKind::RBracket,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_string_filter() {
        assert_eq!(
            tokenize_kinds(r#".h["Getting Started"]"#),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h".into()),
                TokenKind::LBracket,
                TokenKind::String("Getting Started".into()),
                TokenKind::RBracket,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(
            tokenize_kinds(".h2[0]"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h2".into()),
                TokenKind::LBracket,
                TokenKind::Number(0.0),
                TokenKind::RBracket,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_negative_numbers() {
        assert_eq!(
            tokenize_kinds(".h2[-1]"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h2".into()),
                TokenKind::LBracket,
                TokenKind::Number(-1.0),
                TokenKind::RBracket,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_hierarchy() {
        assert_eq!(
            tokenize_kinds(".h1 > .h2"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h1".into()),
                TokenKind::Gt,
                TokenKind::Dot,
                TokenKind::Ident("h2".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_descendant() {
        assert_eq!(
            tokenize_kinds(".h1 >> .code"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("h1".into()),
                TokenKind::GtGt,
                TokenKind::Dot,
                TokenKind::Ident("code".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_comparison() {
        assert_eq!(
            tokenize_kinds(".level == 2"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("level".into()),
                TokenKind::Eq,
                TokenKind::Number(2.0),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_subtraction_not_negative_literal() {
        // `5-3` must lex as 5, minus, 3 — not 5 then -3.
        assert_eq!(
            tokenize_kinds("5-3"),
            vec![
                TokenKind::Number(5.0),
                TokenKind::Minus,
                TokenKind::Number(3.0),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_negative_literal_in_prefix_position() {
        // After `(` and after `,` a `-N` is a negative literal.
        assert_eq!(
            tokenize_kinds("(-3)"),
            vec![
                TokenKind::LParen,
                TokenKind::Number(-3.0),
                TokenKind::RParen,
                TokenKind::Eof
            ]
        );
        assert_eq!(
            tokenize_kinds("1, -2"),
            vec![
                TokenKind::Number(1.0),
                TokenKind::Comma,
                TokenKind::Number(-2.0),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_subtraction_after_paren_and_ident() {
        assert_eq!(
            tokenize_kinds("(1)-2"),
            vec![
                TokenKind::LParen,
                TokenKind::Number(1.0),
                TokenKind::RParen,
                TokenKind::Minus,
                TokenKind::Number(2.0),
                TokenKind::Eof
            ]
        );
        assert_eq!(
            tokenize_kinds("a-1"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::Minus,
                TokenKind::Number(1.0),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn test_malformed_number_is_error() {
        assert!(tokenize("1.2.3").is_err());
        assert!(tokenize("1e").is_err());
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            tokenize_kinds("select(contains(\"API\") and not empty)"),
            vec![
                TokenKind::Ident("select".into()),
                TokenKind::LParen,
                TokenKind::Ident("contains".into()),
                TokenKind::LParen,
                TokenKind::String("API".into()),
                TokenKind::RParen,
                TokenKind::And,
                TokenKind::Not,
                TokenKind::Ident("empty".into()),
                TokenKind::RParen,
                TokenKind::Eof
            ]
        );
    }
}
