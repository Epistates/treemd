# treemd

[![Crates.io](https://img.shields.io/crates/v/treemd.svg)](https://crates.io/crates/treemd)
[![Documentation](https://docs.rs/treemd/badge.svg)](https://docs.rs/treemd)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/epistates/treemd/ci.yml?branch=main)](https://github.com/epistates/treemd/actions)

A markdown navigator with tree-based structural navigation. Like `tree`, but interactive—navigate markdown documents using an expandable/collapsible heading tree with a synchronized content view.

<img src="assets/screenshot.png" alt="treemd" style="width: 100%; max-width: 100%; margin: 20px 0;"/>

## Overview

**treemd** is a modern markdown viewer that combines the structural clarity of the `tree` command with powerful interactive navigation. Whether you're exploring large documentation files, analyzing markdown structure, or just reading comfortably in your terminal, treemd provides both CLI tools for scripting and a beautiful TUI for interactive exploration.

## Features

### Interactive TUI

- **Dual-pane interface** - Navigate outline while viewing content
- **Syntax highlighting** - 50+ languages with full syntect integration
- **Vim-style navigation** - j/k, g/G, d/u for efficient browsing
- **Search & filter** - Press `/` to filter headings in real-time
- **Collapsible tree** - Expand/collapse sections with Space/Enter
- **Bookmarks** - Mark positions (`m`) and jump back (`'`)
- **Adjustable layout** - Toggle outline visibility, resize panes
- **Rich rendering** - Bold, italic, inline code, lists, blockquotes, code blocks

### CLI Mode

- **List headings** - Quick overview of document structure
- **Tree visualization** - Hierarchical display with box-drawing
- **Section extraction** - Extract specific sections by heading name
- **Smart filtering** - Filter by text or heading level
- **Multiple formats** - Plain text, JSON output
- **Statistics** - Count headings by level

## Installation

### From crates.io

```bash
cargo install treemd
```

### From source

```bash
git clone https://github.com/epistates/treemd
cd treemd
cargo install --path .
```

## Usage

### TUI Mode (Interactive - Default)

Simply run treemd without flags to launch the interactive interface:

```bash
treemd README.md
```

**Keyboard Shortcuts:**

*Navigation:*
- `j/k` or `↓/↑` - Navigate up/down
- `g/G` - Jump to top/bottom
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

*Search & Help:*
- `/` - Search/filter headings (type to filter, Esc to clear)
- `?` - Toggle help overlay
- `q/Esc` - Quit

**Interface Features:**
- **Syntax-highlighted code blocks** - 50+ languages supported
- **Inline formatting** - Bold, italic, inline code with colors
- **Real-time search** - Filter headings as you type (press `/`)
- **Toggle outline** - Hide for full-width reading (press `w`)
- **Adjustable layout** - Resize outline 20%/30%/40% (press `[` `]`)
- **Quick navigation** - Jump to any heading 1-9 instantly
- **Bookmarks** - Mark and return to positions (press `m` and `'`)
- **Color-coded headings** - 5 distinct levels
- **Scrollbars** - Position indicators on both panes
- **Smart status bar** - Shows position, outline width, bookmark status
- **Help overlay** - Always available (press `?`)

### CLI Mode (Non-Interactive)

### List all headings

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

### Show heading tree

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

### Extract a section

```bash
treemd -s "Installation" README.md
```

Output:
```
## Installation

cargo install --path .
...
```

### Filter headings

```bash
treemd -l --filter "usage" README.md
```

### Show only specific heading level

```bash
treemd -l -L 2 README.md  # Only ## headings
```

### Count headings

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

### JSON output

```bash
treemd -l -o json README.md
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Roadmap

Future enhancements planned:
- Obsidian Flavored Markdown (callouts, wikilinks)
- Multiple color themes (Nord, Dracula, Solarized)
- Configuration file support
- Fuzzy search
- Multiple file tabs
- Link following
- Watch mode (auto-reload on file change)

## Why treemd?

- **Tree-based navigation**: Unlike `less` or `cat`, treemd understands document structure and lets you explore it like a file tree
- **Expandable outline**: Drill down into sections by collapsing/expanding headings—just like `tree` command
- **Interactive TUI**: Beautiful dual-pane interface with vim-style navigation and synchronized scrolling
- **CLI and TUI modes**: Use interactively for reading or in scripts for extraction/filtering
- **Fast**: Built in Rust, optimized binary with syntax highlighting
- **Rich rendering**: Color-coded headings, syntax-highlighted code blocks (50+ languages), styled inline formatting
- **User-friendly**: Scrollbars, help overlays, bookmarks, and fuzzy search

## Similar Tools

- `tree` - File tree explorer (inspiration for outline navigation)
- `glow` - Beautiful markdown rendering (presentation-focused, not interactive)
- `mdcat` - Markdown rendering to terminal (no navigation)
- `bat` - Syntax highlighting pager (not markdown-aware)
- `less` - Classic pager (no structure awareness)

treemd combines the best of these: **tree-based exploration** + interactive navigation + comfortable reading + CLI scriptability.

## License

MIT
