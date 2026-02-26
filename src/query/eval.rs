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
    /// Document metadata
    pub document: DocumentValue,
    /// Raw document content
    pub raw_content: String,
}

impl EvalContext {
    /// Create a new context from a document.
    pub fn from_document(doc: &Document) -> Self {
        let headings = extract_headings(doc);
        let (code_blocks, links, images, tables, lists) = extract_blocks(doc);

        let document = DocumentValue {
            content: doc.content.clone(),
            heading_count: doc.headings.len(),
            word_count: doc.content.split_whitespace().count(),
        };

        Self {
            current: Value::Document(document.clone()),
            headings,
            code_blocks,
            links,
            images,
            tables,
            lists,
            document,
            raw_content: doc.content.clone(),
        }
    }
}

/// Query execution engine.
pub struct Engine<'a> {
    #[allow(dead_code)] // Reserved for future use with document-level operations
    doc: &'a Document,
    registry: Arc<Registry>,
    context: EvalContext,
}

impl<'a> Engine<'a> {
    /// Create a new engine with default registry.
    pub fn new(doc: &'a Document) -> Self {
        Self::with_registry(doc, Registry::with_builtins())
    }

    /// Create a new engine with a custom registry.
    pub fn with_registry(doc: &'a Document, registry: Registry) -> Self {
        let context = EvalContext::from_document(doc);
        Self {
            doc,
            registry: Arc::new(registry),
            context,
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

        for stage in &piped.stages {
            let mut next = Vec::new();
            for input in current {
                self.context.current = input;
                next.extend(self.eval_expr(stage)?);
            }
            current = next;

            // Short-circuit if no results
            if current.is_empty() {
                break;
            }
        }

        Ok(current)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Vec<Value>, QueryError> {
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
            ElementKind::Blockquote => {
                // TODO: extract blockquotes
                Vec::new()
            }
            ElementKind::Paragraph => {
                // TODO: extract paragraphs
                Vec::new()
            }
            ElementKind::FrontMatter => {
                // TODO: parse front matter
                Vec::new()
            }
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
            Filter::Regex { pattern, span } => {
                let re = regex::Regex::new(pattern).map_err(|e| {
                    QueryError::new(
                        QueryErrorKind::InvalidRegex {
                            pattern: pattern.clone(),
                            error: e.to_string(),
                        },
                        *span,
                        String::new(),
                    )
                })?;
                Ok(elements
                    .into_iter()
                    .filter(|v| re.is_match(&v.to_text()))
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
                // Internal pipe handling
                let mut current = vec![self.context.current.clone()];
                for arg in args {
                    let mut next = Vec::new();
                    for input in current {
                        self.context.current = input;
                        next.extend(self.eval_expr(arg)?);
                    }
                    current = next;
                }
                return Ok(current);
            }
            "_index" => {
                // Internal index handling
                if args.len() >= 2 {
                    let values = self.eval_expr(&args[0])?;
                    // Simplified - just return values for now
                    return Ok(values);
                }
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
                            // Find code blocks under this heading
                            // For now, return all code blocks (simplified)
                            // TODO: Implement proper scoping
                            results
                                .extend(self.context.code_blocks.iter().cloned().map(Value::Code));
                        }
                        _ => {
                            // Other element types under headings
                        }
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
            BinaryOp::Mul => mul_values(&left_val, &right_val),
            BinaryOp::Div => div_values(&left_val, &right_val)?,
            BinaryOp::Mod => mod_values(&left_val, &right_val)?,
            BinaryOp::Concat => concat_values(&left_val, &right_val),
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

            // Extract content (simplified - until next heading)
            let content_start = doc.content[h.offset..]
                .find('\n')
                .map(|i| h.offset + i + 1)
                .unwrap_or(h.offset);

            let content_end = doc
                .headings
                .iter()
                .skip(idx + 1)
                .find(|next_h| next_h.level <= h.level)
                .map(|next_h| next_h.offset)
                .unwrap_or(doc.content.len());

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

#[allow(clippy::type_complexity)]
fn extract_blocks(
    doc: &Document,
) -> (
    Vec<CodeValue>,
    Vec<LinkValue>,
    Vec<ImageValue>,
    Vec<TableValue>,
    Vec<ListValue>,
) {
    use crate::parser::content::parse_content;
    use crate::parser::links::extract_links;
    use crate::parser::output::Block;

    let blocks = parse_content(&doc.content, 1);
    let links = extract_links(&doc.content);

    let mut code_blocks = Vec::new();
    let mut images = Vec::new();
    let mut tables = Vec::new();
    let mut lists = Vec::new();

    // Recursively extract blocks from nested structures (e.g., list items)
    fn extract_nested_blocks(
        blocks: &[Block],
        code_blocks: &mut Vec<CodeValue>,
        images: &mut Vec<ImageValue>,
        tables: &mut Vec<TableValue>,
    ) {
        for block in blocks {
            match block {
                Block::Code {
                    language,
                    content,
                    start_line,
                    end_line,
                } => {
                    code_blocks.push(CodeValue {
                        language: language.clone(),
                        content: content.clone(),
                        start_line: *start_line,
                        end_line: *end_line,
                    });
                }
                Block::Image { alt, src, title } => {
                    images.push(ImageValue {
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
                    tables.push(TableValue {
                        headers: headers.clone(),
                        rows: rows.clone(),
                        alignments: alignments
                            .iter()
                            .map(|a| format!("{:?}", a).to_lowercase())
                            .collect(),
                    });
                }
                Block::Blockquote { blocks, .. } => {
                    // Recursively extract from blockquote content
                    extract_nested_blocks(blocks, code_blocks, images, tables);
                }
                Block::Details { blocks, .. } => {
                    // Recursively extract from details content
                    extract_nested_blocks(blocks, code_blocks, images, tables);
                }
                _ => {}
            }
        }
    }

    for block in blocks {
        match block {
            Block::Code {
                language,
                content,
                start_line,
                end_line,
            } => {
                code_blocks.push(CodeValue {
                    language,
                    content,
                    start_line,
                    end_line,
                });
            }
            Block::Image { alt, src, title } => {
                images.push(ImageValue { alt, src, title });
            }
            Block::Table {
                headers,
                rows,
                alignments,
            } => {
                tables.push(TableValue {
                    headers,
                    rows,
                    alignments: alignments
                        .iter()
                        .map(|a| format!("{:?}", a).to_lowercase())
                        .collect(),
                });
            }
            Block::List { ordered, items } => {
                // Extract code blocks and other elements from list item nested blocks
                for item in &items {
                    extract_nested_blocks(&item.blocks, &mut code_blocks, &mut images, &mut tables);
                }

                lists.push(ListValue {
                    ordered,
                    items: items
                        .into_iter()
                        .map(|i| ListItemValue {
                            content: i.content,
                            checked: i.checked,
                        })
                        .collect(),
                });
            }
            Block::Blockquote { blocks, .. } => {
                // Recursively extract from blockquote content
                extract_nested_blocks(&blocks, &mut code_blocks, &mut images, &mut tables);
            }
            Block::Details { blocks, .. } => {
                // Recursively extract from details content
                extract_nested_blocks(&blocks, &mut code_blocks, &mut images, &mut tables);
            }
            _ => {}
        }
    }

    let link_values: Vec<LinkValue> = links
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

    (code_blocks, link_values, images, tables, lists)
}

fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::String(s) => Value::String(s.clone()),
        Literal::Number(n) => Value::Number(*n),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Null => Value::Null,
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

fn mul_values(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
        (Value::String(s), Value::Number(n)) | (Value::Number(n), Value::String(s)) => {
            Value::String(s.repeat(*n as usize))
        }
        _ => Value::Null,
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

fn concat_values(a: &Value, b: &Value) -> Value {
    Value::String(format!("{}{}", a.to_text(), b.to_text()))
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
