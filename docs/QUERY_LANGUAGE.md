# treemd Query Language (tql)

> **Status**: Design Specification
> **Version**: 1.0.0-draft
> **Goal**: jq-like querying for Markdown - maximum DX and utility

## Design Philosophy

1. **jq-compatible where sensible** - Same operators for same operations
2. **Markdown-native selectors** - `.h1`, `.code`, `.link` feel natural
3. **Composable** - Pipes and filters work exactly like jq
4. **Progressive complexity** - Simple things are simple, complex things possible
5. **Excellent error messages** - Rust-quality diagnostics with suggestions
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
treemd -q '.h1[Features] > .h2' doc.md          # h2s under Features

# Properties
treemd -q '.h2 | text' doc.md                   # Just heading text
treemd -q '.link | url' doc.md                  # Just URLs

# Content extraction
treemd -q '.h1[Installation] | content' doc.md  # Section content
```

---

## Element Selectors

### Headings

| Selector | Description | Example |
|----------|-------------|---------|
| `.h` | All headings (any level) | `.h` |
| `.h1` - `.h6` | Headings at specific level | `.h2` |
| `.h[text]` | Heading matching text (fuzzy) | `.h[Install]` |
| `.h["text"]` | Heading matching text (exact) | `.h["Getting Started"]` |
| `.h[/regex/]` | Heading matching regex | `.h[/Chapter \d+/]` |
| `.h1[text]` | Level + text filter combined | `.h1[Features]` |

### Code Blocks

| Selector | Description | Example |
|----------|-------------|---------|
| `.code` | All code blocks | `.code` |
| `.code[lang]` | Code blocks with language | `.code[rust]` |
| `.code["lang"]` | Same, explicit string | `.code["python"]` |
| `.code[/regex/]` | Language matching regex | `.code[/^(js\|ts)$/]` |

### Links

| Selector | Description | Example |
|----------|-------------|---------|
| `.link` | All links | `.link` |
| `.link[anchor]` | Anchor links only | `.link[anchor]` |
| `.link[external]` | External URLs only | `.link[external]` |
| `.link[relative]` | Relative file links | `.link[relative]` |
| `.link[wikilink]` | Wikilinks `[[...]]` | `.link[wikilink]` |

### Other Elements

| Selector | Description |
|----------|-------------|
| `.img` | All images |
| `.table` | All tables |
| `.list` | All lists |
| `.blockquote` | All blockquotes |
| `.para` | All paragraphs |
| `.frontmatter` | YAML front matter |

### Document

| Selector | Description |
|----------|-------------|
| `.` | Entire document |
| `. \| stats` | Document statistics |

---

## Hierarchy Navigation

### Parent-Child Relationships

```bash
# Direct children (CSS > combinator)
.h1 > .h2                    # h2s that are direct children of h1s
.h1[Features] > .h2          # h2s directly under "Features"
.h1 > .code                  # code blocks directly under any h1

# Descendants (CSS >> or space combinator)
.h1 >> .h2                   # h2s anywhere under h1s (any depth)
.h1[Features] >> .code       # code blocks anywhere under Features
.h1 >> .link                 # all links under any h1 section
```

### Navigation Functions

```bash
.h2[0] | parent              # Parent heading of first h2
.h1[0] | children            # All direct child headings
.h2[0] | siblings            # Sibling headings (same level, same parent)
.h2[0] | next                # Next heading at same level
.h2[0] | prev                # Previous heading at same level
.h1 | descendants            # All descendant headings
```

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

---

## Iteration

The `[]` operator without index iterates over all elements:

```bash
.h[]                         # Iterate over all headings
.h[] | text                  # Text of each heading (one per line)
.table[].rows[]              # All rows of all tables
.code[] | lang               # Language of each code block
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

### Multiple Outputs

Comma generates multiple outputs:

```bash
.h1, .h2                     # Both h1s and h2s
(.h1, .h2) | text            # Text of h1s and h2s
.code[rust], .code[python]   # Rust and Python code
```

---

## Property Access

### Heading Properties

| Property | Type | Description |
|----------|------|-------------|
| `.text` | string | Heading text content |
| `.level` | number | Heading level (1-6) |
| `.offset` | number | Byte offset in source |
| `.line` | number | Line number (1-indexed) |
| `.content` | string | Section content (text under heading) |
| `.md` | string | Raw markdown of section |
| `.slug` | string | URL-friendly slug |
| `.children` | array | Child headings |
| `.parent` | heading? | Parent heading (if any) |

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
| `.title` | string? | Title attribute |

---

## Filtering with `select()`

The `select(condition)` function filters elements:

```bash
# Text matching
.h | select(contains("API"))           # Headings containing "API"
.h | select(startswith("Chapter"))     # Headings starting with "Chapter"
.h | select(endswith("Guide"))         # Headings ending with "Guide"
.h | select(matches("[0-9]+"))         # Headings matching regex

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

# Content-based
.code | select(.text | lines > 10)     # Long code blocks
.h | select(.content | words > 100)    # Substantial sections
```

---

## Built-in Functions

### Collection Functions

| Function | Description | Example |
|----------|-------------|---------|
| `count` | Count elements | `.h \| count` |
| `length` | Same as count | `.code \| length` |
| `first` | First element | `.h2 \| first` |
| `last` | Last element | `.h2 \| last` |
| `nth(n)` | Element at index | `.h2 \| nth(3)` |
| `reverse` | Reverse order | `.h \| reverse` |
| `sort` | Sort alphabetically | `.h \| text \| sort` |
| `sort_by(f)` | Sort by function | `.h \| sort_by(.level)` |
| `unique` | Deduplicate | `.code \| lang \| unique` |
| `flatten` | Flatten nested arrays | `.table \| rows \| flatten` |
| `group_by(f)` | Group by function | `.h \| group_by(.level)` |

### String Functions

| Function | Description | Example |
|----------|-------------|---------|
| `upper` | Uppercase | `.h \| text \| upper` |
| `lower` | Lowercase | `.h \| text \| lower` |
| `trim` | Trim whitespace | `.h \| text \| trim` |
| `split(s)` | Split string | `.h \| text \| split(" ")` |
| `join(s)` | Join array | `.h \| text \| join(", ")` |
| `replace(a,b)` | Replace substring | `.h \| text \| replace("-", " ")` |
| `slugify` | URL-friendly slug | `.h \| text \| slugify` |

### Content Functions

| Function | Description | Example |
|----------|-------------|---------|
| `text` | Get text content | `.h \| text` |
| `content` | Section content | `.h1[Install] \| content` |
| `md` | Raw markdown | `.h1[Install] \| md` |
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

### Boolean Functions

| Function | Description | Example |
|----------|-------------|---------|
| `contains(s)` | Contains substring | `select(contains("API"))` |
| `startswith(s)` | Starts with | `select(startswith("Chapter"))` |
| `endswith(s)` | Ends with | `select(endswith("Guide"))` |
| `matches(r)` | Regex match | `select(matches("[0-9]+"))` |
| `empty` | Is empty | `select(.content \| empty)` |
| `any(f)` | Any element matches | `.code \| any(.lang == "rust")` |
| `all(f)` | All elements match | `.h \| all(.level <= 3)` |

---

## Construction (JSON Output)

Build custom JSON structures:

```bash
# Object construction
{title: .h1[0].text}
{title: .h1[0].text, count: (.h | count)}

# Array construction
[.h2[].text]
[.h | {level, text}]

# Nested structure
{
  title: .h1[0].text,
  sections: [.h2[] | {title: .text, subsections: [.children[].text]}]
}

# Property projection (shorthand)
.h | {level, text}              # {level: .level, text: .text}
.link | {text, url, type}
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

# Null coalescing
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
```

---

## Output Formats

### Plain Text (Default)

```bash
$ treemd -q '.h2' doc.md
## Introduction
## Installation
## Usage
```

### JSON

```bash
$ treemd -q '.h2' -o json doc.md
[
  {"level": 2, "text": "Introduction", "line": 5},
  {"level": 2, "text": "Installation", "line": 12},
  {"level": 2, "text": "Usage", "line": 25}
]
```

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
$ treemd -q '.h' -o tree doc.md
├─ # Title
│  ├─ ## Introduction
│  ├─ ## Installation
│  │  └─ ### Prerequisites
│  └─ ## Usage
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
treemd -q '.img | {alt, src}' -o json doc.md

# Find TODOs in code comments
treemd -q '.code | select(.text | contains("TODO"))' doc.md

# Links grouped by type
treemd -q '.link | group_by(.type)' doc.md

# First code block in each h2 section
treemd -q '.h2[] | (.text, (.children | .code[0] | .text // "none"))' doc.md

# Heading hierarchy as nested JSON
treemd -q '[.h1[] | {
  title: .text,
  sections: [.children[] | {title: .text, subsections: [.children[].text]}]
}]' doc.md
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
treemd -q '[.h[] | {level, text, slug: (.text | slugify)}]' -o json doc.md
```

---

## Error Messages

treemd provides helpful, Rust-quality error messages:

```
$ treemd -q '.h99' doc.md
error: Invalid heading level '.h99'
  --> query:1:2
  |
1 | .h99
  |  ^^^ heading levels must be 1-6, or use '.h' for any level
  |
help: did you mean '.h6'?

$ treemd -q '.h1[Missing]' doc.md
warning: No heading matches 'Missing'
  --> doc.md
  |
  = note: available h1 headings: "Introduction", "Installation", "Usage"
  = help: use '.h1' to list all h1 headings

$ treemd -q '.h1 | nonexistent' doc.md
error: Unknown function 'nonexistent'
  --> query:1:8
  |
1 | .h1 | nonexistent
  |       ^^^^^^^^^^^ not a recognized function
  |
help: similar functions: 'next', 'nth', 'not'
      see 'treemd --query-help' for all functions
```

---

## Grammar (EBNF)

```ebnf
query       = expression ( "|" expression )* ;
expression  = term ( "," term )* ;
term        = selector | function | construction | conditional ;

selector    = "." ( element filter* index? | property ) ;
element     = "h" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            | "code" | "link" | "img" | "table" | "list"
            | "blockquote" | "para" | "frontmatter" ;

filter      = "[" filter_expr "]"
            | ">" selector
            | ">>" selector ;
filter_expr = string | regex | number | slice | identifier ;

index       = "[" ( number | slice ) "]" ;
slice       = number? ":" number? ;

property    = identifier ( "." property )? ;
function    = identifier ( "(" args ")" )? ;
args        = expression ( "," expression )* ;

construction = object | array ;
object      = "{" ( pair ( "," pair )* )? "}" ;
pair        = ( identifier | string ) ":" expression ;
array       = "[" ( expression ( "," expression )* )? "]" ;

conditional = "if" expression "then" expression
              ( "elif" expression "then" expression )*
              ( "else" expression )? "end" ;

string      = '"' [^"]* '"' | "'" [^']* "'" ;
regex       = "/" [^/]* "/" ;
number      = "-"? [0-9]+ ;
identifier  = [a-zA-Z_] [a-zA-Z0-9_]* ;
```

---

## Implementation Phases

### Phase 1: Core (MVP)
- Element selectors: `.h`, `.h1-6`, `.code`, `.link`, `.img`, `.table`
- Basic filters: `[text]`, `[lang]`, `[type]`
- Indexing: `[0]`, `[-1]`, `[0:3]`
- Properties: `.text`, `.level`, `.url`, `.lang`
- Pipes: `|`
- Functions: `text`, `content`, `count`, `select`, `contains`
- Output: plain, json

### Phase 2: Navigation
- Hierarchy: `>`, `>>`
- Navigation: `parent`, `children`, `siblings`, `next`, `prev`
- More properties: `.line`, `.offset`, `.slug`
- More filters: `startswith`, `endswith`, `matches`

### Phase 3: Advanced
- Construction: `{}`, `[]`
- Conditionals: `if-then-else`
- String functions: `upper`, `lower`, `split`, `join`, `replace`
- Aggregations: `levels`, `langs`, `types`, `stats`
- Front matter: `.frontmatter`

### Phase 4: Polish
- Iteration: `.[]`
- Multiple outputs: `,`
- Complex filters: `and`, `or`, `not`
- Advanced functions: `sort`, `unique`, `group_by`, `flatten`
- REPL mode
- Shell completions for query syntax
