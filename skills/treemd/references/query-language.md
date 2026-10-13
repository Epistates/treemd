# treemd Query Language (tql) Reference

A jq-like query language for navigating and extracting markdown structure.
Results print one per line. A selector that matches nothing prints nothing and
exits `0`.

## Element Selectors

| Selector              | Description                                 |
| --------------------- | ------------------------------------------- |
| `.h`, `.heading`      | All headings (any level)                    |
| `.h1` - `.h6`         | Headings by level                           |
| `.code`               | All code blocks                             |
| `.link`, `.a`         | All links                                   |
| `.img`, `.image`      | All images                                  |
| `.table`              | All tables                                  |
| `.list`               | All lists (a nested list is its own entry)  |
| `.blockquote`, `.quote` | All blockquotes                           |
| `.para`, `.p`         | All paragraphs                              |
| `.frontmatter`, `.fm` | YAML front matter, as an object             |
| `.`                   | The whole document                          |

After a pipe, a selector applied to a heading looks only inside that heading's
section: `.h2[0] | .code` is the code under the first h2. On an input that has
a key of the same name, `.name` reads the key: `.fm | .image` is the front
matter's `image`.

## Filters & Indexing

```
.h2[Features]        # h2s whose text contains "Features" (case-insensitive)
.h2["Install"]       # h2s whose whole text is "Install" (case-insensitive)
.code[rust]          # Code blocks by language (whole name, case-insensitive)
.link[external]      # Links by type: external, anchor, relative, wikilink
.h2[0]               # First h2
.h2[-1]              # Last h2
.h2[1:3]             # h2s at index 1 and 2
.h2[:3]              # First 3 h2s
.h2[]                # Every h2
.table[0].rows[][0]  # First cell of every row
```

`[external]` and the other link types filter by type only on links; on
headings they are ordinary text filters.

## Hierarchy Operators

```
.h1 > .h2            # h2s directly under each h1
.h1 > .code          # Code in each h1's own body, before its first subheading
.h1 >> .code         # Code anywhere in each h1's section
```

These work for every element kind: headings, code, links, images, tables,
lists, paragraphs and blockquotes.

## Properties

| Value      | Properties                                                          |
| ---------- | ------------------------------------------------------------------- |
| heading    | `.text .level .line .slug .content .md .children .parent`          |
| code       | `.lang .text .lines .start_line .end_line`                          |
| link       | `.text .url .type`                                                  |
| image      | `.alt .src .title`                                                  |
| table      | `.headers .rows .cols .alignments`                                  |
| list       | `.items .ordered`                                                   |
| paragraph, blockquote | `.text`                                                  |
| document   | `.heading_count .word_count .text`                                  |
| object     | any key, as `.key` (front matter too)                               |

A heading's `.content` and `.md` include its subsections. `.line` is the
heading's own line, the same number `-l -n` prints. `.children` is the array
of headings directly under it; `.parent` is the heading above it, or `null`.

## Pipes and Streams

```
.h2 | text           # Heading text (strips ##)
.h2 | count          # Number of h2s
.code | lang         # Code block languages
.link | url          # Link URLs
```

A selector yields a stream, and most functions run once per value. Collection
functions run once on the whole stream instead: `count`, `length`, `first`,
`last`, `nth`, `limit`, `skip`, `sort`, `sort_by`, `unique`, `reverse`,
`group_by`, `join`, `min`, `max`, `add`, `any`, `all`, `levels`, `langs`,
`types`. So `.h | count` is the number of headings and `.h[Missing] | count`
is `0`. The ordering ones hand back a stream, so `.h | sort_by(.level) | text`
works. Given a single array, as in `[.h2] | count`, they work on that array.

## Collection Functions

| Function              | Description                                     | Alias                   |
| --------------------- | ----------------------------------------------- | ----------------------- |
| `count`               | Count values                                    | `length`, `len`, `size` |
| `first`               | First value                                     | `head`                  |
| `last`                | Last value                                      |                         |
| `nth(n)`              | Value at index n (negative from the end)        |                         |
| `limit(n)`            | First n values                                  | `take`                  |
| `skip(n)`             | All but the first n                             | `drop`                  |
| `reverse`             | Reverse order                                   |                         |
| `sort`                | Sort (null, booleans, numbers, strings, ...)    |                         |
| `sort_by(f)`          | Sort by `f`, e.g. `sort_by(.level)`             |                         |
| `unique`              | Drop repeated values, keeping the first         |                         |
| `group_by(f)`         | Object of arrays keyed by `f`                   | `group`                 |
| `flatten`             | Flatten one level of nested arrays              |                         |
| `min`, `max`          | Smallest/largest number                         |                         |
| `add`                 | Sum numbers or concatenate text                 |                         |
| `keys`, `values`      | Keys/values of an object                        |                         |

`sort_by` and `group_by` also take a property name as a string:
`group_by("level")`.

## String Functions

| Function                  | Description                |
| ------------------------- | -------------------------- |
| `text`                    | Get text representation    |
| `upper`, `lower`          | Case conversion            |
| `trim`                    | Strip whitespace           |
| `split(sep)`              | Split into an array        |
| `join(sep)`               | Join values with separator |
| `replace(a, b)`           | Replace every a with b     |
| `slugify`                 | URL-friendly slug          |
| `lines`, `words`, `chars` | Count lines/words/chars    |

## Tests

| Function         | Description                                    | Alias             |
| ---------------- | ---------------------------------------------- | ----------------- |
| `select(cond)`   | Keep the input if cond is true                 | `where`, `filter` |
| `contains(s)`    | Contains substring, case-insensitive           | `includes`        |
| `startswith(s)`  | Starts with prefix, case-insensitive           | `starts_with`     |
| `endswith(s)`    | Ends with suffix, case-insensitive             | `ends_with`       |
| `matches(regex)` | Matches regex pattern (case-sensitive)         |                   |
| `has(key)`       | Object has key                                 |                   |
| `any(cond)`      | cond holds for any value                       |                   |
| `all(cond)`      | cond holds for every value                     |                   |
| `empty`          | Is null, or an empty string, array or object   |                   |
| `type`           | Type name: `heading`, `code`, `string`, ...    |                   |
| `not`            | Negate: `not .x` or `.x \| not`                |                   |

## Content Functions

| Function             | Description                    |
| -------------------- | ------------------------------ |
| `content`            | Section content (for headings) |
| `md`                 | Raw markdown                   |
| `url`, `href`, `src` | Get URL/link/image source      |
| `lang`               | Code block language            |

## Aggregation Functions

| Function | Description                  |
| -------- | ---------------------------- |
| `stats`  | Document statistics          |
| `levels` | Heading count by level       |
| `langs`  | Code block count by language |
| `types`  | Link types count             |

`levels`, `langs` and `types` count the elements they are given (`.h2 |
levels`), or the whole document's when used alone. `stats` always describes
the whole document.

## Operators

| Operator             | Meaning                                                     |
| -------------------- | ----------------------------------------------------------- |
| `==`, `!=`           | Equality; values of different types are never equal         |
| `<`, `<=`, `>`, `>=` | Compare; across types, jq's order (null < bool < number < string < array < object) |
| `and`, `or`, `not`   | Logic                                                       |
| `+ - * / %`          | Arithmetic; `+` also joins strings and arrays               |
| `a // b`             | `a`, or `b` when `a` is null, false, `0` or empty           |

## Construction & Conditionals

```
{title: .text, n: (.h | count)}       # Object
[.h2 | .text]                         # Array
if .level == 1 then "top" else "sub" end
```

With no `else`, a false condition passes the input through. A comma separates
whole queries (`.h1, .h2`); it is not allowed inside parentheses.

## Output

`--query-output json` and `json-pretty` always print an array, `[]` for no
results, even when a query returns one value. `jsonl` prints one value per
line. Every element carries a `type` field.

## Examples

```bash
# List all h2 headings
treemd -q '.h2' doc.md

# Get heading text only
treemd -q '.h2 | text' doc.md

# Count headings
treemd -q '.h2 | count' doc.md

# First 5 headings
treemd -q '.h | limit(5)' doc.md

# Filter headings (three equivalent ways)
treemd -q '.h | select(contains("API"))' doc.md
treemd -q '.h | where(contains("API"))' doc.md
treemd -q '.h[API]' doc.md

# Rust code under the "Install" section
treemd -q '.h2[Install] | .code[rust]' doc.md

# h2s under "Features" section
treemd -q '.h1[Features] > .h2' doc.md

# All link URLs
treemd -q '.link | url' doc.md

# Document stats: one field
treemd -q 'stats | .words' doc.md

# JSON output
treemd -q '.h2 | text' --query-output json doc.md
```
