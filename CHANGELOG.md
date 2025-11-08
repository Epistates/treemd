# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-11-08 - Initial Release 🚀

A modern markdown navigator with tree-based structural navigation and syntax highlighting.

### Features

#### 🎨 Interactive TUI

- **Dual-pane interface** - Navigate outline while viewing content
- **Syntax highlighting** - 50+ languages with full syntect integration
- **Vim-style navigation** - j/k, g/G, d/u for efficient browsing
- **Search & filter** - Press `/` to filter headings in real-time
- **Collapsible tree** - Expand/collapse sections with Space/Enter
- **Bookmarks** - Mark positions (`m`) and jump back (`'`)
- **Adjustable layout** - Toggle outline visibility, resize panes (20%, 30%, 40%)
- **Rich rendering** - Bold, italic, inline code, lists, blockquotes, code blocks
- **8 Beautiful Themes** - Ocean Dark (default), Nord, Dracula, Solarized, Monokai, Gruvbox, Tokyo Night, Catppuccin Mocha
- **Clipboard integration** - Copy section content (`y`) or anchor links (`Y`)
- **Help overlay** - Press `?` for keyboard shortcuts
- **Scrollbars** - Visual position indicators

#### ⚡ CLI Mode

- **List headings** - Quick overview of document structure (`-l`)
- **Tree visualization** - Hierarchical display with box-drawing (`--tree`)
- **Section extraction** - Extract specific sections by heading name (`-s`)
- **Smart filtering** - Filter by text or heading level (`--filter`, `-L`)
- **Multiple formats** - Plain text, JSON output (`-o json`)
- **Statistics** - Count headings by level (`--count`)

### Technical

- Built with Rust for performance and reliability
- Ratatui 0.29 for beautiful TUI rendering
- Syntect 5.2 for syntax highlighting
- Pulldown-cmark 0.13 for markdown parsing
- Arboard 3.4 for cross-platform clipboard support
- Optimized release binary with LTO and size optimization
- Comprehensive documentation on docs.rs
- MIT licensed
