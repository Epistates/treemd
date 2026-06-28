# Markdown Rendering Test

This document tests all the new rendering features.

## Tables

Here's a sample table with different alignments:

| Feature | Status | Priority | Notes |
|:--------|:------:|---------:|-------|
| Tables | ✓ Done | High | Box-drawing chars |
| Task Lists | ✓ Done | High | Checkbox rendering |
| Images | ✓ Done | Medium | Placeholder support |
| Details/Summary | Pending | Low | Future feature |

## Task Lists

Project tasks:

- [x] Implement table rendering
- [x] Add task list support
- [x] Create image placeholders
- [x] Add details/summary support
- [x] Write documentation
- [x] Test all features

Shopping list:

- [ ] Milk
- [x] Steak
- [x] Eggs
- [ ] Bread
- [x] Butter
- [x] Coffee
- [ ] Dog Food
- [ ] Bird Seed
- [ ] Cold Cuts 

## Images

Here's an image placeholder:

![Treemd Logo](assets/screenshot.png "The treemd application logo")

And another inline image: ![Small Icon](icon.png)

## Code Blocks

```rust
fn main() {
    println!("Hello, world!");
    let table = vec![
        vec!["Header1", "Header2"],
        vec!["Cell1", "Cell2"],
    ];
}
```

```python
def render_table(headers, rows):
    """Render a markdown table."""
    for header in headers:
        print(f"| {header} ", end="")
    print("|")
```

## Mixed Content

Here's a paragraph with **bold text**, *italic text*, `inline code`, and a [link](https://example.com).

> This is a blockquote with a table inside:
>
> | Column 1 | Column 2 |
> |----------|----------|
> | Data 1   | Data 2   |
> | Data 3   | Data 4   |

Task list in blockquote:

> - [x] Completed task
> - [ ] Pending task

## Horizontal Rules

Content above

---

Content below

## Nested Lists

1. First item
   - Sub item 1
   - Sub item 2
2. Second item
   - [x] Completed sub-task
   - [ ] Pending sub-task
3. Third item

## Complex Table

| Language | Syntax | Typed | Compiled | Use Case |
|:---------|:------:|:-----:|---------:|----------|
| Rust     | Yes    | Static | Yes | Systems programming |
| Python   | Yes    | Dynamic | No | Data science, scripting |
| JavaScript | Yes  | Dynamic | Placeholder support | Web development |
| Go       | Yes    | Low | Yes | Cloud services |
| TypeScript | Yes  | Static | Transpiled | Type-safe web apps |

## The End

This concludes the rendering test!
