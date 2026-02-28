# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.8] - 2026-02-28

### Added

- **Folder navigation in file picker** - Browse directories without leaving the TUI ([#43](https://github.com/Epistates/treemd/issues/43))
  - Subdirectories shown at bottom of file picker with `[DIR]` prefix and separator
  - Press Enter on a directory to navigate into it
  - Press Backspace to go to parent directory (works in both browse and search modes)
  - Search filter applies to both files and directories

- **Auto-hide outline for single files** - Outline panel hides when directory contains 0-1 markdown files ([#43](https://github.com/Epistates/treemd/issues/43))
  - Automatically detected at startup and when navigating directories
  - Can still be toggled manually with keybinding

### Fixed

- **Table crash at wide terminals** - Fixed panic when rendering tables at terminal width >= 146 ([#43](https://github.com/Epistates/treemd/issues/43))
  - `MIN_COL_WIDTH` clamping could push total column width over budget after proportional shrink
  - Added iterative trim loop to guarantee convergence
  - Fixed infinite loop in padding reduction when `needed_reduction < col_count`

- **EOF scroll accuracy** - Scroll now stops precisely when last line reaches viewport bottom ([#43](https://github.com/Epistates/treemd/issues/43))
  - Uses ratatui's `Paragraph::line_count()` for accurate visual line count after word-wrapping
  - Previously used raw line count, causing ~22% premature stop on wrapped content
  - Consistent behavior across Down, Page Down, End, and all scroll methods

- **Inline LaTeX command filtering** - Commands now stripped when appearing inline, not just on their own line ([#43](https://github.com/Epistates/treemd/issues/43))
  - Handles: `\begin{...}`, `\end{...}`, `\fontsize{...}{...}`, `\pagestyle{...}`, `\setlength{...}`, `\usepackage{...}`, `\renewcommand{...}`, `\newcommand{...}`, `\sethlcolor{...}`, `\titlespacing{...}`, `\pagenumbering{...}`, `\thispagestyle{...}`, and more
  - Strips bare commands like `\Box$`, `\no`, `\yes` that produce no visible content
  - Inline font size commands (`\small`, `\large`, etc.) now removed when not on their own line

- **Gapless cursor** - Command palette cursor uses reverse-video space instead of `█` for gap-free rendering ([#43](https://github.com/Epistates/treemd/issues/43))

### Changed

- **Dependencies updated**
  - `ratatui` 0.30: added `unstable-rendered-line-info` feature for accurate line counting
  - `gif` 0.13 → 0.14
  - `strum` 0.27 → 0.28

### Technical

- **Performance: LaTeX regex caching** - All ~80 regex patterns in `strip_latex()` now compiled once via `OnceLock` (previously compiled every render frame)
- **Performance: Superscript/subscript lookup** - Replaced O(N) linear array scans with `match` jump tables
- **Performance: Directory scanning** - Single-pass classification using `entry.file_type()` instead of separate `is_file()`/`is_dir()` stat syscalls
- **Refactoring** - Extracted `max_content_scroll()`, `effective_picker_dir()`, `navigate_picker_to_dir()`, `is_markdown_extension()`, and `filter_by_name()` helpers to eliminate code duplication
- **Bug fix** - `scan_markdown_files()` now recognizes `.mdown` extension (was missing from file picker scan)

## [0.5.7] - 2026-02-26

### Added

- **Home/End key bindings** - Navigate to first/last with Home/End keys ([#43](https://github.com/Epistates/treemd/issues/43))
  - Works in Normal, Interactive, Help, and FilePicker modes
  - Also added PageUp/PageDown bindings in Normal mode for consistency

- **Directory and multi-file support** - Open file picker with directory argument ([#43](https://github.com/Epistates/treemd/issues/43))
  - `treemd .` opens file picker in current directory
  - `treemd docs/` opens file picker in specified directory
  - Multiple file arguments supported (e.g., `treemd *.md`)

- **Compact tree style** - Gapless box-drawing characters for tree visualization ([#43](https://github.com/Epistates/treemd/issues/43))
  - Now uses `├──` instead of `├─ ` (connected, no gaps)
  - Config option `tree_style`: "compact" (default) or "spaced"
  - Works in both `--tree` CLI output and query tree output

- **Todo filter for outline** - Filter heading tree to show only headings with open todos ([#44](https://github.com/Epistates/treemd/issues/44))
  - Press `t` to toggle filtering by open todos (`- [ ]` or `* [ ]`)
  - Shows only headings that contain open todos (directly or in descendants)
  - Preserves hierarchy: parent headings shown if any child has todos
  - Status message shows count of headings with open todos

- **SOTA Content filtering** - Robust YAML frontmatter and LaTeX handling ([#43](https://github.com/Epistates/treemd/issues/43))
  - **Unicode Approximation**: LaTeX math symbols like `\alpha`, `\sum`, `\infty` are now rendered as readable Unicode (`α`, `∑`, `∞`) instead of being stripped.
  - **Superscript/Subscript support**: Common exponents and indices (like `x^2`, `n_i`) are converted to Unicode (`x²`, `nᵢ`).
  - **Environment Preservation**: Content inside LaTeX environments (like `equation` or `align`) is preserved while stripping the tags.
  - `hide_frontmatter` option strips `---\n...\n---` blocks at document start.
  - `hide_latex` option handles math and LaTeX commands robustly without "half measures".
  - Aggressive filtering remains available but the standard mode is now preferred.

- **Smart Responsive Tables** - Tables now wrap and collapse intelligently ([#43](https://github.com/Epistates/treemd/issues/43))
  - **Cell Wrapping**: Long content now wraps into multiple lines within columns, ensuring data remains readable even on narrow terminals.
  - **Content-weighted widths**: Uses 70% average + 30% max for fairer column distribution.
  - **Adaptive padding**: Dynamically reduces cell padding (2 → 1 → 0) to save space.
  - **Unicode ellipsis**: Optimized truncation using `…` for maximum information density.

- **File picker quit** - Press `q` to exit file picker dialog ([#43](https://github.com/Epistates/treemd/issues/43))

### Fixed

- **EOF scroll behavior** - Content no longer scrolls past the last line ([#43](https://github.com/Epistates/treemd/issues/43))
  - Now uses actual *rendered* line count (not raw markdown lines) for scroll limits
  - Scroll stops when last line is visible at bottom of viewport
  - Consistent behavior across all scroll methods (j/k, Page Up/Down, Home/End)

### Technical

- **Config additions** (`src/config.rs`)
  - Added `ContentConfig` struct with `hide_frontmatter`, `hide_latex`, `latex_aggressive` fields
  - Added `tree_style` field to `UiConfig` (defaults to "compact")
  - Added `is_compact_tree()` helper method to Config

- **Content filtering utilities** (`src/tui/ui/util.rs`)
  - Added `strip_frontmatter()` for YAML frontmatter removal
  - Added SOTA `strip_latex()` with Unicode symbol mapping and environment preservation
  - Added `wrap_text()` utility for Unicode-aware word wrapping
  - Added `filter_content()` combining all filters

- **Tree rendering** (`src/parser/document.rs`, `src/query/output.rs`)
  - Added `render_box_tree_styled()` method with compact parameter
  - Updated `format_tree_value()` to support compact mode

- **App state** (`src/tui/app.rs`)
  - Added `file_picker_dir` field for custom directory support
  - Added `should_hide_frontmatter()` and `should_hide_latex()` getters
  - Updated `scan_markdown_files()` to use custom directory

- **Table rendering** (`src/tui/ui/table.rs`, `src/tui/ui/mod.rs`)
  - Refactored `render_table_row` to return `Vec<Line>` for multi-line wrapping
  - Implemented smart wrapping logic using `util::wrap_text`
  - Propagated width adjustments through nested content structures
  - Comprehensive test updates for new wrapping behavior


## [0.5.6] - 2026-01-09

### Added

- **Image configuration** - Configure image rendering via `[image]` section in config
  - Specify preferred renderer: `renderer = "kitty"` or `renderer = "software"`
  - Add custom arguments: `args = ["--no-animations"]`
  - Example config:
    ```toml
    [image]
    renderer = "kitty"
    args = ["--no-animations"]
    ```

## [0.5.5] - 2026-01-02

### Added

- **Inline image rendering** - Full image support with Kitty graphics protocol ([#40](https://github.com/Epistates/treemd/issues/40))
  - Images render inline with markdown content using ratatui-image
  - Supports PNG, JPEG, GIF, WebP and other common formats
  - Image modal view - press Enter on an image in interactive mode for full-screen view
  - Press `q` to close image modal
  - Automatic image caching and lazy loading for performance
  - Works in Kitty, iTerm2, WezTerm, and other terminals with image protocol support
  - Fallback to halfblock Unicode rendering on unsupported terminals

- **GIF animation support** - Animated GIFs with playback controls
  - GIFs animate automatically in image modal view
  - Press `Space` to play/pause animation
  - Press `n` for next frame, `p` for previous frame (manual stepping)
  - First frame extraction with proper transparency handling
  - Software rendering mode for flicker-free animation

- **File picker on startup** - Interactive file picker when no file is specified
  - Fuzzy search through markdown files in current directory
  - Navigate with `j`/`k` or arrow keys
  - Press `Enter` to open selected file
  - Shows file list with real-time filtering

- **macOS XDG config path support** - `~/.config/treemd` now supported on macOS ([#41](https://github.com/Epistates/treemd/issues/41))
  - Checks `~/.config/treemd/config.toml` first on macOS
  - Falls back to `~/Library/Application Support/treemd/config.toml`
  - Enables easier dotfiles management and cross-platform config sharing

- **Alpine Linux support** - musl builds for Alpine and other musl-based distributions ([#42](https://github.com/Epistates/treemd/issues/42))
  - Added `x86_64-unknown-linux-musl` binary to releases
  - Added `aarch64-unknown-linux-musl` binary to releases
  - Statically-linked binaries work without glibc dependency

### Fixed

- **GIF animation flicker** - Eliminated flicker by pre-creating protocols and optimizing background clearing
- **Image rendering in normal mode** - Images now render correctly outside of interactive mode
- **Inline images in paragraphs** - Fixed detection of images within paragraph text
- **Theme preservation** - Theme colors maintained correctly during image rendering

### Technical

- **New modules**
  - `src/tui/kitty_animation.rs` - GIF animation state machine and frame management
  - `src/tui/image_cache.rs` - Image caching and lazy loading system

- **Dependencies**
  - Added `ratatui-image 10` for terminal image rendering
  - Added `image 0.25` for image processing
  - Added `gif 0.13` for GIF parsing and frame extraction

- **Release workflow**
  - Added musl targets (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`) to CI

## [0.5.4] - 2025-12-15

### Added

- **Navigation save confirmation** - Prompts before navigating away with unsaved changes
  - Triggered when pressing backspace to go back in file history
  - Also triggered when following links to other files
  - Dialog options:
    - `[y/Enter]` Save & Navigate - saves changes then navigates
    - `[d]` Discard & Navigate - discards changes and proceeds
    - `[q]` Discard & Quit - discards changes and exits
    - `[Esc]` Cancel - stays on current file

- **Quit without saving option** - Added to save before quit dialog
  - Press `[q]` to quit immediately without saving changes
  - Press `[y/Enter]` to save and quit (existing behavior)
  - Press `[Esc]` to cancel and stay (existing behavior)

- **Editor configuration** - Configure external editor via `[editor]` section in config
  - Specify preferred editor: `editor = "nvim"` or `editor_kind = "NeoVim"`
  - Add custom arguments: `args = ["--noplugin"]`
  - Uses [opensesame](https://crates.io/crates/opensesame) EditorConfig
  - Example config:
    ```toml
    [editor]
    editor = "nvim"
    args = ["--noplugin"]
    ```

### Changed

- **opensesame dependency** - Updated to use serde feature for config serialization

## [0.5.3] - 2025-12-13

### Added

- **Styled keybinding hints footer** - New context-aware footer bar showing relevant keybindings
  - Styled key badges with theme colors (`help_key_bg`, `help_key_fg`, `help_desc_fg`, `footer_bg`)
  - Hints update based on current mode (Normal, Interactive, LinkFollow, DocSearch, etc.)
  - Element-specific hints in interactive mode (Checkbox, Table, Link, Details, CodeBlock, Image)
  - Table mode shows cell navigation hints (j/k Row, h/l Col, e Edit, y Copy)

- **Vim-style count prefixes** - Repeat motions with numeric prefixes like vim
  - `5j` moves down 5 items, `10k` moves up 10 items
  - Works in Normal mode (outline/content navigation) and Interactive mode
  - Supports: j/k navigation, h/l table columns, content scrolling
  - `0` without count goes to first item (vim behavior preserved)
  - Link follow mode still uses `1-9` for direct link jumping

- **Collapse/Expand commands** - New command palette commands for outline management
  - `:collapse` / `:ca` - Collapse all headings with children
  - `:expand` / `:ea` - Expand all headings
  - `:collapse N` - Collapse all headings at level N (e.g., `:collapse 2` for h2)
  - `:expand N` - Expand all headings at level N
  - Status messages show count of affected headings

- **Inline HTML tag rendering** - Parse HTML tags in details block summaries
  - `<strong>`, `<b>` render as bold
  - `<em>`, `<i>` render as italic
  - `<code>` renders as inline code
  - No longer shows literal `<strong>` tags in rendered view

- **Nested interactive elements in details blocks** - Select elements inside expanded details
  - Tables, links, code blocks, images inside details are now selectable
  - Hierarchical status display: `▸Navigation > Table: 5×3`
  - Expansion state persists after exiting interactive mode

- **Safe edit buffer system** - Table cell edits are now buffered in memory instead of immediately written to file
  - Changes are applied to in-memory document for immediate display
  - Explicit save required with `:w` command to write changes to disk
  - Status shows "X unsaved change(s)" after edits
  - Prevents accidental data loss from unforeseen bugs

- **Save command (`:w`)** - New command to save pending edits
  - `:w`, `:write`, or `:save` writes all buffered edits to file atomically
  - Shows confirmation: "Saved X change(s) to filename.md"

- **Undo command (`:u` and `Ctrl+z`)** - Undo table cell edits before saving
  - `:u` or `:undo` in command palette undoes last edit
  - `Ctrl+z` keybinding in Interactive and InteractiveTable modes
  - Stack-based undo: each edit can be individually reverted
  - Shows remaining unsaved changes count after undo

- **Quit confirmation for unsaved changes** - Prompts before quitting with unsaved edits
  - Dialog shows number of unsaved changes
  - `Enter`/`y` saves changes and quits
  - `Escape` cancels and returns to normal mode

### Changed

- **Status bar shows context-aware position** - Position info based on focused pane
  - Outline focused: `[Outline] 3/15 (20%)` - heading position
  - Content focused: `[Content] Line 42 (35%)` - scroll position
  - Cleaner status bar without inline keybinding hints (moved to footer)

- **Esc key behavior in normal mode** - Shows helpful hint instead of doing nothing
  - Displays: "Press q to quit • : for commands • ? for help"
  - Guides new users on how to exit or access features

### Fixed

- **Table navigation in interactive mode** - j/k now moves cells when in table mode
  - Previously j/k moved between elements instead of table rows
  - Esc now exits table mode before exiting interactive mode

- **Table row bounds** - Can now navigate to last row in tables
  - Fixed off-by-one error in `table_move_down()`

- **Table cell edit affecting all tables** - Fixed critical bug where editing one table cell modified all tables
  - Root cause: table index wasn't being tracked correctly across multiple tables
  - Now correctly identifies and modifies only the target table using table index calculation

- **Table cell edit mode not triggering** - Fixed Enter key not entering edit mode when in table navigation
  - `InteractiveActivate` now checks `is_in_table_mode()` before deciding behavior

### Technical

- **Theme footer colors** (`src/tui/theme.rs`)
  - Added `help_key_bg`, `help_key_fg`, `help_desc_fg`, `footer_bg` to all 16 theme variants
  - Added `help_key_style()`, `help_desc_style()`, `footer_style()` helper methods
  - Updated `with_custom_colors()` and `with_color_mode_custom()` for footer fields

- **Config footer colors** (`src/config.rs`)
  - Added footer color fields to `CustomThemeConfig` for user customization

- **Layout footer section** (`src/tui/ui/layout.rs`)
  - Added `Section::Footer` to layout system

- **Count prefix system** (`src/tui/app.rs`, `src/tui/mod.rs`)
  - Added `count_prefix: Option<usize>` to App state
  - `accumulate_count_digit()`, `take_count()`, `clear_count()`, `has_count()` methods
  - Event loop accumulates digits before motion commands

- **Collapse/expand methods** (`src/tui/app.rs`)
  - `collapse_all()`, `expand_all()`, `collapse_level(n)`, `expand_level(n)`
  - Added `CollapseAll`, `ExpandAll`, `CollapseLevel`, `ExpandLevel` to `CommandAction`

- **HTML parsing utility** (`src/parser/utils.rs`)
  - Added `parse_inline_html()` function for HTML tag to InlineElement conversion

- **Edit buffer system** (`src/tui/app.rs`)
  - Added `PendingEdit` struct with table_index, row, col, original_value, new_value fields
  - Added `pending_edits: Vec<PendingEdit>` stack for buffered edits
  - Added `has_unsaved_changes: bool` flag for quit confirmation
  - Added `cell_edit_original_value` field to track original value before editing
  - `save_edited_cell()` now buffers to memory instead of writing to file
  - `save_pending_edits_to_file()` performs atomic file write
  - `undo_last_edit()` pops from edit stack and reverts in-memory content
  - `calculate_current_table_index()` helper for accurate table identification

- **Edit buffer commands** (`src/tui/app.rs`)
  - Added `CommandAction::SaveFile` and `CommandAction::Undo` to command palette
  - Added `AppMode::ConfirmSaveBeforeQuit` for quit confirmation dialog
  - Updated `handle_confirm_action()` to return `Option<ActionResult>` for quit flow

- **Undo keybinding** (`src/keybindings/action.rs`, `src/keybindings/defaults.rs`)
  - Added `Action::UndoEdit` for undo functionality
  - Bound `Ctrl+z` to `UndoEdit` in Interactive and InteractiveTable modes

- **Save confirmation dialog** (`src/tui/ui/popups.rs`)
  - Added `render_save_before_quit_confirm()` for unsaved changes prompt

## [0.5.1] - 2025-12-12

### Fixed

- **Search navigation after locking in results** - Fixed `n`/`N` and `Tab`/`Shift+Tab` not cycling through matches after pressing Enter to accept search
  - Added missing keybindings in DocSearch mode for match navigation
  - Both outline search (`s`) and content search (`/`) now properly support cycling

- **Escape clears search instead of quitting** - When search is locked in (after pressing Enter), Escape now clears the search and returns to normal mode instead of exiting the application

- **Re-enter search input with `/`** - After locking in a search, pressing `/` re-enters input mode to edit the query (keeps existing query)

### Added

- `Shift+Tab` keybinding in Normal mode for `ToggleFocusBack` action

## [0.5.0] - 2025-12-11

### Added

- **Customizable keybindings system** - Full keybinding customization via config file
  - Configure any key for any action using intuitive TOML syntax
  - Multi-key sequences supported (e.g., `"g g" = "First"`)
  - 12 distinct modes: Normal, Help, ThemePicker, Interactive, InteractiveTable, LinkFollow, LinkSearch, Search, DocSearch, CommandPalette, ConfirmDialog, CellEdit
  - 70+ bindable actions covering all application functionality
  - Uses [keybinds-rs](https://crates.io/crates/keybinds) for robust key parsing
  - Built-in defaults following vim conventions
  - Example configuration:
    ```toml
    [keybindings.Normal]
    "j" = "Next"
    "k" = "Previous"
    "Ctrl+c" = "Quit"
    "g g" = "First"  # Multi-key sequences!

    [keybindings.Interactive]
    "Escape" = "ExitInteractiveMode"
    ```

- **Unified search system** - Consistent search experience across outline and content
  - Press `s` to search/filter the outline tree
  - Press `/` to search document content
  - Press `Tab` to toggle between outline and content search (preserving query)
  - Both modes highlight matches with themeable colors
  - `n`/`N` navigate matches in both modes after pressing Enter
  - Visual search bar with cursor and clear mode indicator

- **Open links from search** - Follow links directly from document search results
  - When search matches a link, press Enter to follow it
  - Works with anchor links, file links, wikilinks, and external URLs

- **Open editor at location** - Jump to specific line when editing
  - Press `e` to open current file at the selected heading's line number
  - Uses [opensesame](https://crates.io/crates/opensesame) for cross-editor line support
  - Works with VS Code, vim, neovim, emacs, and most editors

- **Themeable search highlighting** - Customize search match colors in config
  - `search_match_bg` / `search_match_fg` for matches
  - `search_current_bg` / `search_current_fg` for focused match
  - Works consistently in both outline and content views

### Changed

- **Parser rewrite using turbovault** - Switched to turbovault-parser for markdown parsing
  - More robust handling of complex nested structures
  - Better performance on large documents
  - Improved code block detection in list items
  - Proper handling of inline formatting in all contexts

### Fixed

- **Checkbox toggle with inline markdown** - Fixed toggling checkboxes when task items contain inline formatting ([PR #38](https://github.com/Epistates/treemd/pull/38) by @viniciussoares)
  - Checkboxes with bold, italic, code, or links now toggle correctly
  - Uses regex-based markdown stripping instead of brittle character parsing
  - Example: `- [x] **Important** task` now works properly

- **Query engine missing code blocks in list items** - Code blocks nested inside numbered/bulleted list items are now correctly extracted for queries
  - Queries like `.code` and `.code[pattern]` now find code blocks inside list items
  - Also extracts images and tables nested within list items
  - Recursively extracts from blockquotes and details blocks as well

- **Nested code blocks in lists** - Code blocks inside list items now render correctly
  - Fixed indentation detection for nested blocks
  - Proper syntax highlighting maintained

- **Interactive element parsing** - Fixed element detection after parser rewrite
  - All interactive elements (checkboxes, tables, code blocks, links) correctly indexed

- **Wikilink resolution** - Fixed wikilinks with path separators
  - `[[docs/guide]]` now correctly resolves to `docs/guide.md`

- **Search mode stability** - Fixed various search mode issues
  - Backspace, Escape, Enter all work correctly in search modes
  - Ctrl+U clears search query
  - Search state properly preserved when toggling between modes

### Technical

- **Keybindings module** (`src/keybindings/`)
  - `action.rs` - 70+ actions with descriptive names and categories
  - `defaults.rs` - Built-in vim-style defaults for all modes
  - `mod.rs` - `Keybindings` struct wrapping keybinds-rs with mode dispatch
  - Actions serializable for config file persistence

- **Parser refactoring** (`src/parser/`)
  - Migrated from pulldown-cmark to turbovault-parser 1.2.3
  - Simplified content.rs from ~1000 lines to focused wrapper
  - Better link extraction with improved anchor handling

- **Search infrastructure** (`src/tui/`)
  - `outline_search_active` and `doc_search_active` for input state tracking
  - `toggle_search_mode()` for seamless Tab switching
  - Shared highlighting utilities in `ui/util.rs`

- **Recursive block extraction** (`src/query/eval.rs`)
  - Added `extract_nested_blocks()` helper function
  - `extract_blocks()` now descends into List, Blockquote, and Details blocks

### Dependencies

- Added `keybinds = "0.2"` with crossterm and serde features
- Added `strum = "0.27"` for action enum iteration
- Added `opensesame = "0.1"` for editor line positioning
- Updated `turbovault-parser = "1.2.3"` (replaces direct pulldown-cmark usage)

## [0.4.8] - 2025-12-07

### Added

- **Live file watching and auto-reload** - Watch the current file for external changes
  - Automatically reload when file is modified externally (e.g., edited in another editor)
  - Scroll position and interactive elements preserved on reload
  - Status message shows "↻ File reloaded (external change)"
  - Gracefully handles file system watching across platforms (FSEvents/inotify/etc)
  - Smart debouncing prevents double-reload on internal saves

- **Document search from interactive mode** - Search while interacting with elements
  - Press `/` to start document search from interactive mode
  - Use `n`/`N` to navigate matches while in interactive mode
  - Returns to interactive mode when search is closed
  - Works alongside interactive element navigation

### Fixed

- **Document search from interactive mode** - Document search now accessible in interactive mode ([#30](https://github.com/Epistates/treemd/issues/30))
  - Can press `/` to search without leaving interactive mode
  - Search matches navigable with `n`/`N` from any mode
  - Returns to interactive mode after search closes

- **Interactive elements not re-indexed on external reload** - New interactive elements added externally now appear
  - Re-indexes interactive elements after external file reload
  - Restores element selection if still valid after reload
  - Maintains viewport position during reload

- **Viewport jumping on checkbox toggle** - No longer re-centers viewport when toggling checkboxes ([#31](https://github.com/Epistates/treemd/issues/31))
  - Fixed double-reload race condition (internal save + file watcher)
  - Suppresses file watcher after internal saves to prevent duplicate reload
  - Interactive state fully preserved during checkbox toggle

### Technical

- **File watcher module** (`src/tui/watcher.rs`)
  - `FileWatcher` struct using notify 8.0 crate
  - Cross-platform file system event monitoring
  - Debounced event detection (100ms)
  - Handles modification, write, and create events

- **Live reload integration** (`src/tui/mod.rs`)
  - File watcher created at TUI startup
  - Event loop checks for changes during idle periods (100ms poll timeout)
  - State preservation on external reload (scroll, selection, interactive elements)
  - File path change tracking for watcher updates

- **Interactive state preservation** (`src/tui/app.rs`)
  - `suppress_file_watch` flag prevents double-reload on internal saves
  - `sync_previous_selection()` prevents spurious scroll resets
  - `reindex_interactive_elements()` now public for external reload handling
  - Proper state restoration during concurrent operations

### Dependencies

- Added `notify = "8.0"` for cross-platform file system watching

## [0.4.7] - 2025-12-07

### Added

- **Command palette** - Press `:` to open a fuzzy-searchable command palette ([#32](https://github.com/Epistates/treemd/issues/32))
  - Type to filter commands with fuzzy matching
  - Navigate with `j`/`k` or arrow keys, execute with `Enter`
  - Commands include: Save width, Toggle outline, Toggle help, Toggle raw source, Jump to top/bottom, Quit
  - Each command has aliases (e.g., `w`/`write`/`save` for save width)

- **Save outline width with confirmation** - Press `S` to save current outline width to config with modal confirmation ([#32](https://github.com/Epistates/treemd/issues/32))
  - Shows confirmation dialog before saving
  - Respects power users: manual config values are session-only until explicitly saved
  - New users with default config get auto-save behavior

- **Document search with n/N navigation** - Full in-document search with match highlighting ([#30](https://github.com/Epistates/treemd/issues/30))
  - Press `/` in content pane to search within the document
  - Press `n` for next match, `N` for previous match
  - Matches highlighted in content view
  - Status bar shows match count and current position

### Fixed

- **Anchor links in interactive mode** - Following anchor links to headings in current file now works correctly ([#29](https://github.com/Epistates/treemd/issues/29))
  - Changed from `select_by_text()` to `jump_to_anchor()` for proper anchor handling
  - Anchor links like `#installation` now jump to correct heading

- **Wikilinks with path separators** - Wikilinks containing `/` now work correctly ([#28](https://github.com/Epistates/treemd/issues/28))
  - Removed overly restrictive check that blocked all paths with `/`
  - Still blocks `..` for security (prevents directory traversal)
  - `[[docs/guide]]` now resolves to `docs/guide.md`

- **Checkbox toggle scroll jump** - Toggling checkboxes no longer causes page to jump to top ([#31](https://github.com/Epistates/treemd/issues/31))
  - Saves and restores scroll position and element index on file reload
  - Interactive mode state preserved after checkbox toggle

- **Config value protection** - Outline width cycling no longer overwrites custom config values ([#32](https://github.com/Epistates/treemd/issues/32))
  - Tracks whether config has custom outline width at startup
  - Power users with custom values: cycling is session-only
  - New users with standard values: auto-save for convenience

### Technical

- **Command palette system** (`src/tui/app.rs`)
  - `CommandAction` enum for available actions
  - `PaletteCommand` struct with fuzzy matching and scoring
  - `PALETTE_COMMANDS` constant with all commands and aliases
  - Filter/navigation/execution methods for palette state

- **Save confirmation modal** (`src/tui/ui/popups.rs`)
  - `render_save_width_confirm()` for confirmation dialog
  - `render_command_palette()` for command palette UI

- **Config tracking** (`src/tui/app.rs`)
  - `config_has_custom_outline_width` flag to detect power user configs
  - Standard widths: 20%, 30%, 40% - anything else is custom

## [0.4.6] - 2025-12-04

### Fixed

- **File creation modal rendering** - Fixed the file creation confirmation dialog not displaying properly

## [0.4.5] - 2025-12-04

### Added

- **Document overview for headerless files** - Files without a top-level heading now show a "(Document)" entry in the outline ([#25](https://github.com/Epistates/treemd/issues/25))
  - Displays 📄 icon in outline for the document overview
  - Shows entire file content including tables and text before the first heading
  - Automatically added when there's preamble content or no headings at all

- **Wikilink rendering in content** - Wikilinks now render as clickable links in the content pane
  - `[[target]]` displays as link with target as text
  - `[[target|alias]]` displays alias text linking to target
  - Works in both interactive mode and link follow mode
  - Preprocessing converts wikilinks to standard markdown links for consistent parsing

- **Links with spaces in URLs** - Links like `[text](path/to/my file.md)` now work correctly
  - CommonMark doesn't support spaces in URLs, but many wikis use them
  - Preprocessing converts to angle bracket syntax for compatibility

- **File creation prompts** - Following links to non-existent files prompts to create them
  - Confirmation dialog with `[y]` to create, `[n/Esc]` to cancel
  - Creates file with default heading based on filename
  - Automatically opens the newly created file
  - Works for both relative links and wikilinks

- **Page navigation in interactive mode** - Scroll content while staying in interactive mode
  - Press `u` or `PgUp` to scroll up
  - Press `d` or `PgDn` to scroll down
  - Maintains element selection while scrolling

### Fixed

- **Screen artifacts when scrolling** - Fixed rendering artifacts caused by tab characters in code blocks ([#26](https://github.com/Epistates/treemd/issues/26))
  - Tabs are now converted to 4 spaces in code block syntax highlighting
  - Also applies to raw markdown view for consistency

- **Shift+Tab navigation** - Fixed Shift+Tab not working for backwards navigation ([#18](https://github.com/Epistates/treemd/issues/18))
  - Now uses `KeyCode::BackTab` instead of checking modifiers
  - Works correctly in both interactive mode and link follow mode

- **Interactive mode scroll preservation** - Entering interactive mode no longer jumps to first element
  - Now selects the element closest to current scroll position
  - Preserves user's view when toggling interactive mode

- **Wikilink anchor support** - Wikilinks now support section anchors
  - `[[filename#section]]` loads file and jumps to heading
  - `[[#section]]` jumps to heading in current document

- **Relative file link improvements** - Better handling of wiki-style links without extensions
  - Files without extension now try `.md` first before opening in editor
  - Improves compatibility with Obsidian and other wiki tools

### Changed

- **Interactive mode status bar** - Updated to show page navigation hints
  - Now displays: `[INTERACTIVE] Tab:Next Shift+Tab:Prev u/d:Page Esc:Exit`

- **Help text updated** - Interactive mode section includes new keybindings
  - Added `u/d` for page up/down navigation

### Technical

- **Wikilink preprocessing** (`src/parser/content.rs`)
  - `preprocess_wikilinks()` converts `[[target]]` to `[target](wikilink:target)` before parsing
  - `preprocess_links_with_spaces()` wraps URLs containing spaces in angle brackets
  - Both use compiled regex with `OnceLock` for performance

- **File creation flow** (`src/tui/app.rs`)
  - `AppMode::ConfirmFileCreate` for pending file creation state
  - `pending_file_create` and `pending_file_create_message` fields
  - `confirm_file_create()` and `cancel_file_create()` methods

- **Interactive mode improvements** (`src/tui/interactive.rs`)
  - `enter_at_scroll_position()` selects element closest to scroll position
  - Elements sorted by line position after indexing for proper navigation order
  - Wikilinks detected via `wikilink:` URL prefix from preprocessing

- **Page navigation** (`src/tui/app.rs`, `src/tui/mod.rs`)
  - `scroll_page_down_interactive()` and `scroll_page_up_interactive()` methods
  - Keybindings for `u`/`d`/`PgUp`/`PgDn` in interactive mode

- **File creation popup** (`src/tui/ui/popups.rs`)
  - `render_file_create_confirm()` renders themed confirmation dialog

## [0.4.4] - 2025-12-04

### Added

- **Raw source view toggle** - Press `r` to toggle between rendered markdown and raw source view ([#19](https://github.com/Epistates/treemd/issues/19))
  - Shows original markdown with line numbers for debugging rendering issues
  - `[RAW]` indicator in title bar and status bar when active
  - Maintains scroll position when toggling

- **Link search/filter in link navigator** - Press `/` in link follow mode to filter links by text or URL
  - Case-insensitive search across link text and targets
  - Selection stays within filtered results
  - Press `Esc` to clear filter or exit search mode

- **Links in list items** - Interactive mode now extracts and navigates to links within list item content
  - Previously only standalone links were indexed; now links embedded in list items are accessible
  - Links are indexed per-item with proper highlighting

- **Selection indicator backgrounds** - Added background colors to selection indicators for better visibility
  - `selection_indicator_bg` theme field for customizing the background color
  - Improves contrast in all themes, especially on light backgrounds

### Changed

- **Status messages auto-dismiss** - Temporary status messages now auto-clear after 1 second
  - Event loop uses polling with 100ms timeout for responsive UI updates
  - No more stale "Rendered view enabled" messages lingering

### Technical

- **Event polling for piped stdin** - Added `poll_event()` to `tty` module for non-blocking event handling
  - Supports the same stdin redirection logic as `read_event()` for piped input scenarios
  - Enables timed UI updates without user input

## [0.4.3] - 2025-12-03

### Added

- **Themeable UI colors** - Replaced hardcoded colors with theme-based colors for better customization
  - Added 6 new themeable color fields: `title_bar_fg`, `scrollbar_fg`, `selection_indicator_fg`, `link_fg`, `link_selected_bg`, `table_border`
  - All 8 themes now include appropriate colors for these new fields (both RGB and 256-color variants)
  - Users can now customize title bar, scrollbars, selection indicators, links, and table borders via config file
  - Consistent theming across all UI elements

### Fixed

- **Search bar overlapping filtered outline results** - Search bar no longer overlaps the outline when filtering headings ([PR #14](https://github.com/Epistates/treemd/pull/14))
- **Content panes overlapping status bar** - Fixed layout issue where content panes could overlap the status bar ([PR #13](https://github.com/Epistates/treemd/pull/13))

### Refactored

- **Layout builder** - Replaced string-based section IDs with `Section` enum for type-safe layout management

## [0.4.2] - 2025-12-02

### Fixed

- **Sub-headings not displayed in content pane** - Fixed regression where sub-headings within a section were not rendered in the content pane ([#10](https://github.com/Epistates/treemd/issues/10))
  - Added `Block::Heading` variant to the parser's block types
  - Sub-headings now render with proper styling (colored, bold, underlined) matching the screenshot in README
  - Content structure and hierarchy are preserved when viewing sections

- **Link selection visibility in interactive mode** - Selected links now have clear visual highlighting
  - Previously only a block-level arrow indicated selection, making it unclear which specific link was selected
  - Now selected links show a `▸` prefix indicator that moves with the selection
  - Plus cyan background highlight (matching table cell selection style)
  - Also applies to images in interactive mode

- **Help popup infinite scroll** - Prevented scrolling past the end of help content ([PR #11](https://github.com/Epistates/treemd/pull/11))

- **Numbered lists with nested code blocks** - Fixed markdown display issue where numbered list items containing code blocks would render incorrectly ([#8](https://github.com/Epistates/treemd/issues/8))
  - List items now properly contain their nested code blocks, blockquotes, and other block elements
  - Parser correctly associates indented blocks with their parent list items
  - Renderer handles nested block rendering within list item context

- **Interactive mode toggle** - Pressing `i` now correctly toggles out of interactive mode (previously only entered it)

### Changed

- **Link navigator layout stability** - Link targets now display inline to prevent layout shift when cycling through links ([PR #9](https://github.com/Epistates/treemd/pull/9))
  - Previously, selected links showed target on a separate line causing list to jump
  - Now all links show target inline (e.g., `[1] Link Text → target`) for stable navigation

### Refactored

- **Help text module** - Extracted help text content into dedicated `src/tui/help_text.rs` module ([PR #11](https://github.com/Epistates/treemd/pull/11))
  - Uses typed `HelpLine` enum for clean separation of data and rendering
  - Compile-time const construction with `const fn` builders
  - Makes help content easily maintainable and extensible

- **TUI UI module architecture** - Refactored monolithic `ui.rs` (~1700 lines) into modular components for better maintainability
  - `ui/mod.rs` (~940 lines) - Core rendering orchestration
  - `ui/util.rs` (~265 lines) - Utility functions: `centered_area`, `detect_checkbox_in_text`, `align_text`
  - `ui/popups.rs` (~460 lines) - Popup rendering: help, link picker, search, theme selector, cell edit
  - `ui/table.rs` (~460 lines) - Table rendering: `render_table`, `render_table_row`, `TableRenderContext`
  - Added comprehensive unit tests for extracted modules (29 new tests)
  - Zero regressions - all 90 tests pass

### Technical

- **Parser improvements** (`src/parser/content.rs`, `src/parser/output.rs`)
  - Added `Block::Heading` variant with level, content, and inline elements for sub-heading support
  - Added `Block::ListItemStart` variant to track list item context during parsing
  - `parse_content()` now parses headings within content and creates `Block::Heading` blocks
  - Nested blocks are properly associated with their parent list items instead of being siblings

- **UI module organization** (`src/tui/ui/`)
  - Clean separation of concerns: utilities, popups, tables, and core rendering
  - Each module has focused responsibility and comprehensive test coverage
  - Added `ContentBlock::Heading` rendering with level-appropriate colors and styling
  - `render_inline_elements()` now accepts optional selection index for inline element highlighting
  - Links and images in interactive mode get background highlight when selected
  - Improved code discoverability and maintainability

## [0.4.1] - 2025-12-01

### Fixed

- **Config file `color_mode` setting ignored** - The `color_mode` setting in `config.toml` is now properly respected ([#5](https://github.com/Epistates/treemd/issues/5))
  - Priority order: CLI flags > config file > auto-detection
  - Set `color_mode = "rgb"` or `color_mode = "256"` in config to override auto-detection
  - `color_mode = "auto"` (default) uses improved auto-detection

- **RGB auto-detection fails for truecolor terminals** - Improved terminal color detection for Kitty, Alacritty, WezTerm, and other RGB-capable terminals ([#5](https://github.com/Epistates/treemd/issues/5))
  - Now checks `COLORTERM` environment variable for `truecolor` or `24bit` (primary standard per [termstandard/colors](https://github.com/termstandard/colors))
  - Checks `TERM` for known truecolor terminals (`kitty`, `alacritty`, `wezterm`) and suffixes (`-truecolor`, `-direct`)
  - Checks `TERM_PROGRAM` for known apps (iTerm, Kitty, VS Code, Hyper, etc.)
  - Falls back to `supports_color` crate detection

### Technical

- **Enhanced color detection** (`src/tui/terminal_compat.rs`)
  - New `detect_truecolor_support()` method with multi-method detection
  - Environment variable checks before crate-based detection
  - Better compatibility with terminals that set `COLORTERM=truecolor`

- **Config priority in main** (`src/main.rs`)
  - Color mode selection now checks config file before auto-detection
  - Clear priority: CLI args (highest) > config file > auto-detection (lowest)

## [0.4.0] - 2025-11-30

### Added

- **Query Language (tql)** - A comprehensive jq-like query language for navigating and extracting markdown structure
  - Element selectors: `.h`, `.h1`-`.h6`, `.code`, `.link`, `.img`, `.table`, `.list`, `.blockquote`
  - Filters and indexing: `[Features]` (fuzzy), `["exact"]`, `[0]`, `[-1]`, `[1:3]`, `[:3]`
  - Hierarchy operators: `>` (direct child), `>>` (descendant)
  - Pipes for chaining: `.h2 | text | upper`
  - Multiple output formats: plain, json, json-pretty, jsonl, markdown, tree
  - 50+ built-in functions with extensive aliases for discoverability

- **CLI Query Flags**
  - `-q, --query <EXPR>` - Execute a query expression
  - `--query-help` - Display comprehensive query language documentation
  - `--query-output <FORMAT>` - Set output format (plain, json, jsonl, etc.)

- **Query Functions - Collections**
  - `count`, `length` (aliases: `len`, `size`) - Count elements
  - `first`, `last` (alias: `head`) - Get first/last element
  - `limit(n)`, `skip(n)` (aliases: `take`, `drop`) - Pagination
  - `nth(n)` - Get element at index (supports negative)
  - `reverse`, `sort`, `sort_by(key)` - Ordering
  - `unique`, `flatten` - Array operations
  - `group_by(key)` - Group elements by property
  - `keys`, `values` - Object access
  - `min`, `max`, `add` - Numeric/string aggregation
  - `any`, `all` - Boolean aggregation

- **Query Functions - Strings**
  - `text` - Get text representation
  - `upper`, `lower` (aliases: `ascii_upcase`, `ascii_downcase`) - Case conversion
  - `trim` - Strip whitespace
  - `split(sep)`, `join(sep)` - Split/join
  - `replace(from, to)` - Substring replacement
  - `slugify` - URL-friendly slug
  - `lines`, `words`, `chars` - Count lines/words/characters

- **Query Functions - Filtering**
  - `select(cond)` (aliases: `where`, `filter`) - Keep elements matching condition
  - `contains(s)` (alias: `includes`) - Check for substring
  - `startswith(s)`, `endswith(s)` (aliases: `starts_with`, `ends_with`) - Prefix/suffix check
  - `matches(regex)` - Regex matching
  - `has(key)` - Check for property
  - `not` - Negate boolean
  - `type` - Get value type

- **Query Functions - Content**
  - `content` - Section content (for headings)
  - `md` (alias: `markdown`) - Raw markdown
  - `url` (aliases: `href`, `src`) - Get URL/link/image source
  - `lang` (alias: `language`) - Code block language

- **Query Functions - Aggregation**
  - `stats` - Document statistics (headings, code blocks, links, etc.)
  - `levels` - Heading count by level
  - `langs` - Code block count by language
  - `types` - Link types count

- **Element Selector Aliases** - Multiple names for discoverability
  - `.heading`, `.headers` → `.h`
  - `.codeblock`, `.pre` → `.code`
  - `.a`, `.anchor` → `.link`
  - `.ul`, `.ol` → `.list`
  - `.bq`, `.quotes` → `.blockquote`
  - `.para`, `.paragraph`, `.p` → `.para`
  - `.fm`, `.meta`, `.yaml` → `.frontmatter`

- **Stdin/Pipe Support** - Read markdown from stdin for CLI workflows
  - `cat doc.md | treemd -q '.h2'` - Pipe markdown content
  - `tree | treemd` - Pipe tree output (auto-converted to markdown)
  - `treemd -` - Explicit stdin reading
  - Security limits: 100MB max input, 10MB max line length
  - UTF-8 validation with clear error messages

- **TTY Handling for Piped Input** - TUI mode works even when stdin is piped
  - Opens `/dev/tty` for keyboard input when stdin is piped
  - Enables raw mode on correct terminal device
  - Seamless `tree | treemd` workflow with full interactivity

### Technical

- **New Query Module** (`src/query/` - ~3000 lines)
  - `mod.rs` - Public API: `execute()`, `parse()`, `engine()`, `format_output()`
  - `lexer.rs` - Tokenizer with span tracking
  - `parser.rs` - Recursive descent parser with operator precedence
  - `ast.rs` - Complete AST types (Query, Expr, ElementKind, Filter, IndexOp, etc.)
  - `eval.rs` - Evaluator with pluggable function registry
  - `registry.rs` - Function and extractor registry with Levenshtein suggestions
  - `value.rs` - Runtime value types (Heading, Code, Link, Image, Table, etc.)
  - `builtins/mod.rs` - 50+ built-in functions
  - `output.rs` - Multi-format output rendering
  - `error.rs` - Rich error messages with source spans and suggestions
  - `extractors.rs` - Pluggable element extraction (reserved for future)

- **New Input Module** (`src/input.rs` - 195 lines)
  - `InputSource` enum for file vs stdin sources
  - `InputError` with descriptive messages
  - `determine_input_source()` - Smart source detection
  - `process_input()` - Content processing with format detection
  - Security limits to prevent DoS attacks

- **New TTY Module** (`src/tui/tty.rs` - 229 lines)
  - `enable_raw_mode()` / `disable_raw_mode()` - TTY-aware raw mode
  - `read_event()` - Event reading from correct source
  - Uses `MaybeUninit` for safer uninitialized memory handling
  - Cross-platform support (Unix with `/dev/tty`, Windows fallback)

- **New Dependencies**
  - `indexmap 2.7` - Ordered maps for predictable output
  - `regex 1.11` - Regex matching in queries

- **Architecture**
  - Pluggable function registry for custom extensions
  - Trait-based extractor system for future element types
  - Comprehensive test suite (35 query tests)

### Examples

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

# External link URLs
treemd -q '.link[external] | url' doc.md

# h2s under "Features" section
treemd -q '.h1[Features] > .h2' doc.md

# Group headings by level
treemd -q '[.h] | group_by("level")' --query-output json doc.md

# Document statistics
treemd -q '. | stats' doc.md

# Pipe from tree command
tree | treemd -q '.h'
```

## [0.3.2] - 2025-11-22

### Fixed

- **BSD and Unix Platform Support** - Fixed compilation error on NetBSD and other BSD systems
  - External URL opening now supports all Unix-like platforms (NetBSD, FreeBSD, OpenBSD, etc.)
  - Platform detection changed from explicit Linux-only to fallback for all non-macOS/Windows systems
  - All BSD variants now use `xdg-open` command for opening external links
  - Resolves compilation error: "cannot find value `open_cmd` in this scope"

### Technical

- **Platform Detection** (`src/tui/app.rs:1402-1407`)
  - Changed from OS-specific `#[cfg(target_os = "linux")]` to inclusive `#[cfg(not(any(target_os = "macos", target_os = "windows")))]`
  - macOS continues to use `open` command
  - Windows continues to use `start` command
  - All other Unix-like systems (Linux, *BSD, etc.) use `xdg-open`

### Platform-Specific Notes

- **BSD Systems (NetBSD, FreeBSD, OpenBSD)**
  - External links now open correctly in default browser
  - Full compilation support without errors
  - NetBSD tested by contributor @0323pin

- **All Platforms**
  - No functional changes for existing macOS, Windows, or Linux users
  - More robust platform detection for edge cases

## [0.3.1] - 2025-11-21

### Fixed

- **Theme Color Mode Application** - Improved theme initialization order for more consistent color rendering
  - Color mode now applied before custom colors for better accuracy
  - Theme-specific 256-color variants now correctly loaded based on color mode
  - Fixes color inconsistencies when using custom theme colors with 256-color fallback

### Changed

- **Code Formatting** - Applied consistent code formatting throughout codebase
- **Documentation** - Enhanced README with more detailed interactive mode instructions
  - Added comprehensive keyboard shortcuts for interactive mode
  - Clarified table navigation and editing workflow
  - Better organization of feature descriptions

### Technical

- **Theme Architecture** (`src/tui/theme.rs`)
  - Added `from_name_256()` method to load theme-specific 256-color variants
  - Reordered `with_color_mode()` to apply before `with_custom_colors()`
  - Each theme now has dedicated 256-color palette matching official theme colors
  - Better color accuracy in 256-color terminals

- **App State** (`src/tui/app.rs`)
  - Updated theme initialization: `from_name() → with_color_mode() → with_custom_colors()`
  - Custom colors now properly respect color mode constraints
  - Improved formatting consistency

## [0.3.0] - 2025-11-20

### Added

- **Interactive Element Navigation System** - Complete system for navigating and interacting with markdown elements
  - Press `i` to enter interactive mode and navigate all interactive elements
  - Interactive element types: Details blocks, Links, Checkboxes, Code blocks, Tables, and Images
  - Navigate with Tab/Shift+Tab, j/k, or Up/Down arrow keys
  - Auto-scroll to keep selected element in viewport
  - Element-specific actions (expand details, follow links, toggle checkboxes, copy code/images)
  - Visual selection indicator (→ arrow) for current element
  - Status bar guidance showing available actions for each element type

- **Details/Collapsible Blocks** - HTML `<details>` element support in markdown
  - `<details><summary>Title</summary>Content</details>` now renders as expandable sections
  - Collapsible blocks with expand/collapse indicators (▼/▶)
  - Nested content support within details blocks
  - Toggle expansion with Enter key in interactive mode
  - Automatic re-indexing when expansion state changes

- **Interactive Table Editing and Navigation**
  - Press Enter on a table in interactive mode to enter table navigation
  - Navigate cells with h/j/k/l or arrow keys
  - Copy individual cell with `y`, entire row with `Y`, full table with `r`
  - Edit cell values with Enter key - inline editor with Esc to cancel
  - Cell position shown in status bar: `[TABLE] Cell(row,col)`
  - Save edited cells directly to file with automatic reload

- **Enhanced Interactive Element Interactions**
  - Details blocks: Press Enter to toggle expand/collapse
  - Links: Press Enter to follow, works with all link types (anchors, relative files, wikilinks, external URLs)
  - Checkboxes: Press Space to toggle, saves to file automatically
  - Code blocks: Press y to copy code to clipboard
  - Images: Press y to copy image path, i to view image info
  - Full keyboard support for all interactive operations

### Technical

- **New Interactive Module** (`src/tui/interactive.rs` - 614 lines)
  - `InteractiveState` struct for managing interactive elements
  - `ElementId` and `ElementType` enums for element identification
  - `DetailMode` enum for fine-grained navigation (tables, lists)
  - Element indexing from parsed blocks
  - Navigation methods: next(), previous(), enter(), exit()
  - Table-specific navigation: table_move_up/down/left/right()
  - Cell and row retrieval: get_table_cell(), get_table_row()
  - Status text generation for UI feedback

- **Parser Enhancement** (`src/parser/content.rs`)
  - `extract_details_blocks()` for preprocessing `<details>` HTML
  - Recursive parsing of nested content within details blocks
  - Placeholder-based approach for proper block structure integration
  - `Block::Details` variant with summary and nested blocks

- **App State Integration** (`src/tui/app.rs`)
  - `interactive_state: InteractiveState` field
  - `AppMode::Interactive` and `AppMode::CellEdit` modes
  - `enter_interactive_mode()` - Index elements and begin navigation
  - `exit_interactive_mode()` - Return to normal mode
  - `activate_interactive_element()` - Dispatch to element-specific handlers
  - Cell edit state: `cell_edit_value`, `cell_edit_row`, `cell_edit_col`
  - `enter_cell_edit_mode()` and `save_edited_cell()` for table editing
  - `reindex_interactive_elements()` after state changes
  - `copy_table_cell()`, `copy_table_row()`, `copy_table_markdown()`
  - `scroll_to_interactive_element()` for auto-scroll to viewport

- **Event Handling** (`src/tui/mod.rs`)
  - Interactive mode key bindings: Tab, j/k, Up/Down, Enter, Space, y, Esc
  - Table navigation mode: h/j/k/l, y/Y/r for copying, Enter for edit
  - Cell edit mode: Character input, Backspace for delete, Enter to save, Esc to cancel
  - Mode-specific status message updates
  - Integration with existing help, search, and link follow modes

- **Rendering Enhancement** (`src/tui/ui.rs`)
  - `render_markdown_enhanced()` updated for interactive elements
  - Details block rendering with expand/collapse indicators
  - Interactive element selection highlighting
  - Table cell highlighting during edit mode
  - Cell edit overlay popup
  - Status text reflecting interactive mode state
  - Support for rendering nested blocks within details

- **File I/O for Table Editing**
  - `replace_table_cell_in_markdown()` - Find and update cells in markdown
  - `replace_table_cell_in_file()` - Locate target table in file
  - `replace_cell_in_row()` - Parse and modify individual cells
  - Automatic document reload after table edits
  - State preservation after interactive changes

### Changed

- **Keyboard Shortcuts** - New keybinding `i` for interactive mode
  - `i` - Enter interactive element navigation mode
  - In interactive mode: Tab, j/k, Up/Down to navigate
  - Element-specific actions: Enter, Space, y, r to interact

- **App Rendering** - Enhanced UI for interactive state
  - Title bar, outline, content, and status all reflect interactive mode
  - Selection indicators on all element types
  - Table cells highlight in edit mode
  - Status bar shows element-specific guidance

- **Documentation Updates**
  - Help screen includes new interactive mode section
  - Keybinding reference updated with interactive shortcuts
  - Cell editing workflow documented

### Platform-Specific Notes

- **All Platforms**
  - Interactive navigation works identically across OS
  - Table editing saves directly to file with cross-platform paths
  - Status messages provide consistent feedback

## [0.2.3] - 2025-11-17

### Added

- **Custom Theme Colors** - Full color customization support via configuration file
  - Override any color from built-in themes with your own RGB/named/indexed colors
  - Three color format options: Named ("Cyan"), RGB (`{ rgb = [r, g, b] }`), Indexed (`{ indexed = 235 }`)
  - All 22 theme colors are customizable: headings, borders, backgrounds, status bar, code blocks, etc.
  - Color overrides applied before color mode conversion (custom RGB → 256-color on incompatible terminals)
  - Example configs provided in README for dark blue theme and high contrast accessibility theme
  - Perfect for personalizing themes or adapting to terminal color schemes

### Fixed

- **Modal Rendering on macOS Terminal.app** - Fixed theme picker, help, and link navigator modals using hardcoded RGB colors
  - All modal popups now respect the configured color mode (256-color fallback on Terminal.app < macOS 26)
  - Replaced 69 hardcoded `Color::Rgb()`, `Color::Cyan`, `Color::Yellow`, etc. with theme-aware colors
  - Added modal color helper methods to Theme struct: `modal_bg()`, `modal_border()`, `modal_title()`, `modal_text()`, etc.
  - Modal colors automatically converted to 256-color palette on incompatible terminals
  - Fixes reported issue: "while the colors are better the theme modal isn't rendered right" on macOS Sequoia

### Technical

- **Config structure** (`src/config.rs`)
  - Added `CustomThemeConfig` struct with 22 optional color fields
  - Added `ColorValue` enum for flexible color parsing (Named/RGB/Indexed)
  - Implemented `to_color()` conversion supporting 17 named colors
  - TOML serialization with `skip_serializing_if` to keep config files clean

- **Theme enhancements** (`src/tui/theme.rs`)
  - Added `with_custom_colors()` method to apply overrides before color mode conversion
  - Uses macro for DRY override application across all 22 color fields
  - Custom colors automatically convert to 256-color mode when needed
  - Added 8 modal color helper methods that derive from existing theme colors
  - Modal colors automatically respect `with_color_mode()` transformation
  - No changes needed to theme definitions - works for all 8 themes

- **App integration** (`src/tui/app.rs`)
  - Updated initialization to apply custom colors: `Theme::from_name() → with_custom_colors() → with_color_mode()`
  - Theme switching preserves custom color overrides
  - Clean separation: base theme → custom → color mode

- **UI modal updates** (`src/tui/ui.rs`)
  - `render_theme_picker()` - All colors now use `theme.modal_*()` methods
  - `render_help_popup()` - All 40+ color references updated to use theme methods
  - `render_link_picker()` - All colors now theme-aware
  - Consistent modal appearance across all color modes

### Documentation

- **README Configuration section**
  - Complete guide to custom color configuration
  - Three example configs (basic, dark blue, high contrast)
  - All 22 color fields documented with descriptions
  - Color format examples for all three types
  - Explains color application order and graceful degradation

### Platform-Specific Notes

- **macOS Sequoia (Darwin 24) with Terminal.app**
  - Modals now render correctly in 256-color mode
  - No more RGB color artifacts or rendering glitches
  - Consistent appearance across all popups

- **All Platforms**
  - Modal colors automatically adapt to terminal capabilities
  - Theme switching works properly in modals
  - Better visual consistency
  - Custom colors work seamlessly with 256-color fallback

## [0.2.2] - 2025-11-17

### Added

- **Theme Persistence** - User theme preferences now persist across sessions
  - Theme choice automatically saved when changed (press `t` to select theme)
  - Config file at `~/.config/treemd/config.toml` (Linux/Unix) or `~/Library/Application Support/treemd/config.toml` (macOS)
  - No need to reset theme every launch - remembered indefinitely
  - Works for all 8 themes: OceanDark, Nord, Dracula, Solarized, Monokai, Gruvbox, TokyoNight, CatppuccinMocha

- **Outline Width Persistence** - Panel width settings now persist
  - Outline width (20%, 30%, or 40%) saved when cycling with `[` / `]`
  - Restored automatically on next launch
  - Seamless UX - interface remembers your preferred layout

- **Terminal Compatibility Detection** - Intelligent color mode selection for optimal rendering
  - Automatic detection of terminal RGB/true-color capabilities
  - Special handling for Apple Terminal.app on macOS < 26 (Sequoia and earlier)
  - Detects macOS version via Darwin kernel (e.g., Darwin 24 = Sequoia, 26 = Tahoe)
  - One-time warning for Terminal.app users with recommendations for better alternatives
  - Warning never shown again after first acknowledgment (saved in config)
  - macOS 26+ (Tahoe and later) uses RGB mode without warnings

- **256-Color Fallback Mode** - Graceful degradation for limited terminals
  - Automatic RGB → 256-color conversion when needed
  - Smart color quantization algorithm:
    - Grayscale detection with dedicated 24-shade ramp (palette 232-255)
    - RGB cube mapping for colors (palette 16-231)
    - Preserves theme appearance while ensuring compatibility
  - All 8 themes work beautifully in both RGB and 256-color modes
  - Applied automatically for Terminal.app on macOS < 26

- **CLI Theme Override** - Command-line theme selection
  - `--theme <THEME>` flag to override saved theme for single session
  - Example: `treemd --theme Nord README.md`
  - Available themes: OceanDark, Nord, Dracula, Solarized, Monokai, Gruvbox, TokyoNight, CatppuccinMocha
  - Useful for screenshots, demos, or quick theme testing

- **CLI Color Mode Override** - Force specific color mode
  - `--color-mode <MODE>` flag to override automatic detection
  - `auto` - Use automatic detection (default)
  - `rgb` - Force RGB/true-color mode (16M colors)
  - `256` - Force 256-color mode
  - Example: `treemd --color-mode 256 README.md`
  - Power user feature for debugging or preference

### Changed

- **App Initialization** - Enhanced to support configuration and terminal detection
  - `App::new()` now accepts `Config` and `ColorMode` parameters
  - Loads user preferences before launching TUI
  - Applies color mode to theme during initialization
  - Theme changes now trigger config save automatically

- **Terminal Warning Flow** - User-friendly compatibility notice
  - Shows one-time warning before TUI launch (not in alternate screen)
  - Waits for user acknowledgment before continuing
  - Warning includes:
    - Clear explanation of Terminal.app RGB limitations
    - Confirmation of 256-color fallback activation
    - Recommendations for iTerm2, Kitty, and Alacritty
    - macOS version information for context
  - Never blocks users on modern macOS (26+) or other terminals

### Technical

- **New Dependencies**
  - `dirs 6.0` - Cross-platform config directory paths
  - `toml 0.9` - Configuration file serialization
  - `supports-color 3.0` - Terminal color capability detection

- **New Modules**
  - `src/config.rs` - Configuration management and persistence
    - TOML-based config with `[ui]` and `[terminal]` sections
    - Auto-creates config directory if missing
    - Graceful fallback to defaults if config missing or corrupt
    - Theme name string ↔ enum conversion
  - `src/tui/terminal_compat.rs` - Terminal capability detection
    - `TerminalCapabilities::detect()` - Analyzes terminal and OS
    - `TERM_PROGRAM` environment variable detection
    - macOS version detection via `uname -r`
    - Smart warning logic (only macOS < 26 Terminal.app)
  - `src/tui/theme.rs` enhancements - Color mode conversion
    - `Theme::with_color_mode()` - Apply color mode to theme
    - `rgb_to_256()` - RGB to 256-color quantization
    - Preserves all theme definitions while adapting output

- **App State Enhancements** (`src/tui/app.rs`)
  - Added `config: Config` field for persistence
  - Added `color_mode: ColorMode` field to remember active mode
  - `apply_selected_theme()` now saves to config and applies color mode
  - `cycle_outline_width()` now saves to config automatically
  - Constructor signature updated: `new(doc, filename, path, config, color_mode)`

- **CLI Argument Additions** (`src/cli/commands.rs`)
  - `--theme` optional flag with theme name validation
  - `--color-mode` optional flag with enum: Auto, Rgb, Color256
  - `ColorModeArg` enum exported for use in main

- **Main Initialization Flow** (`src/main.rs`)
  - Config loaded before TUI initialization
  - CLI flag overrides applied to config
  - Terminal capabilities detected and evaluated
  - Color mode determined from CLI flag or auto-detection
  - Warning displayed if needed (before TUI init)
  - Config updated with warning acknowledgment

### Platform-Specific Notes

- **macOS Sequoia (Darwin 24) and earlier**
  - Terminal.app users see one-time compatibility notice
  - Automatic 256-color fallback for better rendering
  - Recommended to use iTerm2, Kitty, or Alacritty for RGB support

- **macOS Tahoe (Darwin 26) and later**
  - Terminal.app works well with RGB mode
  - No warnings or fallbacks needed
  - Full theme fidelity

- **Linux/Unix**
  - Config at `~/.config/treemd/config.toml` (XDG standard)
  - Terminal detection respects `supports-color` crate findings
  - Most modern terminals support RGB

- **All Platforms**
  - Theme persistence "just works"
  - CLI overrides respected for all flags
  - 256-color fallback available when needed

## [0.2.1] - 2025-11-16

### Fixed

- **Linux X11 Clipboard Support** - Resolved critical clipboard copy bug on Linux X11 environments (Arch, i3wm, etc.)
  - Clipboard instance now persists throughout app lifetime (required for X11 to serve paste requests)
  - Previously, clipboard was immediately dropped after copy, causing content to disappear on Linux
  - Fixes reported issue: "unable to copy the content of section using keybindings 'y/Y'" on Arch Linux + i3
  - macOS and Windows unaffected (different clipboard models)

- **Modal State Blocking** - Copy operations now work in all application modes
  - Added `y` (copy content) and `Y` (copy anchor) handlers to link follow mode
  - Added `y`/`Y` handlers to help mode (`?`)
  - Added `y`/`Y` handlers to theme picker mode (`t`)
  - Previously only worked in normal mode, causing confusion for users

### Added

- **Clipboard Status Feedback** - All copy operations now provide visual confirmation
  - Success: "✓ Section copied to clipboard"
  - Success: "✓ Anchor link copied: #heading-name"
  - Error: "✗ No heading selected"
  - Error: "✗ Could not extract section"
  - Error: "✗ Clipboard not available"
  - Error: "✗ Clipboard error: {details}"

- **Linux Clipboard Manager Recommendation** - Help screen now includes setup guidance
  - Recommends installing clipboard manager (clipit, parcellite, xclip) for best results on Linux
  - Helps users understand X11 clipboard behavior and workarounds

### Changed

- **Persistent Clipboard Architecture** - App struct now maintains clipboard instance
  - `clipboard: Option<arboard::Clipboard>` field added to App struct
  - Initialized once in `App::new()` and kept alive for entire session
  - Comprehensive error handling with Result pattern instead of silent failures
  - All clipboard errors now properly surfaced to user

- **Help Documentation** - Updated clipboard keybinding descriptions
  - Clarified that `y` and `Y` work in all modes (not just normal mode)
  - Added prominent note about Linux clipboard manager recommendation

### Technical

- **App State Enhancement** (`src/tui/app.rs`)
  - Added `clipboard: Option<arboard::Clipboard>` field (line 60)
  - Initialize clipboard in `App::new()` with `.ok()` fallback (line 134)
  - Rewrote `copy_content()` with comprehensive error handling (lines 608-631)
  - Rewrote `copy_anchor()` with comprehensive error handling (lines 633-657)

- **Event Handling Updates** (`src/tui/mod.rs`)
  - Added `y`/`Y` handlers to help mode (lines 61-62)
  - Added `y`/`Y` handlers to theme picker mode (lines 75-76)
  - Added `y`/`Y` handlers to link follow mode (lines 110-111)

- **UI Documentation** (`src/tui/ui.rs`)
  - Updated help text for copy operations (lines 504, 508)
  - Added Linux clipboard manager recommendation (lines 515-523)

- **Code Quality**
  - Zero clippy warnings
  - Clean compilation
  - Proper error propagation (no more silent `let _ =` failures)
  - Follows Rust best practices for Option and Result handling

### Platform-Specific Notes

- **Linux (X11)**: Persistent clipboard instance fixes paste failures. Clipboard manager recommended.
- **Linux (Wayland)**: Uses `wayland-data-control` feature, persistent instance recommended.
- **macOS**: Works as before (system manages clipboard, no persistence needed).
- **Windows**: Works as before (system manages clipboard, no persistence needed).

## [0.2.0] - 2025-11-13

### Added

- **Link Following System** - Complete markdown link navigation with visual feedback and multi-file support
  - Press `f` to enter link follow mode with interactive link picker popup
  - Navigate links with `Tab`/`Shift+Tab`, `j`/`k`, or arrow keys
  - Jump directly to links using number keys (`1-9`)
  - Visual popup shows all links in current section with highlighting
  - Selected link indicated with green arrow (▶), bold, and underline
  - Real-time status messages for all actions

- **Link Type Support** - Handles all markdown link formats
  - **Anchor links** - `[Go](#installation)` jumps to heading in current file
  - **Relative file links** - `[API](./docs/api.md)` loads markdown files
  - **File + anchor links** - `[Guide](./guide.md#usage)` loads file and jumps to section
  - **WikiLinks** - `[[README]]` and `[[README|docs]]` with Obsidian-style syntax
  - **External URLs** - `[GitHub](https://...)` opens in default browser + copies to clipboard

- **Navigation History** - Back/forward navigation between files
  - Press `b` or `Backspace` to go back to previous file
  - Press `Shift+F` to go forward in navigation history
  - Full state preservation (scroll position, selected heading)
  - Separate history stacks for back and forward navigation

- **Parent Jump** - Quick navigation to parent headings
  - Press `p` in normal mode to jump to parent heading in outline
  - Press `p` in link follow mode to jump to parent's links (stays in link mode)
  - Searches backwards for nearest heading with lower level
  - Status messages indicate when already at top-level

- **Cross-Platform Browser Integration** - Reliable URL opening
  - Uses `open` crate for macOS, Linux, Windows, and WSL support
  - Automatically opens external links in default browser
  - Fallback to clipboard if browser fails
  - User-friendly status messages for all outcomes

- **Live File Editing** - Edit files in default editor with auto-reload
  - Press `e` to open current file in editor (respects `$VISUAL` and `$EDITOR`)
  - Proper terminal suspension and restoration (follows ratatui best practices)
  - Auto-reloads file after editing with position preservation
  - Restores heading selection and scroll position when possible
  - Works with vim, nano, emacs, VS Code, or any configured editor
  - Uses `edit` crate for reliable cross-platform editor detection

### Changed

- **App State Enhancement** - Added comprehensive link following state management
  - New `AppMode` enum: `Normal`, `LinkFollow`, `Search`, `ThemePicker`, `Help`
  - `FileState` struct for navigation history with full document state
  - Link tracking: `links_in_view`, `selected_link_idx`, `file_history`, `file_future`
  - Temporary status message system with icons (✓, ⚠, ✗)

- **UI Enhancements** - Better visual feedback for all operations
  - Link navigator popup with styled content (80% width, 60% height)
  - Enhanced status bar shows current link details in link mode
  - Content title displays link count: `[Links: 3]`
  - Help screen updated with link following keybindings section

- **Event Handling** - New keyboard shortcuts for link navigation and editing
  - `f` - Enter link follow mode
  - `Tab`/`Shift+Tab` - Navigate links forward/backward
  - `j`/`k`/`↓`/`↑` - Navigate links (vim-style + arrows)
  - `1-9` - Jump directly to link by number
  - `Enter` - Follow selected link
  - `Esc` - Exit link follow mode
  - `p` - Jump to parent (context-aware)
  - `b`/`Backspace` - Go back
  - `Shift+F` - Go forward
  - `e` - Edit current file in default editor

### Technical

- **New Parser Module** - `src/parser/links.rs` (320 lines)
  - `Link` struct with text, target, and byte offset
  - `LinkTarget` enum for type-safe link representation
  - `extract_links()` function with two-pass parsing
  - 10 comprehensive tests covering all link types
  - Custom wikilink regex parser for `[[filename]]` syntax

- **Link Detection** - Robust parsing using pulldown-cmark
  - First pass: Standard markdown links via pulldown-cmark events
  - Second pass: Custom regex for wikilink syntax
  - Extracts link text, target, and byte offset for each link
  - Handles malformed links gracefully

- **File Resolution** - Smart path and wikilink handling
  - Resolves relative file paths from current file location
  - Wikilink search in current directory (`.md` extension added automatically)
  - Anchor normalization (lowercase, dash-separated)
  - Error handling with descriptive messages

- **Visual Rendering** - Popup overlay system
  - `render_link_picker()` function (130 lines)
  - Centered popup with styled spans for each link
  - Color-coded elements (green/yellow/white/gray)
  - Scrollable for many links
  - Footer with keybinding hints

- **State Management** - Clean separation of concerns
  - Link mode completely separate from normal navigation
  - History stacks preserve full document state
  - Status messages cleared on next keypress
  - Mode transitions preserve relevant state

- **Terminal Management** - Proper TUI suspension for external programs
  - `run_editor()` function handles terminal state transitions
  - Suspends TUI: LeaveAlternateScreen → disable_raw_mode
  - Spawns editor with full terminal control
  - Restores TUI: EnterAlternateScreen → enable_raw_mode → clear
  - Follows official ratatui best practices for external process spawning
  - Prevents rendering artifacts and ANSI escape code leakage

- **Dependencies Added**
  - `open = "5.3"` - Cross-platform URL/file opening
  - `edit = "0.1"` - Cross-platform editor detection and invocation

- **Code Quality**
  - Zero clippy warnings
  - All 21 tests passing (18 unit + 3 doc tests)
  - Comprehensive documentation
  - Clean error handling throughout

## [0.1.7] - 2025-01-10

### Fixed

- **Tab completion for current directory files** - Fixed bug where `treemd R<tab>` wouldn't complete to `README.md` in the current directory. Path::parent() returns empty string for simple filenames, which is now normalized to "." for proper completion matching.

### Added

- **Filename in title bar** - Title bar now displays the filename being viewed: "treemd - README.md - 15 headings"
- **Current heading in content pane** - Content pane header now shows the selected heading name instead of the generic "Content" label, providing better context while reading

### Changed

- **App struct enhancement** - Added `filename` field to track the source file for display purposes
- **Content pane title logic** - Title dynamically updates based on selected heading, falling back to "Content" when none selected

### Technical

- Normalized empty parent paths in file completer to fix `Path::new("R").parent()` returning `Some("")` instead of `Some(".")`
- Extracted filename from PathBuf when launching TUI mode using `file_name()` and `to_str()`

## [0.1.6] - 2025-01-09

### Fixed

- **TUI section extraction with inline markdown** - Fixed critical bug where selecting headings with inline formatting (like `**bold**`) would display the entire document instead of just that section
- **JSON output content extraction** - Fixed nested JSON output where parent sections incorrectly included child heading text in their content
- **Parent directory completions** - Shell completions now work with `../` relative paths and absolute paths, enabling navigation to parent directories

### Added

- **Offset-based parsing** - Implemented pulldown-cmark's `into_offset_iter()` for direct byte offset tracking, eliminating fragile string searching
- **Shared utilities module** - Created `parser/utils.rs` with `strip_markdown_inline()` and `get_heading_level()` helpers
- **Comprehensive test suite** - Added 8+ new tests covering bold headings, numbered headings, offset tracking, and section extraction edge cases
- **Context-aware completions** - Upgraded from `ArgValueCandidates` to `ArgValueCompleter` for dynamic path-based completions

### Changed

- **Heading struct enhancement** - Added `offset: usize` field to store byte position for O(1) section extraction
- **Parser optimization** - Changed from O(n²) string searching to O(n) offset-based extraction
- **Code deduplication** - Eliminated 40+ lines of duplicate code by centralizing utilities
- **Completion logic** - Completions now parse input path to determine target directory and filter appropriately

### Technical

- **DRY principle compliance** - Removed duplicate `strip_markdown_inline` and `get_heading_level` functions
- **Best practices adoption** - Using pulldown-cmark's built-in offset tracking as recommended by Rust markdown ecosystem
- **Performance improvement** - Section extraction now O(1) lookup instead of O(n) string search
- **Robustness improvement** - Handles all inline markdown formatting (bold, italic, code, strikethrough) correctly
- **Architecture cleanup** - Better separation of concerns with dedicated utils module
- **Zero clippy warnings** - Clean codebase with all lints addressed

## [0.1.5] - 2025-11-09

### Fixed

- **Binary corruption in releases** - Wrap binaries in platform-appropriate archives to preserve file permissions and prevent corruption through GitHub Actions artifact system
- **Executable permissions lost** - Fixed issue where Unix binaries lost +x permission during artifact upload/download
- **SHA256 mismatch** - Generate checksums at build time and verify after extraction to ensure binary integrity
- **Cache conflicts** - Use unique cache keys per target to prevent 409 conflicts in parallel builds
- **Cross-platform compatibility** - Use native tools on each platform (PowerShell on Windows, bash on Unix)

### Added

- **Individual SHA256 files** - Each binary now has a corresponding .sha256 file generated at build time
- **Combined SHA256SUMS** - All binaries included in a single SHA256SUMS file in releases for easy verification
- **Checksum verification** - Automated verification step that checksums match after artifact extraction
- **Build-time checksums** - SHA256 printed during build for traceability
- **Platform-specific packaging** - Unix binaries distributed as .tar.gz, Windows binaries as .zip

### Changed

- **Updated GitHub Actions** - Upgraded all actions to latest major versions for automatic updates and security
- **Optimized cross installation** - Use cargo-binstall for 100x faster installation (2s vs 3m39s)
  - Install from main branch (cross-rs hasn't released since v0.2.5 in Feb 2023)
  - Pinned to commit 8633ec6 (Nov 2025) for reproducibility
  - Follows 2025 best practices for actively-developed-but-not-released projects

### Technical

- Unix: tar.gz archives preserve executable permissions through artifact system
- Windows: zip archives created with PowerShell's Compress-Archive
- Unix: SHA256 generated with sha256sum, Windows: Get-FileHash cmdlet
- Cache keys now include both target and Cargo.lock hash
- Comprehensive logging at each stage (build, extract, verify)
- Release extraction handles both tar.gz and zip formats

## [0.1.4] - 2025-11-09

### Changed

- Version bump for development

## [0.1.3] - 2025-11-08

### Added

- **Shell tab completions** - Native dynamic completions for bash, zsh, and fish shells
- **Intelligent file filtering** - Tab completion intelligently filters to only show `.md` and `.markdown` files
- **Interactive setup helper** - `--setup-completions` flag to auto-detect shell and configure completion with one command
- **Auto shell detection** - Automatically detects bash, zsh, or fish and locates shell config files
- **Enhanced help system** - Comprehensive `--help` menu with detailed descriptions, examples, and usage patterns
- **Setup instructions** - Clear instructions for manual completion setup if automated setup is declined

### Changed

- **Feature enablement** - `unstable-dynamic` feature now enabled by default for seamless completion experience
- **Help documentation** - All CLI options now have detailed descriptions with inline examples

### Technical

- Integrated `clap_complete 4.5.60` with `unstable-dynamic` feature flag
- Implemented `CompleteEnv` for runtime completion generation
- Created custom `ArgValueCandidates` for markdown file filtering
- Added interactive setup module with shell detection (`src/cli/setup.rs`)

## [0.1.2] - 2025-11-08

### Fixed

- **Content display robustness** - Fixed critical bugs where content would disappear or show incorrect sections after navigation and collapse/expand operations
- **Selection preservation** - Selection now correctly preserved by heading text instead of index during collapse/expand operations
- **Content scroll reset** - Content scroll now properly resets to top when navigating between different sections
- **Dynamic content height** - Content height and scrollbar now correctly update based on the currently selected section
- **Collapse parent behavior** - Collapsing a parent heading now correctly selects the parent instead of an arbitrary item
- **Search filter preservation** - Search filtering now maintains the current selection when possible instead of always jumping to first item
- **Bookmark stability** - Bookmarks now store heading text instead of indices, remaining valid after collapse operations

### Technical

- Added `select_by_text()` helper method for robust text-based selection
- Added `update_content_metrics()` to synchronize content height and scroll state
- Added `previous_selection` tracking to detect selection changes
- Changed bookmark storage from `Option<usize>` to `Option<String>`

## [0.1.1] - 2025-11-08 - Add library

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
