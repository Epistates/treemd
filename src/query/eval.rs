//! Query evaluator.
//!
//! Executes parsed queries against markdown documents.

use indexmap::IndexMap;
use std::sync::Arc;

use super::ast::*;
use super::error::{QueryError, QueryErrorKind};
use super::registry::{Registry, StreamMode};
use super::value::*;
use crate::parser::Document;
use crate::parser::output::InlineElement;

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
        // Past a byte order mark, which otherwise sits in front of the first
        // block and keeps a leading table, quote or list from being one.
        let body = doc.content.strip_prefix('\u{feff}').unwrap_or(&doc.content);
        let mut extracted = extract_blocks(body);
        // In document order, which section scoping searches.
        extracted.code_blocks.sort_by_key(|code| code.start_line);
        let mut links = extract_doc_links(&doc.content);
        links.sort_by_key(|link| link.offset);
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
            links,
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
        let document = Value::Document(self.context.document.clone());
        self.eval_stages(&piped.stages, document)
    }

    /// Run pipeline `stages` left to right, starting from `input`.
    ///
    /// Each stage normally runs once per value from the stage before. A
    /// collection function (see [`StreamMode`]) after a stage that yields a
    /// stream runs once on the whole stream instead, so `.h | count` counts
    /// headings rather than printing `1` per heading. That holds for an
    /// empty stream too, which is how `.h[Missing] | count` gives `0`.
    ///
    /// `current` is saved and restored around the pipeline so a stage's
    /// value does not leak into the surrounding expression (in
    /// `{a: (.text | upper), b: .level}`, `.level` must still see the
    /// heading).
    fn eval_stages(&mut self, stages: &[Expr], input: Value) -> Result<Vec<Value>, QueryError> {
        let saved = self.context.current.clone();
        let result = self.run_stages(stages, input);
        self.context.current = saved;
        result
    }

    fn run_stages(&mut self, stages: &[Expr], input: Value) -> Result<Vec<Value>, QueryError> {
        let mut current = vec![input];
        let mut is_stream = false;

        for stage in stages {
            let mode = self.stream_mode(stage);
            let collect = is_stream && mode != StreamMode::PerValue;

            current = if collect {
                self.context.current = Value::Array(current);
                let out = self.eval_expr(stage)?;
                match (mode, <[Value; 1]>::try_from(out)) {
                    (StreamMode::Reorder, Ok([Value::Array(items)])) => items,
                    (_, Ok(single)) => single.into(),
                    (_, Err(out)) => out,
                }
            } else {
                let mut next = Vec::new();
                for value in current {
                    self.context.current = value;
                    next.extend(self.eval_expr(stage)?);
                }
                next
            };

            is_stream = self.yields_stream(stage, is_stream);
        }

        Ok(current)
    }

    /// How the function a pipeline stage calls treats a piped stream.
    ///
    /// A stage that starts with a reducing function, as `count > 3` and
    /// `count + 1` do, reduces too: it compares or adds to the count of the
    /// whole stream. It used to run once per value, so `.h | count > 3`
    /// printed `false` for each heading.
    fn stream_mode(&self, stage: &Expr) -> StreamMode {
        match stage {
            Expr::Function { name, .. } if !is_internal_form(name) => self
                .registry
                .get_function(name)
                .map(|f| f.stream)
                .unwrap_or_default(),
            Expr::Binary { left, .. } => match self.stream_mode(left) {
                StreamMode::Reduce => StreamMode::Reduce,
                _ => StreamMode::PerValue,
            },
            _ => StreamMode::PerValue,
        }
    }

    /// Whether `expr`, given an input that is (`input`) or is not a stream,
    /// can yield more than one value per pipeline.
    ///
    /// Decided from the query rather than from how many values came back,
    /// so that `.h | text | first` is the first heading's text even in a
    /// document with one heading, rather than that text's first character.
    fn yields_stream(&self, expr: &Expr, input: bool) -> bool {
        match expr {
            Expr::Element { kind, index, .. } => match (kind, index) {
                (ElementKind::FrontMatter, _) | (_, Some(IndexOp::Single(_))) => input,
                _ => true,
            },
            Expr::Hierarchy { .. } => true,
            Expr::Group { expr, .. } => self.yields_stream(expr, input),
            Expr::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                // With no `else` the input passes through unchanged.
                self.yields_stream(then_branch, input)
                    || else_branch
                        .as_deref()
                        .map_or(input, |e| self.yields_stream(e, input))
            }
            Expr::Function { name, args, .. } => match name.as_str() {
                "_pipe" => args
                    .iter()
                    .fold(input, |s, stage| self.yields_stream(stage, s)),
                // An index or a slice keeps its target's shape: one array
                // sliced is one array, and a stream sliced is a stream.
                "_index" => match args.get(1).map(decode_index_arg) {
                    Some(IndexOp::Single(_) | IndexOp::Slice { .. }) => {
                        self.yields_stream(&args[0], input)
                    }
                    _ => true,
                },
                _ => match self.stream_mode(expr) {
                    StreamMode::PerValue | StreamMode::Reorder => input,
                    StreamMode::Reduce => false,
                },
            },
            Expr::Binary { .. } if self.stream_mode(expr) == StreamMode::Reduce => false,
            _ => input,
        }
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
                name,
                filters,
                index,
                span,
            } => self.eval_element(kind, name, filters, index.as_ref(), *span),

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
        name: &str,
        filters: &[Filter],
        index: Option<&IndexOp>,
        _span: Span,
    ) -> Result<Vec<Value>, QueryError> {
        // An element name is also a plausible key, so `.image` on front
        // matter or `.p` on a constructed object reads that key when the
        // input has it. The selector used to win everywhere, which turned
        // `{p: 1} | .p` into every paragraph in the document. On the
        // document itself the selector still wins, so `.h2` and `. | .code`
        // select elements as they always have.
        if !matches!(self.context.current, Value::Document(_))
            && let Some(value) = self.context.current.get_property(name)
        {
            let mut values = match value {
                Value::Array(items) if index.is_some() || !filters.is_empty() => items,
                other => vec![other],
            };
            for filter in filters {
                values = self.apply_filter(values, filter)?;
            }
            if let Some(idx) = index {
                values = apply_index(values, idx)?;
            }
            return Ok(values);
        }

        // A selector piped from a heading selects within that heading's
        // section, the same as `>>`, so `.h2[0] | .code` is the code under
        // that h2 rather than every block in the document.
        let mut elements = match &self.context.current {
            Value::Heading(parent) => self.scoped_elements(parent, kind, false),
            _ => self.all_elements(kind),
        };

        for filter in filters {
            elements = self.apply_filter(elements, filter)?;
        }
        if let Some(idx) = index {
            elements = apply_index(elements, idx)?;
        }

        Ok(elements)
    }

    /// Every element of `kind` in the document.
    fn all_elements(&self, kind: &ElementKind) -> Vec<Value> {
        match kind {
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
        }
    }

    /// The elements of `kind` inside `parent`'s section.
    ///
    /// `direct` (`>`) stops at the next heading of any level, so it covers
    /// the heading's own body; otherwise (`>>`) the section runs to the next
    /// heading at the same level or above and includes every subsection.
    /// Front matter belongs to the document rather than to any section, so
    /// it is returned whatever the heading.
    fn scoped_elements(
        &self,
        parent: &HeadingValue,
        kind: &ElementKind,
        direct: bool,
    ) -> Vec<Value> {
        let headings = &self.context.headings;
        let parent_idx = parent.index;
        let parent_level = parent.level;

        let scope_start = parent.offset;
        let end_heading = headings
            .iter()
            .skip(parent_idx + 1)
            .find(|h| direct || h.level <= parent_level);
        let scope_end = end_heading.map_or(self.context.raw_content.len(), |h| h.offset);

        match kind {
            ElementKind::Heading(level_filter) => {
                // Walking the subtree with the lowest level seen so far: a
                // heading is a direct child when nothing between it and the
                // parent sits at a shallower level than it does.
                let mut shallowest = u8::MAX;
                let mut results = Vec::new();
                for h in headings[parent_idx + 1..]
                    .iter()
                    .take_while(|h| h.level > parent_level)
                {
                    let is_child = h.level <= shallowest;
                    shallowest = shallowest.min(h.level);
                    if direct && !is_child {
                        continue;
                    }
                    if level_filter.is_some_and(|level| h.level != level) {
                        continue;
                    }
                    results.push(Value::Heading(h.clone()));
                }
                results
            }
            ElementKind::Code => {
                // A block's start_line is its opening fence's row in the
                // document, so the global list scopes by line. Re-parsing
                // the section instead numbered its blocks from the
                // section's own first line. The blocks are in document
                // order and the headings know their lines, so this is a
                // search: counting lines from the top of the document and
                // scanning every block, for each heading, took 38 seconds
                // for `.h1 >> .code` on 20,000 headings.
                let blocks = &self.context.code_blocks;
                let past = end_heading.map_or(usize::MAX, |h| h.line);
                let first = blocks.partition_point(|code| code.start_line < parent.line);
                let end = blocks.partition_point(|code| code.start_line < past);
                blocks[first..end.max(first)]
                    .iter()
                    .cloned()
                    .map(Value::Code)
                    .collect()
            }
            // Links carry byte offsets, so scope by byte range.
            ElementKind::Link => {
                let links = &self.context.links;
                let first = links.partition_point(|link| link.offset < scope_start);
                let end = links.partition_point(|link| link.offset < scope_end);
                links[first..end.max(first)]
                    .iter()
                    .cloned()
                    .map(Value::Link)
                    .collect()
            }
            ElementKind::FrontMatter => self.all_elements(kind),
            // Images, tables, lists, paragraphs and blockquotes carry no
            // source position, so the section is parsed on its own. None of
            // them holds a line number for that to get wrong.
            ElementKind::Image
            | ElementKind::Table
            | ElementKind::List
            | ElementKind::Blockquote
            | ElementKind::Paragraph => {
                let section = extract_blocks(&self.context.raw_content[scope_start..scope_end]);
                match kind {
                    ElementKind::Image => section.images.into_iter().map(Value::Image).collect(),
                    ElementKind::Table => section.tables.into_iter().map(Value::Table).collect(),
                    ElementKind::List => section.lists.into_iter().map(Value::List).collect(),
                    ElementKind::Blockquote => section
                        .blockquotes
                        .into_iter()
                        .map(Value::Blockquote)
                        .collect(),
                    _ => section
                        .paragraphs
                        .into_iter()
                        .map(Value::Paragraph)
                        .collect(),
                }
            }
        }
    }

    fn apply_filter(
        &self,
        elements: Vec<Value>,
        filter: &Filter,
    ) -> Result<Vec<Value>, QueryError> {
        match filter {
            Filter::Text { pattern, exact, .. } => Ok(elements
                .into_iter()
                .filter(|v| text_filter_matches(v, pattern, *exact))
                .collect()),
            // A link type only means something to a link. Everywhere else
            // the word is an ordinary text filter, so `.h[external]` finds
            // "External APIs" rather than nothing.
            Filter::Type { type_name, .. } => Ok(elements
                .into_iter()
                .filter(|v| match v {
                    Value::Link(link) => link.link_type.as_str() == type_name,
                    _ => text_filter_matches(v, type_name, false),
                })
                .collect()),
        }
    }

    fn eval_property(&mut self, name: &str, span: Span) -> Result<Vec<Value>, QueryError> {
        let current = &self.context.current;

        // A heading's place in the outline needs the document's other
        // headings, which the heading value itself does not hold.
        if let Value::Heading(heading) = current {
            match name {
                "children" => {
                    let children = self.scoped_elements(heading, &ElementKind::Heading(None), true);
                    return Ok(vec![Value::Array(children)]);
                }
                "parent" => {
                    let parent = self.context.headings[..heading.index]
                        .iter()
                        .rev()
                        .find(|h| h.level < heading.level)
                        .map_or(Value::Null, |h| Value::Heading(h.clone()));
                    return Ok(vec![parent]);
                }
                _ => {}
            }
        }

        if let Some(value) = current.get_property(name) {
            Ok(vec![value])
        } else if current.as_map().is_some() || matches!(current, Value::Null) {
            // A key that is not there is null, as in jq, so `.fm.draft //
            // false` works on a page without `draft`. An element's
            // properties are fixed, and a wrong one is still an error.
            Ok(vec![Value::Null])
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
                let input = self.context.current.clone();
                return self.eval_stages(args, input);
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
                        expected: describe_arity(&func.arity),
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
    /// When every value of the target is an array, each one is indexed in
    /// turn, as jq does, so `.table[0].rows[][0]` is the first cell of every
    /// row. Indexing the stream instead gave the first row. As in jq, a
    /// slice of an array is an array, and an index past either end is
    /// `null`. Strings slice by character. When the values are neither, as
    /// in `(.h | select(.level == 2))[0]`, the stream itself is indexed.
    /// Negative indices count from the end either way.
    fn eval_index(&mut self, target: &Expr, index_arg: &Expr) -> Result<Vec<Value>, QueryError> {
        let target_vals = self.eval_expr(target)?;
        let index = decode_index_arg(index_arg);

        if !target_vals.is_empty() && target_vals.iter().all(|v| matches!(v, Value::Array(_))) {
            let mut results = Vec::new();
            for value in target_vals {
                if let Value::Array(items) = value {
                    match index {
                        IndexOp::Single(_) => {
                            results.push(apply_index(items, &index)?.pop().unwrap_or(Value::Null))
                        }
                        IndexOp::Slice { .. } => {
                            results.push(Value::Array(apply_index(items, &index)?));
                        }
                        IndexOp::Iterate => results.extend(items),
                    }
                }
            }
            return Ok(results);
        }

        if let IndexOp::Slice { .. } = index
            && !target_vals.is_empty()
            && target_vals.iter().all(|v| matches!(v, Value::String(_)))
        {
            let mut results = Vec::new();
            for value in target_vals {
                let chars = value
                    .to_text()
                    .chars()
                    .map(|c| Value::String(c.to_string()))
                    .collect();
                let kept: String = apply_index(chars, &index)?
                    .iter()
                    .map(Value::to_text)
                    .collect();
                results.push(Value::String(kept));
            }
            return Ok(results);
        }

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
    ///
    /// A string literal names a property, so `group_by("level")` means
    /// `group_by(.level)`. Evaluated as an expression it would be the same
    /// string for every element, and everything landed in one group.
    fn eval_key(&mut self, element: &Value, key_expr: &Expr) -> Result<Value, QueryError> {
        if let Expr::Literal {
            value: Literal::String(name),
            ..
        } = key_expr
        {
            return Ok(element.get_property(name).unwrap_or(Value::Null));
        }
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
        keyed.sort_by(|a, b| a.0.sort_cmp(&b.0));
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
        if let Expr::Element { kind, .. } = child {
            for parent_val in &parent_values {
                if let Value::Heading(parent_heading) = parent_val {
                    results.extend(self.scoped_elements(parent_heading, kind, direct));
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
            // jq's order across types, the one `sort` uses: null, booleans,
            // numbers, strings, arrays, objects. Values of different types
            // used to compare as equal, so `<` and `>` were false while `<=`
            // and `>=` were true for the same pair.
            BinaryOp::Lt => Value::Bool(left_val.sort_cmp(&right_val).is_lt()),
            BinaryOp::Le => Value::Bool(left_val.sort_cmp(&right_val).is_le()),
            BinaryOp::Gt => Value::Bool(left_val.sort_cmp(&right_val).is_gt()),
            BinaryOp::Ge => Value::Bool(left_val.sort_cmp(&right_val).is_ge()),
            BinaryOp::And => Value::Bool(left_val.is_truthy() && right_val.is_truthy()),
            BinaryOp::Or => Value::Bool(left_val.is_truthy() || right_val.is_truthy()),
            BinaryOp::Add => add_values(&left_val, &right_val)?,
            BinaryOp::Sub => sub_values(&left_val, &right_val)?,
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
            UnaryOp::Neg => match val {
                Value::Number(n) => Value::Number(-n),
                other => {
                    return Err(QueryError::new(
                        QueryErrorKind::InvalidOperation(format!(
                            "{} cannot be negated",
                            other.kind()
                        )),
                        Span::default(),
                        String::new(),
                    ));
                }
            },
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
            // No `else` passes the input through, as in jq 1.7, so
            // `if .level == 1 then upper end` leaves other headings alone
            // rather than turning them into null.
            Ok(vec![self.context.current.clone()])
        }
    }
}

// Helper functions

fn extract_headings(doc: &Document) -> Vec<HeadingValue> {
    let lines = doc.heading_lines();
    doc.headings
        .iter()
        .enumerate()
        .map(|(idx, h)| {
            let line = lines[idx];

            // Body bounds shared with the document extractor: handles CRLF,
            // setext underlines, and EOF-without-newline uniformly.
            let content_end = doc.section_end(idx);
            let content_start = doc.body_start(idx).min(content_end);

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
    images: Vec<ImageValue>,
    tables: Vec<TableValue>,
    lists: Vec<ListValue>,
    paragraphs: Vec<ParagraphValue>,
    blockquotes: Vec<BlockquoteValue>,
}

/// Collects images carried as inline elements rather than standalone blocks.
///
/// A standalone `![alt](src)` line is a paragraph in CommonMark, so without
/// this the most common form of image would never reach `.img` or `stats`.
/// List items carry theirs the same way.
///
/// Headings do not, despite the arm below that calls this for them: the parser
/// still hoists a heading's image out to a top-level `Block::Image`, so that
/// arm is what actually reports it. Both paths are needed, and neither is
/// redundant. See [`Images`] for why the list arm must not do both.
fn collect_inline_images(inline: &[InlineElement], out: &mut ExtractedBlocks) {
    for element in inline {
        if let InlineElement::Image {
            alt, src, title, ..
        } = element
        {
            out.images.push(ImageValue {
                alt: alt.clone(),
                src: src.clone(),
                title: title.clone(),
            });
        }
    }
}

/// Extracts every block-level element from `content`, which is a whole
/// document or, for scoping, one heading's section of it.
fn extract_blocks(content: &str) -> ExtractedBlocks {
    use crate::parser::content::parse_content;
    use crate::parser::output::Block;

    let blocks = parse_content(content, 1);

    let mut out = ExtractedBlocks::default();

    /// Whether a `walk` pass should collect images.
    ///
    /// A list item's `inline` already carries every image in the item, at any
    /// depth, so the pass over that item's nested blocks has to leave images
    /// alone. Collecting from both counts each one twice.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Images {
        Collect,
        Skip,
    }

    // Recursively extract blocks from nested structures (e.g., list items,
    // blockquotes, details). Top-level paragraphs/blockquotes are collected by
    // the outer loop; nested ones are collected here too.
    fn walk(blocks: &[Block], out: &mut ExtractedBlocks, images: Images) {
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
                Block::Image { alt, src, title } if images == Images::Collect => {
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
                Block::Paragraph { content, inline } => {
                    if images == Images::Collect {
                        collect_inline_images(inline, out);
                    }
                    out.paragraphs.push(ParagraphValue {
                        content: content.clone(),
                    });
                }
                Block::Heading { inline, .. } => {
                    if images == Images::Collect {
                        collect_inline_images(inline, out);
                    }
                }
                Block::List { ordered, items } => {
                    // Pushed before descending so a nested list follows its
                    // parent, in document order.
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
                    for item in items {
                        if images == Images::Collect {
                            collect_inline_images(&item.inline, out);
                        }
                        walk(&item.blocks, out, Images::Skip);
                    }
                }
                Block::Blockquote { content, blocks } => {
                    out.blockquotes.push(BlockquoteValue {
                        content: content.clone(),
                    });
                    walk(blocks, out, images);
                }
                Block::Details { blocks, .. } => {
                    walk(blocks, out, images);
                }
                _ => {}
            }
        }
    }

    walk(&blocks, &mut out, Images::Collect);

    out
}

/// Every link in the document, with its byte offset for scoping.
fn extract_doc_links(content: &str) -> Vec<LinkValue> {
    crate::parser::links::extract_links(content)
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
        .collect()
}

/// The most YAML aliases (`*name`) a frontmatter block may use.
///
/// Each alias repeats what its anchor holds, and anchors can hold aliases,
/// so a few hundred lines expand to gigabytes: a 225 KB block took 5 GB
/// and 8 seconds, on every query, whether or not it read the frontmatter.
/// Frontmatter written by hand has a handful at most.
const MAX_FRONTMATTER_ALIASES: usize = 64;

/// Parse the document's YAML frontmatter into an ordered, sorted-key map of
/// query [`Value`]s. Returns `None` when there is no frontmatter.
///
/// Only a block at the very start of the document counts, as
/// [`crate::parser::frontmatter_range`] finds it. Handed the whole document,
/// the parser also took a `key: value` block between two `---` rules
/// further down for frontmatter, over the real one if there was one, and
/// missed the real one behind a byte order mark.
fn extract_frontmatter(doc: &Document) -> Option<IndexMap<String, Value>> {
    use turbovault_parser::{ParseOptions, ParsedContent};

    let block = &doc.content[crate::parser::frontmatter_range(&doc.content)?];
    let block = block.strip_prefix('\u{feff}').unwrap_or(block);
    if yaml_alias_count(block) > MAX_FRONTMATTER_ALIASES {
        return None;
    }
    let parsed = ParsedContent::parse_with_options(block, ParseOptions::none().with_frontmatter());
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

/// How many YAML aliases `yaml` uses: a `*` that starts a node, after a
/// line's indentation, a `- `, a `: `, or `[`, `{` and `,` in a flow
/// collection, followed by a name. A `*` inside text is not counted, so this
/// can only miss aliases in odd layouts, never refuse ordinary frontmatter.
fn yaml_alias_count(yaml: &str) -> usize {
    let bytes = yaml.as_bytes();
    (0..bytes.len())
        .filter(|&i| {
            bytes[i] == b'*'
                && bytes
                    .get(i + 1)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                && match bytes[..i].iter().rposition(|b| *b != b' ' && *b != b'\t') {
                    None => true,
                    Some(j) => matches!(bytes[j], b'\n' | b'-' | b':' | b'[' | b'{' | b','),
                }
        })
        .count()
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
/// An argument count range in words, for arity errors: "no arguments",
/// "1 argument", "1 to 2 arguments". It printed as a Rust range, `0..=0`.
fn describe_arity(arity: &std::ops::RangeInclusive<usize>) -> String {
    let plural = |n: usize| if n == 1 { "argument" } else { "arguments" };
    match (*arity.start(), *arity.end()) {
        (0, 0) => "no arguments".to_string(),
        (a, b) if a == b => format!("{a} {}", plural(a)),
        (a, b) => format!("{a} to {b} {}", plural(b)),
    }
}

/// The function names the parser uses for its own pipe and postfix index
/// forms. They are not registry functions.
fn is_internal_form(name: &str) -> bool {
    matches!(name, "_pipe" | "_index")
}

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

/// Whether `value` passes a bracket text filter: case-insensitive, whole
/// text when `exact` (a quoted pattern) and a substring otherwise.
///
/// A code block is matched by language instead, never its body, since
/// `.code[rust]` means Rust code blocks. `select(contains(...))` searches
/// bodies.
fn text_filter_matches(value: &Value, pattern: &str, exact: bool) -> bool {
    if let Value::Code(code) = value {
        return code_matches_language(code, pattern);
    }
    let text = value.to_text().to_lowercase();
    let pattern = pattern.to_lowercase();
    if exact {
        text == pattern
    } else {
        text.contains(&pattern)
    }
}

/// Matches a code block against a language name from a bracket filter.
///
/// The comparison is whole-string and case-insensitive, so `.code[java]` never
/// matches a JavaScript block. Fenced blocks with no info string match nothing.
fn code_matches_language(code: &CodeValue, language: &str) -> bool {
    code.language
        .as_deref()
        .is_some_and(|lang| lang.eq_ignore_ascii_case(language))
}

/// `==` as jq has it: values of different types are never equal, so
/// `1 == "1"` and `null == ""` are false. Comparing text made them true.
/// Arrays, objects and elements are equal when they hold the same values.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
        (Value::String(a), Value::String(b)) => a == b,
        _ => a.identity_key() == b.identity_key(),
    }
}

/// jq's error for an operator its operands do not support:
/// `string ("a") and number (1) cannot be subtracted`. These used to give
/// `null`, or for `+` the two values' text run together, so `{} + 1` was
/// `{}1` and a typo in a query passed as a result.
fn cannot(a: &Value, b: &Value, done: &str) -> QueryError {
    let describe = |v: &Value| {
        let mut json = v.to_json().to_string();
        if json.len() > 20 {
            let cut = (0..=17)
                .rev()
                .find(|&i| json.is_char_boundary(i))
                .unwrap_or(0);
            json.truncate(cut);
            json.push_str("...");
        }
        format!("{} ({json})", v.kind())
    };
    let mut error = QueryError::new(
        QueryErrorKind::InvalidOperation(format!(
            "{} and {} cannot be {done}",
            describe(a),
            describe(b)
        )),
        Span::default(),
        String::new(),
    );
    if done == "added" && (matches!(a, Value::String(_)) || matches!(b, Value::String(_))) {
        error = error.with_help("turn a value into text first: (.level | tostring)");
    }
    error
}

/// `+` as jq has it: numbers add, strings, arrays and objects join, and
/// `null` adds nothing.
fn add_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    Ok(match (a, b) {
        (Value::Null, other) | (other, Value::Null) => other.clone(),
        (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
        (Value::String(a), Value::String(b)) => Value::String(format!("{}{}", a, b)),
        (Value::Array(a), Value::Array(b)) => {
            let mut result = a.clone();
            result.extend(b.clone());
            Value::Array(result)
        }
        // Keys from the right replace the left's.
        _ => match (a.as_map(), b.as_map()) {
            (Some(left), Some(right)) => {
                let mut merged = left.clone();
                merged.extend(right.iter().map(|(k, v)| (k.clone(), v.clone())));
                Value::Object(merged)
            }
            _ => return Err(cannot(a, b, "added")),
        },
    })
}

/// `-` as jq has it: numbers subtract, and an array loses every item that
/// is in the other.
fn sub_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
        (Value::Array(a), Value::Array(b)) => Ok(Value::Array(
            a.iter()
                .filter(|item| !b.iter().any(|gone| values_equal(item, gone)))
                .cloned()
                .collect(),
        )),
        _ => Err(cannot(a, b, "subtracted")),
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
        _ => Err(cannot(a, b, "multiplied")),
    }
}

fn division_by_zero() -> QueryError {
    QueryError::new(
        QueryErrorKind::DivisionByZero,
        Span::default(),
        String::new(),
    )
}

/// `/` as jq has it: numbers divide, and a string divided by a string is
/// split on it.
fn div_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    match (a, b) {
        (Value::Number(_), Value::Number(b)) if *b == 0.0 => Err(division_by_zero()),
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a / b)),
        (Value::String(text), Value::String(sep)) => Ok(split_text(text, sep)),
        _ => Err(cannot(a, b, "divided")),
    }
}

/// `text` split at each `sep` into an array, as jq's `/` and `split` do: an
/// empty `sep` splits into characters, and empty text into nothing. Rust's
/// `split("")` gave an empty string at each end.
pub(crate) fn split_text(text: &str, sep: &str) -> Value {
    let parts: Vec<Value> = if text.is_empty() {
        Vec::new()
    } else if sep.is_empty() {
        text.chars().map(|c| Value::String(c.to_string())).collect()
    } else {
        text.split(sep)
            .map(|s| Value::String(s.to_string()))
            .collect()
    };
    Value::Array(parts)
}

/// `%` as jq has it: both sides are cut to whole numbers first, so
/// `5.9 % 3` is `2`, where the float remainder was `2.9000000000000004`.
fn mod_values(a: &Value, b: &Value) -> Result<Value, QueryError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            // `as` saturates, so a huge or infinite operand stays in range.
            let (a, b) = (*a as i64, *b as i64);
            if b == 0 {
                Err(division_by_zero())
            } else {
                Ok(Value::Number(a.wrapping_rem(b) as f64))
            }
        }
        _ => Err(cannot(a, b, "divided")),
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

    /// Each result's text, for comparing against a short expected list.
    fn texts(md: &str, query: &str) -> Vec<String> {
        eval(md, query).iter().map(Value::to_text).collect()
    }

    /// An element name after a pipe reads the key or property of that name
    /// when the input has one. The selector used to win, so each of these
    /// returned elements from the whole document instead.
    #[test]
    fn test_element_names_read_keys_on_inputs_that_have_them() {
        let md =
            "---\nimage: cover.png\n---\n# T\n\npara\n\n[l](http://x.example)\n\n![i](i.png)\n";
        assert_eq!(texts(md, "{p: 1} | .p"), ["1"]);
        assert_eq!(texts(md, ".h | {a: .text} | .a"), ["T"]);
        assert_eq!(texts(md, ".fm | .image"), ["cover.png"]);
        assert_eq!(texts(md, ".fm.image"), ["cover.png"]);
        assert_eq!(texts(md, "{a: [1, 2]} | .a[1]"), ["2"]);

        // On the document the selector still wins.
        assert_eq!(texts(md, ".p")[0], "para");
        assert_eq!(texts(md, ". | .img"), ["i"]);
        assert_eq!(texts(md, ". | .headings"), ["T"]);
        assert_eq!(texts(md, ". | .heading_count"), ["1"]);
        // An object without the key still gets the selector.
        assert_eq!(texts(md, "{x: 1} | .link"), ["l"]);
    }

    /// Code under a heading keeps its document line numbers. Scoping used to
    /// re-parse the section, which numbered its blocks from the section's
    /// first line, so `.h2[Install] > .code | .start_line` disagreed with
    /// `.code | .start_line` for the same block.
    #[test]
    fn test_scoped_code_blocks_keep_document_line_numbers() {
        let md = "# Doc\n\n```sh\na\n```\n\n## Install\n\ntext\n\n```bash\nb\n```\n\n\
                  ### Deep\n\n```rust\nc\n```\n\n## Other\n\n```py\nd\n```\n";
        let lines = |q: &str| -> Vec<usize> {
            eval(md, q)
                .into_iter()
                .filter_map(|v| match v {
                    Value::Code(c) => Some(c.start_line),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(lines(".code"), [3, 11, 17, 23]);
        assert_eq!(lines(".h2[Install] > .code"), [11]);
        assert_eq!(lines(".h2[Install] >> .code"), [11, 17]);
        assert_eq!(lines(".h1 >> .code"), [3, 11, 17, 23]);
        assert_eq!(lines(".h2[Other] > .code"), [23]);
    }

    /// A selector piped from a heading selects within its section, as `>>`
    /// does. It used to select from the whole document, so `.h2[0] | .code`
    /// returned every code block.
    #[test]
    fn test_selector_piped_from_a_heading_is_scoped_to_its_section() {
        let md = "# Doc\n\n```sh\na\n```\n\n## One\n\n```rust\nb\n```\n\n\
                  ### Deep\n\n```py\nc\n```\n\n## Two\n\n```go\nd\n```\n";
        assert_eq!(texts(md, ".h2[0] | .code | .lang"), ["rust", "py"]);
        assert_eq!(texts(md, ".h2[1] | .code | .lang"), ["go"]);
        assert_eq!(texts(md, ".h2[0] | .h"), ["Deep"]);
        assert_eq!(texts(md, ".h3 | .h"), Vec::<String>::new());
        // Each heading scopes its own selection.
        assert_eq!(texts(md, ".h2 | .code | .lang"), ["rust", "py", "go"]);
    }

    /// `>` and `>>` returned nothing at all for these kinds, which carry no
    /// source position. The section is parsed on its own instead.
    #[test]
    fn test_hierarchy_scopes_every_block_kind() {
        let md = "# A\n\nintro\n\n## B\n\n| x | y |\n|---|---|\n| 1 | 2 |\n\n- one\n\n\
                  > quoted\n\n![pic](p.png)\n\nbody\n\n### C\n\n- deep\n\n## D\n\nlast\n";
        assert_eq!(eval(md, ".h2[B] > .table").len(), 1);
        assert_eq!(texts(md, ".h2[B] > .list"), ["one"]);
        assert_eq!(texts(md, ".h2[B] >> .list"), ["one", "deep"]);
        assert_eq!(texts(md, ".h2[B] > .blockquote"), ["quoted"]);
        assert_eq!(texts(md, ".h2[B] > .img"), ["pic"]);
        assert_eq!(
            texts(md, ".h2[B] > .para | select(.text == \"body\")"),
            ["body"]
        );
        assert_eq!(texts(md, ".h1 > .para"), ["intro"]);
        assert_eq!(eval(md, ".h2[D] > .table").len(), 0);
        assert_eq!(texts(md, ".h2[B] | .list"), ["one", "deep"]);
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

    /// A link type name filtered every non-link out, so `.h[external]` missed
    /// "External APIs", and a keyword in brackets was a parse error.
    #[test]
    fn test_link_type_words_and_keywords_filter_text_outside_links() {
        let md = "# External APIs\n\n## Anchor points\n\n## The End\n\n## If Needed\n\n\
                  [x](https://x.example) [y](#y)\n";
        assert_eq!(texts(md, ".h[external]"), ["External APIs"]);
        assert_eq!(texts(md, ".h[anchor]"), ["Anchor points"]);
        assert_eq!(texts(md, ".h[end]"), ["The End"]);
        assert_eq!(texts(md, ".h[if]"), ["If Needed"]);
        assert_eq!(eval(md, ".h[relative]").len(), 0);
        // On links the word is still the link type.
        assert_eq!(texts(md, ".link[external]"), ["x"]);
        assert_eq!(texts(md, ".link[anchor]"), ["y"]);
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
    fn test_code_blocks_with_language_filter_in_list() {
        let md = r#"## Examples

1. Python example:
   ```python
   print("hello")
   ```

2. Rust example:
   ```rust
   fn main() {}
   ```"#;

        let results = eval(md, ".code[rust]");
        assert_eq!(results.len(), 1, "Should find 1 Rust code block");

        if let Value::Code(c) = &results[0] {
            assert_eq!(c.language.as_deref(), Some("rust"));
            assert!(c.content.contains("fn main"));
        } else {
            panic!("Expected Code value");
        }
    }

    #[test]
    fn test_code_language_filter_ignores_body_text() {
        let md = r#"```python
# rust is mentioned here
```

```rust
fn main() {}
```"#;

        // `[rust]` selects by language, so the Python block that merely
        // mentions "rust" in its body must not match.
        let results = eval(md, ".code[rust]");
        assert_eq!(results.len(), 1);
        if let Value::Code(c) = &results[0] {
            assert_eq!(c.language.as_deref(), Some("rust"));
        } else {
            panic!("Expected Code value");
        }

        // Bodies stay searchable through select().
        let by_body = eval(md, r#".code | select(contains("mentioned"))"#);
        assert_eq!(by_body.len(), 1);
        if let Value::Code(c) = &by_body[0] {
            assert_eq!(c.language.as_deref(), Some("python"));
        } else {
            panic!("Expected Code value");
        }
    }

    #[test]
    fn test_code_language_filter_is_whole_string_and_case_insensitive() {
        let md = r#"```javascript
let x = 1;
```

```Rust
fn main() {}
```"#;

        // A prefix must not match a longer language name.
        assert_eq!(eval(md, ".code[java]").len(), 0);
        assert_eq!(eval(md, ".code[javascript]").len(), 1);

        // Case-insensitive, in both the quoted and unquoted forms.
        assert_eq!(eval(md, ".code[rust]").len(), 1);
        assert_eq!(eval(md, r#".code["rust"]"#).len(), 1);
    }

    #[test]
    fn test_code_language_filter_skips_blocks_without_language() {
        let md = "```\nplain fence\n```\n\n```rust\nfn main() {}\n```";

        assert_eq!(eval(md, ".code").len(), 2);
        assert_eq!(eval(md, ".code[rust]").len(), 1);
    }

    fn image_srcs(md: &str) -> Vec<String> {
        eval(md, ".img")
            .into_iter()
            .filter_map(|v| match v {
                Value::Image(i) => Some(i.src),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_images_are_found_in_every_block_position() {
        // A standalone image is a paragraph containing one inline image, so
        // each of these reaches `.img` only by scanning inline elements.
        assert_eq!(image_srcs("![a](a.png)"), ["a.png"]);
        assert_eq!(image_srcs("# T\n\n![a](a.png)"), ["a.png"]);
        assert_eq!(image_srcs("Text with ![a](a.png) inside."), ["a.png"]);
        assert_eq!(image_srcs("# Title ![a](a.png)"), ["a.png"]);
        assert_eq!(image_srcs("- item ![a](a.png)"), ["a.png"]);
        assert_eq!(image_srcs("> ![a](a.png)"), ["a.png"]);
    }

    /// Both were unreachable until turbovault-parser 2.1.0. A blockquote used
    /// to be rebuilt from its raw text and re-parsed in a pass that flattened
    /// inline elements, so an image inside one arrived as bare alt text with
    /// its source already gone, and a heading's image was hoisted out with its
    /// alt left behind in the heading.
    #[test]
    fn test_image_alt_survives_a_blockquote_and_a_heading() {
        let details = |md: &str| -> Vec<(String, String)> {
            eval(md, ".img")
                .into_iter()
                .filter_map(|v| match v {
                    Value::Image(i) => Some((i.alt, i.src)),
                    _ => None,
                })
                .collect()
        };

        assert_eq!(
            details("> ![a](a.png)"),
            [("a".into(), "a.png".into())],
            "a quoted image keeps its alt and its source"
        );
        assert_eq!(
            details("# Title ![a](a.png)"),
            [("a".into(), "a.png".into())],
            "a heading's image keeps its alt"
        );
        // Still wrong, and the other half of the same defect: the alt reaches
        // the image now, but it was also left behind in the heading's own
        // text, so a heading renders as `Title a` in the outline and in
        // `--list`. Tracked at Epistates/turbovault#78.
        assert_eq!(
            eval("# Title ![a](a.png)", ".h1 | text")
                .into_iter()
                .map(|v| v.to_text())
                .collect::<Vec<_>>(),
            ["Title a"],
            "upstream fixed the heading text, drop the note in QUERY_LANGUAGE.md"
        );
    }

    /// A list item's indented continuation inside a blockquote was joined onto
    /// the item with no separator, so `step` and `text` came back as
    /// `steptext`, until turbovault-parser 2.2.0 (Epistates/turbovault#77).
    /// The continuation is the item's own paragraph now, in and out of a quote.
    #[test]
    fn test_a_quoted_list_item_continuation_keeps_its_own_block() {
        let item_text = |md: &str| -> Vec<String> {
            eval(md, ".list")
                .into_iter()
                .filter_map(|v| match v {
                    Value::List(l) => Some(l),
                    _ => None,
                })
                .flat_map(|l| l.items.into_iter().map(|i| i.content))
                .collect()
        };
        let paragraphs = |md: &str| -> Vec<String> {
            eval(md, ".para").into_iter().map(|v| v.to_text()).collect()
        };

        for md in [
            "> - step\n>\n>   text\n",
            "- step\n\n  text\n",
            "> - step\n>\n> text\n",
        ] {
            assert_eq!(item_text(md), ["step"], "{md:?}");
            assert_eq!(paragraphs(md), ["text"], "{md:?}");
        }
    }

    /// A nested list is its own entry since turbovault-parser 2.2.0, and is
    /// reported after the list holding it, not before.
    #[test]
    fn test_nested_lists_are_reported_in_document_order() {
        let items = eval("- outer\n  - inner\n    - deep\n- second\n", ".list")
            .into_iter()
            .filter_map(|v| match v {
                Value::List(l) => Some(l.items.into_iter().map(|i| i.content).collect()),
                _ => None,
            })
            .collect::<Vec<Vec<String>>>();

        assert_eq!(
            items,
            [vec!["outer", "second"], vec!["inner"], vec!["deep"]]
        );
    }

    /// Links inside a blockquote kept their text but lost their destination
    /// until 2.1.0.
    #[test]
    fn test_a_quoted_link_keeps_its_destination() {
        let urls: Vec<String> = eval("> see [t](http://x.example)", ".link")
            .into_iter()
            .filter_map(|v| match v {
                Value::Link(l) => Some(l.url),
                _ => None,
            })
            .collect();
        assert_eq!(urls, ["http://x.example"]);
    }

    /// Whatever precedes a fence inside a blockquote, the fence is still a
    /// code block. turbovault-parser 2.0.0 dropped it when a list came
    /// directly before, and merged its text into the quote's content, so the
    /// code resurfaced as prose. Fixed in 2.1.0 (Epistates/turbovault#71); all
    /// three orderings are kept here because only one of them ever broke.
    #[test]
    fn test_a_fence_in_a_blockquote_is_reported_whatever_precedes_it() {
        let after_list = "> - bullet\n>\n> ```rust\n> A();\n> ```\n";
        assert_eq!(eval(after_list, ".code").len(), 1);

        let after_prose = "> text\n>\n> ```rust\n> A();\n> ```\n";
        assert_eq!(eval(after_prose, ".code").len(), 1);

        let after_both = "> - bullet\n>\n> text\n>\n> ```rust\n> A();\n> ```\n";
        assert_eq!(eval(after_both, ".code").len(), 1);
    }

    #[test]
    fn test_multiple_images_are_collected_in_document_order() {
        let md = "![a](a.png)\n\n![b](b.png)\n\n- item ![c](c.png)";
        assert_eq!(image_srcs(md), ["a.png", "b.png", "c.png"]);
    }

    #[test]
    fn test_image_is_not_double_counted() {
        // Guards the block and inline paths from both claiming one image.
        assert_eq!(image_srcs("- item ![a](a.png)").len(), 1);
        assert_eq!(image_srcs("![a](a.png)").len(), 1);
    }

    /// A list item's `inline` carries every image in the item, including the
    /// ones inside its nested blocks, so walking those blocks as well reports
    /// each of them twice. Only multi-block items expose this: in a one-block
    /// item there is nothing for the walk to find, which is why the single
    /// paragraph cases above stayed green while `.img` was returning doubles.
    #[test]
    fn test_images_in_a_multi_block_list_item_are_reported_once() {
        assert_eq!(image_srcs("- item\n\n  ![a](a.png)\n"), ["a.png"]);
        assert_eq!(image_srcs("- [ ] task\n\n  ![t](t.png)\n"), ["t.png"]);
        assert_eq!(
            image_srcs("- item ![z](z.png)\n\n  more ![y](y.png)\n"),
            ["z.png", "y.png"],
            "an item's own image and its nested one, each once and in order"
        );
        assert_eq!(
            image_srcs("- a ![1](1.png)\n\n  b ![2](2.png)\n\n  c ![3](3.png)\n"),
            ["1.png", "2.png", "3.png"]
        );
        assert_eq!(
            image_srcs("- item\n\n  ```rust\n  code\n  ```\n\n  ![c](c.png)\n"),
            ["c.png"],
            "a fenced block between the text and the image changes nothing"
        );
    }

    /// Suppressing images while walking a list item's nested blocks must not
    /// suppress anything else those blocks carry.
    #[test]
    fn test_nested_blocks_in_a_list_item_still_yield_non_image_content() {
        let md = "- item\n\n  ```rust\n  code\n  ```\n\n  | a | b |\n  |---|---|\n  | 1 | 2 |\n";
        assert_eq!(eval(md, ".code").len(), 1);
        assert_eq!(eval(md, ".table").len(), 1);
    }

    /// The image forms that turbovault-parser 2.0.0 fixed. Each returned
    /// nothing usable on 1.6.0: the linked image was dropped outright, the
    /// title was folded into the source, and a list item reported differently
    /// depending only on whether a blank line sat between the items.
    #[test]
    fn test_image_forms_fixed_in_parser_2_0() {
        let details = |md: &str| -> Vec<(String, String, Option<String>)> {
            eval(md, ".img")
                .into_iter()
                .filter_map(|v| match v {
                    Value::Image(i) => Some((i.alt, i.src, i.title)),
                    _ => None,
                })
                .collect()
        };

        assert_eq!(
            details("[![badge](b.png)](https://ci.example)"),
            [("badge".into(), "b.png".into(), None)],
            "an image wrapped in a link is still an image"
        );
        assert_eq!(
            details(r#"![a](x.png "Title")"#),
            [("a".into(), "x.png".into(), Some("Title".into()))],
            "the title is its own field, not part of the source"
        );
        assert_eq!(
            details(r#"![a](my image.png "Title")"#),
            [("a".into(), "my image.png".into(), Some("Title".into()))],
            "a spaced destination keeps both its source and its title"
        );
        assert_eq!(
            image_srcs("- item ![a](a.png)\n\n- second ![b](b.png)"),
            ["a.png", "b.png"],
            "a loose list reports the same images as a tight one"
        );
    }

    /// Different types compared as equal, so `<` was false while `<=` was
    /// true for the same pair. They follow jq's order now, as `sort` does.
    #[test]
    fn comparisons_across_types_follow_jq_order() {
        for (query, expected) in [
            (r#"1 >= "a""#, "false"),
            (r#"1 < "a""#, "true"),
            ("null <= 1", "true"),
            ("null > 1", "false"),
            (r#""b" > "a""#, "true"),
            ("[1, 2] < [1, 3]", "true"),
        ] {
            assert_eq!(eval("# a", query)[0].to_text(), expected, "{query}");
        }
    }

    /// Only a block opening the document is frontmatter. A `key: value`
    /// block between two rules further down was reported as frontmatter,
    /// over the real one, and a byte order mark hid the real one.
    #[test]
    fn frontmatter_is_only_the_block_that_opens_the_document() {
        let keys = |md: &str| -> Vec<String> {
            eval(md, ".fm | keys")
                .iter()
                .flat_map(|v| match v {
                    Value::Array(items) => items.iter().map(|v| v.to_text()).collect(),
                    _ => Vec::new(),
                })
                .collect()
        };
        assert!(keys("# H\n\n---\ny: 2\n---\n\n## Two\n").is_empty());
        assert_eq!(keys("---\nx: 1\n---\n# H\n\n---\ny: 2\n---\n"), ["x"]);
        assert_eq!(keys("\u{feff}---\nx: 1\n---\n# H\n"), ["x"]);
        assert_eq!(keys("---\nx: 1\n...\n# H\n"), ["x"]);
    }

    /// Aliases repeat their anchors, so a small block can expand to
    /// gigabytes. Past the limit it is not parsed.
    #[test]
    fn frontmatter_with_too_many_aliases_is_not_parsed() {
        let bomb: String = std::iter::once("---\np: &p [1, 2, 3]\n".to_string())
            .chain((0..100).map(|i| format!("x{i}: &x{i} [*p, *p, *p]\n")))
            .chain(std::iter::once("---\n# H\n".to_string()))
            .collect();
        assert!(eval(&bomb, ".fm").is_empty());
        assert_eq!(
            yaml_alias_count("a: *x\nb: [*y, *z]\nc: \"5 * 3\"\nd: **bold**\n"),
            3
        );
    }

    #[test]
    fn test_frontmatter_is_treated_as_a_map_by_builtins() {
        let md = "---\ntitle: Doc\ndraft: false\n---\n\n# T";

        let keys = eval(md, ".frontmatter | keys");
        assert_eq!(keys.len(), 1);
        if let Value::Array(items) = &keys[0] {
            let names: Vec<String> = items.iter().map(|v| v.to_text()).collect();
            assert!(names.contains(&"title".to_string()));
            assert!(names.contains(&"draft".to_string()));
        } else {
            panic!("Expected Array of keys");
        }

        assert_eq!(
            eval(md, r#".frontmatter | has("title")"#)[0].to_text(),
            "true"
        );
        assert_eq!(
            eval(md, r#".frontmatter | has("nope")"#)[0].to_text(),
            "false"
        );
        assert_eq!(eval(md, ".frontmatter | count")[0].to_text(), "2");
        assert_eq!(eval(md, ".frontmatter | empty")[0].to_text(), "false");
    }

    #[test]
    fn test_object_literals_still_behave_as_maps() {
        let md = "# T";

        assert_eq!(eval(md, r#"{a: 1} | has("a")"#)[0].to_text(), "true");
        assert_eq!(eval(md, "{a: 1, b: 2} | count")[0].to_text(), "2");
    }
}
