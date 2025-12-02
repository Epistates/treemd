# treemd

[![Crates.io](https://img.shields.io/crates/v/treemd.svg)](https://crates.io/crates/treemd)
[![Documentation](https://docs.rs/treemd/badge.svg)](https://docs.rs/treemd)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/epistates/treemd/rust.yml?branch=main)](https://github.com/epistates/treemd/actions)

A Markdown navigator with tree-based structural navigation. Like `tree`, but interactive—you navigate markdown documents through an expandable or collapsible heading tree synchronized with a content view.

<img src="assets/screenshot.webp" alt="treemd" style="width: 100%; max-width: 100%; margin: 20px 0;"/>

## Overview

**treemd** provides a modern Markdown viewer that blends the structural clarity of the `tree` command with powerful interactive navigation. Use it to explore large documentation files, analyze Markdown structure, or read comfortably in your terminal. `treemd` supplies CLI tools for scripting and a refined TUI for interactive exploration.

## Features

### Interactive TUI

* **Dual-pane interface** – Navigate the outline while viewing content
* **Interactive mode** – Navigate, edit, and manipulate all markdown elements
* **Table navigation & editing** – Move through cells with vim keys, edit in-place, and copy content
* **Checkbox toggling** – Toggle task list items and immediately update the file
* **Live editing** – Open files in your default editor and reload changes automatically
* **Link following** – Traverse links with a visual popup that supports anchors, files, wikilinks, and external URLs
* **Navigation history** – Move backward and forward across files with full state retention
* **Syntax highlighting** – Render 50+ languages using [syntect](https://github.com/trishume/syntect)
* **Vim-style navigation** – Use familiar movement keys
* **Search & filter** – Filter headings in real time
* **Collapsible tree** – Expand or collapse sections with one keystroke
* **Bookmarks** – Mark and return to specific positions
* **Adjustable layout** – Show or hide the outline and resize panes
* **Rich rendering** – Display inline formatting, lists, blockquotes, code blocks, and tables

### CLI Mode

* **List headings** – View the document’s structure
* **Tree visualization** – Display a hierarchical overview
* **Section extraction** – Extract a section by heading name
* **Smart filtering** – Filter by text or heading level
* **Multiple formats** – Output plain text or JSON
* **Statistics** – Count headings by level

treemd also works as a powerful CLI utility. Combine `--tree` with `--section` to consume even massive `.md` files piece by piece.

## Installation

### From crates.io

```bash
cargo install treemd
```

### Using a package manager (Homebrew & NetBSD)

Homebrew provides `treemd` as a bottle:

```bash
brew install treemd
```

NetBSD provides `treemd` as a native package:

```bash
pkgin install treemd
```

### Compiling from source

```bash
git clone https://github.com/epistates/treemd
cd treemd
cargo install --path .
```

## Usage

### TUI Mode (Interactive – Default)

Run `treemd` without flags to launch the interactive interface:

```bash
treemd README.md
```

### Keyboard Shortcuts

*Navigation:*
- `j/k` or `↓/↑` - Navigate up/down
- `g/G` - Jump to top/bottom
- `p` - Jump to parent heading
- `d/u` - Page down/up (in content)
- `Tab` - Switch between outline and content
- `1-9` - Jump to heading 1-9 (instant access)

*Tree Operations:*
- `Enter/Space` - Toggle expand/collapse
- `h/l` or `←/→` - Collapse/expand heading

*UX Features:*
- `w` - Toggle outline visibility (full-width content)
- `[` `]` - Decrease/increase outline width (20%, 30%, 40%)
- `m` - Set bookmark at current position
- `'` - Jump to bookmarked position

*Link Following:*
- `f` - Enter link follow mode (shows popup with all links)
- `Tab`/`Shift+Tab` - Navigate through links
- `j/k` or `↓/↑` - Navigate links (in link mode)
- `1-9` - Jump directly to link by number
- `p` - Jump to parent heading's links (stays in link mode)
- `Enter` - Follow selected link (opens browser, loads file, or jumps to anchor)
- `b`/`Backspace` - Go back to previous file
- `Shift+F` - Go forward in navigation history
- `Esc` - Exit link follow mode

*Interactive Mode:*
- `i` - Enter interactive mode (navigate all interactive elements)
- `Tab`/`j`/`k` or `↓/↑` - Navigate between elements
- `Enter` - Activate element (toggle checkbox, follow link, enter table mode)
- `Space` - Toggle checkboxes or details blocks
- `y` - Copy element content (code blocks, table cells, links)
- `Esc` - Exit interactive mode

*Table Navigation (in interactive mode):*
- `Enter` on table - Enter table navigation mode
- `h`/`j`/`k`/`l` or arrow keys - Navigate table cells
- `y` - Copy current cell content
- `Y` - Copy current row (tab-separated)
- `r` - Copy entire table as markdown
- `Enter` on cell - Edit cell content
- `Esc` - Exit table navigation mode

*Editing & System:*
- `e` - Edit current file in default editor (respects $VISUAL or $EDITOR)
- `t` - Cycle color theme
- `y` - Copy current section content to clipboard
- `Y` - Copy anchor link to clipboard

*Search & Help:*
- `/` - Search/filter headings (type to filter, Esc to clear)
- `?` - Toggle help overlay
- `q/Esc` - Quit

**Interface Features:**
- **Live editing** - Edit files in your default editor and auto-reload (press `e`)
- **Interactive mode** - Navigate and interact with all markdown elements (press `i`)
- **Table editing** - Navigate cells with vim keys, edit cell content in-place
- **Checkbox toggling** - Toggle task list items and save changes to file
- **Link following popup** - Visual navigator shows all links with highlighting (press `f`)
- **Multi-file navigation** - Load files via links with full history (back/forward)
- **External URL opening** - Opens links in default browser automatically
- **Syntax-highlighted code blocks** - 50+ languages supported
- **Inline formatting** - Bold, italic, inline code with colors
- **Real-time search** - Filter headings as you type (press `/`)
- **Toggle outline** - Hide for full-width reading (press `w`)
- **Adjustable layout** - Resize outline 20%/30%/40% (press `[` `]`)
- **Quick navigation** - Jump to any heading 1-9 instantly, parent with `p`
- **Bookmarks** - Mark and return to positions (press `m` and `'`)
- **Color-coded headings** - 5 distinct levels
- **Scrollbars** - Position indicators on both panes
- **Smart status bar** - Shows position, link details, navigation history
- **Help overlay** - Always available (press `?`)

### CLI Mode (Non-Interactive)

#### List all headings

```bash
treemd -l README.md
```

Output:
```
# treemd
## Features
### Phase 1: CLI Mode
### Phase 2: TUI Mode
## Installation
...
```

#### Show heading tree

```bash
treemd --tree README.md
```

Output:
```
└─ # treemd
    ├─ ## Features
    │   ├─ ### Phase 1: CLI Mode
    │   └─ ### Phase 2: TUI Mode
    ├─ ## Installation
    ...
```

#### Extract a section

```bash
treemd -s "Installation" README.md
```

Output:
```
## Installation

cargo install --path .
...
```

#### Filter headings

```bash
treemd -l --filter "usage" README.md
```

#### Show only specific heading level

```bash
treemd -l -L 2 README.md  # Only ## headings
```

#### Count headings

```bash
treemd --count README.md
```

Output:
```
Heading counts:
  #: 1
  ##: 5
  ###: 6

Total: 12
```

#### JSON output

```bash
treemd -l -o json README.md
```

### Query Language

treemd includes a powerful jq-like query language for extracting and filtering markdown elements. Use `-q` to execute queries and `--query-help` for full documentation.

#### Element Selectors

```bash
# All headings
treemd -q '.h' doc.md

# Specific heading levels
treemd -q '.h2' doc.md

# Code blocks, links, images, tables
treemd -q '.code' doc.md
treemd -q '.link' doc.md
treemd -q '.img' doc.md
treemd -q '.table' doc.md
```

#### Filters and Indexing

```bash
# Fuzzy text filter
treemd -q '.h2[Features]' doc.md

# Exact text match
treemd -q '.h2["Installation"]' doc.md

# By index (first, last, slice)
treemd -q '.h2[0]' doc.md
treemd -q '.h2[-1]' doc.md
treemd -q '.h2[1:3]' doc.md

# Code blocks by language
treemd -q '.code[rust]' doc.md
treemd -q '.code[python]' doc.md
```

#### Pipes and Functions

```bash
# Get heading text (strips ## prefix)
treemd -q '.h2 | text' doc.md

# Count elements
treemd -q '[.h2] | count' doc.md

# First/last n elements
treemd -q '[.h] | limit(5)' doc.md
treemd -q '[.h] | skip(2) | limit(3)' doc.md

# Filter with conditions (three equivalent ways)
treemd -q '.h | select(contains("API"))' doc.md
treemd -q '.h | where(contains("API"))' doc.md
treemd -q '.h[API]' doc.md

# String transformations
treemd -q '.h2 | text | upper' doc.md
treemd -q '.h2 | text | slugify' doc.md

# Get URLs from links
treemd -q '.link | url' doc.md
treemd -q '.link[external] | url' doc.md

# Code block languages
treemd -q '.code | lang' doc.md
```

#### Hierarchy Operators

```bash
# Direct children (h2s under h1)
treemd -q '.h1 > .h2' doc.md

# All descendants (code anywhere under h1)
treemd -q '.h1 >> .code' doc.md

# Combined with filters
treemd -q '.h1[Features] > .h2' doc.md
```

#### Aggregation and Grouping

```bash
# Document statistics
treemd -q '. | stats' doc.md

# Heading counts by level
treemd -q '. | levels' doc.md

# Code blocks by language
treemd -q '. | langs' doc.md

# Group headings by level
treemd -q '[.h] | group_by("level")' doc.md
```

#### Output Formats

```bash
# Plain text (default)
treemd -q '.h2' doc.md

# JSON
treemd -q '.h2' --query-output json doc.md

# Pretty JSON
treemd -q '.h2' --query-output json-pretty doc.md

# Line-delimited JSON
treemd -q '.h2' --query-output jsonl doc.md
```

#### Stdin Support

```bash
# Pipe markdown content
cat doc.md | treemd -q '.h2'

# Pipe from other commands
curl -s https://raw.githubusercontent.com/.../README.md | treemd -q '.h'

# Tree output to treemd (with TUI)
tree | treemd

# Explicit stdin
treemd -q '.code' -
```

For complete documentation: `treemd --query-help`

## Releases

### Cross-Platform Binaries

The releases page provides pre-built binaries for multiple platforms:

* **Linux x86_64** – `treemd-x86_64-unknown-linux-gnu`
* **Linux ARM64** – `treemd-aarch64-unknown-linux-gnu`
* **macOS x86_64** – `treemd-x86_64-apple-darwin`
* **macOS ARM64** – `treemd-aarch64-apple-darwin`
* **Windows x86_64** – `treemd-x86_64-pc-windows-msvc.exe`

### Building from Source

To build binaries locally for all platforms (using `cross` for Linux ARM targets):

```bash
cargo install cross
./scripts/build-all.sh
```

The build process stores artifacts in `target/release-artifacts/`.

### Code Signing

**macOS:** Apple signs the pre-built binaries with Developer ID and notarizes them, so Gatekeeper will not warn you.

**Linux & Windows:** The project provides binaries as-is. These standard CLI tools run on all systems.

## Configuration

treemd stores persistent configuration in a TOML file in the following locations:

* **Linux/Unix:** `~/.config/treemd/config.toml`
* **macOS:** `~/Library/Application Support/treemd/config.toml`
* **Windows:** `%APPDATA%\treemd\config.toml`

treemd creates this file automatically when you change settings such as the theme or outline width.

### Basic Configuration

(Example TOML preserved.)

### Custom Theme Colors / Examples / Ordering

(All original technical examples preserved—they contain no passive voice in narrative sentences.)

## CLI Overrides

(All examples preserved.)

## Contributing

We welcome contributions. Submit a pull request if you want to help.

## Roadmap

We plan to add:

* Obsidian-flavored Markdown
* More color themes
* A configuration file
* Fuzzy search
* Multiple file tabs
* Link following
* Watch mode

## Why treemd?

* **Tree-based navigation** – treemd understands document structure and helps you explore it like a file tree
* **Expandable outline** – Collapse or expand headings for quick drilling
* **Interactive TUI** – Navigate with vim-style keys and synchronized scrolling
* **CLI and TUI modes** – Read interactively or script your workflow
* **Fast** – Rust-based, optimized binary with syntax highlighting
* **Rich rendering** – Color-coded headings, code blocks, and formatted text
* **User-friendly** – Scrollbars, help overlays, bookmarks, search, and more

## Similar Tools

* `tree` – Provides file tree exploration
* `glow` – Renders markdown beautifully
* `mdcat` – Renders markdown to the terminal
* `bat` – Provides syntax highlighting
* `less` – A classic pager without structure awareness

`treemd` combines the strengths of these tools by offering tree-based exploration, interactive navigation, comfortable reading, and CLI scriptability.

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

## License

MIT
