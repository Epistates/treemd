# Link Following Test Document

This document demonstrates the link following feature in treemd.

## Internal Anchor Links

You can jump to other sections in this document:
- [Jump to Features](#features)
- [Jump to Installation](#installation)
- [Jump to Usage](#usage)

## External Links

Visit these external sites:
- [GitHub](https://github.com)
- [Rust Documentation](https://doc.rust-lang.org)
- [Ratatui](https://ratatui.rs)

## Wikilinks

These use Obsidian-style syntax:
- [[README]] - Links to README.md
- [[CHANGELOG]] - Links to CHANGELOG.md
- [[README|Custom Display Text]] - With custom alias

### Wikilinks with Internal Anchors (Issue #29 Test)

These should jump to sections within THIS document:
- [[#Features]] - Jump to Features section (shorthand)
- [[#Installation]] - Jump to Installation section
- [[test_links#Usage]] - Explicit same-file anchor
- [[#Mixed Links Test|Mixed Links (alias)]] - With display text

### Wikilinks with External File Anchors (Issue #29 Test)

These should jump to sections in OTHER markdown files:
- [[README#Features]] - Jump to Features in README
- [[README#installation]] - Lowercase anchor style
- [[README#Interactive TUI]] - Heading with spaces
- [[CHANGELOG#Added|What's New]] - CHANGELOG section with alias

## Relative File Links

Navigate to other files in the project:
- [Main README](./README.md) - Link to README in current directory
- [Changelog](./CHANGELOG.md) - View the changelog
- [README with anchor](./README.md#installation) - Jump to specific section

## Features

Some key features of link following:
- Press `f` to enter link follow mode
- Use `Tab` to cycle through links
- Press `Enter` to follow the selected link
- Press `Esc` to exit link follow mode

External URLs are copied to clipboard when followed.

## Installation

1. Install from crates.io:
   ```bash
   cargo install treemd
   ```

2. Or build from source:
   ```bash
   git clone https://github.com/epistates/treemd
   cd treemd
   cargo install --path .
   ```

See the [main README](./README.md#installation) for more details.

## Usage

Basic usage is simple. Just run:

```bash
treemd your-file.md
```

Then press `f` to enter link follow mode and start navigating!

Check out the [full documentation](./README.md) for all features.

## Navigation

File navigation includes:
- `b` or Backspace - Go back to previous file
- `F` (Shift+F) - Go forward in history

The back/forward stack maintains your position and scroll state in each file.

## Mixed Links Test

Here's a paragraph with multiple types of links: Check the [Features](#features) section above,
visit [[CHANGELOG]] for version history, read the [README](./README.md), and learn more at
[ratatui.rs](https://ratatui.rs). You can also see [[README|our readme with an alias]].

## Search Issue Tests (Issue #34)

These test cases verify search highlighting works correctly with links.

### Test 1: Links Preserve Styling During Search

Search for "con" - links should remain styled as links (underlined, colored):
- [Configuration Guide](./README.md) - markdown link containing "con"
- [[CHANGELOG|Contribution history]] - wikilink with "con" in display text
- Visit the [console documentation](https://docs.rs) for more info

### Test 2: Wikilink Target and Text Both Match

Search for "read" - both target and display text contain the search term:
- [[README|README file]] - "read" appears in both target and text
- [[README|Read the docs]] - "Read" in text, "README" in target
- [[CHANGELOG|Already read]] - only text matches

### Test 3: Search Within Link Text

Search for "jump" to test highlighting within link text:
- [Jump to top](#navigation) - "jump" at start
- [Click to jump here](#features) - "jump" in middle
- [[#Installation|Jump down]] - wikilink with "jump"

### Test 4: Multiple Matches in One Line

Search for "link" - multiple matches with mixed link types:
- This line has a [markdown link](#features) and a [[wikilink|wiki link]] together.
- Multiple links: [link one](#usage), [link two](#features), [[link three]]

### Test 5: Case Insensitive Search

Search for "README" (or "readme") - should match regardless of case:
- [[README]] - uppercase target
- [[readme]] - lowercase target (if file exists)
- [The README file](./README.md) - mixed case in text
- Check the readme for details

### Test 6: Scroll Position Preservation

Instructions for manual testing:
1. Scroll down to this section
2. Press `/` to search
3. Type "test" and press Enter
4. Navigate with `n`/`N` to different matches
5. Press `Esc` - should stay at current position, NOT jump to top

### Test 7: Mode Transitions from Search

After searching (press Enter to accept):
- Press `i` - should enter interactive mode
- Press `f` - should enter link follow mode
- Press `Esc` - should clear search and stay in normal mode
