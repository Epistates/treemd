//! A snapshot of everything treemd extracts from one document covering every
//! construct that has broken here.
//!
//! This exists because three separate bugs shipped from this repo while the
//! whole suite was green. Each time the tests were asking a question narrow
//! enough to miss the answer: callout rendering was checked against
//! hand-written strings rather than parser output, list-item images were
//! checked only in the one shape where the two collection paths cannot
//! overlap, and nothing at all watched for a dependency changing extraction
//! wholesale. That last one took a manual diff of two built binaries across 82
//! documents to catch, and it found real code blocks being replaced by prose.
//!
//! So this test asserts the full extracted shape rather than a property of it.
//! Any change to what the parser reports fails it with a line diff, including
//! changes nobody thought to assert. A failure is not necessarily a bug: an
//! upstream fix will fail it too. Read the diff, decide which it is, and paste
//! the new output in.
//!
//! Regenerate with:
//!
//! ```sh
//! UPDATE_CONTRACT=1 cargo test --test parser_contract
//! ```

use treemd::parse_markdown;
use treemd::query::{self, Value};

/// Every construct that has broken in this repo, plus the ones next to them.
///
/// Do not tidy this file. The oddities are the point: the spaced destination,
/// the image inside a link, the list sitting directly against a fence.
const FIXTURE: &str = r#"---
title: Contract
draft: false
---

# Heading with an image ![heading-img](heading.png)

A paragraph with an ![inline](inline.png) image in it.

![standalone](standalone.png)

[![linked](badge.png)](https://ci.example)

![titled](titled.png "The Title")

![spaced](my image.png "Spaced Title")

## Lists

- tight item ![tight](tight.png)
- tight item with a [link](https://example.com)

- loose item ![loose](loose.png)

- loose second ![loose2](loose2.png)

- multi-block item

  ![nested-in-item](nested.png)

- item with its own image ![own](own.png)

  and a nested one ![nested](nested2.png)

- item holding a fence

  ```rust
  fn in_list_item() {}
  ```

## Blockquotes

> a plain quote

> first line
> second line
> third line

> ![quoted-img](quoted.png)

> a [quoted link](https://quoted.example)

> - a list inside a quote
>
> ```rust
> fn after_list_in_quote() {}
> ```

> some prose inside a quote
>
> ```rust
> fn after_prose_in_quote() {}
> ```

## Callouts

> [!NOTE] Single line callout

> [!WARNING] Multi line callout
> body line one
> body line two

> [!TIP] Callout with a fence
> before the fence
>
> ```rust
> fn in_callout() {}
> ```

## Code

```rust
fn top_level() {}
```

```
no language
```

## Table

| Column A | B |
|----------|---|
| 1        | 2 |
"#;

/// One line per extracted value, in a fixed order, so a change shows up as a
/// line diff rather than a wall of restructured debug output.
fn contract(md: &str) -> String {
    let doc = parse_markdown(md);
    let mut out = String::new();

    let q = |query: &str| -> Vec<Value> {
        query::execute(&doc, query).unwrap_or_else(|e| panic!("query {query:?} failed: {e}"))
    };
    out.push_str("== headings ==\n");
    for v in q(".h") {
        if let Value::Heading(h) = v {
            out.push_str(&format!("h{} {:?}\n", h.level, h.text));
        }
    }

    out.push_str("== images ==\n");
    for v in q(".img") {
        if let Value::Image(i) = v {
            out.push_str(&format!(
                "alt={:?} src={:?} title={:?}\n",
                i.alt, i.src, i.title
            ));
        }
    }

    out.push_str("== code ==\n");
    for v in q(".code") {
        if let Value::Code(c) = v {
            out.push_str(&format!(
                "lang={:?} start={} end={} body={:?}\n",
                c.language, c.start_line, c.end_line, c.content
            ));
        }
    }

    out.push_str("== blockquotes ==\n");
    for v in q(".quote") {
        if let Value::Blockquote(b) = v {
            out.push_str(&format!("{:?}\n", b.content));
        }
    }

    out.push_str("== lists ==\n");
    for v in q(".list") {
        if let Value::List(l) = v {
            out.push_str(&format!("ordered={} items={}\n", l.ordered, l.items.len()));
            for item in &l.items {
                out.push_str(&format!("  - {:?}\n", item.content));
            }
        }
    }

    out.push_str("== links ==\n");
    for v in q(".link") {
        if let Value::Link(l) = v {
            out.push_str(&format!("text={:?} url={:?}\n", l.text, l.url));
        }
    }

    out.push_str("== tables ==\n");
    for v in q(".table") {
        if let Value::Table(t) = v {
            out.push_str(&format!("headers={:?} rows={}\n", t.headers, t.rows.len()));
        }
    }

    out.push_str("== counts ==\n");
    for sel in [
        ".para", ".img", ".code", ".quote", ".list", ".link", ".table",
    ] {
        out.push_str(&format!("{} {}\n", sel, q(sel).len()));
    }

    out
}

#[test]
fn the_extracted_shape_of_every_construct_is_unchanged() {
    let actual = contract(FIXTURE);

    if std::env::var("UPDATE_CONTRACT").is_ok() {
        println!("\n----- BEGIN CONTRACT -----\n{actual}----- END CONTRACT -----\n");
        panic!("UPDATE_CONTRACT set: copy the block above into EXPECTED");
    }

    if actual != EXPECTED {
        // Compared as sets rather than pairwise. One inserted line shifts every
        // line after it, and a positional diff reports all of them, which
        // buries the one line that actually changed.
        let expected_lines: Vec<&str> = EXPECTED.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();

        let mut diff = String::new();
        for line in &expected_lines {
            if !actual_lines.contains(line) {
                diff.push_str(&format!("  gone: {line}\n"));
            }
        }
        for line in &actual_lines {
            if !expected_lines.contains(line) {
                diff.push_str(&format!("  new:  {line}\n"));
            }
        }
        if diff.is_empty() {
            diff.push_str("  (same lines, different order)\n");
        }

        panic!(
            "what treemd extracts has changed.\n\n{diff}\nIf this came from a parser upgrade, \
             check each line is an improvement, then regenerate with \
             UPDATE_CONTRACT=1 cargo test --test parser_contract"
        );
    }
}

/// Generated output, not hand-written. Lines worth knowing are wrong today:
///
/// - the first heading reads `Heading with an image heading-img` and the first
///   image has an empty `alt`, because a heading's image is hoisted out and its
///   alt is left behind in the heading text
/// - `fn after_list_in_quote` is absent from the code section and has instead
///   been merged into the blockquote content, which is Epistates/turbovault#71
/// - quoted fences report `start=0 end=0`
/// - `quoted-img` appears as blockquote text rather than as an image
///
/// The last three are tracked upstream. When a parser upgrade fixes one, this
/// test fails and the fixed line is the diff.
const EXPECTED: &str = r#"== headings ==
h1 "Heading with an image heading-img"
h2 "Lists"
h2 "Blockquotes"
h2 "Callouts"
h2 "Code"
h2 "Table"
== images ==
alt="" src="heading.png" title=None
alt="inline" src="inline.png" title=None
alt="standalone" src="standalone.png" title=None
alt="linked" src="badge.png" title=None
alt="titled" src="titled.png" title=Some("The Title")
alt="spaced" src="my image.png" title=Some("Spaced Title")
alt="tight" src="tight.png" title=None
alt="loose" src="loose.png" title=None
alt="loose2" src="loose2.png" title=None
alt="nested-in-item" src="nested.png" title=None
alt="own" src="own.png" title=None
alt="nested" src="nested2.png" title=None
== code ==
lang=Some("rust") start=1 end=1 body="fn in_list_item() {}"
lang=Some("rust") start=0 end=0 body="fn after_prose_in_quote() {}"
lang=Some("rust") start=0 end=0 body="fn in_callout() {}"
lang=Some("rust") start=1 end=1 body="fn top_level() {}"
lang=None start=1 end=1 body="no language"
== blockquotes ==
"a plain quote"
"first line\nsecond line\nthird line"
"quoted-img"
"a quoted link"
"a list inside a quote```rust\nfn after_list_in_quote() {}\n```"
"some prose inside a quote\n\n```rust\nfn after_prose_in_quote() {}\n```"
"[!NOTE] Single line callout"
"[!WARNING] Multi line callout\nbody line one\nbody line two"
"[!TIP] Callout with a fence\nbefore the fence\n\n```rust\nfn in_callout() {}\n```"
== lists ==
ordered=false items=7
  - "tight item ![tight](tight.png)"
  - "tight item with a [link](https://example.com)"
  - "loose item ![loose](loose.png)"
  - "loose second ![loose2](loose2.png)"
  - "multi-block item"
  - "item with its own image ![own](own.png)"
  - "item holding a fence"
ordered=false items=1
  - ""
== links ==
text="linked" url="https://ci.example"
text="link" url="https://example.com"
text="quoted link" url="https://quoted.example"
== tables ==
headers=["Column A", "B"] rows=1
== counts ==
.para 16
.img 12
.code 5
.quote 9
.list 2
.link 3
.table 1
"#;
