---
name: treemd
description: Markdown document analysis and navigation using the treemd CLI. Use when exploring markdown structure (heading trees, section extraction), querying markdown elements via tql (treemd query language), or piping markdown content for programmatic analysis. Also use before reading or editing large markdown files to survey structure, locate relevant sections, and avoid loading entire documents into context.
version: 1.0.1
homepage: https://github.com/Epistates/treemd
license: MIT
---

# treemd Skill

Skill for working with the `treemd` markdown viewer and query tool.

## Overview

`treemd` is a Rust-based CLI for markdown document analysis. It handles two primary workflows:

1. **Structural Navigation** — Explore document hierarchy top-down
2. **Query Navigation** — Navigate by asking structured questions

For scripted/agent tasks, always use CLI mode. TUI mode is reserved for human interactive viewing.

Verify that the installed CLI is available and record its version before relying on version-sensitive tql or output-format behavior:

```bash
command -v treemd >/dev/null && treemd --version
```

> **Project**: https://github.com/Epistates/treemd  
> **Install**: `cargo install treemd` or download binary from [releases](https://github.com/Epistates/treemd/releases)

---

## Line A: Structural Navigation

Use this when encountering an unfamiliar document. Progress from overview → locate → extract.

### Step 1: Overview

Understand the document skeleton before diving in.

```bash
treemd --tree FILE.md              # Visual tree with box-drawing characters
treemd --count FILE.md             # Heading count by level (h1–h6 breakdown + total)
treemd -l FILE.md | head -20       # Quick scan of all headings
```

### Step 2: Locate

Pinpoint the sections relevant to your goal.

```bash
treemd -l --filter "install" FILE.md        # Fuzzy heading search (case-insensitive)
treemd -l -L 2 --filter "API" FILE.md      # Narrow by heading level + keyword
treemd --at-line 150 FILE.md               # "Which heading covers line 150?"
```

### Step 3: Extract

Pull entire sections or pipe content for downstream processing.

```bash
treemd -s "Full Heading Text" FILE.md     # Extract heading + content (must match full heading text exactly)
cat FILE.md | treemd -s "Full Heading Text" -  # Pipe stdin, extract from stream
```

> **Important**: `-s` requires the **exact full heading text** (including emoji and parentheses). Partial/fuzzy matches like `treemd -s "Installation"` will return `Section 'Installation' not found`. The `-o` flag has no effect in `-s` mode — output is always plain markdown.

#### Output Format Options

Attach `-o` to `--list` or `--tree` only (not `-s`):

- `-o plain`: Human-readable text (default)
- `-o json`: JSON array for scripting/parsing (works with `--list` and `--tree`)
- `-o tree`: Box-drawing tree structure (`--tree` only; using with `--list` returns an error)

For tql (`-q`) queries, use `--query-output` instead of `-o`. Available formats:

- `--query-output plain`: Human-readable text (default)
- `--query-output json`: Compact JSON
- `--query-output json-pretty`: Pretty-printed JSON
- `--query-output jsonl`: Line-delimited JSON
- `--query-output md`: Raw markdown rendering
- `--query-output tree`: Box-drawing tree structure

---

## Line B: Query Navigation

Use this when you already know what to look for. Jump directly to answers via the tql (treemd query language) — a jq-like markdown DOM traversal engine.

### Element Selectors

Query syntax mirrors CSS/JQuery selectors operating on markdown AST:

```bash
treemd -q '.h2' FILE.md                # All h2 headings
treemd -q '.code[rust]' FILE.md        # Rust code block elements
treemd -q '.link | url' FILE.md        # All link URLs (pipe extraction)
treemd -q '.h2 | text' FILE.md         # Strip markdown syntax, get plain text
```

### Hierarchy & Filters

Navigate parent-child relationships and apply predicate filters:

```bash
treemd -q '.h1[Features] > .h2' FILE.md           # Direct child h2 under "Features"
treemd -q '.h1 >> .code' FILE.md                  # Code blocks anywhere under h1
treemd -q '.h | select(contains("API"))' FILE.md  # Headings containing "API"
treemd -q '[.h2] | limit(5)' FILE.md              # First 5 h2 elements
```

### Aggregation & Document Statistics

```bash
treemd -q 'stats' FILE.md              # Document metrics (headings, links, code blocks)
treemd -q 'levels' FILE.md             # Heading count per level
treemd -q 'langs' FILE.md              # Code block language distribution
treemd -q '[.h2] | count' FILE.md      # Total h2 count
```

> **Note**: Aggregation functions (`stats`, `levels`, `langs`, `types`) do not require `. |` prefix — use them directly as shown above.

#### tql Query Output Formats

Use `--query-output` for tql results:

```bash
treemd -q '.h2 | text' --query-output json FILE.md      # Compact JSON
treemd -q '.h2 | text' --query-output json-pretty FILE.md # Pretty-printed JSON
treemd -q '.link' --query-output jsonl FILE.md          # Line-delimited JSON
treemd -q '.h2 | text' --query-output md FILE.md       # Markdown rendering
treemd -q '.h1' --query-output tree FILE.md              # Box-drawing tree
```

Full tql syntax reference: `references/query-language.md`.

---

## Stdin Input (CLI Mode)

```bash
cat README.md | treemd -l -            # Read markdown from stdin
```

## TUI Mode

Reserved for human interactive sessions only. Not usable by agents.

```bash
treemd FILE.md          # Launch dual-pane interactive viewer
treemd ./docs/          # Open file picker in directory
treemd *.md             # Open multi-file picker
```

Keybindings: vim-style (`j`/`k` for up/down, `h`/`l` for collapse/expand, `/` for search, `q` for quit).

Themes: `--theme <OceanDark|Nord|Dracula|Solarized|Monokai|Gruvbox|TokyoNight|CatppuccinMocha>`

## Integration Patterns

### Pattern: Extract Section for Analysis

```bash
SECTION=$(treemd -s "Full Heading Text (with emoji)" README.md)  # Must use exact full heading
```

### Pattern: Extract Heading Section Content (tql)

```bash
treemd -q '.h1["API Reference"] | content' FILE.md   # Full section under heading
```

### Pattern: Heading Tree as JSON

```bash
treemd --tree -o json FILE.md | jq '.'
```

### Pattern: Find Specific Headings

```bash
# Structural line
treemd -l --filter "Config" FILE.md

# Query line (equivalent)
treemd -q '.h | select(contains("Config"))' FILE.md
```

### Pattern: Document Statistics Pipeline

```bash
treemd -q 'stats' --query-output json FILE.md | jq '.code_blocks'
treemd -q 'levels' --query-output json FILE.md | jq '.h1'
```

> **Note**: Aggregation queries output plain text by default. Always add `--query-output json` when piping to `jq`.

## Error Handling

- **Missing section**: `treemd -s "NonExistent" FILE.md` exits with code `1` and prints `Section 'NonExistent' not found` to stderr. Check for non-zero exit code to detect missing sections.
- **Invalid tql syntax**: Exits with a non-zero code and an error message on stderr.
- **Unsupported `--query-output` value**: Exits with error "Unknown output format" — use supported values only (`plain`, `json`, `json-pretty`, `jsonl`, `md`, `tree`).

## Notes

- File picker filters to `.md` / `.markdown` extensions only
- tql supports element selectors, hierarchy operators (`>`, `>>`), pipes (`|`), and collection/string/filter/aggregation functions
- JSON output is compatible with `jq` pipelines
- Run `treemd --query-help` for the complete built-in tql reference (same content as `references/query-language.md`)
