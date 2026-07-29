# treemd Query Language (tql) Reference

A jq-like query language for navigating and extracting markdown structure.

## Element Selectors

| Selector          | Description              |
| ----------------- | ------------------------ |
| `.h`, `.heading`  | All headings (any level) |
| `.h1` - `.h6`     | Headings by level        |
| `.code`           | All code blocks          |
| `.code[rust]`     | Code blocks by language  |
| `.link`, `.a`     | All links                |
| `.link[external]` | External links only      |
| `.img`            | All images               |
| `.table`          | All tables               |
| `.list`           | All lists                |
| `.blockquote`     | All blockquotes          |

## Filters & Indexing

```
.h2[Features]        # Heading containing "Features" (fuzzy match)
.h2["Installation"]  # Heading with exact text (must match full heading including emoji)
.h2[0]               # First h2
.h2[-1]              # Last h2
.h2[1:3]             # h2s at index 1 and 2
.h2[:3]              # First 3 h2s
```

## Hierarchy Operators

```
.h1 > .h2            # Direct child h2s under h1s
.h1 >> .code         # Code blocks anywhere under h1s (descendant)
```

## Pipes

```
.h2 | text           # Get heading text (strips ##)
[.h2] | count       # Count all h2s
.code | lang        # Get code block languages
.link | url         # Get link URLs
```

## Collection Functions

| Function              | Description                   | Alias                   |
| --------------------- | ----------------------------- | ----------------------- |
| `count`               | Count elements                | `length`, `len`, `size` |
| `first`               | First element                 | `head`                  |
| `last`                | Last element                  |                         |
| `limit(n)`, `take(n)` | First n elements              |                         |
| `skip(n)`, `drop(n)`  | Skip first n elements         |                         |
| `nth(n)`              | Get element at index          |                         |
| `reverse`             | Reverse order                 |                         |
| `sort`                | Sort alphabetically           |                         |
| `sort_by(key)`        | Sort by property              |                         |
| `unique`              | Remove duplicates             |                         |
| `flatten`             | Flatten nested arrays         |                         |
| `group_by(key)`       | Group elements by key         |                         |
| `min`, `max`          | Min/max numeric value         |                         |
| `add`                 | Sum numbers or concat strings |                         |

## String Functions

| Function                  | Description             |
| ------------------------- | ----------------------- |
| `text`                    | Get text representation |
| `upper`, `lower`          | Case conversion         |
| `trim`                    | Strip whitespace        |
| `split(sep)`              | Split by separator      |
| `join(sep)`               | Join with separator     |
| `replace(a, b)`           | Replace substring       |
| `slugify`                 | URL-friendly slug       |
| `lines`, `words`, `chars` | Count lines/words/chars |

## Filter Functions

| Function         | Description             | Alias             |
| ---------------- | ----------------------- | ----------------- |
| `select(cond)`   | Keep if condition true  | `where`, `filter` |
| `contains(s)`    | Contains substring      | `includes`        |
| `startswith(s)`  | Starts with prefix      |                   |
| `endswith(s)`    | Ends with suffix        |                   |
| `matches(regex)` | Matches regex pattern   |                   |
| `any`, `all`     | Check if any/all truthy |                   |
| `not`            | Negate boolean          |                   |

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

## Examples

```bash
# List all h2 headings
treemd -q '.h2' doc.md

# Get heading text only
treemd -q '.h2 | text' doc.md

# Count headings
treemd -q '[.h2] | count' doc.md

# First 5 headings
treemd -q '[.h] | limit(5)' doc.md

# Filter headings (three equivalent ways)
treemd -q '.h | select(contains("API"))' doc.md
treemd -q '.h | where(contains("API"))' doc.md
treemd -q '.h[API]' doc.md

# All Rust code blocks
treemd -q '.code[rust]' doc.md

# h2s under "Features" section
treemd -q '.h1[Features] > .h2' doc.md

# All link URLs
treemd -q '.link | url' doc.md

# Document stats
stats | .words       # Extract 'words' field (use dot notation, not 'stats | words')

# JSON output
treemd -q '.h2 | text' --query-output json doc.md
```
