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

/// A dot followed by an operator or brackets is the identity. Only the end
/// of the query and a few closing tokens were accepted after it, so each of
/// these was a parse error.
#[test]
fn identity_before_an_operator_or_brackets() {
    assert_eq!(run("# X\n", "\"a\" | . == \"a\""), vec!["true"]);
    assert_eq!(run("# X\n", "5 | . + 1"), vec!["6"]);
    assert_eq!(run("# X\n", "[1, 2] | .[]"), vec!["1", "2"]);
    assert_eq!(run("# X\n", "[1, 2] | .[1]"), vec!["2"]);
    assert_eq!(run("# X\n", "[3, 4] | .[] | select(. == 4)"), vec!["4"]);
}

/// An index after a stream of arrays applies to each array. It indexed the
/// stream, so `.table[0].rows[][0]` returned the first row's cells rather
/// than the first cell of each row.
#[test]
fn index_applies_to_each_array_in_a_stream() {
    let md = "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n\n| c |\n|---|\n| 5 |\n";
    assert_eq!(run(md, ".table[0].rows[][0]"), vec!["1", "3"]);
    assert_eq!(run(md, ".table[0].rows[][-1]"), vec!["2", "4"]);
    assert_eq!(run(md, ".table[0].rows[0][1]"), vec!["2"]);
    assert_eq!(run(md, ".table.rows[0] | .[0]"), vec!["1", "5"]);
    assert_eq!(run(md, ".table[].rows[] | count"), vec!["3"]);
    // A stream of non-arrays is still indexed as a stream.
    assert_eq!(
        run("# A\n## B\n## C\n", "(.h | select(.level == 2))[0] | .text"),
        vec!["B"]
    );
}

/// `nth(-1e300)` overflowed negating the saturated index (a panic in debug
/// builds, the first element in release), and any index too far back gave
/// the first element.
#[test]
fn nth_out_of_range_is_empty() {
    let md = "# X\n";
    assert_eq!(run(md, "[1, 2, 3] | nth(-1)"), vec!["3"]);
    assert_eq!(run(md, "[1, 2, 3] | nth(1)"), vec!["2"]);
    assert!(run(md, "[1, 2, 3] | nth(-4)").is_empty());
    assert!(run(md, "[1, 2, 3] | nth(-1e300)").is_empty());
    assert!(run(md, "[1, 2, 3] | nth(1e300)").is_empty());
    assert_eq!(run(md, "\"abc\" | nth(-1)"), vec!["c"]);
    assert!(run(md, "\"abc\" | nth(-1e300)").is_empty());
}

/// Each result as compact JSON, for results that are arrays or objects.
fn json(md: &str, q: &str) -> Vec<String> {
    let doc = parse_markdown(md);
    query::execute(&doc, q)
        .unwrap_or_else(|e| panic!("query {q:?} failed: {e}"))
        .iter()
        .map(|v| v.to_json().to_string())
        .collect()
}

/// A slice of an array is an array, as in jq. It was a stream of the
/// items, so `.fm.tags[1:] | count` counted 1.
#[test]
fn array_slice() {
    let md = "---\ntags: [a, b, c]\n---\n# X\n## Y\n## Z\n";
    assert_eq!(json(md, "[10,20,30][0:2]"), ["[10,20]"]);
    assert_eq!(json(md, "[1, 2, 3] | .[1:]"), ["[2,3]"]);
    assert_eq!(json(md, ".fm.tags[1:]"), [r#"["b","c"]"#]);
    assert_eq!(run(md, ".fm.tags[1:] | count"), ["2"]);
    // Past either end of an array is null.
    assert_eq!(json(md, "[1, 2, 3] | .[7]"), ["null"]);
    assert_eq!(json(md, "[1, 2, 3] | .[-9]"), ["null"]);
    // Strings slice by character.
    assert_eq!(run(md, r#""trée" | .[1:3]"#), ["ré"]);
    // Selector results stay a stream.
    assert_eq!(run(md, ".h2[0:2] | count"), ["2"]);
    assert_eq!(run(md, "(.h | select(.level == 2))[0:1] | .text"), ["Y"]);
}

/// Arithmetic follows jq. Mismatched types used to give `null`, or for
/// `+` the two values' text run together (`{} + 1` was `{}1`), so a typo
/// passed as a result.
#[test]
fn arithmetic_follows_jq() {
    let md = "# X\n";
    for q in [
        r#""a" - 1"#,
        r#""a" + 1"#,
        "{} + 1",
        "[1] * 2",
        r#""10" / 2"#,
        r#"-"a""#,
        "true % 2",
        ".h | .text + 1",
    ] {
        run_err(md, q);
    }
    assert_eq!(json(md, "{a: 1, b: 1} + {b: 2}"), [r#"{"a":1,"b":2}"#]);
    assert_eq!(json(md, "[1, 2, 3, 2] - [2]"), ["[1,3]"]);
    assert_eq!(json(md, r#""a,b" / ",""#), [r#"["a","b"]"#]);
    assert_eq!(json(md, r#""ab" | split("")"#), [r#"["a","b"]"#]);
    assert_eq!(json(md, r#""" | split(",")"#), ["[]"]);
    assert_eq!(run(md, "1 + null"), ["1"]);
    assert_eq!(run(md, "null + null"), [""]);
    assert_eq!(run(md, "5.9 % 3"), ["2"]);
    assert_eq!(run(md, "-7 % 3"), ["-1"]);
    run_err(md, "1 % 0.5");
    assert_eq!(
        run(md, r#".h | .text + " " + (.level | tostring)"#),
        ["X 1"]
    );
}

/// A key that is not there is null, as in jq, so a default works. It was an
/// error, which `//` could not catch.
#[test]
fn a_missing_key_is_null() {
    let md = "---\ntitle: T\n---\n# X\n";
    assert_eq!(run(md, ".fm.draft // false"), ["false"]);
    assert_eq!(run(md, "{a: 1} | .b // 2"), ["2"]);
    assert_eq!(run(md, ".h1[0].parent.text // \"top\""), ["top"]);
    // An element's properties are fixed; a typo is still an error.
    run_err(md, ".h | .txt");
}

/// Keys with a dash or a space had no syntax: `.fm["k"]` matched nothing,
/// and `.["k"]` and `."k"` were parse errors.
#[test]
fn quoted_keys() {
    let md = "---\ndraft-of: 3\nmy key: 4\n---\n# X\n";
    assert_eq!(run(md, r#".fm["draft-of"]"#), ["3"]);
    assert_eq!(run(md, r#".fm."draft-of""#), ["3"]);
    assert_eq!(run(md, r#".fm | .["my key"]"#), ["4"]);
    assert_eq!(run(md, r#".fm | ."my key""#), ["4"]);
    assert_eq!(run(md, r#"{"a-b": 1} | .["a-b"]"#), ["1"]);
    // A quoted name is a key even when it spells a selector.
    assert_eq!(run(md, r#"{h2: 5} | ."h2""#), ["5"]);
    // On a selector a quoted filter still matches whole text.
    assert_eq!(run(md, r#".h["X"] | .text"#), ["X"]);
}

/// Three parser fixes from turbovault-parser 4.0.1 (Epistates/turbovault#120).
#[test]
fn fences_and_quoted_tables_parse_as_written() {
    // A block's language is the first word of its info string, so rustdoc's
    // `rust,no_run` and `python title="x"` are found by language.
    let md = "# T\n\n```rust,no_run\nfn a() {}\n```\n\n```python title=\"x\"\nb\n```\n";
    assert_eq!(run(md, ".code | .lang"), ["rust", "python"]);
    assert_eq!(run(md, ".code[rust] | .text"), ["fn a() {}"]);

    // An empty fence is closed, so the paragraph after it is still one.
    let md = "# T\n\n```\n```\n\nafter\n\n```sh\nx\n```\n";
    assert_eq!(run(md, ".para | .text"), ["after"]);
    assert_eq!(run(md, ".code | count"), ["2"]);

    // A table inside a blockquote keeps its cells.
    let md = "# T\n\n> | q | r |\n> |---|---|\n> | 1 | 2 |\n";
    assert_eq!(json(md, ".table | .headers"), [r#"["q","r"]"#]);
    assert_eq!(json(md, ".table | .rows"), [r#"[["1","2"]]"#]);
}

/// A bare filter runs to the `]`. Lexed as tokens, `c++` was a parse error,
/// `c#` an unexpected character, and `-d` lost its `-`.
#[test]
fn bare_filters_take_the_text_as_written() {
    let md = "# Getting Started\n## d-pad\n## dial\n\n```c++\na\n```\n\n```c#\nb\n```\n\n\
              ```objective-c\nc\n```\n";
    assert_eq!(run(md, ".code[c++] | .text"), ["a"]);
    assert_eq!(run(md, ".code[c#] | .text"), ["b"]);
    assert_eq!(run(md, ".code[objective-c] | .text"), ["c"]);
    assert_eq!(run(md, ".h[Getting Started] | .text"), ["Getting Started"]);
    assert_eq!(run(md, ".h[-pad] | .text"), ["d-pad"]);
    // Indexes, slices and single words are read as before.
    assert_eq!(run(md, ".h2[ 1 ] | .text"), ["dial"]);
    assert_eq!(run(md, ".h2[-1:] | .text"), ["dial"]);
    assert_eq!(run(md, ".h2[d][0] | .text"), ["d-pad"]);
    // An array literal is not a filter.
    assert_eq!(json(md, "[.h2[] | .text]"), [r#"["d-pad","dial"]"#]);
}

/// `not .level == 2` was `(not .level) == 2`, which is never true.
#[test]
fn prefix_not_takes_a_comparison() {
    let md = "# A\n## B\n### C\n";
    assert_eq!(run(md, ".h | select(not .level == 2) | .text"), ["A", "C"]);
    assert_eq!(
        run(md, ".h | select(not .level == 2 and .level > 1) | .text"),
        ["C"]
    );
    assert_eq!(run(md, "not false or false"), ["true"]);
}

/// A stage that starts with a reducing function reduces, so these give one
/// answer. They gave one per heading.
#[test]
fn a_stage_led_by_count_reduces() {
    let md = "# A\n## B\n## C\n";
    assert_eq!(run(md, ".h | count > 2"), ["true"]);
    assert_eq!(run(md, ".h | count + 1"), ["4"]);
    assert_eq!(run(md, ".h2 | count == 2"), ["true"]);
    // `null | length` is 0, as in jq.
    assert_eq!(run(md, ".h1[0] | .parent | length"), ["0"]);
}

// ---------------------------------------------------------------------------
// Conditionals (item 4b)
// ---------------------------------------------------------------------------

/// `not` as a pipe stage was a parse error, since the keyword always
/// expected an operand, so the registered `not` function was unreachable.
#[test]
fn not_works_as_a_pipe_stage_and_as_a_prefix() {
    let md = "# A\n## B\n";
    assert_eq!(run(md, "true | not"), vec!["false"]);
    assert_eq!(run(md, "not true"), vec!["false"]);
    assert_eq!(run(md, ".h | select(.level == 1 | not) | .text"), vec!["B"]);
    assert_eq!(run(md, ".h | select(not (.level == 1)) | .text"), vec!["B"]);
    assert_eq!(run(md, "[false | not, true]"), vec!["true\ntrue"]);
}

/// Mixed types compared by text, so `1 == "1"` was true.
#[test]
fn equality_is_type_strict() {
    let md = "# X\n";
    assert_eq!(run(md, "1 == \"1\""), vec!["false"]);
    assert_eq!(run(md, "null == \"\""), vec!["false"]);
    assert_eq!(run(md, "true == \"true\""), vec!["false"]);
    assert_eq!(run(md, "1 != \"1\""), vec!["true"]);
    assert_eq!(run(md, "[1, 2] == [1, 2]"), vec!["true"]);
    assert_eq!(run(md, "[1] == [\"1\"]"), vec!["false"]);
    assert_eq!(run(md, "{a: 1} == {a: 1}"), vec!["true"]);
    assert_eq!(run(md, ".h1[0] == .h1[0]"), vec!["true"]);
    assert_eq!(run(md, ".h1[0].level == 1"), vec!["true"]);
}

/// An `if` with no `else` returned null when the condition failed. jq
/// returns the input unchanged.
#[test]
fn if_without_else_passes_the_input_through() {
    let md = "# A\n## B\n";
    assert_eq!(
        run(md, ".h | .text | if . == \"A\" then \"first\" end"),
        vec!["first", "B"]
    );
    assert_eq!(run(md, "5 | if false then 1 end"), vec!["5"]);
    assert_eq!(
        run(md, "5 | if false then 1 elif false then 2 end"),
        vec!["5"]
    );
}

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

/// `.children` and `.parent` were documented and reported "Property not
/// found".
#[test]
fn heading_children_and_parent() {
    let md = "# A\n## B\n### C\n## D\n#### E\n# F\n";
    assert_eq!(run(md, ".h1[0].children[] | .text"), vec!["B", "D"]);
    assert_eq!(run(md, ".h2[D].children[] | .text"), vec!["E"]);
    assert_eq!(run(md, ".h3[0].children | count"), vec!["0"]);
    assert_eq!(run(md, ".h3.parent.text"), vec!["B"]);
    assert_eq!(run(md, ".h4.parent.text"), vec!["D"]);
    assert_eq!(run(md, ".h1[1].parent"), vec![""]);
    assert_eq!(
        run(
            md,
            "[.h1[0].children[] | {t: .text, n: (.children | count)}] | .[] | .n"
        ),
        vec!["1", "1"]
    );
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

/// `sort` compared text, so numbers sorted as strings, and `unique` merged
/// distinct values that printed alike.
#[test]
fn sort_and_unique_compare_values_by_type() {
    let md = "# Same\n## Same\n";
    assert_eq!(run(md, "[10, 9, 1] | sort | .[]"), vec!["1", "9", "10"]);
    assert_eq!(
        run(md, "[\"b\", 2, null, \"a\", 1, true] | sort | .[]"),
        vec!["", "true", "1", "2", "a", "b"]
    );
    assert_eq!(run(md, ".h | .level | sort"), vec!["1", "2"]);
    assert_eq!(run(md, "[.h] | unique | count"), vec!["2"]);
    assert_eq!(run(md, "[1, \"1\", 1] | unique | count"), vec!["2"]);
    assert_eq!(run(md, ".h | text | unique"), vec!["Same"]);
}

/// A string key names a property, as the `--query-help` example
/// `group_by("level")` assumes. It was evaluated as an expression, the same
/// string for every element, so everything landed in one group and
/// `sort_by` left the order unchanged.
#[test]
fn string_keys_name_a_property() {
    let md = "# A\n### C\n## B\n# D\n";
    assert_eq!(
        run(md, "[.h] | group_by(\"level\") | keys"),
        vec!["1\n3\n2"]
    );
    assert_eq!(
        run(md, ".h | sort_by(\"level\") | .text"),
        vec!["A", "D", "B", "C"]
    );
    assert_eq!(
        run(md, ".h | sort_by(\"text\") | .text"),
        vec!["A", "B", "C", "D"]
    );
}

/// An element inside an object went through serde's derive, which printed
/// the byte offset and the whole section body and no `type`, while
/// `--query-output json` printed the tagged form. Both use one serializer.
#[test]
fn an_element_serializes_the_same_everywhere() {
    let md = "# Title\n\nbody\n";
    let doc = parse_markdown(md);
    let expected = r#"{"type":"heading","level":1,"text":"Title","line":1}"#;

    assert_eq!(
        run(md, "{a: .h1[0]} | text"),
        vec![format!(r#"{{"a":{expected}}}"#)]
    );
    let heading = query::execute(&doc, ".h1[0]").unwrap();
    assert_eq!(
        query::format_output(&heading, query::OutputFormat::JsonLines),
        expected
    );
    assert_eq!(serde_json::to_string(&heading[0]).unwrap(), expected);
}

// ---------------------------------------------------------------------------
// String repeat overflow / regex (items 5c, 6c)
// ---------------------------------------------------------------------------

#[test]
fn string_repeat_overflow_errors_not_aborts() {
    run_err("# X\n", "\"x\" * 1e300");
}

/// The expected count printed as a Rust range: "expects 0..=0 arguments".
#[test]
fn arity_errors_say_the_count_in_words() {
    let message = |q: &str| {
        let doc = parse_markdown("# X\n");
        query::execute(&doc, q).unwrap_err().to_string()
    };
    assert!(
        message("count(1)").contains("Function 'count' takes no arguments, got 1"),
        "{}",
        message("count(1)")
    );
    assert!(message("split").contains("takes 1 argument, got 0"));
    assert!(message("replace(\"a\")").contains("takes 2 arguments, got 1"));
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
    assert_eq!(run(md, ".h1[0].content | count"), vec!["3".to_string()]);
}

// ---------------------------------------------------------------------------
// Collection functions see a piped stream whole
// ---------------------------------------------------------------------------

/// Each of these ran once per piped value: `.h | count` printed `1` per
/// heading and `.h | sort_by(.level)` sorted one heading at a time.
#[test]
fn collection_functions_collect_a_piped_stream() {
    let md = "# A\n### C\n## B\n## B\n";
    assert_eq!(run(md, ".h | count"), vec!["4"]);
    assert_eq!(run(md, ".h | length"), vec!["4"]);
    assert_eq!(run(md, ".h | text | first"), vec!["A"]);
    assert_eq!(run(md, ".h | text | last"), vec!["B"]);
    assert_eq!(run(md, ".h | nth(1) | text"), vec!["C"]);
    assert_eq!(run(md, ".h | text | join(\", \")"), vec!["A, C, B, B"]);
    assert_eq!(run(md, ".h | all(.level <= 3)"), vec!["true"]);
    assert_eq!(run(md, ".h | any(.level == 3)"), vec!["true"]);
    assert_eq!(run(md, ".h | .level | min"), vec!["1"]);
    assert_eq!(run(md, ".h | .level | max"), vec!["3"]);
    assert_eq!(run(md, ".h | .level | add"), vec!["8"]);
    assert_eq!(run(md, ".h | group_by(.level) | keys"), vec!["1\n3\n2"]);

    // Ordering functions hand the stream back as a stream.
    assert_eq!(
        run(md, ".h | sort_by(.level) | .text"),
        vec!["A", "B", "B", "C"]
    );
    assert_eq!(run(md, ".h | text | unique"), vec!["A", "C", "B"]);
    assert_eq!(run(md, ".h | text | reverse"), vec!["B", "B", "C", "A"]);
    assert_eq!(run(md, ".h | limit(2) | .text"), vec!["A", "C"]);
    assert_eq!(run(md, ".h | skip(3) | .text"), vec!["B"]);

    // `levels` counts what it is given.
    assert_eq!(run(md, ".h2 | levels | keys"), vec!["h2"]);
    assert_eq!(run(md, ".h2 | levels | .h2"), vec!["2"]);
    assert_eq!(run(md, "levels | keys"), vec!["h1\nh2\nh3"]);
}

/// A stream of one is still a stream, so the result does not depend on how
/// many matches a document happens to have.
#[test]
fn a_one_value_stream_is_still_collected() {
    let md = "# Only\n";
    assert_eq!(run(md, ".h | count"), vec!["1"]);
    assert_eq!(run(md, ".h | text | first"), vec!["Only"]);
    // An empty stream is collected too, and a single value is not.
    assert_eq!(run(md, ".h2 | count"), vec!["0"]);
    assert_eq!(run(md, ".h1[0].text | count"), vec!["4"]);
    assert_eq!(run(md, "[.h] | count"), vec!["1"]);
}

/// A collecting function inside a nested pipe sees that pipe's stream.
#[test]
fn nested_pipes_collect_their_own_stream() {
    let md = "# A\n\n```rs\nx\n```\n\n# B\n";
    assert_eq!(run(md, ".h | select(.code | count > 0) | .text"), vec!["A"]);
    assert_eq!(run(md, "{n: (.h | count)} | .n"), vec!["2"]);
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
