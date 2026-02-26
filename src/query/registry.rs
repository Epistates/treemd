//! Plugin registry for functions and element extractors.
//!
//! The registry system provides extensibility points for:
//! - Custom functions (e.g., `my_fn(...)`)
//! - Element extractors (e.g., custom `.my_element` selectors)
//! - Output formatters
//!
//! # Example: Registering a Custom Function
//!
//! ```rust
//! use treemd::query::{Registry, Function, Value, EvalContext, QueryError};
//!
//! fn my_uppercase(args: &[Value], _ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
//!     let input = args.first().map(|v| v.to_text()).unwrap_or_default();
//!     Ok(vec![Value::String(input.to_uppercase())])
//! }
//!
//! let mut registry = Registry::with_builtins();
//! registry.register_function("my_upper", Function::new(my_uppercase, 0..=1));
//! ```

use crate::parser::Document;
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::Arc;

use super::error::QueryError;
use super::eval::EvalContext;
use super::value::Value;

/// Type alias for function implementations.
///
/// Functions receive:
/// - `args`: The evaluated arguments passed to the function
/// - `ctx`: The evaluation context (provides access to current value, document, etc.)
///
/// Returns a vector of values (functions can produce multiple outputs).
pub type FunctionFn =
    Arc<dyn Fn(&[Value], &EvalContext) -> Result<Vec<Value>, QueryError> + Send + Sync>;

/// Type alias for element extractor implementations.
///
/// Extractors receive:
/// - `doc`: The document to extract from
/// - `ctx`: The evaluation context
///
/// Returns all matching elements as values.
pub type ExtractorFn =
    Arc<dyn Fn(&Document, &EvalContext) -> Result<Vec<Value>, QueryError> + Send + Sync>;

/// A registered function with metadata.
#[derive(Clone)]
pub struct Function {
    /// The function implementation
    pub func: FunctionFn,
    /// Valid range of argument counts
    pub arity: RangeInclusive<usize>,
    /// Function description for help
    pub description: String,
    /// Whether this function consumes the current value as first arg
    pub takes_input: bool,
}

impl Function {
    /// Create a new function with the given implementation and arity.
    pub fn new<F>(func: F, arity: RangeInclusive<usize>) -> Self
    where
        F: Fn(&[Value], &EvalContext) -> Result<Vec<Value>, QueryError> + Send + Sync + 'static,
    {
        Self {
            func: Arc::new(func),
            arity,
            description: String::new(),
            takes_input: true,
        }
    }

    /// Set the function description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set whether this function takes the current value as input.
    pub fn with_takes_input(mut self, takes: bool) -> Self {
        self.takes_input = takes;
        self
    }

    /// Check if the given argument count is valid.
    pub fn accepts_arity(&self, count: usize) -> bool {
        self.arity.contains(&count)
    }

    /// Call the function with arguments.
    pub fn call(&self, args: &[Value], ctx: &EvalContext) -> Result<Vec<Value>, QueryError> {
        (self.func)(args, ctx)
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function")
            .field("arity", &self.arity)
            .field("description", &self.description)
            .field("takes_input", &self.takes_input)
            .finish()
    }
}

/// Registry for functions, extractors, and other extensibility points.
#[derive(Default)]
pub struct Registry {
    functions: HashMap<String, Function>,
    extractors: HashMap<String, ExtractorFn>,
    aliases: HashMap<String, String>,
}

impl Registry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all built-in functions registered.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        super::builtins::register_all(&mut registry);
        registry
    }

    /// Register a function.
    pub fn register_function(&mut self, name: impl Into<String>, func: Function) {
        self.functions.insert(name.into(), func);
    }

    /// Register a function alias.
    pub fn register_alias(&mut self, alias: impl Into<String>, target: impl Into<String>) {
        self.aliases.insert(alias.into(), target.into());
    }

    /// Get a function by name (resolving aliases).
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        // First try direct lookup
        if let Some(func) = self.functions.get(name) {
            return Some(func);
        }

        // Try alias resolution
        if let Some(target) = self.aliases.get(name) {
            return self.functions.get(target);
        }

        None
    }

    /// Check if a function exists.
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name) || self.aliases.contains_key(name)
    }

    /// Get all function names (for suggestions/completions).
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }

    /// Find similar function names for error suggestions.
    pub fn suggest_function(&self, name: &str) -> Vec<&str> {
        let name_lower = name.to_lowercase();
        let mut suggestions: Vec<_> = self
            .functions
            .keys()
            .filter(|n| {
                let n_lower = n.to_lowercase();
                // Simple similarity: starts with, contains, or edit distance <= 2
                n_lower.starts_with(&name_lower)
                    || name_lower.starts_with(&n_lower)
                    || n_lower.contains(&name_lower)
                    || name_lower.contains(&n_lower)
                    || levenshtein(&n_lower, &name_lower) <= 2
            })
            .map(|s| s.as_str())
            .collect();

        suggestions.sort_by_key(|s| levenshtein(&s.to_lowercase(), &name_lower));
        suggestions.truncate(3);
        suggestions
    }

    /// Register an element extractor.
    pub fn register_extractor(&mut self, name: impl Into<String>, extractor: ExtractorFn) {
        self.extractors.insert(name.into(), extractor);
    }

    /// Get an element extractor by name.
    pub fn get_extractor(&self, name: &str) -> Option<&ExtractorFn> {
        self.extractors.get(name)
    }
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("functions", &self.functions.keys().collect::<Vec<_>>())
            .field("extractors", &self.extractors.keys().collect::<Vec<_>>())
            .field("aliases", &self.aliases)
            .finish()
    }
}

/// Function registry trait for extensibility.
///
/// Implement this trait to create modules of related functions.
pub trait FunctionRegistry {
    /// Register all functions from this module into the registry.
    fn register(registry: &mut Registry);
}

/// Simple Levenshtein distance for suggestions.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, item) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *item = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        assert_eq!(levenshtein("abc", "ab"), 1);
        assert_eq!(levenshtein("abc", "abd"), 1);
        assert_eq!(levenshtein("abc", "xyz"), 3);
        assert_eq!(levenshtein("count", "conut"), 2); // transposition
    }

    #[test]
    fn test_registry_functions() {
        let mut registry = Registry::new();

        let test_fn = Function::new(|_args, _ctx| Ok(vec![Value::String("test".into())]), 0..=0);

        registry.register_function("test", test_fn);
        assert!(registry.has_function("test"));
        assert!(!registry.has_function("nonexistent"));
    }

    #[test]
    fn test_registry_aliases() {
        let mut registry = Registry::new();

        let count_fn = Function::new(|_args, _ctx| Ok(vec![Value::Number(0.0)]), 0..=0);

        registry.register_function("count", count_fn);
        registry.register_alias("length", "count");

        assert!(registry.has_function("count"));
        assert!(registry.has_function("length"));
        assert!(registry.get_function("length").is_some());
    }

    #[test]
    fn test_suggest_function() {
        let mut registry = Registry::new();
        registry.register_function("contains", Function::new(|_, _| Ok(vec![]), 1..=1));
        registry.register_function("count", Function::new(|_, _| Ok(vec![]), 0..=0));
        registry.register_function("startswith", Function::new(|_, _| Ok(vec![]), 1..=1));

        let suggestions = registry.suggest_function("cont");
        assert!(suggestions.contains(&"contains"));
        assert!(suggestions.contains(&"count"));
    }
}
