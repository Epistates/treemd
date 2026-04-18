# Backtick Rendering Test

A reference file for verifying all markdown inline code and code block variants render correctly.

---

## Inline Code in Paragraphs

Single backtick: `hello world`

Inline code with colons: `foo:bug` and `foo:task`

Inline code with slashes: `foo:bug/SKILL.md`

Inline code with symbols: `foo(bar)`, `x := y`, `$HOME`, `~/.config`

Multiple inline spans in one sentence: use `foo` or `bar` to enable `baz`.

Inline code at start of sentence: `cargo run` starts the binary.

Inline code at end of sentence: run the command `just build`.

---

## Inline Code in Headings

### Heading with `single` backtick code

### Convert `foo:bug` to alias → `foo:task` with type "Bug"

### Multiple spans: `foo`, `bar`, and `baz`

---

## Inline Code in Lists

- Item with `inline code` in it
- Use `cargo test` to run tests
- Another item with `foo` and `bar` together
- `code` at the start of a list item
- A list item ending with `code`

Ordered list:

1. First step: run `cargo build`
2. Second step: run `cargo test`
3. Third step: check `target/release/treemd`

---

## Inline Code in Blockquotes

> Use `cargo run` to start the app.

> The `wikilink` syntax is an Obsidian extension.

---

## Inline Code in Tables

Expected: code in cells renders with code styling.
Known issue: table cells are stored as plain strings — inline code formatting is dropped.

| Command | Description |
|---------|-------------|
| `cargo build` | Compile the project |
| `cargo test` | Run all tests |
| `cargo run -- file.md` | Run with a file |
| `just ci` | Full CI check |

Table with `code` in headers:

Expected: backtick-wrapped headers render as inline code.
Known issue: same plain-string limitation applies to table headers.

| `Key` | `Value` | Notes |
|-------|---------|-------|
| `j` / `k` | Move down/up | Vim-style |
| `g` | Jump to top | |
| `q` | Quit | |

---

## Double-Backtick Inline Code

Use double backticks to include a literal backtick: `` `literal backtick` ``

Double-backtick span: ``code with `backtick` inside``

---

## Fenced Code Blocks (for contrast)

A plain fenced block:

```
plain text code block
no language specified
```

A Rust block:

```rust
fn main() {
    println!("Hello, `world`!");
}
```

A shell block:

```sh
cargo run -- test_backticks.md
just run test_backticks.md
```

---

## Indented Code Blocks (4-space indent)

Regular paragraph, then an indented block:

    this is an indented code block
    backticks here are literal: `not inline code`

---

## Mixed Inline Formatting

Bold and code: **`bold code`** should render distinctly.

Italic and code: *`italic code`* — note: most renderers treat code as code regardless.

Strikethrough and code: ~~`struck code`~~

Bold text with inline code: **use `foo` to enable** the feature.

---

## Edge Cases

Code span with leading/trailing spaces: ` spaced ` (spaces are preserved by CommonMark).

Unclosed backtick (literal): this `has no closing backtick so it's plain text.

Two separate spans: `one` then `two` then `three` in sequence.

Empty-looking content: ` ` (single space is valid code content).
