//! Parser for the query language.
//!
//! Parses tokens into an Abstract Syntax Tree (AST).

use super::ast::Span;
use super::ast::*;
use super::error::{QueryError, QueryErrorKind};
use super::lexer::{Token, TokenKind};

/// Maximum expression nesting depth. Guards against stack overflow on
/// pathological input such as tens of thousands of nested parentheses.
///
/// Each nesting level descends through ~11 recursive-descent frames, and a
/// debug build's frames are large, so the guard must trip well before the
/// thread stack is exhausted (empirically ~380 levels in a debug `cargo test`
/// thread). 256 leaves comfortable margin while still allowing far deeper
/// nesting than any real query needs.
const MAX_DEPTH: usize = 256;

/// Parser state.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    source: &'a str,
    /// Current recursion depth through `parse_unary_expr`. Every recursive
    /// descent cycle passes through `parse_unary_expr`, so guarding there
    /// bounds total recursion.
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            depth: 0,
        }
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn current_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        let current_pos = self.pos;
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.tokens
            .get(current_pos)
            .unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, QueryError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(QueryError::new(
                QueryErrorKind::UnexpectedToken {
                    expected: vec![kind.name()],
                    found: self.current_kind().clone(),
                },
                self.current_span(),
                self.source.to_string(),
            ))
        }
    }

    fn matches(&mut self, kinds: &[TokenKind]) -> bool {
        for kind in kinds {
            if self.check(kind) {
                self.advance();
                return true;
            }
        }
        false
    }
}

/// Parse tokens into a Query AST.
pub fn parse(tokens: &[Token], source: &str) -> Result<Query, QueryError> {
    let mut parser = Parser::new(tokens, source);
    parse_query(&mut parser)
}

fn parse_query(p: &mut Parser) -> Result<Query, QueryError> {
    let mut expressions = vec![parse_piped_expr(p)?];

    // Handle multiple expressions separated by commas
    while p.matches(&[TokenKind::Comma]) {
        expressions.push(parse_piped_expr(p)?);
    }

    if !p.is_at_end() {
        return Err(QueryError::new(
            QueryErrorKind::UnexpectedToken {
                expected: vec!["end of query"],
                found: p.current_kind().clone(),
            },
            p.current_span(),
            p.source.to_string(),
        ));
    }

    Ok(Query::new(expressions))
}

fn parse_piped_expr(p: &mut Parser) -> Result<PipedExpr, QueryError> {
    let mut stages = vec![parse_hierarchy_expr(p)?];

    // Handle pipes
    while p.matches(&[TokenKind::Pipe]) {
        stages.push(parse_hierarchy_expr(p)?);
    }

    Ok(PipedExpr::new(stages))
}

/// Whether `expr` is an element-selector expression — the only thing the
/// hierarchy operators (`>`, `>>`) are meaningful between. Used to disambiguate
/// `.h1 > .h2` (hierarchy) from `2 > .level` (numeric comparison).
fn is_element_selector(expr: &Expr) -> bool {
    match expr {
        Expr::Element { .. } | Expr::Hierarchy { .. } => true,
        Expr::Group { expr, .. } => is_element_selector(expr),
        _ => false,
    }
}

fn parse_hierarchy_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut expr = parse_or_expr(p)?;

    // Handle hierarchy operators (> and >>).
    //
    // `>>` is always the descendant operator. `>` is only the direct-child
    // hierarchy operator when BOTH operands are element selectors; otherwise it
    // is the numeric greater-than comparison (e.g. `select(2 > .level)`).
    loop {
        let descendant = p.check(&TokenKind::GtGt);
        let child_op = p.check(&TokenKind::Gt);

        if descendant {
            p.advance();
            let start_span = expr.span();
            let child = parse_or_expr(p)?;
            let end_span = child.span();
            expr = Expr::Hierarchy {
                parent: Box::new(expr),
                child: Box::new(child),
                direct: false,
                span: start_span.merge(end_span),
            };
        } else if child_op {
            p.advance();
            let start_span = expr.span();
            let right = parse_or_expr(p)?;
            let end_span = right.span();

            if is_element_selector(&expr) && is_element_selector(&right) {
                expr = Expr::Hierarchy {
                    parent: Box::new(expr),
                    child: Box::new(right),
                    direct: true,
                    span: start_span.merge(end_span),
                };
            } else {
                expr = Expr::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(expr),
                    right: Box::new(right),
                    span: start_span.merge(end_span),
                };
            }
        } else {
            break;
        }
    }

    Ok(expr)
}

fn parse_or_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_and_expr(p)?;

    while p.matches(&[TokenKind::Or]) {
        let start_span = left.span();
        let right = parse_and_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op: BinaryOp::Or,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_and_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_equality_expr(p)?;

    while p.matches(&[TokenKind::And]) {
        let start_span = left.span();
        let right = parse_equality_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op: BinaryOp::And,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_equality_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_comparison_expr(p)?;

    loop {
        let op = if p.matches(&[TokenKind::Eq]) {
            BinaryOp::Eq
        } else if p.matches(&[TokenKind::Ne]) {
            BinaryOp::Ne
        } else {
            break;
        };

        let start_span = left.span();
        let right = parse_comparison_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_comparison_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_alt_expr(p)?;

    loop {
        let op = if p.matches(&[TokenKind::Lt]) {
            BinaryOp::Lt
        } else if p.matches(&[TokenKind::Le]) {
            BinaryOp::Le
        } else if p.check(&TokenKind::Gt)
            && !matches!(
                p.tokens.get(p.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Gt)
            )
            && !matches!(
                p.tokens.get(p.pos + 1).map(|t| &t.kind),
                Some(TokenKind::Dot)
            )
        {
            // Be careful not to consume > if it's >> (descendant) or > . (hierarchy)
            p.advance();
            BinaryOp::Gt
        } else if p.matches(&[TokenKind::Ge]) {
            BinaryOp::Ge
        } else {
            break;
        };

        let start_span = left.span();
        let right = parse_alt_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_alt_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_additive_expr(p)?;

    while p.matches(&[TokenKind::SlashSlash]) {
        let start_span = left.span();
        let right = parse_additive_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op: BinaryOp::Alt,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_additive_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_multiplicative_expr(p)?;

    loop {
        let op = if p.matches(&[TokenKind::Plus]) {
            BinaryOp::Add
        } else if p.matches(&[TokenKind::Minus]) {
            BinaryOp::Sub
        } else {
            break;
        };

        let start_span = left.span();
        let right = parse_multiplicative_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_multiplicative_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut left = parse_unary_expr(p)?;

    loop {
        let op = if p.matches(&[TokenKind::Star]) {
            BinaryOp::Mul
        } else if p.matches(&[TokenKind::Slash]) {
            BinaryOp::Div
        } else if p.matches(&[TokenKind::Percent]) {
            BinaryOp::Mod
        } else {
            break;
        };

        let start_span = left.span();
        let right = parse_unary_expr(p)?;
        let end_span = right.span();

        left = Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
            span: start_span.merge(end_span),
        };
    }

    Ok(left)
}

fn parse_unary_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    // Depth guard: every recursive-descent cycle (parens, unary chains,
    // pipes inside groups, …) routes through here, so bounding this bounds
    // total parser recursion and prevents stack overflow on adversarial input.
    p.depth += 1;
    if p.depth > MAX_DEPTH {
        p.depth -= 1;
        return Err(QueryError::new(
            QueryErrorKind::RecursionLimit,
            p.current_span(),
            p.source.to_string(),
        ));
    }
    let result = parse_unary_expr_inner(p);
    p.depth -= 1;
    result
}

fn parse_unary_expr_inner(p: &mut Parser) -> Result<Expr, QueryError> {
    let start_span = p.current_span();

    if p.matches(&[TokenKind::Not]) {
        let expr = parse_unary_expr(p)?;
        let end_span = expr.span();
        return Ok(Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(expr),
            span: start_span.merge(end_span),
        });
    }

    if p.matches(&[TokenKind::Minus]) {
        let expr = parse_unary_expr(p)?;
        let end_span = expr.span();
        return Ok(Expr::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(expr),
            span: start_span.merge(end_span),
        });
    }

    parse_postfix_expr(p)
}

/// Build a pipe expression `left | right`, flattening when `left` is already a
/// `_pipe` so chains stay flat (`a | b | c` rather than `(a | b) | c`).
fn _pipe(left: Expr, right: Expr, span: Span) -> Expr {
    match left {
        Expr::Function {
            name,
            mut args,
            span: left_span,
        } if name == "_pipe" => {
            args.push(right);
            Expr::Function {
                name,
                args,
                span: left_span.merge(span),
            }
        }
        other => Expr::Function {
            name: "_pipe".to_string(),
            args: vec![other, right],
            span,
        },
    }
}

fn parse_postfix_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let mut expr = parse_primary_expr(p)?;

    loop {
        if p.matches(&[TokenKind::Dot]) {
            // Property access on the *result of* `expr`, e.g. `.h2.text`.
            //
            // The property must be applied to each value produced by the left
            // expression, not to the whole document. Model that as a pipe:
            // `expr | .name`. (`_pipe` is the evaluator's piping special form,
            // and it save/restores `current` around each stage.)
            let start_span = expr.span();
            let (name, name_span) = parse_identifier(p)?;

            let property = Expr::Property {
                name,
                span: start_span.merge(name_span),
            };
            expr = _pipe(expr, property, start_span.merge(name_span));
        } else if p.check(&TokenKind::LBracket) {
            // Index or filter: [0], [-1], [0:3], []
            let (index, span) = parse_index_or_filter(p)?;

            // Apply index to current expression
            // For now, we handle this in evaluation
            if let Expr::Element {
                kind,
                filters,
                index: _,
                span: elem_span,
            } = expr
            {
                expr = Expr::Element {
                    kind,
                    filters,
                    index: Some(index),
                    span: elem_span.merge(span),
                };
            } else {
                // Create a synthetic index expression
                // This will be handled in evaluation
                let start_span = expr.span();
                expr = Expr::Function {
                    name: "_index".to_string(),
                    args: vec![
                        expr,
                        match index {
                            IndexOp::Single(n) => Expr::Literal {
                                value: Literal::Number(n as f64),
                                span,
                            },
                            IndexOp::Slice { start, end } => Expr::Array {
                                elements: vec![
                                    Expr::Literal {
                                        value: start
                                            .map(|n| Literal::Number(n as f64))
                                            .unwrap_or(Literal::Null),
                                        span,
                                    },
                                    Expr::Literal {
                                        value: end
                                            .map(|n| Literal::Number(n as f64))
                                            .unwrap_or(Literal::Null),
                                        span,
                                    },
                                ],
                                span,
                            },
                            IndexOp::Iterate => Expr::Literal {
                                value: Literal::Null,
                                span,
                            },
                        },
                    ],
                    span: start_span.merge(span),
                };
            }
        } else if p.check(&TokenKind::LParen) {
            // Function call with current expression as first argument
            // This handles: expr | func(args) where func might already be parsed
            break;
        } else {
            break;
        }
    }

    Ok(expr)
}

fn parse_primary_expr(p: &mut Parser) -> Result<Expr, QueryError> {
    let span = p.current_span();

    // Identity: .
    if p.check(&TokenKind::Dot) {
        p.advance();

        // Check what comes after the dot
        if p.is_at_end()
            || p.check(&TokenKind::Pipe)
            || p.check(&TokenKind::Comma)
            || p.check(&TokenKind::Gt)
            || p.check(&TokenKind::GtGt)
            || p.check(&TokenKind::RParen)
            || p.check(&TokenKind::RBracket)
        {
            // Just a dot - identity
            return Ok(Expr::Identity);
        }

        // Element or property selector
        if let TokenKind::Ident(name) = p.current_kind().clone() {
            let name_span = p.current_span();
            p.advance();

            // Check if it's an element type
            if let Some(kind) = ElementKind::from_str(&name) {
                // Parse optional filters
                let mut filters = Vec::new();
                while p.check(&TokenKind::LBracket) {
                    let (filter_or_index, filter_span) = parse_filter_or_index(p)?;

                    match filter_or_index {
                        FilterOrIndex::Filter(f) => filters.push(f),
                        FilterOrIndex::Index(idx) => {
                            // Index found - return element with index
                            return Ok(Expr::Element {
                                kind,
                                filters,
                                index: Some(idx),
                                span: span.merge(filter_span),
                            });
                        }
                    }
                }

                return Ok(Expr::Element {
                    kind,
                    filters,
                    index: None,
                    span: span.merge(name_span),
                });
            } else {
                // Property access
                return Ok(Expr::Property {
                    name,
                    span: span.merge(name_span),
                });
            }
        }

        // Invalid selector
        return Err(QueryError::new(
            QueryErrorKind::UnexpectedToken {
                expected: vec!["identifier"],
                found: p.current_kind().clone(),
            },
            p.current_span(),
            p.source.to_string(),
        ));
    }

    // Parenthesized expression
    if p.matches(&[TokenKind::LParen]) {
        let expr = parse_piped_expr(p)?;
        let end_span = p.current_span();
        p.expect(&TokenKind::RParen)?;
        return Ok(Expr::Group {
            expr: Box::new(Expr::from(expr)),
            span: span.merge(end_span),
        });
    }

    // Object literal
    if p.matches(&[TokenKind::LBrace]) {
        return parse_object_literal(p, span);
    }

    // Array literal
    if p.matches(&[TokenKind::LBracket]) {
        return parse_array_literal(p, span);
    }

    // Conditional
    if p.matches(&[TokenKind::If]) {
        return parse_conditional(p, span);
    }

    // Literals
    if let TokenKind::String(s) = p.current_kind().clone() {
        p.advance();
        return Ok(Expr::Literal {
            value: Literal::String(s),
            span,
        });
    }

    if let TokenKind::Number(n) = p.current_kind().clone() {
        p.advance();
        return Ok(Expr::Literal {
            value: Literal::Number(n),
            span,
        });
    }

    if p.matches(&[TokenKind::True]) {
        return Ok(Expr::Literal {
            value: Literal::Bool(true),
            span,
        });
    }

    if p.matches(&[TokenKind::False]) {
        return Ok(Expr::Literal {
            value: Literal::Bool(false),
            span,
        });
    }

    if p.matches(&[TokenKind::Null]) {
        return Ok(Expr::Literal {
            value: Literal::Null,
            span,
        });
    }

    // Function call or identifier
    if let TokenKind::Ident(name) = p.current_kind().clone() {
        let name_span = p.current_span();
        p.advance();

        // Check for function call
        if p.matches(&[TokenKind::LParen]) {
            let args = parse_function_args(p)?;
            let end_span = p.current_span();
            p.expect(&TokenKind::RParen)?;
            return Ok(Expr::Function {
                name,
                args,
                span: span.merge(end_span),
            });
        }

        // Bare identifier - could be a zero-arg function
        return Ok(Expr::Function {
            name,
            args: vec![],
            span: name_span,
        });
    }

    Err(QueryError::new(
        QueryErrorKind::UnexpectedToken {
            expected: vec!["expression"],
            found: p.current_kind().clone(),
        },
        span,
        p.source.to_string(),
    ))
}

fn parse_identifier(p: &mut Parser) -> Result<(String, Span), QueryError> {
    let span = p.current_span();
    if let TokenKind::Ident(name) = p.current_kind().clone() {
        p.advance();
        Ok((name, span))
    } else {
        Err(QueryError::new(
            QueryErrorKind::UnexpectedToken {
                expected: vec!["identifier"],
                found: p.current_kind().clone(),
            },
            span,
            p.source.to_string(),
        ))
    }
}

enum FilterOrIndex {
    Filter(Filter),
    Index(IndexOp),
}

fn parse_filter_or_index(p: &mut Parser) -> Result<(FilterOrIndex, Span), QueryError> {
    let start_span = p.current_span();
    p.expect(&TokenKind::LBracket)?;

    // Empty brackets: []
    if p.check(&TokenKind::RBracket) {
        let end_span = p.current_span();
        p.advance();
        return Ok((
            FilterOrIndex::Index(IndexOp::Iterate),
            start_span.merge(end_span),
        ));
    }

    // Check for number (index or slice)
    if let TokenKind::Number(n) = p.current_kind().clone() {
        p.advance();
        let n = n as i64;

        // Check for slice
        if p.matches(&[TokenKind::Colon]) {
            let end = if let TokenKind::Number(e) = p.current_kind().clone() {
                p.advance();
                Some(e as i64)
            } else {
                None
            };
            let end_span = p.current_span();
            p.expect(&TokenKind::RBracket)?;
            return Ok((
                FilterOrIndex::Index(IndexOp::Slice {
                    start: Some(n),
                    end,
                }),
                start_span.merge(end_span),
            ));
        }

        let end_span = p.current_span();
        p.expect(&TokenKind::RBracket)?;
        return Ok((
            FilterOrIndex::Index(IndexOp::Single(n)),
            start_span.merge(end_span),
        ));
    }

    // Slice starting with :
    if p.matches(&[TokenKind::Colon]) {
        let end = if let TokenKind::Number(e) = p.current_kind().clone() {
            p.advance();
            Some(e as i64)
        } else {
            None
        };
        let end_span = p.current_span();
        p.expect(&TokenKind::RBracket)?;
        return Ok((
            FilterOrIndex::Index(IndexOp::Slice { start: None, end }),
            start_span.merge(end_span),
        ));
    }

    // Negative number
    if p.matches(&[TokenKind::Minus])
        && let TokenKind::Number(n) = p.current_kind().clone()
    {
        p.advance();
        let end_span = p.current_span();
        p.expect(&TokenKind::RBracket)?;
        return Ok((
            FilterOrIndex::Index(IndexOp::Single(-(n as i64))),
            start_span.merge(end_span),
        ));
    }

    // String filter (exact match)
    if let TokenKind::String(s) = p.current_kind().clone() {
        p.advance();
        let end_span = p.current_span();
        p.expect(&TokenKind::RBracket)?;
        return Ok((
            FilterOrIndex::Filter(Filter::Text {
                pattern: s,
                exact: true,
                span: start_span.merge(end_span),
            }),
            start_span.merge(end_span),
        ));
    }

    // Identifier filter (fuzzy match or type filter)
    if let TokenKind::Ident(name) = p.current_kind().clone() {
        p.advance();
        let end_span = p.current_span();
        p.expect(&TokenKind::RBracket)?;

        // Check if it's a type filter for links
        let filter = if matches!(
            name.as_str(),
            "anchor" | "external" | "relative" | "wikilink"
        ) {
            Filter::Type {
                type_name: name,
                span: start_span.merge(end_span),
            }
        } else {
            Filter::Text {
                pattern: name,
                exact: false,
                span: start_span.merge(end_span),
            }
        };

        return Ok((FilterOrIndex::Filter(filter), start_span.merge(end_span)));
    }

    Err(QueryError::new(
        QueryErrorKind::InvalidFilter("expected filter pattern or index".to_string()),
        p.current_span(),
        p.source.to_string(),
    ))
}

fn parse_index_or_filter(p: &mut Parser) -> Result<(IndexOp, Span), QueryError> {
    let (filter_or_index, span) = parse_filter_or_index(p)?;
    match filter_or_index {
        FilterOrIndex::Index(idx) => Ok((idx, span)),
        FilterOrIndex::Filter(_) => Err(QueryError::new(
            QueryErrorKind::InvalidFilter("expected index, got filter".to_string()),
            span,
            p.source.to_string(),
        )),
    }
}

fn parse_function_args(p: &mut Parser) -> Result<Vec<Expr>, QueryError> {
    let mut args = Vec::new();

    if !p.check(&TokenKind::RParen) {
        args.push(parse_piped_expr(p).map(Expr::from)?);

        while p.matches(&[TokenKind::Comma]) {
            args.push(parse_piped_expr(p).map(Expr::from)?);
        }
    }

    Ok(args)
}

fn parse_object_literal(p: &mut Parser, start_span: Span) -> Result<Expr, QueryError> {
    let mut pairs = Vec::new();

    if !p.check(&TokenKind::RBrace) {
        loop {
            // Key: identifier or string
            let key = if let TokenKind::String(s) = p.current_kind().clone() {
                p.advance();
                s
            } else if let TokenKind::Ident(s) = p.current_kind().clone() {
                p.advance();
                s
            } else {
                return Err(QueryError::new(
                    QueryErrorKind::UnexpectedToken {
                        expected: vec!["identifier", "string"],
                        found: p.current_kind().clone(),
                    },
                    p.current_span(),
                    p.source.to_string(),
                ));
            };

            // Colon
            p.expect(&TokenKind::Colon)?;

            // Value
            let value = parse_piped_expr(p).map(Expr::from)?;
            pairs.push((key, value));

            if !p.matches(&[TokenKind::Comma]) {
                break;
            }
        }
    }

    let end_span = p.current_span();
    p.expect(&TokenKind::RBrace)?;

    Ok(Expr::Object {
        pairs,
        span: start_span.merge(end_span),
    })
}

fn parse_array_literal(p: &mut Parser, start_span: Span) -> Result<Expr, QueryError> {
    let mut elements = Vec::new();

    if !p.check(&TokenKind::RBracket) {
        loop {
            elements.push(parse_piped_expr(p).map(Expr::from)?);

            if !p.matches(&[TokenKind::Comma]) {
                break;
            }
        }
    }

    let end_span = p.current_span();
    p.expect(&TokenKind::RBracket)?;

    Ok(Expr::Array {
        elements,
        span: start_span.merge(end_span),
    })
}

fn parse_conditional(p: &mut Parser, start_span: Span) -> Result<Expr, QueryError> {
    // Parse the `if … then … [elif …]* [else …]` body, then consume a single
    // closing `end`. `elif` chains are parsed recursively by the body parser
    // *without* each level consuming its own `end`, so
    // `if a then b elif c then d else e end` needs exactly one `end`.
    let expr = parse_conditional_body(p, start_span)?;

    let end_span = p.current_span();
    if !p.matches(&[TokenKind::End]) {
        return Err(QueryError::new(
            QueryErrorKind::MissingEnd,
            p.current_span(),
            p.source.to_string(),
        ));
    }

    // Stretch the outermost conditional's span to include the `end`.
    if let Expr::Conditional {
        condition,
        then_branch,
        else_branch,
        span,
    } = expr
    {
        Ok(Expr::Conditional {
            condition,
            then_branch,
            else_branch,
            span: span.merge(end_span),
        })
    } else {
        Ok(expr)
    }
}

/// Parse the body of a conditional (`if … then … [elif … | else …]`) but do
/// NOT consume the closing `end`. The caller (`parse_conditional`) consumes the
/// single `end` for the whole chain.
fn parse_conditional_body(p: &mut Parser, start_span: Span) -> Result<Expr, QueryError> {
    // Condition
    let condition = parse_piped_expr(p).map(Expr::from)?;

    // then
    if !p.matches(&[TokenKind::Then]) {
        return Err(QueryError::new(
            QueryErrorKind::MissingThen,
            p.current_span(),
            p.source.to_string(),
        ));
    }

    // Then branch
    let then_branch = parse_piped_expr(p).map(Expr::from)?;
    let then_span = then_branch.span();

    // Optional elif/else. `elif` recurses into the body parser (no `end`),
    // `else` is a plain expression.
    let else_branch = if p.matches(&[TokenKind::Elif]) {
        let elif_span = p.current_span();
        Some(Box::new(parse_conditional_body(p, elif_span)?))
    } else if p.matches(&[TokenKind::Else]) {
        Some(Box::new(parse_piped_expr(p).map(Expr::from)?))
    } else {
        None
    };

    Ok(Expr::Conditional {
        condition: Box::new(condition),
        then_branch: Box::new(then_branch),
        else_branch,
        span: start_span.merge(then_span),
    })
}

// Convert PipedExpr to Expr (wrapping single stage or creating pipe chain)
impl From<PipedExpr> for Expr {
    fn from(piped: PipedExpr) -> Self {
        if piped.stages.len() == 1 {
            piped.stages.into_iter().next().unwrap()
        } else {
            // For multi-stage pipes, we wrap in a special form
            // The evaluator will handle this
            Expr::Function {
                name: "_pipe".to_string(),
                args: piped.stages,
                span: Span::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::lexer::tokenize;

    fn parse_str(s: &str) -> Result<Query, QueryError> {
        let tokens = tokenize(s)?;
        parse(&tokens, s)
    }

    #[test]
    fn test_identity() {
        let query = parse_str(".").unwrap();
        assert_eq!(query.expressions.len(), 1);
        assert!(matches!(query.expressions[0].stages[0], Expr::Identity));
    }

    #[test]
    fn test_element_selector() {
        let query = parse_str(".h2").unwrap();
        if let Expr::Element { kind, .. } = &query.expressions[0].stages[0] {
            assert_eq!(*kind, ElementKind::Heading(Some(2)));
        } else {
            panic!("Expected Element");
        }
    }

    #[test]
    fn test_element_with_filter() {
        let query = parse_str(".h2[Features]").unwrap();
        if let Expr::Element { kind, filters, .. } = &query.expressions[0].stages[0] {
            assert_eq!(*kind, ElementKind::Heading(Some(2)));
            assert_eq!(filters.len(), 1);
        } else {
            panic!("Expected Element with filter");
        }
    }

    #[test]
    fn test_element_with_index() {
        let query = parse_str(".h2[0]").unwrap();
        if let Expr::Element { index, .. } = &query.expressions[0].stages[0] {
            assert!(matches!(index, Some(IndexOp::Single(0))));
        } else {
            panic!("Expected Element with index");
        }
    }

    #[test]
    fn test_pipe() {
        let query = parse_str(".h2 | text").unwrap();
        assert_eq!(query.expressions[0].stages.len(), 2);
    }

    #[test]
    fn test_function_call() {
        let query = parse_str("select(contains(\"API\"))").unwrap();
        if let Expr::Function { name, args, .. } = &query.expressions[0].stages[0] {
            assert_eq!(name, "select");
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected Function");
        }
    }

    #[test]
    fn test_hierarchy() {
        let query = parse_str(".h1 > .h2").unwrap();
        if let Expr::Hierarchy { direct, .. } = &query.expressions[0].stages[0] {
            assert!(*direct);
        } else {
            panic!("Expected Hierarchy");
        }
    }

    #[test]
    fn test_comparison() {
        let query = parse_str(".level == 2").unwrap();
        if let Expr::Binary { op, .. } = &query.expressions[0].stages[0] {
            assert_eq!(*op, BinaryOp::Eq);
        } else {
            panic!("Expected Binary");
        }
    }

    #[test]
    fn test_elif_single_end() {
        // elif chains close with exactly one `end`.
        let query = parse_str("if true then 1 elif true then 2 else 3 end").unwrap();
        assert!(matches!(
            query.expressions[0].stages[0],
            Expr::Conditional { .. }
        ));
    }

    #[test]
    fn test_elif_too_many_ends_is_error() {
        // A stray second `end` is a parse error (trailing token).
        assert!(parse_str("if true then 1 elif true then 2 end end").is_err());
    }

    #[test]
    fn test_property_chain_is_pipe() {
        // `.h2.text` desugars to `.h2 | .text`.
        let query = parse_str(".h2.text").unwrap();
        if let Expr::Function { name, args, .. } = &query.expressions[0].stages[0] {
            assert_eq!(name, "_pipe");
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0], Expr::Element { .. }));
            assert!(matches!(args[1], Expr::Property { .. }));
        } else {
            panic!(
                "Expected _pipe function, got {:?}",
                query.expressions[0].stages[0]
            );
        }
    }

    #[test]
    fn test_gt_between_non_selectors_is_comparison() {
        // `2 > .level` is a numeric comparison, not a hierarchy.
        let query = parse_str("2 > .level").unwrap();
        if let Expr::Binary { op, .. } = &query.expressions[0].stages[0] {
            assert_eq!(*op, BinaryOp::Gt);
        } else {
            panic!(
                "Expected Binary Gt, got {:?}",
                query.expressions[0].stages[0]
            );
        }
    }

    #[test]
    fn test_gt_between_selectors_is_hierarchy() {
        let query = parse_str(".h1 > .h2").unwrap();
        assert!(matches!(
            query.expressions[0].stages[0],
            Expr::Hierarchy { direct: true, .. }
        ));
    }

    #[test]
    fn test_deeply_nested_parens_errors_not_overflow() {
        // 50k nested parens must error with RecursionLimit, not stack-overflow.
        // Run on a thread with a production-sized (8 MiB) stack: the depth guard
        // (MAX_DEPTH) is tuned for the real main thread, whereas libtest worker
        // threads default to a smaller 2 MiB stack.
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let depth = 50_000;
                let mut q = String::with_capacity(depth * 2 + 1);
                q.push_str(&"(".repeat(depth));
                q.push('.');
                q.push_str(&")".repeat(depth));
                let err = parse_str(&q).unwrap_err();
                assert!(matches!(err.0.kind, QueryErrorKind::RecursionLimit));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
