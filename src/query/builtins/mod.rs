//! Built-in functions for the query language.
//!
//! This module provides all the standard functions available in queries.

mod collection;
mod string;

use super::error::QueryError;
use super::eval::EvalContext;
use super::registry::{Function, Registry};
use super::value::Value;

/// Register all built-in functions.
pub fn register_all(registry: &mut Registry) {
    // Collection functions
    registry.register_function("count", Function::new(fn_count, 0..=0));
    registry.register_function("length", Function::new(fn_count, 0..=0));
    registry.register_function("first", Function::new(fn_first, 0..=0));
    registry.register_function("last", Function::new(fn_last, 0..=0));
    registry.register_function("reverse", Function::new(fn_reverse, 0..=0));
    registry.register_function("sort", Function::new(fn_sort, 0..=0));
    registry.register_function("unique", Function::new(fn_unique, 0..=0));
    registry.register_function("flatten", Function::new(fn_flatten, 0..=0));
    registry.register_function("keys", Function::new(fn_keys, 0..=0));
    registry.register_function("values", Function::new(fn_values, 0..=0));
    registry.register_function("empty", Function::new(fn_empty, 0..=0));

    // String functions
    registry.register_function("text", Function::new(fn_text, 0..=0));
    registry.register_function("upper", Function::new(fn_upper, 0..=0));
    registry.register_function("lower", Function::new(fn_lower, 0..=0));
    registry.register_function("trim", Function::new(fn_trim, 0..=0));
    registry.register_function("split", Function::new(fn_split, 1..=1));
    registry.register_function("join", Function::new(fn_join, 1..=1));
    registry.register_function("replace", Function::new(fn_replace, 2..=2));
    registry.register_function("lines", Function::new(fn_lines, 0..=0));
    registry.register_function("words", Function::new(fn_words, 0..=0));
    registry.register_function("chars", Function::new(fn_chars, 0..=0));
    registry.register_function("slugify", Function::new(fn_slugify, 0..=0));

    // Boolean/filter functions
    registry.register_function(
        "select",
        Function::new(fn_select, 1..=1).with_takes_input(true),
    );
    registry.register_function("contains", Function::new(fn_contains, 1..=1));
    registry.register_function("startswith", Function::new(fn_startswith, 1..=1));
    registry.register_function("endswith", Function::new(fn_endswith, 1..=1));
    registry.register_function("matches", Function::new(fn_matches, 1..=1));
    registry.register_function("has", Function::new(fn_has, 1..=1));
    registry.register_function("type", Function::new(fn_type, 0..=0));

    // Content functions
    registry.register_function("content", Function::new(fn_content, 0..=0));
    registry.register_function("md", Function::new(fn_md, 0..=0));
    registry.register_function("url", Function::new(fn_url, 0..=0));
    registry.register_function("lang", Function::new(fn_lang, 0..=0));

    // Aggregation functions
    registry.register_function("stats", Function::new(fn_stats, 0..=0));
    registry.register_function("levels", Function::new(fn_levels, 0..=0));
    registry.register_function("langs", Function::new(fn_langs, 0..=0));
    registry.register_function("types", Function::new(fn_types, 0..=0));

    // Utility functions
    registry.register_function("limit", Function::new(fn_limit, 1..=1));
    registry.register_function("skip", Function::new(fn_skip, 1..=1));
    registry.register_function("nth", Function::new(fn_nth, 1..=1));
    registry.register_function("any", Function::new(fn_any, 1..=1).with_takes_input(true));
    registry.register_function("all", Function::new(fn_all, 1..=1).with_takes_input(true));
    registry.register_function("min", Function::new(fn_min, 0..=0));
    registry.register_function("max", Function::new(fn_max, 0..=0));
    registry.register_function("add", Function::new(fn_add, 0..=0));
    registry.register_function("not", Function::new(fn_not, 0..=0));
    registry.register_function("null", Function::new(fn_null, 0..=0));
    registry.register_function("debug", Function::new(fn_debug, 0..=0));
    registry.register_function("group_by", Function::new(fn_group_by, 1..=1));
    registry.register_function("sort_by", Function::new(fn_sort_by, 1..=1));

    // Aliases - comprehensive for discoverability
    // Length/count
    registry.register_alias("len", "length");
    registry.register_alias("size", "length");

    // Filter/select (multiple conventions)
    registry.register_alias("where", "select");
    registry.register_alias("filter", "select");

    // String contains (JavaScript style)
    registry.register_alias("includes", "contains");

    // Underscore variants (Rust/Python style)
    registry.register_alias("starts_with", "startswith");
    registry.register_alias("ends_with", "endswith");
    registry.register_alias("group", "group_by");

    // First/last alternatives (FP style)
    registry.register_alias("head", "first");
    registry.register_alias("take", "limit");
    registry.register_alias("drop", "skip");

    // jq compatibility
    registry.register_alias("ascii_downcase", "lower");
    registry.register_alias("ascii_upcase", "upper");

    // Content extraction
    registry.register_alias("markdown", "md");
    registry.register_alias("language", "lang");
    registry.register_alias("src", "url");
    registry.register_alias("href", "url");
}

// ============================================================================
// Collection functions
// ============================================================================

fn fn_count(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let count = match input {
        Value::Array(a) => a.len(),
        Value::String(s) => s.len(),
        Value::Object(o) => o.len(),
        _ => 1,
    };
    Ok(vec![Value::Number(count as f64)])
}

fn fn_first(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => Ok(a.first().cloned().map(|v| vec![v]).unwrap_or_default()),
        Value::String(s) => Ok(s
            .chars()
            .next()
            .map(|c| vec![Value::String(c.to_string())])
            .unwrap_or_default()),
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_last(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => Ok(a.last().cloned().map(|v| vec![v]).unwrap_or_default()),
        Value::String(s) => Ok(s
            .chars()
            .last()
            .map(|c| vec![Value::String(c.to_string())])
            .unwrap_or_default()),
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_reverse(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            let mut reversed = a.clone();
            reversed.reverse();
            Ok(vec![Value::Array(reversed)])
        }
        Value::String(s) => Ok(vec![Value::String(s.chars().rev().collect())]),
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_sort(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            let mut sorted = a.clone();
            sorted.sort_by_key(|a| a.to_text());
            Ok(vec![Value::Array(sorted)])
        }
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_unique(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            let mut seen = std::collections::HashSet::new();
            let unique: Vec<Value> = a
                .iter()
                .filter(|v| seen.insert(v.to_text()))
                .cloned()
                .collect();
            Ok(vec![Value::Array(unique)])
        }
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_flatten(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            let mut flat = Vec::new();
            for item in a {
                if let Value::Array(inner) = item {
                    flat.extend(inner.clone());
                } else {
                    flat.push(item.clone());
                }
            }
            Ok(vec![Value::Array(flat)])
        }
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_keys(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Object(o) => {
            let keys: Vec<Value> = o.keys().map(|k| Value::String(k.clone())).collect();
            Ok(vec![Value::Array(keys)])
        }
        Value::Array(a) => {
            let keys: Vec<Value> = (0..a.len()).map(|i| Value::Number(i as f64)).collect();
            Ok(vec![Value::Array(keys)])
        }
        _ => Ok(vec![Value::Array(vec![])]),
    }
}

fn fn_values(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Object(o) => {
            let values: Vec<Value> = o.values().cloned().collect();
            Ok(vec![Value::Array(values)])
        }
        Value::Array(a) => Ok(vec![Value::Array(a.clone())]),
        _ => Ok(vec![Value::Array(vec![])]),
    }
}

fn fn_empty(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let is_empty = match input {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    };
    Ok(vec![Value::Bool(is_empty)])
}

// ============================================================================
// String functions
// ============================================================================

fn fn_text(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    Ok(vec![Value::String(input.to_text())])
}

fn fn_upper(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    Ok(vec![Value::String(input.to_text().to_uppercase())])
}

fn fn_lower(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    Ok(vec![Value::String(input.to_text().to_lowercase())])
}

fn fn_trim(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    Ok(vec![Value::String(input.to_text().trim().to_string())])
}

fn fn_split(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let sep = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    let parts: Vec<Value> = input
        .to_text()
        .split(&sep)
        .map(|s| Value::String(s.to_string()))
        .collect();
    Ok(vec![Value::Array(parts)])
}

fn fn_join(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let sep = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    let result = match input {
        Value::Array(a) => a.iter().map(|v| v.to_text()).collect::<Vec<_>>().join(&sep),
        _ => input.to_text(),
    };
    Ok(vec![Value::String(result)])
}

fn fn_replace(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let from = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    let to = args.get(2).map(|v| v.to_text()).unwrap_or_default();
    Ok(vec![Value::String(input.to_text().replace(&from, &to))])
}

fn fn_lines(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let count = input.to_text().lines().count();
    Ok(vec![Value::Number(count as f64)])
}

fn fn_words(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let count = input.to_text().split_whitespace().count();
    Ok(vec![Value::Number(count as f64)])
}

fn fn_chars(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let count = input.to_text().chars().count();
    Ok(vec![Value::Number(count as f64)])
}

fn fn_slugify(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let text = input.to_text();
    let slug = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' {
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
        .join("-");
    Ok(vec![Value::String(slug)])
}

// ============================================================================
// Boolean/filter functions
// ============================================================================

fn fn_select(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    // args[0] is the input, args[1] is the condition result
    let input = args.first().unwrap_or(&Value::Null);
    let condition = args.get(1).unwrap_or(&Value::Bool(false));

    if condition.is_truthy() {
        Ok(vec![input.clone()])
    } else {
        Ok(vec![])
    }
}

fn fn_contains(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let pattern = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    let result = input
        .to_text()
        .to_lowercase()
        .contains(&pattern.to_lowercase());
    Ok(vec![Value::Bool(result)])
}

fn fn_startswith(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let pattern = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    let result = input
        .to_text()
        .to_lowercase()
        .starts_with(&pattern.to_lowercase());
    Ok(vec![Value::Bool(result)])
}

fn fn_endswith(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let pattern = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    let result = input
        .to_text()
        .to_lowercase()
        .ends_with(&pattern.to_lowercase());
    Ok(vec![Value::Bool(result)])
}

fn fn_matches(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let pattern = args.get(1).map(|v| v.to_text()).unwrap_or_default();

    let result = regex::Regex::new(&pattern)
        .map(|re| re.is_match(&input.to_text()))
        .unwrap_or(false);

    Ok(vec![Value::Bool(result)])
}

fn fn_has(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let key = args.get(1).map(|v| v.to_text()).unwrap_or_default();

    let result = match input {
        Value::Object(o) => o.contains_key(&key),
        _ => false,
    };

    Ok(vec![Value::Bool(result)])
}

fn fn_type(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    Ok(vec![Value::String(input.kind().to_string())])
}

// ============================================================================
// Content functions
// ============================================================================

fn fn_content(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Heading(h) => Ok(vec![Value::String(h.content.clone())]),
        _ => Ok(vec![Value::String(input.to_text())]),
    }
}

fn fn_md(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Heading(h) => Ok(vec![Value::String(h.raw_md.clone())]),
        _ => Ok(vec![Value::String(input.to_text())]),
    }
}

fn fn_url(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Link(l) => Ok(vec![Value::String(l.url.clone())]),
        Value::Image(i) => Ok(vec![Value::String(i.src.clone())]),
        _ => Ok(vec![Value::Null]),
    }
}

fn fn_lang(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Code(c) => Ok(vec![
            c.language.clone().map(Value::String).unwrap_or(Value::Null),
        ]),
        _ => Ok(vec![Value::Null]),
    }
}

// ============================================================================
// Aggregation functions
// ============================================================================

fn fn_stats(args: &[Value], ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let _ = args;
    let mut obj = indexmap::IndexMap::new();
    obj.insert(
        "headings".to_string(),
        Value::Number(ctx.headings.len() as f64),
    );
    obj.insert(
        "code_blocks".to_string(),
        Value::Number(ctx.code_blocks.len() as f64),
    );
    obj.insert("links".to_string(), Value::Number(ctx.links.len() as f64));
    obj.insert("images".to_string(), Value::Number(ctx.images.len() as f64));
    obj.insert("tables".to_string(), Value::Number(ctx.tables.len() as f64));
    obj.insert("lists".to_string(), Value::Number(ctx.lists.len() as f64));
    obj.insert(
        "words".to_string(),
        Value::Number(ctx.document.word_count as f64),
    );
    Ok(vec![Value::Object(obj)])
}

fn fn_levels(args: &[Value], ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let _ = args;
    let mut counts = std::collections::HashMap::new();
    for h in &ctx.headings {
        *counts.entry(h.level).or_insert(0) += 1;
    }

    let mut obj = indexmap::IndexMap::new();
    for level in 1..=6u8 {
        if let Some(count) = counts.get(&level) {
            obj.insert(format!("h{}", level), Value::Number(*count as f64));
        }
    }
    Ok(vec![Value::Object(obj)])
}

fn fn_langs(args: &[Value], ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let _ = args;
    let mut counts = std::collections::HashMap::new();
    for code in &ctx.code_blocks {
        let lang = code.language.as_deref().unwrap_or("none");
        *counts.entry(lang.to_string()).or_insert(0) += 1;
    }

    let mut obj = indexmap::IndexMap::new();
    for (lang, count) in counts {
        obj.insert(lang, Value::Number(count as f64));
    }
    Ok(vec![Value::Object(obj)])
}

fn fn_types(args: &[Value], ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let _ = args;
    let mut counts = std::collections::HashMap::new();
    for link in &ctx.links {
        *counts
            .entry(link.link_type.as_str().to_string())
            .or_insert(0) += 1;
    }

    let mut obj = indexmap::IndexMap::new();
    for (typ, count) in counts {
        obj.insert(typ, Value::Number(count as f64));
    }
    Ok(vec![Value::Object(obj)])
}

// ============================================================================
// Utility functions
// ============================================================================

fn fn_limit(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let n = args
        .get(1)
        .and_then(|v| {
            if let Value::Number(n) = v {
                Some(*n as usize)
            } else {
                None
            }
        })
        .unwrap_or(0);

    match input {
        Value::Array(a) => Ok(vec![Value::Array(a.iter().take(n).cloned().collect())]),
        Value::String(s) => Ok(vec![Value::String(s.chars().take(n).collect())]),
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_skip(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let n = args
        .get(1)
        .and_then(|v| {
            if let Value::Number(n) = v {
                Some(*n as usize)
            } else {
                None
            }
        })
        .unwrap_or(0);

    match input {
        Value::Array(a) => Ok(vec![Value::Array(a.iter().skip(n).cloned().collect())]),
        Value::String(s) => Ok(vec![Value::String(s.chars().skip(n).collect())]),
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_nth(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let n = args
        .get(1)
        .and_then(|v| {
            if let Value::Number(n) = v {
                Some(*n as i64)
            } else {
                None
            }
        })
        .unwrap_or(0);

    match input {
        Value::Array(a) => {
            let idx = if n < 0 {
                a.len().saturating_sub((-n) as usize)
            } else {
                n as usize
            };
            Ok(a.get(idx).cloned().map(|v| vec![v]).unwrap_or_default())
        }
        Value::String(s) => {
            let idx = if n < 0 {
                s.chars().count().saturating_sub((-n) as usize)
            } else {
                n as usize
            };
            Ok(s.chars()
                .nth(idx)
                .map(|c| vec![Value::String(c.to_string())])
                .unwrap_or_default())
        }
        _ => Ok(vec![]),
    }
}

fn fn_any(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let condition = args.get(1).unwrap_or(&Value::Bool(false));

    let result = match input {
        Value::Array(a) => a.iter().any(|v| {
            // If condition is a function result applied to each element
            // For now, check if any element is truthy
            v.is_truthy()
        }),
        _ => condition.is_truthy(),
    };
    Ok(vec![Value::Bool(result)])
}

fn fn_all(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let _condition = args.get(1).unwrap_or(&Value::Bool(true));

    let result = match input {
        Value::Array(a) => a.iter().all(|v| v.is_truthy()),
        _ => input.is_truthy(),
    };
    Ok(vec![Value::Bool(result)])
}

fn fn_min(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            let min = a
                .iter()
                .filter_map(|v| {
                    if let Value::Number(n) = v {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .fold(f64::INFINITY, f64::min);
            if min.is_infinite() {
                Ok(vec![Value::Null])
            } else {
                Ok(vec![Value::Number(min)])
            }
        }
        Value::Number(n) => Ok(vec![Value::Number(*n)]),
        _ => Ok(vec![Value::Null]),
    }
}

fn fn_max(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            let max = a
                .iter()
                .filter_map(|v| {
                    if let Value::Number(n) = v {
                        Some(*n)
                    } else {
                        None
                    }
                })
                .fold(f64::NEG_INFINITY, f64::max);
            if max.is_infinite() {
                Ok(vec![Value::Null])
            } else {
                Ok(vec![Value::Number(max)])
            }
        }
        Value::Number(n) => Ok(vec![Value::Number(*n)]),
        _ => Ok(vec![Value::Null]),
    }
}

fn fn_add(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    match input {
        Value::Array(a) => {
            // Check if all are numbers
            let all_numbers = a.iter().all(|v| matches!(v, Value::Number(_)));
            if all_numbers {
                let sum: f64 = a
                    .iter()
                    .filter_map(|v| {
                        if let Value::Number(n) = v {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .sum();
                Ok(vec![Value::Number(sum)])
            } else {
                // Concatenate as strings
                let concat: String = a.iter().map(|v| v.to_text()).collect();
                Ok(vec![Value::String(concat)])
            }
        }
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_not(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    Ok(vec![Value::Bool(!input.is_truthy())])
}

fn fn_null(_args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    Ok(vec![Value::Null])
}

fn fn_debug(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    // Print to stderr for debugging
    eprintln!("[DEBUG] {:?}", input);
    Ok(vec![input.clone()])
}

fn fn_group_by(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let key_name = args
        .get(1)
        .map(|v| v.to_text())
        .unwrap_or_else(|| "key".to_string());

    match input {
        Value::Array(a) => {
            let mut groups: indexmap::IndexMap<String, Vec<Value>> = indexmap::IndexMap::new();
            for item in a {
                let key = match item {
                    Value::Object(o) => o.get(&key_name).map(|v| v.to_text()).unwrap_or_default(),
                    Value::Heading(h) => format!("h{}", h.level),
                    Value::Code(c) => c.language.clone().unwrap_or_else(|| "none".to_string()),
                    _ => item.to_text(),
                };
                groups.entry(key).or_default().push(item.clone());
            }

            let result: indexmap::IndexMap<String, Value> = groups
                .into_iter()
                .map(|(k, v)| (k, Value::Array(v)))
                .collect();
            Ok(vec![Value::Object(result)])
        }
        _ => Ok(vec![input.clone()]),
    }
}

fn fn_sort_by(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
    let input = args.first().unwrap_or(&Value::Null);
    let key_name = args
        .get(1)
        .map(|v| v.to_text())
        .unwrap_or_else(|| "key".to_string());

    match input {
        Value::Array(a) => {
            let mut sorted = a.clone();
            sorted.sort_by(|a, b| {
                let a_key = match a {
                    Value::Object(o) => o.get(&key_name).map(|v| v.to_text()).unwrap_or_default(),
                    Value::Heading(h) if key_name == "level" => h.level.to_string(),
                    Value::Heading(h) if key_name == "text" => h.text.clone(),
                    _ => a.to_text(),
                };
                let b_key = match b {
                    Value::Object(o) => o.get(&key_name).map(|v| v.to_text()).unwrap_or_default(),
                    Value::Heading(h) if key_name == "level" => h.level.to_string(),
                    Value::Heading(h) if key_name == "text" => h.text.clone(),
                    _ => b.to_text(),
                };
                a_key.cmp(&b_key)
            });
            Ok(vec![Value::Array(sorted)])
        }
        _ => Ok(vec![input.clone()]),
    }
}
