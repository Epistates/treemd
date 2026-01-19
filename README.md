# treemd

[![Crates.io](https://img.shields.io/crates/v/treemd.svg)](https://crates.io/crates/treemd)
[![Documentation](https://docs.rs/treemd/badge.svg)](https://docs.rs/treemd)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/epistates/treemd/rust.yml?branch=main)](https://github.com/epistates/treemd/actions)

A markdown navigator with tree-based structural navigation. Like the `tree` command, but interactive—navigate markdown documents using an expandable/collapsible heading tree with a synchronized content view.

![treemd screenshot showing dual-pane interface](assets/output.gif)

## Table of Contents

- [Overview](#overview)
- [Features](#features)
  - [Interactive TUI](#interactive-tui)
  - [CLI Mode](#cli-mode)
- [Installation](#installation)
- [Usage](#usage)
  - [TUI Mode](#tui-mode-interactive)
  - [CLI Mode](#cli-mode-non-interactive)
  - [Query Language](#query-language)
- [Releases](#releases)
- [Configuration](#configuration)
  - [Basic Settings](#basic-configuration)
  - [Custom Keybindings](#custom-keybindings)
  - [Custom Theme Colors](#custom-theme-colors)
- [Contributing](#contributing)
- [Roadmap](#roadmap)
- [Why treemd?](#why-treemd)
- [Similar Tools](#similar-tools)
- [License](#license)

## Overview

**treemd** is a modern markdown viewer that combines the structural clarity of the `tree` command with powerful interactive navigation. Whether you're exploring large documentation files, analyzing markdown structure, or reading comfortably in your terminal, treemd provides both CLI tools for scripting and a beautiful TUI for interactive exploration.

Use it to:
- **Navigate** large documents by collapsing/expanding heading sections
- **Search** headings or full document content with highlighted matches
- **Edit** tables, toggle checkboxes, and follow links—all without leaving the terminal
- **Extract** specific sections or query markdown elements with a [jq](https://jqlang.github.io/jq/)-like syntax
- **Pipe** markdown from stdin for shell-scripted workflows

## Features

### Interactive TUI

| Feature | Description |
|---------|-------------|
| **Dual-pane interface** | Navigate the outline while viewing synchronized content |
| **Interactive mode** | Navigate, edit, and interact with tables, checkboxes, links, and code blocks |
| **Table editing** | Navigate cells with vim keys (`hjkl`), edit in-place, copy cells/rows/tables |
| **Checkbox toggling** | Toggle task list items with instant file updates |
| **Live editing** | Open files in your editor with auto-reload (respects `$VISUAL`/`$EDITOR`) |
| **Link following** | Follow markdown links via visual popup—supports anchors, files, wikilinks, and URLs |
| **Navigation history** | Go back/forward between files with full state preservation |
| **Syntax highlighting** | 50+ languages via [syntect](https://crates.io/crates/syntect) |
| **Vim-style navigation** | `j`/`k`, `g`/`G`, `d`/`u`, `p` (parent) |
| **Search & filter** | Filter headings (`s`) or search content (`/`) with `n`/`N` navigation |
| **Collapsible tree** | Expand/collapse sections with `Space`/`Enter` |
| **Bookmarks** | Mark positions (`m`) and jump back (`'`) |
| **8 color themes** | Nord, Dracula, Solarized, Monokai, Gruvbox, Tokyo Night, Catppuccin Mocha, Ocean Dark |
| **Customizable keybindings** | Remap any key via [config file](#custom-keybindings) |

### CLI Mode

| Feature | Description |
|---------|-------------|
| **[Query language](#query-language)** | jq-like syntax for extracting markdown elements |
| **List headings** | Quick overview with `-l` |
| **Tree visualization** | Hierarchical display with `--tree` |
| **Section extraction** | Extract by heading name with `-s` |
| **Smart filtering** | Filter by text or level (`--filter`, `-L`) |
| **Multiple formats** | Plain text or JSON output (`-o json`) |
| **Statistics** | Count headings by level (`--count`) |
| **Stdin support** | Pipe markdown content (`cat doc.md \| treemd -q '.h'`) |

**Pro tip:** Combine `--tree` with `--section` for rapid navigation of large files. The query language brings [jq](https://jqlang.github.io/jq/)-like power to markdown extraction.

## Installation

### From [crates.io](https://crates.io/crates/treemd)

```bash
cargo install treemd
```

### From source

```bash
git clone https://github.com/epistates/treemd
cd treemd
cargo install --path .
```

### Package managers

**[Homebrew](https://brew.sh)** (macOS/Linux):
```bash
brew install treemd
```

**[Arch Linux](https://archlinux.org/packages/extra/x86_64/treemd/)** (`extra` repo):
```bash
pacman -S treemd
```

**[NetBSD](https://pkgsrc.se/textproc/treemd)**:
```bash
pkgin install treemd
```

## Usage

### TUI Mode (Interactive)

Launch the interactive interface by running treemd with a file:

```bash
treemd README.md        # Open specific file
treemd .                # Open file picker in current directory
treemd docs/            # Open file picker in specified directory
treemd *.md             # Open file picker with matched files
```

#### Keyboard Shortcuts

<details>
<summary><strong>Navigation</strong></summary>

| Key | Action |
|-----|--------|
| `j` / `k` or `↓` / `↑` | Move down/up |
| `g` / `G` or `Home` / `End` | Jump to top/bottom |
| `p` | Jump to parent heading |
| `d` / `u` or `PageDown` / `PageUp` | Page down/up |
| `Tab` | Switch focus between outline and content |
| `1`-`9` | Jump to heading by number |

</details>

<details>
<summary><strong>Tree Operations</strong></summary>

| Key | Action |
|-----|--------|
| `Enter` / `Space` | Toggle expand/collapse |
| `h` / `l` or `←` / `→` | Collapse/expand heading |
| `w` | Toggle outline visibility |
| `[` / `]` | Adjust outline width (20%, 30%, 40%) |

</details>

<details>
<summary><strong>Search</strong></summary>

| Key | Action |
|-----|--------|
| `s` | Search/filter headings in outline |
| `/` | Search document content |
| `Tab` (in search) | Toggle between outline and content search |
| `n` / `N` | Next/previous match |
| `Esc` | Exit search mode |

</details>

<details>
<summary><strong>Link Following</strong></summary>

| Key | Action |
|-----|--------|
| `f` | Enter link follow mode |
| `Tab` / `Shift+Tab` | Navigate links |
| `1`-`9` | Jump to link by number |
| `Enter` | Follow selected link |
| `b` / `Backspace` | Go back |
| `F` (Shift+F) | Go forward |
| `Esc` | Exit link mode |

</details>

<details>
<summary><strong>Interactive Mode</strong></summary>

| Key | Action |
|-----|--------|
| `i` | Enter interactive mode |
| `Tab` / `j` / `k` | Navigate elements |
| `Enter` | Activate element |
| `Space` | Toggle checkboxes/details |
| `y` | Copy content |
| `Esc` | Exit interactive mode |

</details>

<details>
<summary><strong>Table Navigation</strong> (within interactive mode)</summary>

| Key | Action |
|-----|--------|
| `h` / `j` / `k` / `l` | Navigate cells |
| `y` | Copy cell |
| `Y` | Copy row |
| `r` | Copy table as markdown |
| `Enter` | Edit cell |
| `Esc` | Exit table mode |

</details>

<details>
<summary><strong>Editing & System</strong></summary>

| Key | Action |
|-----|--------|
| `e` | Edit file in `$VISUAL` or `$EDITOR` |
| `t` | Cycle color theme |
| `y` | Copy current section |
| `Y` | Copy anchor link |
| `m` | Set bookmark |
| `'` | Jump to bookmark |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit |

</details>

### CLI Mode (Non-Interactive)

#### List headings

```bash
treemd -l README.md
```

#### Show heading tree

```bash
treemd --tree README.md
```

#### Extract a section

```bash
treemd -s "Installation" README.md
```

#### Filter and level options

```bash
treemd -l --filter "usage" README.md    # Filter by text
treemd -l -L 2 README.md                # Only ## headings
```

#### Count and JSON output

```bash
treemd --count README.md                # Count by level
treemd -l -o json README.md             # JSON output
```

### Query Language

treemd includes a powerful [jq](https://jqlang.github.io/jq/)-like query language for extracting markdown elements. Use `-q` to execute queries and `--query-help` for full documentation.

#### Element Selectors

```bash
treemd -q '.h' doc.md         # All headings
treemd -q '.h2' doc.md        # Level 2 headings
treemd -q '.code' doc.md      # Code blocks
treemd -q '.link' doc.md      # Links
treemd -q '.img' doc.md       # Images
treemd -q '.table' doc.md     # Tables
```

#### Filters and Indexing

```bash
treemd -q '.h2[Features]' doc.md       # Fuzzy match
treemd -q '.h2["Installation"]' doc.md # Exact match
treemd -q '.h2[0]' doc.md              # First h2
treemd -q '.h2[-1]' doc.md             # Last h2
treemd -q '.h2[1:3]' doc.md            # Slice
treemd -q '.code[rust]' doc.md         # By language
```

#### Pipes and Functions

```bash
treemd -q '.h2 | text' doc.md                    # Get text (strips ##)
treemd -q '[.h2] | count' doc.md                 # Count elements
treemd -q '[.h] | limit(5)' doc.md               # First 5
treemd -q '.h | select(contains("API"))' doc.md  # Filter
treemd -q '.h2 | text | slugify' doc.md          # URL slug
treemd -q '.link | url' doc.md                   # Extract URLs
```

#### Hierarchy Operators

```bash
treemd -q '.h1 > .h2' doc.md           # Direct children
treemd -q '.h1 >> .code' doc.md        # All descendants
treemd -q '.h1[Features] > .h2' doc.md # Combined
```

#### Aggregation

```bash
treemd -q '. | stats' doc.md           # Document statistics
treemd -q '. | levels' doc.md          # Heading counts by level
treemd -q '. | langs' doc.md           # Code blocks by language
```

#### Output Formats

```bash
treemd -q '.h2' --query-output json doc.md        # JSON
treemd -q '.h2' --query-output json-pretty doc.md # Pretty JSON
treemd -q '.h2' --query-output jsonl doc.md       # JSON Lines
```

#### Stdin Support

```bash
cat doc.md | treemd -q '.h2'
curl -s https://raw.githubusercontent.com/.../README.md | treemd -q '.h'
tree | treemd                          # Pipe tree output to TUI
```

Run `treemd --query-help` for complete documentation.

## Releases

### Pre-built Binaries

Download from the [releases page](https://github.com/epistates/treemd/releases):

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `treemd-x86_64-unknown-linux-gnu` |
| Linux ARM64 | `treemd-aarch64-unknown-linux-gnu` |
| macOS x86_64 | `treemd-x86_64-apple-darwin` |
| macOS ARM64 (Apple Silicon) | `treemd-aarch64-apple-darwin` |
| Windows x86_64 | `treemd-x86_64-pc-windows-msvc.exe` |

**macOS binaries** are signed with Developer ID and notarized by Apple.

### Building from Source

```bash
cargo install cross              # For ARM cross-compilation
./scripts/build-all.sh           # Build all platforms
```

Artifacts are output to `target/release-artifacts/`.

## Configuration

treemd stores configuration in a TOML file:

| Platform | Location |
|----------|----------|
| Linux/Unix | `~/.config/treemd/config.toml` |
| macOS | `~/Library/Application Support/treemd/config.toml` |
| Windows | `%APPDATA%\treemd\config.toml` |

The file is created automatically when you change settings (theme with `t`, outline width with `[`/`]`).

### Basic Configuration

```toml
[ui]
theme = "Nord"         # OceanDark, Nord, Dracula, Solarized, Monokai, Gruvbox, TokyoNight, CatppuccinMocha
outline_width = 30     # 20, 30, or 40
tree_style = "spaced"  # "spaced" (default) or "compact" (gapless box characters)

[terminal]
color_mode = "auto"    # "auto", "rgb", or "256"

[content]
hide_frontmatter = true  # Hide YAML frontmatter (---\n...\n---) in content view
hide_latex = true        # Hide LaTeX math expressions ($...$, $$...$$, \begin{...})
```

### Custom Keybindings

Remap any key for any mode using intuitive TOML syntax. Multi-key sequences are supported.

```toml
[keybindings.Normal]
"j" = "Next"
"k" = "Previous"
"Ctrl+c" = "Quit"
"g g" = "First"          # Multi-key sequence

[keybindings.Interactive]
"Escape" = "ExitInteractiveMode"
"Tab" = "InteractiveNext"

[keybindings.Search]
"Ctrl+n" = "NextMatch"
"Ctrl+p" = "PrevMatch"
```

Available modes: `Normal`, `Help`, `ThemePicker`, `Interactive`, `InteractiveTable`, `LinkFollow`, `LinkSearch`, `Search`, `DocSearch`, `CommandPalette`, `ConfirmDialog`, `CellEdit`

See the built-in defaults in [`src/keybindings/defaults.rs`](src/keybindings/defaults.rs) for all available actions.

### Custom Theme Colors

Override any color from your base theme. Colors can be specified as:

- **Named**: `"Red"`, `"Cyan"`, `"White"`, `"DarkGray"`
- **RGB**: `{ rgb = [255, 128, 0] }`
- **Indexed**: `{ indexed = 235 }` (256-color palette)

```toml
[ui]
theme = "Nord"

[theme]
background = { rgb = [25, 25, 35] }
heading_1 = { rgb = [120, 200, 255] }
heading_2 = "LightBlue"
border_focused = "Cyan"
search_match_bg = { rgb = [100, 100, 50] }
search_match_fg = "White"
```

<details>
<summary><strong>All available color fields</strong></summary>

```toml
[theme]
# Background and text
background = { rgb = [25, 25, 35] }
foreground = { rgb = [220, 220, 230] }

# Headings (5 levels)
heading_1 = { rgb = [120, 200, 255] }
heading_2 = "LightBlue"
heading_3 = { indexed = 114 }
heading_4 = "Yellow"
heading_5 = "Gray"

# Borders and UI
border_focused = "Cyan"
border_unfocused = "DarkGray"
selection_bg = { rgb = [45, 45, 60] }
selection_fg = "White"

# Status bar
status_bar_bg = { rgb = [30, 30, 45] }
status_bar_fg = { rgb = [200, 200, 210] }

# Code
inline_code_fg = { rgb = [255, 200, 100] }
inline_code_bg = { rgb = [40, 40, 50] }
code_fence = { rgb = [150, 180, 200] }

# Text formatting
bold_fg = "White"
italic_fg = { rgb = [180, 140, 200] }
list_bullet = "Cyan"

# Blockquotes
blockquote_border = { rgb = [100, 100, 120] }
blockquote_fg = { rgb = [150, 150, 170] }

# Search highlighting
search_match_bg = { rgb = [100, 100, 50] }
search_match_fg = "White"
search_current_bg = "Yellow"
search_current_fg = "Black"
```

</details>

### CLI Overrides

Override settings for a single session:

```bash
treemd --theme Dracula README.md
treemd --color-mode 256 README.md
treemd --color-mode rgb README.md
```

## Contributing

Contributions are welcome! Please feel free to submit a [Pull Request](https://github.com/epistates/treemd/pulls).

## Roadmap

**Completed:**
- [x] Query language (jq-like syntax)
- [x] Stdin/pipe support
- [x] 8 color themes
- [x] Configuration file
- [x] Link following with history
- [x] WikiLinks support
- [x] Customizable keybindings
- [x] Live file watching (auto-reload)

**Planned:**
- [ ] Obsidian Flavored Markdown (callouts)
- [ ] Fuzzy search improvements
- [ ] Multiple file tabs
- [ ] Custom query language plugins

## Why treemd?

| Feature | treemd | [`less`](https://linux.die.net/man/1/less) | [`glow`](https://github.com/charmbracelet/glow) | [`bat`](https://github.com/sharkdp/bat) |
|---------|--------|------|------|-----|
| Tree-based navigation | Yes | No | No | No |
| Interactive exploration | Yes | Limited | No | No |
| Syntax highlighting | Yes | No | Yes | Yes |
| Markdown-aware | Yes | No | Yes | No |
| Query language | Yes | No | No | No |
| Edit tables/checkboxes | Yes | No | No | No |

**treemd** understands markdown structure. Navigate documents like a file tree, drill into sections, search content, edit tables, and follow links—all from your terminal.

## Similar Tools

- [`tree`](https://linux.die.net/man/1/tree) — File tree explorer (inspiration for outline navigation)
- [`glow`](https://github.com/charmbracelet/glow) — Beautiful markdown rendering (presentation-focused)
- [`mdcat`](https://github.com/swsnr/mdcat) — Markdown rendering to terminal
- [`bat`](https://github.com/sharkdp/bat) — Syntax highlighting pager
- [`less`](https://linux.die.net/man/1/less) — Classic pager

treemd combines the best of these: **tree-based exploration** + interactive navigation + comfortable reading + CLI scriptability.

## Acknowledgements

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

## License

[MIT](LICENSE)
