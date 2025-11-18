# treemd

[![Crates.io](https://img.shields.io/crates/v/treemd.svg)](https://crates.io/crates/treemd)
[![Documentation](https://docs.rs/treemd/badge.svg)](https://docs.rs/treemd)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/epistates/treemd/rust.yml?branch=main)](https://github.com/epistates/treemd/actions)

A markdown navigator with tree-based structural navigation. Like `tree`, but interactive—navigate markdown documents using an expandable/collapsible heading tree with a synchronized content view.

<img src="assets/screenshot.webp" alt="treemd" style="width: 100%; max-width: 100%; margin: 20px 0;"/>

## Overview

**treemd** is a modern markdown viewer that combines the structural clarity of the `tree` command with powerful interactive navigation. Whether you're exploring large documentation files, analyzing markdown structure, or just reading comfortably in your terminal, treemd provides both CLI tools for scripting and a beautiful TUI for interactive exploration.

## Features

### Interactive TUI

- **Dual-pane interface** - Navigate outline while viewing content
- **Live editing** - Edit files in default editor with auto-reload (respects $VISUAL/$EDITOR)
- **Link following** - Follow markdown links with visual popup, supports anchor/file/wikilink/external URLs
- **Navigation history** - Back/forward between files with full state preservation
- **Syntax highlighting** - 50+ languages with full syntect integration
- **Vim-style navigation** - j/k, g/G, d/u, p (parent) for efficient browsing
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

**treemd** is incredibly powerful as a CLI utility. Using the `--tree` visualizations with `--section` enables rapid piecewise consumption of even the largest `.md` files.

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

## Releases

### Cross-Platform Binaries

Pre-built binaries for multiple platforms are available on the [releases page](https://github.com/epistates/treemd/releases). Supported platforms:

- **Linux x86_64** - `treemd-x86_64-unknown-linux-gnu`
- **Linux ARM64** - `treemd-aarch64-unknown-linux-gnu`
- **macOS x86_64** - `treemd-x86_64-apple-darwin`
- **macOS ARM64** (Apple Silicon) - `treemd-aarch64-apple-darwin`
- **Windows x86_64** - `treemd-x86_64-pc-windows-msvc.exe`

### Building from Source

To build binaries locally for all platforms (requires `cross` for Linux ARM targets):

```bash
# Install cross for Linux ARM support
cargo install cross

# Build all platforms
./scripts/build-all.sh
```

Artifacts will be in `target/release-artifacts/`.

#### Code Signing

**macOS:** Pre-built binaries are signed with Developer ID and notarized by Apple, so you won't see any Gatekeeper warnings.

**Linux & Windows:** Binaries are provided as-is. These are standard CLI tools that work on all systems.

## Configuration

treemd supports persistent configuration through a TOML file located at:
- **Linux/Unix**: `~/.config/treemd/config.toml`
- **macOS**: `~/Library/Application Support/treemd/config.toml`
- **Windows**: `%APPDATA%\treemd\config.toml`

The configuration file is created automatically when you change settings (like theme selection with `t` or outline width with `[` `]`).

### Basic Configuration

```toml
[ui]
theme = "Nord"              # Selected base theme
outline_width = 30          # Outline panel width (20, 30, or 40)

[terminal]
color_mode = "auto"         # Color mode: "auto", "rgb", or "256"
warned_terminal_app = false # Whether Terminal.app warning was shown
```

### Custom Theme Colors

You can override any color from your selected base theme by adding a `[theme]` section. This is perfect for personalizing your favorite theme or adapting it to your terminal's color scheme.

#### Color Format Options

Colors can be specified in three formats:

1. **Named colors**: `"Red"`, `"Cyan"`, `"White"`, `"DarkGray"`, etc.
2. **RGB colors**: `{ rgb = [red, green, blue] }` (0-255 for each value)
3. **Indexed colors**: `{ indexed = 235 }` (0-255 for 256-color palette)

#### Available Color Fields

All color fields are optional—only override what you want to change:

```toml
[ui]
theme = "Nord"  # Start with Nord as base

[theme]
# Override specific colors while keeping the rest from Nord
background = { rgb = [25, 25, 35] }        # Darker background
foreground = { rgb = [220, 220, 230] }     # Lighter text

# Headings (5 levels)
heading_1 = { rgb = [120, 200, 255] }      # Custom cyan
heading_2 = "LightBlue"                     # Named color
heading_3 = { indexed = 114 }               # 256-color palette
heading_4 = "Yellow"
heading_5 = "Gray"

# Borders and UI elements
border_focused = "Cyan"                     # Active pane border
border_unfocused = "DarkGray"               # Inactive pane border
selection_bg = { rgb = [45, 45, 60] }       # Selection highlight
selection_fg = "White"

# Status bar
status_bar_bg = { rgb = [30, 30, 45] }
status_bar_fg = { rgb = [200, 200, 210] }

# Code styling
inline_code_fg = { rgb = [255, 200, 100] }  # Inline `code` color
inline_code_bg = { rgb = [40, 40, 50] }
code_fence = { rgb = [150, 180, 200] }      # Code block fence

# Text formatting
bold_fg = "White"
italic_fg = { rgb = [180, 140, 200] }
list_bullet = "Cyan"

# Blockquotes
blockquote_border = { rgb = [100, 100, 120] }
blockquote_fg = { rgb = [150, 150, 170] }
```

#### Example: Dark Blue Custom Theme

```toml
[ui]
theme = "OceanDark"  # Start with OceanDark

[theme]
# Make it even darker with more blue tint
background = { rgb = [15, 18, 25] }
foreground = { rgb = [200, 210, 220] }
heading_1 = { rgb = [80, 180, 255] }
heading_2 = { rgb = [100, 200, 255] }
heading_3 = { rgb = [120, 220, 255] }
selection_bg = { rgb = [30, 35, 50] }
border_focused = { rgb = [80, 180, 255] }
```

#### Example: High Contrast

```toml
[ui]
theme = "Dracula"

[theme]
# Maximize contrast for accessibility
background = { rgb = [0, 0, 0] }
foreground = { rgb = [255, 255, 255] }
heading_1 = { rgb = [0, 255, 255] }       # Bright cyan
heading_2 = { rgb = [255, 255, 0] }       # Bright yellow
heading_3 = { rgb = [0, 255, 0] }         # Bright green
border_focused = { rgb = [255, 0, 255] }  # Bright magenta
selection_bg = { rgb = [50, 50, 50] }
selection_fg = { rgb = [255, 255, 255] }
```

#### Color Application Order

Colors are applied in this order:
1. **Base theme** - One of 8 built-in themes (OceanDark, Nord, Dracula, etc.)
2. **Custom overrides** - Your `[theme]` section colors (optional)
3. **Color mode conversion** - Automatic RGB → 256-color on incompatible terminals

This means your custom RGB colors will automatically degrade gracefully on terminals that don't support true color.

### CLI Overrides

You can override settings for a single session using command-line flags:

```bash
# Use a different theme for this session
treemd --theme Dracula README.md

# Force 256-color mode (useful for testing or screenshots)
treemd --color-mode 256 README.md

# Force RGB mode (override terminal detection)
treemd --color-mode rgb README.md
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
