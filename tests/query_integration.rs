//! Integration tests for the public query API (`query::execute`) and
//! `parse_markdown`, exercising the correctness fixes end-to-end.

use treemd::parse_markdown;
use treemd::query::{self, Value};

/// Run a query and return the result values' plain-text representations.
fn run(md: &str, q: &str) -> Vec<String> {
    let doc = parse_markdown(md);
    query::execute(&doc, q)
        .unwrap_or_else(|e| panic!("query {q:?} failed: {e}"))
        .iter()
        .map(|v| v.to_text())
        .collect()
}

/// Run a query expecting it to error.
fn run_err(md: &str, q: &str) {
    let doc = parse_markdown(md);
    assert!(
        query::execute(&doc, q).is_err(),
        "expected query {q:?} to error"
    );
}

// ---------------------------------------------------------------------------
// Parser consolidation (item 1)
// ---------------------------------------------------------------------------

#[test]
fn crlf_section_extraction_excludes_heading_and_underline() {
    // CRLF file with a multibyte char — must not panic and must exclude the
    // heading lines from the body.
    let md = "# Top\r\naaa\r\nccé\r\n# Next\r\n";
    let out = run(md, ".h[Top] | .content");
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("aaa"));
    assert!(out[0].contains("ccé"));
    assert!(!out[0].contains("# Top"));
    assert!(!out[0].contains("# Next"));
}

#[test]
fn setext_content_excludes_underline() {
    let md = "Title\n=====\nbody\n\nSub\n-----\nsub body\n";
    let out = run(md, ".h2 | .content");
    assert_eq!(out, vec!["sub body".to_string()]);
}

#[test]
fn eof_solo_heading_has_empty_content() {
    let md = "# First\nbody\n\n# Solo";
    let out = run(md, ".h[Solo] | .content");
    assert_eq!(out, vec![String::new()]);
}

// ---------------------------------------------------------------------------
// Property chaining (item 4a)
// ---------------------------------------------------------------------------

#[test]
fn property_chain_applies_to_each_element() {
    let md = "# A\ntext\n## Sub\nx\n";
    assert_eq!(run(md, ".h2.text"), vec!["Sub".to_string()]);
}

#[test]
fn property_chain_multi() {
    let md = "# H1\n## H2a\n## H2b\n";
    assert_eq!(run(md, ".h2.text"), vec!["H2a", "H2b"]);
}

// ---------------------------------------------------------------------------
// Pipe context restore (item 5a)
// ---------------------------------------------------------------------------

#[test]
fn pipe_inside_object_restores_current() {
    let md = "# Title\ntext\n";
    let doc = parse_markdown(md);
    let out = query::execute(&doc, ".h1 | {a: (.text | upper), b: .level}").unwrap();
    assert_eq!(out.len(), 1);
    if let Value::Object(o) = &out[0] {
        assert_eq!(o.get("a").map(|v| v.to_text()), Some("TITLE".to_string()));
        assert_eq!(o.get("b").map(|v| v.to_text()), Some("1".to_string()));
    } else {
        panic!("expected object, got {:?}", out[0]);
    }
}

// ---------------------------------------------------------------------------
// Arithmetic / lexer (item 3)
// ---------------------------------------------------------------------------

#[test]
fn subtraction_lexes_as_two_numbers() {
    assert_eq!(run("# X\n", "5-3"), vec!["2".to_string()]);
}

#[test]
fn negative_literal_in_prefix_position() {
    assert_eq!(run("# X\n", "-3"), vec!["-3".to_string()]);
}

#[test]
fn malformed_number_errors() {
    run_err("# X\n", "1.2.3");
}

// ---------------------------------------------------------------------------
// Indexing / slicing (item 5b)
// ---------------------------------------------------------------------------

#[test]
fn array_single_index() {
    assert_eq!(run("# X\n", "[10,20,30][1]"), vec!["20".to_string()]);
}

#[test]
fn array_negative_index() {
    assert_eq!(run("# X\n", "[10,20,30][-1]"), vec!["30".to_string()]);
}

#[test]
fn array_slice() {
    assert_eq!(run("# X\n", "[10,20,30][0:2]"), vec!["10", "20"]);
}

// ---------------------------------------------------------------------------
// Conditionals (item 4b)
// ---------------------------------------------------------------------------

#[test]
fn elif_chain_single_end() {
    assert_eq!(
        run("# X\n", "if false then 1 elif true then 2 else 3 end"),
        vec!["2".to_string()]
    );
}

// ---------------------------------------------------------------------------
// Hierarchy vs comparison disambiguation (item 4d)
// ---------------------------------------------------------------------------

#[test]
fn gt_between_non_selectors_is_comparison() {
    // select(2 > .level) keeps only headings with level < 2.
    let md = "# A\n## B\n";
    assert_eq!(run(md, ".h | select(2 > .level) | .text"), vec!["A"]);
}

#[test]
fn hierarchy_direct_child_headings() {
    let md = "# A\n## B\n### C\n## D\n";
    assert_eq!(run(md, ".h1 > .h2 | .text"), vec!["B", "D"]);
}

// ---------------------------------------------------------------------------
// Higher-order forms (item 6a)
// ---------------------------------------------------------------------------

#[test]
fn any_evaluates_per_element() {
    assert_eq!(run("# X\n", "[1,2,3] | any(. > 10)"), vec!["false"]);
    assert_eq!(run("# X\n", "[1,2,3] | any(. > 2)"), vec!["true"]);
}

#[test]
fn all_evaluates_per_element() {
    assert_eq!(run("# X\n", "[1,2,3] | all(. > 0)"), vec!["true"]);
    assert_eq!(run("# X\n", "[1,2,3] | all(. > 1)"), vec!["false"]);
}

#[test]
fn sort_by_level() {
    // Input order A(1) C(3) B(2) → sorted by level A B C.
    let md = "# A\n### C\n## B\n";
    let doc = parse_markdown(md);
    let out = query::execute(&doc, "[.h] | sort_by(.level)").unwrap();
    assert_eq!(out.len(), 1);
    if let Value::Array(items) = &out[0] {
        let levels: Vec<String> = items
            .iter()
            .filter_map(|v| v.get_property("level").map(|l| l.to_text()))
            .collect();
        assert_eq!(levels, vec!["1", "2", "3"]);
    } else {
        panic!("expected array, got {:?}", out[0]);
    }
}

#[test]
fn group_by_level() {
    let md = "# A\n# B\n## C\n";
    let doc = parse_markdown(md);
    let out = query::execute(&doc, "[.h] | group_by(.level)").unwrap();
    assert_eq!(out.len(), 1);
    if let Value::Object(o) = &out[0] {
        // Two groups: level 1 (A, B) and level 2 (C).
        assert_eq!(o.len(), 2);
        if let Some(Value::Array(g1)) = o.get("1") {
            assert_eq!(g1.len(), 2);
        } else {
            panic!("missing level-1 group");
        }
        if let Some(Value::Array(g2)) = o.get("2") {
            assert_eq!(g2.len(), 1);
        } else {
            panic!("missing level-2 group");
        }
    } else {
        panic!("expected object, got {:?}", out[0]);
    }
}

// ---------------------------------------------------------------------------
// String repeat overflow / regex (items 5c, 6c)
// ---------------------------------------------------------------------------

#[test]
fn string_repeat_overflow_errors_not_aborts() {
    run_err("# X\n", "\"x\" * 1e300");
}

#[test]
fn matches_invalid_regex_errors() {
    run_err("# X\nx\n", ".h1.text | matches(\"[\")");
}

// ---------------------------------------------------------------------------
// Codepoint length (item 6b)
// ---------------------------------------------------------------------------

#[test]
fn count_counts_codepoints_not_bytes() {
    // "ccé" is 3 codepoints but 4 bytes.
    let md = "# A\n\nccé\n";
    assert_eq!(run(md, ".h1.content | count"), vec!["3".to_string()]);
}

// ---------------------------------------------------------------------------
// New element kinds (item 5e)
// ---------------------------------------------------------------------------

#[test]
fn paragraphs_are_extracted() {
    let md = "# A\n\nfirst para\n\nsecond para\n";
    let out = run(md, ".para");
    assert!(out.iter().any(|p| p.contains("first para")));
    assert!(out.iter().any(|p| p.contains("second para")));
}

#[test]
fn blockquotes_are_extracted() {
    let md = "# A\n\n> quoted text\n";
    let out = run(md, ".blockquote.text");
    assert_eq!(out.len(), 1);
    assert!(out[0].contains("quoted text"));
}

#[test]
fn frontmatter_is_parsed() {
    let md = "---\ntitle: Hi\nn: 3\n---\n# A\n";
    assert_eq!(run(md, ".frontmatter.title"), vec!["Hi".to_string()]);
}

// ---------------------------------------------------------------------------
// Heading-scoped code blocks (item 5d)
// ---------------------------------------------------------------------------

#[test]
fn descendant_code_blocks_are_scoped_to_heading() {
    let md = "\
# First
```rust
let a = 1;
```

# Second
```python
b = 2
```
";
    // `.h[First] >> .code` must return only the rust block, not the python one.
    let doc = parse_markdown(md);
    let out = query::execute(&doc, ".h[First] >> .code").unwrap();
    assert_eq!(out.len(), 1, "expected exactly one scoped code block");
    assert!(out[0].to_text().contains("let a = 1"));
}

// ---------------------------------------------------------------------------
// Recursion guard (item 4c)
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_parens_error_not_overflow() {
    // Run on an 8 MiB stack to mirror the real CLI main thread (libtest workers
    // default to a smaller stack).
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let depth = 50_000;
            let mut q = String::with_capacity(depth * 2 + 1);
            q.push_str(&"(".repeat(depth));
            q.push('.');
            q.push_str(&")".repeat(depth));
            run_err("# X\n", &q);
        })
        .unwrap()
        .join()
        .unwrap();
}
