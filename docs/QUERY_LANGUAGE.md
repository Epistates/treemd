# treemd Query Language (tql)

> **Status**: Reference for the language as treemd ships it. Constructs that
> do not exist are marked **Not implemented** and name the working
> alternative where there is one.
> **Goal**: jq-like querying for Markdown, with maximum DX and utility

`treemd --query-help` prints a shorter summary of the same language.

## Design Philosophy

1. **jq-compatible where sensible** - Same operators for same operations
2. **Markdown-native selectors** - `.h1`, `.code`, `.link` feel natural
3. **Composable** - Pipes and filters work like jq
4. **Progressive complexity** - Simple things are simple, complex things possible
5. **Clear error messages** - Parse errors point at the offending token
6. **Unix philosophy** - Plays well with other tools via pipes

## Quick Reference

```bash
# Basic selectors
treemd -q '.h2' doc.md                          # All h2 headings
treemd -q '.code[rust]' doc.md                  # Rust code blocks
treemd -q '.link' doc.md                        # All links

# Filtering
treemd -q '.h2 | select(contains("API"))' doc.md

# Hierarchy
treemd -q '.h1[Features] > .h2' doc.md          # h2s directly under Features

# Properties
treemd -q '.h2 | text' doc.md                   # Just heading text
treemd -q '.link | url' doc.md                  # Just URLs

# Content extraction
treemd -q '.h1[Installation] | content' doc.md  # Section content

# Counting
treemd -q '.h2 | count' doc.md                  # Number of h2s
```

---

## Element Selectors

### Headings

| Selector | Description | Example |
|----------|-------------|---------|
| `.h` | All headings (any level). Aliases: `.heading`, `.headings`, `.header` | `.h` |
| `.h1` - `.h6` | Headings at specific level | `.h2` |
| `.h[text]` | Headings whose text contains `text`, ignoring case | `.h[Install]` |
| `.h["text"]` | Headings whose whole text is `text`, ignoring case | `.h["Getting Started"]` |
| `.h[/regex/]` | **Not implemented**. Use `.h \| select(matches("Chapter [0-9]+"))` | `.h[/Chapter \d+/]` |
| `.h1[text]` | Level + text filter combined | `.h1[Features]` |

A bracketed word is matched as text, keywords included, so `.h[end]` finds
"The End".

### Code Blocks

| Selector | Description | Example |
|----------|-------------|---------|
| `.code` | All code blocks | `.code` |
| `.code[lang]` | Code blocks with language (whole-string, case-insensitive) | `.code[rust]` |
| `.code["lang"]` | Same, explicit string | `.code["python"]` |
| `.code[/regex/]` | **Not implemented**. Use `.code \| select(.lang == "js" or .lang == "ts")` | `.code[/^(js\|ts)$/]` |

A bracket filter on `.code` matches the **language**, never the block body.
To search bodies, use `.code | select(contains("TODO"))`.

`.start_line` and `.end_line` are the opening and closing fence rows, counting
from 1, including for a block nested in a list item or a blockquote.

### Links

| Selector | Description | Example |
|----------|-------------|---------|
| `.link` | All links. Alias: `.a` | `.link` |
| `.link[anchor]` | Anchor links only | `.link[anchor]` |
| `.link[external]` | External URLs only | `.link[external]` |
| `.link[relative]` | Relative file links | `.link[relative]` |
| `.link[wikilink]` | Wikilinks `[[...]]` | `.link[wikilink]` |

The four type names filter by type only on links. On anything else they are
ordinary text filters, so `.h[external]` finds "External APIs".

### Other Elements

| Selector | Aliases | Description |
|----------|---------|-------------|
| `.img` | `.image`, `.images` | All images (see caveats below) |
| `.table` | `.tables` | All tables |
| `.list` | `.ul`, `.ol`, `.lists` | All lists |
| `.blockquote` | `.quote`, `.bq` | All blockquotes |
| `.para` | `.p`, `.paragraph` | All paragraphs |
| `.frontmatter` | `.fm`, `.meta`, `.yaml` | YAML front matter, as an object |

`.img` finds images written standalone, inline in a sentence, in a heading,
inside a blockquote, wrapped in a link, and in a list item whether the list is
tight or loose. One case is still wrong in the underlying parser:

- **A heading image's alt is also copied into the heading text.**
  `# Title ![a](a.png)` reports the image correctly but gives `.h1` a text of
  `Title a` rather than `Title`, and the generated anchor is `title-a`. Tracked
  at [turbovault#78](https://github.com/Epistates/turbovault/issues/78).

`.list` reports every list in document order, a nested list right after the
list that holds it. A nested list is its own entry rather than text in its
parent item's `.content`, so `- outer` holding `- inner` gives two lists, one
item each.

### Document

| Selector | Description |
|----------|-------------|
| `.` | Entire document |
| `. \| stats` | Document statistics |

### Selectors After a Pipe

A selector does not always mean "everywhere in the document":

- **On a heading, it looks inside that heading's section**, the same as `>>`.
  `.h2[0] | .code` is the code under the first h2, and
  `.h | {title: .text, code: [.code | .lang]}` lists each section's own
  languages. `.frontmatter` belongs to the document and is the same from
  anywhere.
- **On a value that has a key or property of the same name, it reads that
  key.** `.fm | .image` is the front matter's `image` field, and
  `{p: 1} | .p` is `1`, not every paragraph. On the document itself the
  selector always wins.

---

## Hierarchy Navigation

```bash
# Direct children (CSS > combinator)
.h1 > .h2                    # h2s that are direct children of h1s
.h1[Features] > .h2          # h2s directly under "Features"
.h1 > .code                  # code in each h1's own body, before its first subheading

# Descendants
.h1 >> .h2                   # h2s anywhere under h1s (any depth)
.h1[Features] >> .code       # code blocks anywhere under Features
.h1 >> .link                 # all links under any h1 section
.h2[Install] > .table        # works for tables, lists, images, paragraphs, quotes too
```

`>` is the hierarchy operator only between two selectors. Between anything
else it is greater-than, so `select(2 > .level)` is a comparison.

### Heading Relationships

`.children` and `.parent` are heading properties:

```bash
.h1[0].children[] | .text    # Direct child headings of the first h1
.h3 | .parent | .text        # The heading each h3 sits under
```

`.children` is an array (empty when there are none), and `.parent` is a
heading or `null`.

**Not implemented** as functions: `parent`, `children`, `siblings`, `next`,
`prev` and `descendants` all report `Unknown function`. Use the properties
above, `.h1 >> .h` for descendants, and `.h2[0] | .parent | .children[]` for
siblings (which includes the heading itself).

---

## Indexing and Slicing

```bash
# Single element access
.h2[0]                       # First h2 (0-indexed)
.h2[-1]                      # Last h2
.h2[2]                       # Third h2

# Slicing (like Python/jq)
.h2[0:3]                     # First three h2s (indices 0, 1, 2)
.h2[:3]                      # Same as above
.h2[2:]                      # From third h2 to end
.h2[-2:]                     # Last two h2s

# Table access
.table[0]                    # First table
.table[0].rows[0]            # First row of first table
.table[0].rows[0][1]         # Second cell of first row
.table[0].rows[][0]          # First cell of every row
```

After an expression that yields several arrays, an index applies to each one,
as in jq: `.table[0].rows[][0]` takes the first cell of every row. After
anything else it picks from the results, so `(.h | select(.level == 2))[0]`
is the first matching heading.

---

## Iteration

The `[]` operator without index iterates over all elements:

```bash
.h[]                         # Every heading
.h[] | text                  # Text of each heading (one per line)
.table[].rows[]              # All rows of all tables
.code[] | lang               # Language of each code block
[1, 2] | .[]                 # Items of an array
```

---

## Pipes and Composition

Pipes (`|`) chain operations, passing output to input:

```bash
.h2 | text                   # Get text property of h2s
.h2 | select(contains("API")) # Filter h2s
.h2 | text | upper           # Uppercase heading text
.h2 | count                  # Count h2s
.code | select(.lang == "rust") | text  # Rust code content
```

### Streams and Collection Functions

A selector yields a **stream**, one value per match, and most stages run once
per value: `.h2 | text` runs `text` on each heading. Collection functions are
different. After a stream they run **once on all of it**:

```bash
.h | count                   # Number of headings, not 1 per heading
.h | text | join(", ")       # One line, every heading
.h | sort_by(.level) | text  # Sorted, still one per line
.h[Missing] | count          # 0
```

The collection functions are `count`, `length`, `first`, `last`, `nth`,
`limit`, `skip`, `sort`, `sort_by`, `unique`, `reverse`, `group_by`, `join`,
`min`, `max`, `add`, `any`, `all`, `levels`, `langs`, `types` and `stats`. The
ordering ones (`sort`, `sort_by`, `unique`, `reverse`, `limit`, `skip`) hand
back a stream, so a pipe can carry on per value after them.

Given a single value they work on that value, so `[.h2] | count` counts the
array and `.h1[0].text | count` counts characters. Whether something is a
stream depends on the query, not on how many matches the document happens to
have: `.h1.text | count` counts h1s even when there is one. Inside an object
or a `select`, the input is a single value again, so
`.h | {n: (.children | count)}` gives each heading's child count.

### Multiple Outputs

Comma generates multiple outputs:

```bash
.h1, .h2                     # Both h1s and h2s
.code[rust], .code[python]   # Rust and Python code
```

A comma separates whole queries, so it binds more loosely than `|`:
`.h1 | text, .h2` is `(.h1 | text)` then `.h2`. A comma **inside
parentheses** is not implemented. `(.h1, .h2) | text` fails with
`Expected ')', found ','`. Pipe each branch separately instead:
`.h1 | text, .h2 | text`.

---

## Property Access

### Heading Properties

| Property | Type | Description |
|----------|------|-------------|
| `.text` | string | Heading text content |
| `.level` | number | Heading level (1-6) |
| `.offset` | number | Byte offset in source |
| `.line` | number | The heading's own line (1-indexed), as `-l -n` reports it |
| `.content` | string | Section body: everything under the heading, subsections included, up to the next heading at the same or a shallower level |
| `.md` | string | Raw markdown of the section, heading line and subsections included |
| `.slug` | string | URL-friendly slug |
| `.children` | array | Headings directly under this one |
| `.parent` | heading or null | Nearest heading above at a shallower level |

### Code Block Properties

| Property | Type | Description |
|----------|------|-------------|
| `.lang` | string? | Language identifier |
| `.text` | string | Code content |
| `.lines` | number | Line count |
| `.start_line` | number | Starting line in source |
| `.end_line` | number | Ending line in source |

### Link Properties

| Property | Type | Description |
|----------|------|-------------|
| `.text` | string | Display text |
| `.url` | string | Target URL/path |
| `.type` | string | "anchor", "relative", "wikilink", "external" |
| `.offset` | number | Byte offset in source |

### Table Properties

| Property | Type | Description |
|----------|------|-------------|
| `.headers` | array | Header row |
| `.rows` | array | Data rows |
| `.cols` | number | Column count |
| `.alignments` | array | Column alignments |

### Image Properties

| Property | Type | Description |
|----------|------|-------------|
| `.src` | string | Image source URL |
| `.alt` | string | Alt text |
| `.title` | string? | Title attribute, `null` when the image has none |

Access these as properties (`.img[0].alt`). Only `src`/`url`/`href` exist as
pipe functions; `.img | alt` reports `Unknown function`.

### Other Properties

| Value | Properties |
|-------|------------|
| List | `.items` (array of item text), `.ordered` |
| Paragraph, blockquote | `.text` |
| Document | `.heading_count`, `.word_count`, `.text` |
| Object, front matter | any key, as `.key` |

Asking for a property a value does not have is an error,
`Property 'x' not found on object`, rather than `null` as in jq.

---

## Filtering with `select()`

The `select(condition)` function filters elements:

```bash
# Text matching (contains, startswith and endswith ignore case)
.h | select(contains("API"))           # Headings containing "API"
.h | select(startswith("Chapter"))     # Headings starting with "Chapter"
.h | select(endswith("Guide"))         # Headings ending with "Guide"
.h | select(matches("[0-9]+"))         # Headings matching a regex (case-sensitive)

# Property comparison
.h | select(.level >= 2)               # h2 and deeper
.h | select(.level == 2)               # Only h2s
.code | select(.lang == "rust")        # Rust code blocks
.code | select(.lang != null)          # Code blocks with language
.link | select(.type == "external")    # External links only

# Compound conditions
.h | select(.level >= 2 and contains("API"))
.code | select(.lang == "rust" or .lang == "python")
.h | select(not contains("deprecated"))
.h | select(contains("deprecated") | not)

# Content-based
.code | select(.text | lines > 10)     # Long code blocks
.h | select(.content | words > 100)    # Substantial sections
.h | select(.code | count > 0)         # Sections with code
```

---

## Built-in Functions

### Collection Functions

| Function | Description | Example |
|----------|-------------|---------|
| `count` | Count values. Aliases: `length`, `len`, `size` | `.h \| count` |
| `first` | First value. Alias: `head` | `.h2 \| first` |
| `last` | Last value | `.h2 \| last` |
| `nth(n)` | Value at index `n`, negative from the end, nothing when out of range | `.h2 \| nth(3)` |
| `limit(n)` | First `n` values. Alias: `take` | `.h \| limit(5)` |
| `skip(n)` | All but the first `n`. Alias: `drop` | `.h \| skip(1)` |
| `reverse` | Reverse order | `.h \| reverse` |
| `sort` | Sort in jq's order: null, booleans, numbers, strings, arrays, objects | `.h \| text \| sort` |
| `sort_by(f)` | Sort by `f`; a string names a property | `.h \| sort_by(.level)` |
| `unique` | Drop repeated values, keeping the first of each | `.code \| lang \| unique` |
| `flatten` | Flatten one level of nested arrays | `.table \| .rows \| flatten` |
| `group_by(f)` | Object of arrays keyed by `f`; a string names a property. Alias: `group` | `.h \| group_by(.level)` |
| `min`, `max` | Smallest/largest number | `.h \| .level \| max` |
| `add` | Sum numbers, or concatenate as text | `.h \| .level \| add` |
| `keys` | Keys of an object, indices of an array | `.frontmatter \| keys` |
| `values` | Values of an object, items of an array | `.frontmatter \| values` |

### String Functions

| Function | Description | Example |
|----------|-------------|---------|
| `upper` | Uppercase. Alias: `ascii_upcase` | `.h \| text \| upper` |
| `lower` | Lowercase. Alias: `ascii_downcase` | `.h \| text \| lower` |
| `trim` | Trim whitespace | `.h \| text \| trim` |
| `split(s)` | Split string into an array | `.h \| text \| split(" ")` |
| `join(s)` | Join values with `s` | `.h \| text \| join(", ")` |
| `replace(a, b)` | Replace every `a` with `b` | `.h \| text \| replace("-", " ")` |
| `slugify` | URL-friendly slug | `.h \| text \| slugify` |

### Content Functions

| Function | Description | Example |
|----------|-------------|---------|
| `text` | Get text content | `.h \| text` |
| `content` | Section content | `.h1[Install] \| content` |
| `md` | Raw markdown. Alias: `markdown` | `.h1[Install] \| md` |
| `url` | Link URL or image source. Aliases: `href`, `src` | `.link \| url` |
| `lang` | Code block language. Alias: `language` | `.code \| lang` |
| `lines` | Line count | `.code \| text \| lines` |
| `words` | Word count | `.h \| content \| words` |
| `chars` | Character count | `.h \| text \| chars` |

### Aggregation Functions

| Function | Description | Example |
|----------|-------------|---------|
| `stats` | Document statistics | `. \| stats` |
| `levels` | Heading count by level | `.h \| levels` |
| `langs` | Code block count by lang | `.code \| langs` |
| `types` | Link count by type | `.link \| types` |

`levels`, `langs` and `types` count the elements they are given, so
`.h2 | levels` is `{"h2": 3}` and `.h1[API] >> .code | langs` covers one
section. On the document, or on their own, they count the whole document.
`stats` always describes the whole document.

### Boolean Functions

| Function | Description | Example |
|----------|-------------|---------|
| `contains(s)` | Contains substring, ignoring case. Alias: `includes` | `select(contains("API"))` |
| `startswith(s)` | Starts with, ignoring case. Alias: `starts_with` | `select(startswith("Chapter"))` |
| `endswith(s)` | Ends with, ignoring case. Alias: `ends_with` | `select(endswith("Guide"))` |
| `matches(r)` | Regex match | `select(matches("[0-9]+"))` |
| `has(key)` | Object has key | `.frontmatter \| has("draft")` |
| `empty` | Is null, or an empty string, array or object | `select(.content \| empty)` |
| `any(f)` | Any value matches | `.code \| any(.lang == "rust")` |
| `all(f)` | All values match | `.h \| all(.level <= 3)` |
| `not` | Negate | `select(.draft \| not)` |
| `type` | Type name: `heading`, `code`, `string`, `number`, ... | `.h[0] \| type` |

Unlike jq, `empty` is a test that returns `true` or `false`; it does not
produce an empty stream.

---

## Operators

| Operator | Meaning |
|----------|---------|
| `==`, `!=` | Equality. Values of different types are never equal: `1 == "1"` is `false` |
| `<`, `<=`, `>`, `>=` | Comparison of two numbers or two strings |
| `and`, `or`, `not` | Logic. `not` is a prefix (`not .x`) or a pipe stage (`.x \| not`) |
| `+` | Adds numbers, joins strings and arrays |
| `-`, `*`, `/`, `%` | Arithmetic. `"ab" * 3` repeats a string |
| `a // b` | `a`, or `b` when `a` is null, false, `0` or empty |
| `( )` | Grouping |

From tightest to loosest: `* / %`, `+ -`, `//`, comparisons, `== !=`, `and`,
`or`, `> >>` between selectors, `|`, `,`.

`//` falls back on any falsy value, which is wider than jq, where only
`null` and `false` do.

---

## Construction (JSON Output)

Build custom JSON structures:

```bash
# Object construction
{title: .h1[0].text}
{title: .h1[0].text, count: (.h | count)}

# Array construction
[.h2[].text]
[.h | {level: .level, text: .text}]

# Nested structure
[.h2[] | {title: .text, subsections: [.children[].text]}]
```

An object value that yields one value holds that value; several become an
array. Wrap a value in `[...]` when it should always be an array, as
`subsections` is above.

**Property projection shorthand is not implemented.** Every key needs an
explicit value, so `{level, text}` fails with `Expected ':', found ','`.
Write the pairs out:

```bash
.h | {level: .level, text: .text}
.link | {text: .text, url: .url, type: .type}
```

---

## Conditionals

```bash
# If-then-else
.h | if .level == 1 then "# " + .text else .text end

# Multiple conditions
.code | if .lang == "rust" then "Rust: "
       elif .lang == "python" then "Python: "
       else "Other: " end + .text

# No else: a false condition passes the input through unchanged
.h | .text | if . == "Intro" then "Introduction" end

# Fallback
.code | .lang // "unknown"       # Default if null
```

---

## Front Matter Access

For documents with YAML front matter:

```bash
.frontmatter                    # All front matter as object
.frontmatter.title              # Specific field
.frontmatter.tags[]             # Iterate array field
.frontmatter.author.name        # Nested field
.frontmatter | keys             # List all keys
.frontmatter | has("draft")     # Check field existence
.fm | .image                    # A key that is also an element name
```

---

## Output Formats

Queries select their format with `--query-output`, **not** `-o`. The `-o` flag
applies to `--list`/`--tree` and is silently ignored alongside `-q`, so
`treemd -q '.h2' -o json` prints plain text and exits `0`. Supported values are
`plain` (default), `json`, `json-pretty` (alias `jsonp`), `jsonl`, `md`, and
`tree`.

### Plain Text (Default)

```bash
$ treemd -q '.h2' doc.md
## Introduction
## Installation
## Usage
```

### JSON

`json` and `json-pretty` always print an array, whatever the number of
results, so a script can parse the output without knowing the count in
advance. `jsonl` prints one value per line.

```bash
$ treemd -q '.h2' --query-output json doc.md
[{"type":"heading","level":2,"text":"Introduction","line":5},{"type":"heading","level":2,"text":"Installation","line":12}]

$ treemd -q '.h2[0]' --query-output json doc.md
[{"type":"heading","level":2,"text":"Introduction","line":5}]

$ treemd -q '.h2' --query-output json-pretty doc.md   # same data, indented
$ treemd -q '.h2' --query-output jsonl doc.md         # one object per line
```

Every element carries a `type` tag. The same JSON is used when an element is
printed inside an object, as in `{a: .h1[0]} | text`.

### Line-Delimited (for Piping)

```bash
$ treemd -q '.h2 | text' doc.md
Introduction
Installation
Usage
```

### Raw Markdown

```bash
$ treemd -q '.h1[Install] | md' doc.md
## Installation

Run the following command:
...
```

### Tree Format

```bash
$ treemd -q '.h' --query-output tree doc.md
├─ # Title
├─ ## Introduction
├─ ## Installation
├─ ### Prerequisites
└─ ## Usage
```

---

## Complete Examples

### Common Operations

```bash
# List all h2 headings
treemd -q '.h2' doc.md

# Extract all rust code blocks
treemd -q '.code[rust]' doc.md

# Get all external URLs
treemd -q '.link[external] | url' doc.md

# Find headings containing "API"
treemd -q '.h | select(contains("API"))' doc.md

# Extract installation section content
treemd -q '.h1[Installation] | content' doc.md

# Generate table of contents
treemd -q '.h[] | "  " * (.level - 1) + "- " + .text' doc.md

# Count headings by level
treemd -q '.h | levels' doc.md
```

### Advanced Queries

```bash
# Code blocks > 20 lines under "Examples"
treemd -q '.h1[Examples] >> .code | select(.text | lines > 20)' doc.md

# All images with alt text as JSON
treemd -q '.img | {alt: .alt, src: .src}' --query-output json doc.md

# Find TODOs in code comments
treemd -q '.code | select(contains("TODO"))' doc.md

# Links grouped by type
treemd -q '.link | group_by(.type)' doc.md

# Heading hierarchy as nested JSON
treemd -q '[.h1[] | {
  title: .text,
  sections: [.children[] | {title: .text, subsections: [.children[].text]}]
}]' --query-output json doc.md
```

### Scripting Examples

```bash
# Extract all URLs for link checking
treemd -q '.link | url' doc.md | xargs -I{} curl -sI {} | grep "HTTP"

# Word count per section
treemd -q '.h1[] | {title: .text, words: (.content | words)}' doc.md

# Find potential broken anchor links
ANCHORS=$(treemd -q '.link[anchor] | url' doc.md)
HEADINGS=$(treemd -q '.h | text | slugify' doc.md)
for anchor in $ANCHORS; do
  echo "$HEADINGS" | grep -q "^${anchor#\#}$" || echo "Missing: $anchor"
done

# Compare structure of two documents
diff <(treemd -q '.h | text' a.md) <(treemd -q '.h | text' b.md)

# Generate sitemap JSON
treemd -q '[.h[] | {level: .level, text: .text, slug: (.text | slugify)}]' --query-output json doc.md
```

---

## Error Messages

Parse errors carry a source span pointing at the offending token. All errors
go to stderr and exit `1`.

```
$ treemd -q '.h2 | | text' doc.md
error: Expected expression, found '|'
  --> query
  |
1 | .h2 | | text
          ^  unexpected token
```

Errors found while running the query are a single line with no span. An
unknown function may add a "did you mean" line:

```
$ treemd -q '.h99' doc.md
error: Property 'h99' not found on document

$ treemd -q '.h1 | nonexistent' doc.md
error: Unknown function 'nonexistent'

$ treemd -q 'count(1)' doc.md
error: Function 'count' takes no arguments, got 1
```

A filter that matches nothing is **not** an error: it prints nothing and
exits `0`, so scripts should test for empty output rather than exit status:

```
$ treemd -q '.h1[Missing]' doc.md
$ echo $?
0
```

---

## Grammar (EBNF)

The grammar the parser implements, loosest binding first.

```ebnf
query          = pipeline ( "," pipeline )* ;
pipeline       = hierarchy ( "|" hierarchy )* ;

(* ">" is the child operator only between two selectors, and greater-than
   otherwise. *)
hierarchy      = or_expr ( ( ">" | ">>" ) or_expr )* ;
or_expr        = and_expr ( "or" and_expr )* ;
and_expr       = equality ( "and" equality )* ;
equality       = comparison ( ( "==" | "!=" ) comparison )* ;
comparison     = alternative ( ( "<" | "<=" | ">" | ">=" ) alternative )* ;
alternative    = additive ( "//" additive )* ;
additive       = multiplicative ( ( "+" | "-" ) multiplicative )* ;
multiplicative = unary ( ( "*" | "/" | "%" ) unary )* ;
unary          = ( "not" | "-" ) unary
               | "not"                       (* as a stage: negates its input *)
               | postfix ;
postfix        = primary ( "." identifier | index )* ;

primary        = "." identifier filter* index?   (* selector, or key/property *)
               | "."                             (* identity *)
               | "(" pipeline ")"
               | object | array | conditional
               | string | number | "true" | "false" | "null"
               | identifier ( "(" args ")" )? ;   (* function call *)

filter         = "[" ( string | word ) "]" ;
index          = "[" ( integer | integer? ":" integer? )? "]" ;
args           = pipeline ( "," pipeline )* ;

object         = "{" ( key ":" pipeline ( "," key ":" pipeline )* )? "}" ;
key            = identifier | string ;
array          = "[" ( pipeline ( "," pipeline )* )? "]" ;
conditional    = "if" pipeline "then" pipeline
                 ( "elif" pipeline "then" pipeline )*
                 ( "else" pipeline )? "end" ;

string         = '"' ( char | escape )* '"' | "'" ( char | escape )* "'" ;
escape         = "\" ( "n" | "r" | "t" | "\" | quote ) ;
number         = digit+ ( "." digit+ )? ( ( "e" | "E" ) ( "+" | "-" )? digit+ )? ;
integer        = "-"? digit+ ;
identifier     = ( letter | "_" ) ( letter | digit | "_" )* ;
word           = identifier | keyword ;
keyword        = "and" | "or" | "not" | "if" | "then" | "elif" | "else"
               | "end" | "true" | "false" | "null" ;
```

Element names for `primary` are `h`, `h1` to `h6`, `code`, `link`, `img`,
`table`, `list`, `blockquote`, `para`, `frontmatter` and the aliases listed
under [Element Selectors](#element-selectors). A `.name` that is none of
these is a property. A `-` directly before a digit is part of the number
where an operand is expected (`-3`, `[-1]`), and subtraction otherwise
(`5-3`).

---

## Not Implemented

- Regex filters in brackets: `.h[/re/]`, `.code[/re/]`
- Navigation functions: `parent`, `children`, `siblings`, `next`, `prev`,
  `descendants` (the `.parent` and `.children` properties exist)
- Property projection shorthand: `{level, text}`
- A comma inside parentheses: `(.h1, .h2)`
- An interactive query REPL, and shell completion for query syntax
