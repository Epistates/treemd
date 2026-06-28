//! Query evaluator.
//!
//! Executes parsed queries against markdown documents.

use indexmap::IndexMap;
use std::sync::Arc;

use super::ast::*;
use super::error::{QueryError, QueryErrorKind};
use super::registry::Registry;
use super::value::*;
use crate::parser::Document;

/// Evaluation context passed to functions.
pub struct EvalContext {
    /// The current value being processed
    pub current: Value,
    /// All headings in the document
    pub headings: Vec<HeadingValue>,
    /// All code blocks
    pub code_blocks: Vec<CodeValue>,
    /// All links
    pub links: Vec<LinkValue>,
    /// All images
    pub images: Vec<ImageValue>,
    /// All tables
    pub tables: Vec<TableValue>,
    /// All lists
    pub lists: Vec<ListValue>,
    /// All paragraphs
    pub paragraphs: Vec<ParagraphValue>,
    /// All blockquotes
    pub blockquotes: Vec<BlockquoteValue>,
    /// Parsed YAML frontmatter, if present (keys sorted for stable output)
    pub frontmatter: Option<IndexMap<String, Value>>,
    /// Document metadata
    pub document: DocumentValue,
    /// Raw document content
    pub raw_content: String,
}

impl EvalContext {
    /// Create a new context from a document.
    pub fn from_document(doc: &Document) -> Self {
        let headings = extract_headings(doc);
        let extracted = extract_blocks(doc);
        let frontmatter = extract_frontmatter(doc);

        let document = DocumentValue {
            content: doc.content.clone(),
            heading_count: doc.headings.len(),
            word_count: doc.content.split_whitespace().count(),
        };

        Self {
            current: Value::Document(document.clone()),
            headings,
            code_blocks: extracted.code_blocks,
            links: extracted.links,
            images: extracted.images,
            tables: extracted.tables,
            lists: extracted.lists,
            paragraphs: extracted.paragraphs,
            blockquotes: extracted.blockquotes,
            frontmatter,
            document,
            raw_content: doc.content.clone(),
        }
    }
}

/// Maximum evaluation recursion depth. Mirrors the parser's `MAX_DEPTH` and
/// guards against stack overflow when evaluating deeply nested expressions
/// (the public `Query` API allows hand-built ASTs deeper than the parser
/// would produce).
const MAX_EVAL_DEPTH: usize = 256;

/// Query execution engine.
pub struct Engine {
    registry: Arc<Registry>,
    context: EvalContext,
    /// Current evaluation recursion depth (guarded in `eval_expr`).
    depth: usize,
}

impl Engine {
    /// Create a new engine with default registry.
    pub fn new(doc: &Document) -> Self {
        Self::with_registry(doc, Registry::with_builtins())
    }

    /// Create a new engine with a custom registry.
    pub fn with_registry(doc: &Document, registry: Registry) -> Self {
        let context = EvalContext::from_document(doc);
        Self {
            registry: Arc::new(registry),
            context,
            depth: 0,
        }
    }

    /// Execute a query and return results.
    pub fn execute(&mut self, query: &Query) -> Result<Vec<Value>, QueryError> {
        let mut all_results = Vec::new();

        for piped_expr in &query.expressions {
            let results = self.eval_piped(piped_expr)?;
            all_results.extend(results);
        }

        Ok(all_results)
    }

    fn eval_piped(&mut self, piped: &PipedExpr) -> Result<Vec<Value>, QueryError> {
        // Start with the document as input
        let mut current = vec![Value::Document(self.context.document.clone())];

        // Preserve and restore the outer `current` so a top-level pipe doesn't
        // leak its per-stage value into sibling expressions.
        let saved = self.context.current.clone();

        for stage in &piped.stages {
            let mut next = Vec::new();
            for input in current {
                self.context.current = input;
                match self.eval_expr(stage) {
                    Ok(vals) => next.extend(vals),
                    Err(e) => {
                        self.context.current = saved;
                        return Err(e);
                    }
                }
            }
            current = next;

            // Short-circuit if no results
            if current.is_empty() {
                break;
            }
        }

        self.context.current = saved;
        Ok(current)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Vec<Value>, QueryError> {
        self.depth += 1;
        if self.depth > MAX_EVAL_DEPTH {
            self.depth -= 1;
            return Err(QueryError::new(
                QueryErrorKind::RecursionLimit,
                expr.span(),
                String::new(),
            ));
        }
        let result = self.eval_expr_inner(expr);
        self.depth -= 1;
        result
    }

    fn eval_expr_inner(&mut self, expr: &Expr) -> Result<Vec<Value>, QueryError> {
        match expr {
            Expr::Identity => Ok(vec![self.context.current.clone()]),

            Expr::Element {
                kind,
                filters,
                index,
                span,
            } => self.eval_element(kind, filters, index.as_ref(), *span),

            Expr::Property { name, span } => self.eval_property(name, *span),

            Expr::Function { name, args, span } => self.eval_function(name, args, *span),

            Expr::Hierarchy {
                parent,
                child,
                direct,
                span,
            } => self.eval_hierarchy(parent, child, *direct, *span),

            Expr::Binary {
                op,
                left,
                right,
                span,
            } => self.eval_binary(*op, left, right, *span),

            Expr::Unary { op, expr, span } => self.eval_unary(*op, expr, *span),

            Expr::Literal { value, .. } => Ok(vec![literal_to_value(value)]),

            Expr::Object { pairs, span } => self.eval_object(pairs, *span),

            Expr::Array { elements, span } => self.eval_array(elements, *span),

            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => self.eval_conditional(condition, then_branch, else_branch.as_deref()),

            Expr::Group { expr, .. } => self.eval_expr(expr),
        }
    }

    fn eval_element(
        &mut self,
        kind: &ElementKind,
        filters: &[Filter],
        index: Option<&IndexOp>,
        _span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        // Get all elements of the requested kind
        let mut elements: Vec<Value> = match kind {
            ElementKind::Heading(level) => self
                .context
                .headings
                .iter()
                .filter(|h| level.is_none() || Some(h.level) == *level)
                .cloned()
                .map(Value::Heading)
                .collect(),
            ElementKind::Code => self
                .context
                .code_blocks
                .iter()
                .cloned()
                .map(Value::Code)
                .collect(),
            ElementKind::Link => self
                .context
                .links
                .iter()
                .cloned()
                .map(Value::Link)
                .collect(),
            ElementKind::Image => self
                .context
                .images
                .iter()
                .cloned()
                .map(Value::Image)
                .collect(),
            ElementKind::Table => self
                .context
                .tables
                .iter()
                .cloned()
                .map(Value::Table)
                .collect(),
            ElementKind::List => self
                .context
                .lists
                .iter()
                .cloned()
                .map(Value::List)
                .collect(),
            ElementKind::Blockquote => self
                .context
                .blockquotes
                .iter()
                .cloned()
                .map(Value::Blockquote)
                .collect(),
            ElementKind::Paragraph => self
                .context
                .paragraphs
                .iter()
                .cloned()
                .map(Value::Paragraph)
                .collect(),
            ElementKind::FrontMatter => match &self.context.frontmatter {
                Some(fm) => vec![Value::FrontMatter(fm.clone())],
                None => Vec::new(),
            },
        };

        // Apply filters
        for filter in filters {
            elements = self.apply_filter(elements, filter)?;
        }

        // Apply index
        if let Some(idx) = index {
            elements = apply_index(elements, idx)?;
        }

        Ok(elements)
    }

    fn apply_filter(
        &self,
        elements: Vec<Value>,
        filter: &Filter,
    ) -> Result<Vec<Value>, QueryError> {
        match filter {
            Filter::Text { pattern, exact, .. } => {
                let pattern_lower = pattern.to_lowercase();
                Ok(elements
                    .into_iter()
                    .filter(|v| {
                        let text = v.to_text().to_lowercase();
                        if *exact {
                            text == pattern_lower
                        } else {
                            text.contains(&pattern_lower)
                        }
                    })
                    .collect())
            }
            Filter::Type { type_name, .. } => Ok(elements
                .into_iter()
                .filter(|v| {
                    if let Value::Link(link) = v {
                        link.link_type.as_str() == type_name
                    } else if let Value::Code(code) = v {
                        code.language.as_deref() == Some(type_name)
                    } else {
                        false
                    }
                })
                .collect()),
        }
    }

    fn eval_property(&mut self, name: &str, span: Span) -> Result<Vec<Value>, QueryError> {
        let current = &self.context.current;

        if let Some(value) = current.get_property(name) {
            Ok(vec![value])
        } else {
            Err(QueryError::new(
                QueryErrorKind::PropertyNotFound {
                    property: name.to_string(),
                    on_type: current.kind().to_string(),
                },
                span,
                String::new(),
            ))
        }
    }

    fn eval_function(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        // Handle special built-in functions
        match name {
            "_pipe" => {
                // Internal pipe handling. Save/restore `current` so the pipe's
                // per-stage value doesn't leak into the surrounding expression
                // (e.g. `{a: (.text | upper), b: .level}` must still see the
                // heading for `.level`).
                let saved = self.context.current.clone();
                let mut current = vec![self.context.current.clone()];
                for arg in args {
                    let mut next = Vec::new();
                    for input in current {
                        self.context.current = input;
                        match self.eval_expr(arg) {
                            Ok(vals) => next.extend(vals),
                            Err(e) => {
                                self.context.current = saved;
                                return Err(e);
                            }
                        }
                    }
                    current = next;
                    if current.is_empty() {
                        break;
                    }
                }
                self.context.current = saved;
                return Ok(current);
            }
            "_index" if args.len() >= 2 => {
                return self.eval_index(&args[0], &args[1]);
            }
            // Higher-order forms whose single argument is an expression to be
            // evaluated *per element* (with `current` bound to each element),
            // not once against the whole input.
            "any" | "all" if args.len() == 1 => {
                return self.eval_any_all(name == "all", &args[0]);
            }
            "sort_by" if args.len() == 1 => {
                return self.eval_sort_by(&args[0]);
            }
            "group_by" if args.len() == 1 => {
                return self.eval_group_by(&args[0]);
            }
            _ => {}
        }

        // Look up function in registry
        let func = self.registry.get_function(name).cloned();

        if let Some(func) = func {
            // Evaluate arguments
            let mut eval_args = Vec::new();

            // If function takes input, prepend current value
            if func.takes_input {
                eval_args.push(self.context.current.clone());
            }

            for arg in args {
                let arg_values = self.eval_expr(arg)?;
                if arg_values.len() == 1 {
                    eval_args.push(arg_values.into_iter().next().unwrap());
                } else {
                    eval_args.push(Value::Array(arg_values));
                }
            }

            // Check arity
            let provided = if func.takes_input {
                args.len()
            } else {
                eval_args.len()
            };
            if !func.accepts_arity(provided) {
                return Err(QueryError::new(
                    QueryErrorKind::InvalidArity {
                        function: name.to_string(),
                        expected: format!("{:?}", func.arity),
                        found: provided,
                    },
                    span,
                    String::new(),
                ));
            }

            func.call(&eval_args, &self.context)
        } else {
            // Unknown function
            let suggestions = self.registry.suggest_function(name);
            Err(QueryError::new(
                QueryErrorKind::UnknownFunction(name.to_string()),
                span,
                String::new(),
            )
            .with_suggestions(suggestions.into_iter().map(String::from).collect()))
        }
    }

    /// Evaluate the internal `_index(target, index_arg)` form produced by the
    /// parser for postfix `[...]` on non-element expressions.
    ///
    /// `index_arg` is encoded by the parser as:
    /// - `Number(n)`  → single index `[n]`
    /// - `Array[a,b]` → slice `[a:b]` (each of `a`/`b` is `Number` or `Null`)
    /// - `Null`       → iterate `[]`
    ///
    /// If the target evaluates to a single array value, we index *into* that
    /// array (jq semantics, negative indices supported). Otherwise we index the
    /// result stream.
    fn eval_index(&mut self, target: &Expr, index_arg: &Expr) -> Result<Vec<Value>, QueryError> {
        let target_vals = self.eval_expr(target)?;
        let index = decode_index_arg(index_arg);

        // Single array value → index into its elements (jq-style subscripting).
        if target_vals.len() == 1
            && let Value::Array(items) = &target_vals[0]
        {
            return apply_index(items.clone(), &index);
        }

        // Otherwise index the result stream.
        apply_index(target_vals, &index)
    }

    /// Elements of the current value for higher-order forms: the items of an
    /// array, or the value itself wrapped as a single element.
    fn current_elements(&self) -> Vec<Value> {
        match &self.context.current {
            Value::Array(a) => a.clone(),
            other => vec![other.clone()],
        }
    }

    /// Evaluate `cond` against `element` (with `current` bound to it) and return
    /// its truthiness. Restores `current` afterward.
    fn eval_predicate(&mut self, element: &Value, cond: &Expr) -> Result<bool, QueryError> {
        let saved = std::mem::replace(&mut self.context.current, element.clone());
        let result = self.eval_expr(cond);
        self.context.current = saved;
        let vals = result?;
        Ok(vals.into_iter().next().unwrap_or(Value::Null).is_truthy())
    }

    /// Evaluate `key_expr` against `element` (with `current` bound to it) and
    /// return its first result value. Restores `current` afterward.
    fn eval_key(&mut self, element: &Value, key_expr: &Expr) -> Result<Value, QueryError> {
        let saved = std::mem::replace(&mut self.context.current, element.clone());
        let result = self.eval_expr(key_expr);
        self.context.current = saved;
        let vals = result?;
        Ok(vals.into_iter().next().unwrap_or(Value::Null))
    }

    /// `any(cond)` / `all(cond)` — evaluate `cond` per element.
    fn eval_any_all(&mut self, all: bool, cond: &Expr) -> Result<Vec<Value>, QueryError> {
        let elements = self.current_elements();
        let mut acc = all; // all → start true; any → start false
        for el in &elements {
            let truthy = self.eval_predicate(el, cond)?;
            if all {
                acc &= truthy;
                if !acc {
                    break;
                }
            } else {
                acc |= truthy;
                if acc {
                    break;
                }
            }
        }
        Ok(vec![Value::Bool(acc)])
    }

    /// `sort_by(key)` — stable sort the current array by the evaluated key.
    fn eval_sort_by(&mut self, key_expr: &Expr) -> Result<Vec<Value>, QueryError> {
        let elements = self.current_elements();
        let mut keyed: Vec<(Value, Value)> = Vec::with_capacity(elements.len());
        for el in elements {
            let key = self.eval_key(&el, key_expr)?;
            keyed.push((key, el));
        }
        keyed.sort_by(|a, b| sort_key_cmp(&a.0, &b.0));
        Ok(vec![Value::Array(
            keyed.into_iter().map(|(_, v)| v).collect(),
        )])
    }

    /// `group_by(key)` — group the current array into an object keyed by the
    /// stringified evaluated key (per element), preserving first-seen order.
    fn eval_group_by(&mut self, key_expr: &Expr) -> Result<Vec<Value>, QueryError> {
        let elements = self.current_elements();
        let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();
        for el in elements {
            let key = self.eval_key(&el, key_expr)?.to_text();
            groups.entry(key).or_default().push(el);
        }
        let obj: IndexMap<String, Value> = groups
            .into_iter()
            .map(|(k, v)| (k, Value::Array(v)))
            .collect();
        Ok(vec![Value::Object(obj)])
    }

    fn eval_hierarchy(
        &mut self,
        parent: &Expr,
        child: &Expr,
        direct: bool,
        _span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        // Evaluate parent expression
        let parent_values = self.eval_expr(parent)?;

        let mut results = Vec::new();

        for parent_val in parent_values {
            // For headings, find children
            if let Value::Heading(ref parent_heading) = parent_val {
                // Get child element kind
                let child_kind = match child {
                    Expr::Element { kind, .. } => Some(kind.clone()),
                    _ => None,
                };

                if let Some(kind) = child_kind {
                    // Find headings that are children of this parent
                    let parent_idx = parent_heading.index;
                    let parent_level = parent_heading.level;

                    // Byte bounds of this heading's scope:
                    // - direct (`>`):     up to the next heading of ANY level
                    //   (this heading's own body only).
                    // - descendant (`>>`): up to the next heading of level
                    //   <= parent (the whole subtree).
                    let scope_start_offset = parent_heading.offset;
                    let scope_end_offset = self
                        .context
                        .headings
                        .iter()
                        .skip(parent_idx + 1)
                        .find(|h| {
                            if direct {
                                true
                            } else {
                                h.level <= parent_level
                            }
                        })
                        .map(|h| h.offset)
                        .unwrap_or(self.context.raw_content.len());

                    match kind {
                        ElementKind::Heading(level_filter) => {
                            // Find child headings
                            for (idx, h) in self.context.headings.iter().enumerate() {
                                if idx <= parent_idx {
                                    continue;
                                }

                                // Stop if we hit a heading at same or higher level
                                if h.level <= parent_level {
                                    break;
                                }

                                // Check level filter
                                if let Some(target_level) = level_filter
                                    && h.level != target_level
                                {
                                    if direct && h.level > target_level {
                                        // Skip deeper headings in direct mode
                                        continue;
                                    }
                                    if h.level != target_level {
                                        continue;
                                    }
                                }

                                // In direct mode, only include immediate children
                                if direct {
                                    // Find if there's an intermediate heading
                                    let has_intermediate = self.context.headings
                                        [parent_idx + 1..idx]
                                        .iter()
                                        .any(|intermediate| {
                                            intermediate.level > parent_level
                                                && intermediate.level < h.level
                                        });
                                    if has_intermediate {
                                        continue;
                                    }
                                }

                                results.push(Value::Heading(h.clone()));
                            }
                        }
                        ElementKind::Code => {
                            // turbovault's per-block start_line is not reliable
                            // (it reports 1 for every fenced block), so scope by
                            // re-parsing the heading's byte range instead of
                            // filtering the global list by line number.
                            let scope =
                                &self.context.raw_content[scope_start_offset..scope_end_offset];
                            results.extend(extract_code_blocks(scope).into_iter().map(Value::Code));
                        }
                        ElementKind::Link => {
                            // Links carry byte offsets, so scope by byte range.
                            for link in &self.context.links {
                                if link.offset >= scope_start_offset
                                    && link.offset < scope_end_offset
                                {
                                    results.push(Value::Link(link.clone()));
                                }
                            }
                        }
                        // Images, tables, and lists carry no source positions in
                        // the current model, so they cannot be scoped to a
                        // heading yet. Left unimplemented intentionally.
                        _ => {}
                    }
                }
            }
        }

        // Apply child filters if any
        if let Expr::Element { filters, index, .. } = child {
            for filter in filters {
                results = self.apply_filter(results, filter)?;
            }
            if let Some(idx) = index {
                results = apply_index(results, idx)?;
            }
        }

        Ok(results)
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        _span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        let left_vals = self.eval_expr(left)?;
        let right_vals = self.eval_expr(right)?;

        let left_val = left_vals.into_iter().next().unwrap_or(Value::Null);
        let right_val = right_vals.into_iter().next().unwrap_or(Value::Null);

        let result = match op {
            BinaryOp::Eq => Value::Bool(values_equal(&left_val, &right_val)),
            BinaryOp::Ne => Value::Bool(!values_equal(&left_val, &right_val)),
            BinaryOp::Lt => Value::Bool(compare_values(&left_val, &right_val) < 0),
            BinaryOp::Le => Value::Bool(compare_values(&left_val, &right_val) <= 0),
            BinaryOp::Gt => Value::Bool(compare_values(&left_val, &right_val) > 0),
            BinaryOp::Ge => Value::Bool(compare_values(&left_val, &right_val) >= 0),
            BinaryOp::And => Value::Bool(left_val.is_truthy() && right_val.is_truthy()),
            BinaryOp::Or => Value::Bool(left_val.is_truthy() || right_val.is_truthy()),
            BinaryOp::Add => add_values(&left_val, &right_val),
            BinaryOp::Sub => sub_values(&left_val, &right_val),
            BinaryOp::Mul => mul_values(&left_val, &right_val)?,
            BinaryOp::Div => div_values(&left_val, &right_val)?,
            BinaryOp::Mod => mod_values(&left_val, &right_val)?,
            BinaryOp::Alt => {
                if left_val.is_truthy() {
                    left_val
                } else {
                    right_val
                }
            }
        };

        Ok(vec![result])
    }

    fn eval_unary(
        &mut self,
        op: UnaryOp,
        expr: &Expr,
        _span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        let vals = self.eval_expr(expr)?;
        let val = vals.into_iter().next().unwrap_or(Value::Null);

        let result = match op {
            UnaryOp::Not => Value::Bool(!val.is_truthy()),
            UnaryOp::Neg => {
                if let Value::Number(n) = val {
                    Value::Number(-n)
                } else {
                    Value::Null
                }
            }
        };

        Ok(vec![result])
    }

    fn eval_object(
        &mut self,
        pairs: &[(String, Expr)],
        _span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        let mut obj = IndexMap::new();

        for (key, value_expr) in pairs {
            let values = self.eval_expr(value_expr)?;
            let value = if values.len() == 1 {
                values.into_iter().next().unwrap()
            } else {
                Value::Array(values)
            };
            obj.insert(key.clone(), value);
        }

        Ok(vec![Value::Object(obj)])
    }

    fn eval_array(&mut self, elements: &[Expr], _span: Span) -> Result<Vec<Value>, QueryError> {
        let mut arr = Vec::new();

        for elem in elements {
            arr.extend(self.eval_expr(elem)?);
        }

        Ok(vec![Value::Array(arr)])
    }

    fn eval_conditional(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: Option<&Expr>,
    ) -> Result<Vec<Value>, QueryError> {
        let cond_vals = self.eval_expr(condition)?;
        let cond = cond_vals.into_iter().next().unwrap_or(Value::Null);

        if cond.is_truthy() {
            self.eval_expr(then_branch)
        } else if let Some(else_expr) = else_branch {
            self.eval_expr(else_expr)
        } else {
            Ok(vec![Value::Null])
        }
    }
}

// Helper functions

fn extract_headings(doc: &Document) -> Vec<HeadingValue> {
    doc.headings
        .iter()
        .enumerate()
        .map(|(idx, h)| {
            // Calculate line number
            let line = doc.content[..h.offset].lines().count() + 1;

            // Body bounds shared with the document extractor: handles CRLF,
            // setext underlines, and EOF-without-newline uniformly.
            let content_start = doc.body_start(idx);
            let content_end = doc.section_end(idx);

            let content = doc.content[content_start..content_end].trim().to_string();
            let raw_md = doc.content[h.offset..content_end].to_string();

            HeadingValue {
                level: h.level as u8,
                text: h.text.clone(),
                offset: h.offset,
                line,
                content,
                raw_md,
                index: idx,
            }
        })
        .collect()
}

/// Block-level elements extracted from a document, grouped by kind.
#[derive(Default)]
struct ExtractedBlocks {
    code_blocks: Vec<CodeValue>,
    links: Vec<LinkValue>,
    images: Vec<ImageValue>,
    tables: Vec<TableValue>,
    lists: Vec<ListValue>,
    paragraphs: Vec<ParagraphValue>,
    blockquotes: Vec<BlockquoteValue>,
}

fn extract_blocks(doc: &Document) -> ExtractedBlocks {
    use crate::parser::content::parse_content;
    use crate::parser::links::extract_links;
    use crate::parser::output::Block;

    let blocks = parse_content(&doc.content, 1);
    let links = extract_links(&doc.content);

    let mut out = ExtractedBlocks::default();

    // Recursively extract blocks from nested structures (e.g., list items,
    // blockquotes, details). Top-level paragraphs/blockquotes are collected by
    // the outer loop; nested ones are collected here too.
    fn walk(blocks: &[Block], out: &mut ExtractedBlocks) {
        for block in blocks {
            match block {
                Block::Code {
                    language,
                    content,
                    start_line,
                    end_line,
                } => {
                    out.code_blocks.push(CodeValue {
                        language: language.clone(),
                        content: content.clone(),
                        start_line: *start_line,
                        end_line: *end_line,
                    });
                }
                Block::Image { alt, src, title } => {
                    out.images.push(ImageValue {
                        alt: alt.clone(),
                        src: src.clone(),
                        title: title.clone(),
                    });
                }
                Block::Table {
                    headers,
                    rows,
                    alignments,
                } => {
                    out.tables.push(TableValue {
                        headers: headers.clone(),
                        rows: rows.clone(),
                        alignments: alignments
                            .iter()
                            .map(|a| format!("{:?}", a).to_lowercase())
                            .collect(),
                    });
                }
                Block::Paragraph { content, .. } => {
                    out.paragraphs.push(ParagraphValue {
                        content: content.clone(),
                    });
                }
                Block::List { ordered, items } => {
                    for item in items {
                        walk(&item.blocks, out);
                    }
                    out.lists.push(ListValue {
                        ordered: *ordered,
                        items: items
                            .iter()
                            .map(|i| ListItemValue {
                                content: i.content.clone(),
                                checked: i.checked,
                            })
                            .collect(),
                    });
                }
                Block::Blockquote { content, blocks } => {
                    out.blockquotes.push(BlockquoteValue {
                        content: content.clone(),
                    });
                    walk(blocks, out);
                }
                Block::Details { blocks, .. } => {
                    walk(blocks, out);
                }
                _ => {}
            }
        }
    }

    walk(&blocks, &mut out);

    out.links = links
        .into_iter()
        .map(|l| {
            use crate::parser::links::LinkTarget;
            let (url, link_type) = match l.target {
                LinkTarget::Anchor(s) => (format!("#{}", s), LinkType::Anchor),
                LinkTarget::External(s) => (s, LinkType::External),
                LinkTarget::RelativeFile { path, anchor } => {
                    let mut url = path.to_string_lossy().to_string();
                    if let Some(a) = anchor {
                        url.push('#');
                        url.push_str(&a);
                    }
                    (url, LinkType::Relative)
                }
                LinkTarget::WikiLink { target, .. } => (target, LinkType::WikiLink),
            };
            LinkValue {
                text: l.text,
                url,
                link_type,
                offset: l.offset,
            }
        })
        .collect();

    out
}

/// Parse a markdown fragment and return only its code blocks (including those
/// nested in lists/blockquotes/details). Used to scope code blocks to a
/// heading's byte range, since turbovault's per-block line numbers are not
/// reliable for line-range filtering.
fn extract_code_blocks(fragment: &str) -> Vec<CodeValue> {
    use crate::parser::content::parse_content;
    use crate::parser::output::Block;

    fn walk(blocks: &[Block], out: &mut Vec<CodeValue>) {
        for block in blocks {
            match block {
                Block::Code {
                    language,
                    content,
                    start_line,
                    end_line,
                } => out.push(CodeValue {
                    language: language.clone(),
                    content: content.clone(),
                    start_line: *start_line,
                    end_line: *end_line,
                }),
                Block::List { items, .. } => {
                    for item in items {
                        walk(&item.blocks, out);
                    }
                }
                Block::Blockquote { blocks, .. } | Block::Details { blocks, .. } => {
                    walk(blocks, out);
                }
                _ => {}
            }
        }
    }

    let blocks = parse_content(fragment, 1);
    let mut out = Vec::new();
    walk(&blocks, &mut out);
    out
}

/// Parse the document's YAML frontmatter into an ordered, sorted-key map of
/// query [`Value`]s. Returns `None` when there is no frontmatter.
fn extract_frontmatter(doc: &Document) -> Option<IndexMap<String, Value>> {
    use turbovault_parser::{ParseOptions, ParsedContent};

    let parsed =
        ParsedContent::parse_with_options(&doc.content, ParseOptions::none().with_frontmatter());
    let fm = parsed.frontmatter?;

    // Sort keys for stable, deterministic output.
    let mut entries: Vec<(String, &serde_json::Value)> =
        fm.data.iter().map(|(k, v)| (k.clone(), v)).collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut map = IndexMap::new();
    for (k, v) in entries {
        map.insert(k, json_to_value(v));
    }
    Some(map)
}

/// Convert a `serde_json::Value` (from parsed frontmatter) into a query
/// [`Value`], sorting object keys for deterministic output.
fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => Value::Array(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let mut map = IndexMap::new();
            for k in keys {
                map.insert(k.clone(), json_to_value(&obj[k]));
            }
            Value::Object(map)
        }
    }
}

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::String(s) => Value::String(s.clone()),
        Literal::Number(n) => Value::Number(*n),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Null => Value::Null,
    }
}

/// Decode the parser's encoded `_index` argument expression back into an
/// [`IndexOp`]. See [`Engine::eval_index`] for the encoding.
fn decode_index_arg(arg: &Expr) -> IndexOp {
    match arg {
        Expr::Literal {
            value: Literal::Number(n),
            ..
        } => IndexOp::Single(*n as i64),
        Expr::Array { elements, .. } if elements.len() == 2 => {
            let bound = |e: &Expr| -> Option<i64> {
                if let Expr::Literal {
                    value: Literal::Number(n),
                    ..
                } = e
                {
                    Some(*n as i64)
                } else {
                    None
                }
            };
            IndexOp::Slice {
                start: bound(&elements[0]),
                end: bound(&elements[1]),
            }
        }
        _ => IndexOp::Iterate,
    }
}

fn apply_index(mut values: Vec<Value>, index: &IndexOp) -> Result<Vec<Value>, QueryError> {
    match index {
        IndexOp::Single(idx) => {
            let len = values.len() as i64;
            let actual_idx = if *idx < 0 { len + *idx } else { *idx };

            if actual_idx >= 0 && actual_idx < len {
                Ok(vec![values.remove(actual_idx as usize)])
            } else {
                Ok(vec![])
            }
        }
        IndexOp::Slice { start, end } => {
            let len = values.len() as i64;
            let start_idx = start
                .map(|s| if s < 0 { (len + s).max(0) } else { s })
                .unwrap_or(0) as usize;
            let end_idx = end
                .map(|e| if e < 0 { (len + e).max(0) } else { e })
                .unwrap_or(len) as usize;

            let start_idx = start_idx.min(values.len());
            let end_idx = end_idx.min(values.len());

            if start_idx < end_idx {
                Ok(values.drain(start_idx..end_idx).collect())
            } else {
                Ok(vec![])
            }
        }
        IndexOp::Iterate => Ok(values),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
        (Value::String(a), Value::String(b)) => a == b,
        _ => a.to_text() == b.to_text(),
    }
}

/// Total ordering used by `sort_by`. Numbers sort numerically, strings
/// lexically; mixed/other types fall back to their text representation so the
/// sort is always total and deterministic.
fn sort_key_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => a.to_text().cmp(&b.to_text()),
    }
}

fn compare_values(a: &Value, b: &Value) -> i32 {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if a < b {
                -1
            } else if a > b {
                1
            } else {
                0
            }
        }
        (Value::String(a), Value::String(b)) => a.cmp(b) as i32,
        _ => 0,
    }
}

fn add_values(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
        (Value::String(a), Value::String(b)) => Value::String(format!("{}{}", a, b)),
        (Value::Array(a), Value::Array(b)) => {
            let mut result = a.clone();
            result.extend(b.clone());
            Value::Array(result)
        }
        _ => Value::String(format!("{}{}", a.to_text(), b.to_text())),
    }
}

fn sub_values(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
        _ => Value::Null,
    }
}

/// Maximum length (in bytes) of a string produced by the repeat operator
/// (`"x" * n`). Prevents capacity-overflow aborts and runaway allocations.
const MAX_REPEAT_LEN: usize = 10 * 1024 * 1024;

fn mul_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
        (Value::String(s), Value::Number(n)) | (Value::Number(n), Value::String(s)) => {
            // jq-style string repeat. Reject negative/non-finite counts and cap
            // the result size so `"x" * 1e300` errors cleanly instead of
            // aborting the process with a capacity overflow.
            if !n.is_finite() || *n < 0.0 {
                return Err(QueryError::new(
                    QueryErrorKind::InvalidOperation(format!(
                        "string repeat count must be a finite, non-negative number (got {n})"
                    )),
                    Span::default(),
                    String::new(),
                ));
            }
            let count = *n as usize;
            let total = s.len().saturating_mul(count);
            if total > MAX_REPEAT_LEN {
                return Err(QueryError::new(
                    QueryErrorKind::InvalidOperation(format!(
                        "string repeat result too large ({total} bytes, max {MAX_REPEAT_LEN})"
                    )),
                    Span::default(),
                    String::new(),
                ));
            }
            Ok(Value::String(s.repeat(count)))
        }
        _ => Ok(Value::Null),
    }
}

fn div_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if *b == 0.0 {
                Err(QueryError::new(
                    QueryErrorKind::DivisionByZero,
                    Span::default(),
                    String::new(),
                ))
            } else {
                Ok(Value::Number(a / b))
            }
        }
        _ => Ok(Value::Null),
    }
}

fn mod_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            if *b == 0.0 {
                Err(QueryError::new(
                    QueryErrorKind::DivisionByZero,
                    Span::default(),
                    String::new(),
                ))
            } else {
                Ok(Value::Number(a % b))
            }
        }
        _ => Ok(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_markdown;
    use crate::query::parse;

    fn eval(md: &str, query: &str) -> Vec<Value> {
        let doc = parse_markdown(md);
        let query = parse(query).unwrap();
        let mut engine = Engine::new(&doc);
        engine.execute(&query).unwrap()
    }

    #[test]
    fn test_identity() {
        let results = eval("# Hello", ".");
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], Value::Document(_)));
    }

    #[test]
    fn test_heading_selection() {
        let results = eval("# H1\n## H2\n### H3", ".h2");
        assert_eq!(results.len(), 1);
        if let Value::Heading(h) = &results[0] {
            assert_eq!(h.text, "H2");
            assert_eq!(h.level, 2);
        } else {
            panic!("Expected Heading");
        }
    }

    #[test]
    fn test_all_headings() {
        let results = eval("# H1\n## H2\n### H3", ".h");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_heading_index() {
        let results = eval("# H1\n## H2a\n## H2b", ".h2[0]");
        assert_eq!(results.len(), 1);
        if let Value::Heading(h) = &results[0] {
            assert_eq!(h.text, "H2a");
        }
    }

    #[test]
    fn test_heading_filter() {
        let results = eval("# Hello\n## World\n## Goodbye", ".h2[World]");
        assert_eq!(results.len(), 1);
        if let Value::Heading(h) = &results[0] {
            assert_eq!(h.text, "World");
        }
    }

    #[test]
    fn test_code_blocks_in_list_items() {
        // Regression test: code blocks nested inside list items should be extracted
        // See bug report: indented fenced code blocks not parsed
        let md = r#"## Installation

1. Install from crates.io:
   ```bash
   cargo install treemd
   ```

2. Or build from source:
   ```bash
   git clone https://github.com/example/repo
   cd repo
   cargo install --path .
   ```"#;

        let results = eval(md, ".code");
        assert_eq!(
            results.len(),
            2,
            "Should find 2 code blocks nested in list items"
        );

        // Verify first code block
        if let Value::Code(c) = &results[0] {
            assert_eq!(c.language.as_deref(), Some("bash"));
            assert!(c.content.contains("cargo install treemd"));
        } else {
            panic!("Expected Code value");
        }

        // Verify second code block
        if let Value::Code(c) = &results[1] {
            assert_eq!(c.language.as_deref(), Some("bash"));
            assert!(c.content.contains("git clone"));
        } else {
            panic!("Expected Code value");
        }
    }

    #[test]
    fn test_code_blocks_with_content_filter_in_list() {
        // Note: Language filtering via .code[rust] currently uses text filter (matches content)
        // For content-based filtering, we can test with content patterns
        let md = r#"## Examples

1. Python example:
   ```python
   print("hello")
   ```

2. Rust example:
   ```rust
   fn main() {}
   ```"#;

        // Filter by content (text filter matches the code content)
        let results = eval(md, ".code[main]");
        assert_eq!(
            results.len(),
            1,
            "Should find 1 code block containing 'main'"
        );

        if let Value::Code(c) = &results[0] {
            assert!(c.content.contains("fn main"));
        }
    }
}
