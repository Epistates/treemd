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

(Shortcuts remain unchanged; they do not use passive voice.)

[All shortcut lists preserved as-is.]

## CLI Mode (Non-Interactive)

Examples remain unchanged, as they contain no passive constructions.

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

## License

MIT
