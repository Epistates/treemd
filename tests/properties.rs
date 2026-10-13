//! Properties that have to hold for any input, checked over generated
//! documents and queries. A markdown viewer reads whatever it is given, and
//! the query language takes whatever an agent writes, so neither may panic on
//! input nobody thought to write a test for.

use proptest::prelude::*;
use treemd::{parse_markdown, query};

/// Documents built from the constructs that have broken before: headings with
/// inline markup, task lists and nesting, quotes, fences, tables, callouts,
/// HTML blocks, math, and the odd unbalanced bracket.
fn markdown() -> impl Strategy<Value = String> {
    let line = prop_oneof![
        "#{1,6} [a-zé İ`*!\\[\\]()]{0,16}",
        "[ ]{0,6}[-*+] (\\[[ xX]\\] )?[a-z `*_\\[\\]()]{0,16}",
        "[ ]{0,3}[0-9]{1,2}[.)] [a-z]{0,10}",
        ">{1,3} ?(\\[!NOTE\\] )?[a-z\\- ]{0,12}",
        "[ ]{0,4}(```|~~~)[a-z]{0,6}",
        "\\|?[ a-z\\\\|]{0,12}\\|[a-z ]{0,6}\\|?",
        "\\|? ?:?-{1,4}:? ?\\| ?-{1,4} ?\\|?",
        "!?\\[[a-z ]{0,8}\\]\\([a-z./#:]{0,12}\\)",
        "\\[\\[[a-z|#]{0,10}\\]\\]",
        "</?details>|<summary>[a-z]{0,6}</summary>",
        "\\$\\$?[a-z^]{0,6}\\$?\\$?",
        "---|\\*\\*\\*|===",
        "[a-z \t\r\u{feff}\u{1b}-]{0,30}",
        "",
    ];
    prop::collection::vec(line, 0..40).prop_map(|lines| lines.join("\n"))
}

/// Query text from the language's own tokens, so most of it gets past the
/// lexer and into the parser and evaluator, where the interesting failures
/// are.
fn query_text() -> impl Strategy<Value = String> {
    let token = prop_oneof![
        Just(".".to_string()),
        "\\.(h[1-6]?|code|link|img|table|list|para|blockquote|fm|text|level|content)",
        Just(" | ".to_string()),
        Just(" > ".to_string()),
        Just(" >> ".to_string()),
        "\\[-?[0-9]{0,3}(:-?[0-9]{0,3})?\\]",
        "\\[\"?[a-z ]{0,6}\"?\\]",
        "(count|text|first|last|sort|unique|levels|langs|stats|keys|length|not|reverse)",
        "(select|limit|skip|nth|sort_by|group_by|contains|join|any|all|map)\\(",
        Just(")".to_string()),
        Just(", ".to_string()),
        "(==|!=|<=|>=|<|>|and|or|//|\\+|-|\\*|/)",
        "[0-9]{1,4}(\\.[0-9])?",
        "\"[a-zİ ]{0,6}\"?",
        "\\{[a-z]{0,4}: ?",
        Just("}".to_string()),
        Just("[".to_string()),
        Just("]".to_string()),
        "if |then |else |end",
    ];
    prop::collection::vec(token, 0..12).prop_map(|tokens| tokens.concat())
}

// Cases default to proptest's 256; a longer search is `PROPTEST_CASES=20000`.
proptest! {
    #[test]
    fn any_document_parses(md in markdown()) {
        let doc = parse_markdown(&md);
        // Every heading offset indexes the content at a heading.
        for heading in &doc.headings {
            prop_assert!(heading.offset <= doc.content.len());
            prop_assert!(doc.content.is_char_boundary(heading.offset));
        }
        let lines = doc.heading_lines();
        prop_assert!(lines.windows(2).all(|pair| pair[0] <= pair[1]));
        let _ = treemd::parser::build_json_output(&doc, None);
    }

    #[test]
    fn any_query_returns_or_errs(md in markdown(), q in query_text()) {
        let doc = parse_markdown(&md);
        if let Ok(values) = query::execute(&doc, &q) {
            for format in [
                query::OutputFormat::Plain,
                query::OutputFormat::Json,
                query::OutputFormat::JsonLines,
            ] {
                let _ = query::format_output(&values, format);
            }
        }
    }
}
