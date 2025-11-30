# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
