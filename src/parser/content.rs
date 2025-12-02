//! Content parsing for markdown sections
//!
//! Parses markdown content into semantic blocks and inline elements.

use super::output::{Alignment, Block, InlineElement, ListItem};
use pulldown_cmark::{
    Alignment as CmarkAlignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
};

/// Parse markdown content into structured blocks
pub fn parse_content(markdown: &str, start_line: usize) -> Vec<Block> {
    // First, extract any <details> blocks and replace them with placeholders
    let (processed_markdown, details_blocks) = extract_details_blocks(markdown);

    // Enable GitHub Flavored Markdown extensions
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&processed_markdown, options);
    let mut blocks = Vec::new();
    let mut state = ParserState::new(start_line);

    for event in parser {
        process_event(event, &mut state, &mut blocks);
    }

    // Flush any pending block
    state.finalize(&mut blocks);

    // Replace placeholders with actual Details blocks
    let mut final_blocks = Vec::new();
    for block in blocks {
        if let Block::Paragraph { content, .. } = &block {
            // Check if this paragraph contains only the placeholder
            let trimmed = content.trim();
            if trimmed.starts_with("[DETAILS_BLOCK_") && trimmed.ends_with(']') {
                if let Some(index_str) = trimmed.strip_prefix("[DETAILS_BLOCK_") {
                    if let Some(index_str) = index_str.strip_suffix(']') {
                        if let Ok(index) = index_str.parse::<usize>() {
                            if let Some(details_block) = details_blocks.get(index) {
                                final_blocks.push(details_block.clone());
                                continue;
                            }
                        }
                    }
                }
            }
        }
        final_blocks.push(block);
    }

    final_blocks
}

/// Extract <details> blocks from markdown and replace with placeholders
fn extract_details_blocks(markdown: &str) -> (String, Vec<Block>) {
    let mut details_blocks = Vec::new();
    let mut result = String::new();
    let mut current_pos = 0;

    while current_pos < markdown.len() {
        // Look for <details> tag
        if markdown[current_pos..].starts_with("<details") {
            // Find the end of the opening tag
            if let Some(tag_end) = markdown[current_pos..].find('>') {
                let details_start = current_pos + tag_end + 1;

                // Find the matching </details> tag
                if let Some(details_end_pos) = markdown[details_start..].find("</details>") {
                    let details_end = details_start + details_end_pos;
                    let details_content = &markdown[details_start..details_end];

                    // Extract summary
                    let summary = if let Some(summary_start_pos) = details_content.find("<summary")
                    {
                        if let Some(summary_tag_end) =
                            details_content[summary_start_pos..].find('>')
                        {
                            let summary_content_start = summary_start_pos + summary_tag_end + 1;
                            if let Some(summary_end_pos) =
                                details_content[summary_content_start..].find("</summary>")
                            {
                                let summary_end = summary_content_start + summary_end_pos;
                                details_content[summary_content_start..summary_end]
                                    .trim()
                                    .to_string()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    // Extract content (everything after </summary>)
                    let content_start =
                        if let Some(summary_end_pos) = details_content.find("</summary>") {
                            let summary_tag_end = summary_end_pos + "</summary>".len();
                            &details_content[summary_tag_end..]
                        } else {
                            details_content
                        };

                    let content_trimmed = content_start.trim();

                    // Parse the content inside details
                    let nested_blocks = if !content_trimmed.is_empty() {
                        parse_content(content_trimmed, 0)
                    } else {
                        Vec::new()
                    };

                    // Create the Details block
                    details_blocks.push(Block::Details {
                        summary,
                        content: content_trimmed.to_string(),
                        blocks: nested_blocks,
                    });

                    // Add placeholder
                    result.push_str(&format!("\n[DETAILS_BLOCK_{}]\n", details_blocks.len() - 1));

                    // Skip past the entire details block
                    current_pos = details_end + "</details>".len();
                    continue;
                }
            }
        }

        // Copy character to result
        if let Some(ch) = markdown[current_pos..].chars().next() {
            result.push(ch);
            current_pos += ch.len_utf8();
        } else {
            break;
        }
    }

    (result, details_blocks)
}

struct ParserState {
    current_line: usize,
    paragraph_buffer: String,
    inline_buffer: Vec<InlineElement>,
    list_items: Vec<ListItem>,
    list_ordered: bool,
    list_depth: usize,
    item_depth: usize,
    task_list_marker: Option<bool>,
    saved_task_markers: Vec<Option<bool>>,
    /// Blocks accumulated within the current list item (code blocks, paragraphs, etc.)
    item_blocks: Vec<Block>,
    code_buffer: String,
    code_language: Option<String>,
    code_start_line: usize,
    blockquote_buffer: String,
    table_headers: Vec<String>,
    table_alignments: Vec<Alignment>,
    table_rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    in_paragraph: bool,
    in_list: bool,
    in_code: bool,
    in_blockquote: bool,
    in_table: bool,
    in_strong: bool,
    in_emphasis: bool,
    in_strikethrough: bool,
    in_code_inline: bool,
    in_link: bool,
    link_url: String,
    link_text: String,
    image_in_link: bool,
    in_image: bool,
    saved_link_url: String,
}

impl ParserState {
    fn new(start_line: usize) -> Self {
        Self {
            current_line: start_line,
            paragraph_buffer: String::new(),
            inline_buffer: Vec::new(),
            list_items: Vec::new(),
            list_ordered: false,
            list_depth: 0,
            item_depth: 0,
            task_list_marker: None,
            saved_task_markers: Vec::new(),
            item_blocks: Vec::new(),
            code_buffer: String::new(),
            code_language: None,
            code_start_line: 0,
            blockquote_buffer: String::new(),
            table_headers: Vec::new(),
            table_alignments: Vec::new(),
            table_rows: Vec::new(),
            current_row: Vec::new(),
            in_paragraph: false,
            in_list: false,
            in_code: false,
            in_blockquote: false,
            in_table: false,
            in_strong: false,
            in_emphasis: false,
            in_strikethrough: false,
            in_code_inline: false,
            in_link: false,
            link_url: String::new(),
            link_text: String::new(),
            image_in_link: false,
            in_image: false,
            saved_link_url: String::new(),
        }
    }

    fn finalize(&mut self, blocks: &mut Vec<Block>) {
        self.flush_paragraph(blocks);
        self.flush_list(blocks);
        self.flush_code(blocks);
        self.flush_blockquote(blocks);
        self.flush_table(blocks);
    }

    fn flush_paragraph(&mut self, blocks: &mut Vec<Block>) {
        if self.in_paragraph && !self.paragraph_buffer.is_empty() {
            blocks.push(Block::Paragraph {
                content: self.paragraph_buffer.clone(),
                inline: self.inline_buffer.clone(),
            });
            self.paragraph_buffer.clear();
            self.inline_buffer.clear();
            self.in_paragraph = false;
        }
    }

    fn flush_list(&mut self, blocks: &mut Vec<Block>) {
        if self.in_list && !self.list_items.is_empty() {
            blocks.push(Block::List {
                ordered: self.list_ordered,
                items: self.list_items.clone(),
            });
            self.list_items.clear();
            self.in_list = false;
        }
    }

    fn flush_code(&mut self, blocks: &mut Vec<Block>) {
        if self.in_code && !self.code_buffer.is_empty() {
            blocks.push(Block::Code {
                language: self.code_language.clone(),
                content: self.code_buffer.trim_end().to_string(),
                start_line: self.code_start_line,
                end_line: self.current_line,
            });
            self.code_buffer.clear();
            self.code_language = None;
            self.in_code = false;
        }
    }

    fn flush_blockquote(&mut self, blocks: &mut Vec<Block>) {
        if self.in_blockquote && !self.blockquote_buffer.is_empty() {
            let nested_blocks = parse_content(&self.blockquote_buffer, self.current_line);
            blocks.push(Block::Blockquote {
                content: self.blockquote_buffer.clone(),
                blocks: nested_blocks,
            });
            self.blockquote_buffer.clear();
            self.in_blockquote = false;
        }
    }

    fn flush_table(&mut self, blocks: &mut Vec<Block>) {
        if self.in_table && !self.table_headers.is_empty() {
            blocks.push(Block::Table {
                headers: self.table_headers.clone(),
                alignments: self.table_alignments.clone(),
                rows: self.table_rows.clone(),
            });
            self.table_headers.clear();
            self.table_alignments.clear();
            self.table_rows.clear();
            self.current_row.clear();
            self.paragraph_buffer.clear();
            self.inline_buffer.clear();
            self.in_table = false;
        }
    }

    fn add_inline_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let element = if self.in_code_inline {
            InlineElement::Code {
                value: text.to_string(),
            }
        } else if self.in_strong {
            InlineElement::Strong {
                value: text.to_string(),
            }
        } else if self.in_emphasis {
            InlineElement::Emphasis {
                value: text.to_string(),
            }
        } else if self.in_strikethrough {
            InlineElement::Strikethrough {
                value: text.to_string(),
            }
        } else {
            InlineElement::Text {
                value: text.to_string(),
            }
        };

        self.inline_buffer.push(element);
        self.paragraph_buffer.push_str(text);
    }
}

#[allow(clippy::too_many_lines)]
fn process_event(event: Event, state: &mut ParserState, blocks: &mut Vec<Block>) {
    match event {
        Event::Start(Tag::Paragraph) => {
            state.in_paragraph = true;
        }
        Event::End(TagEnd::Paragraph) => {
            // When inside a list item, add paragraph to item_blocks instead of main blocks
            if state.item_depth >= 1 && state.in_paragraph && !state.paragraph_buffer.is_empty() {
                state.item_blocks.push(Block::Paragraph {
                    content: state.paragraph_buffer.clone(),
                    inline: state.inline_buffer.clone(),
                });
                state.paragraph_buffer.clear();
                state.inline_buffer.clear();
                state.in_paragraph = false;
            } else {
                state.flush_paragraph(blocks);
            }
        }
        Event::Start(Tag::CodeBlock(kind)) => {
            state.in_code = true;
            state.code_start_line = state.current_line;
            state.code_language = match kind {
                CodeBlockKind::Fenced(lang) => {
                    if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    }
                }
                CodeBlockKind::Indented => None,
            };
        }
        Event::End(TagEnd::CodeBlock) => {
            // When inside a list item, add code block to item_blocks instead of main blocks
            if state.item_depth >= 1 && state.in_code && !state.code_buffer.is_empty() {
                state.item_blocks.push(Block::Code {
                    language: state.code_language.clone(),
                    content: state.code_buffer.trim_end().to_string(),
                    start_line: state.code_start_line,
                    end_line: state.current_line,
                });
                state.code_buffer.clear();
                state.code_language = None;
                state.in_code = false;
            } else {
                state.flush_code(blocks);
            }
        }
        Event::Start(Tag::List(start_number)) => {
            state.list_depth += 1;
            // Only set list properties for the outermost list
            if state.list_depth == 1 {
                state.in_list = true;
                state.list_ordered = start_number.is_some();
            }
        }
        Event::End(TagEnd::List(_)) => {
            state.list_depth = state.list_depth.saturating_sub(1);
            // Only flush when exiting the outermost list
            if state.list_depth == 0 {
                state.flush_list(blocks);
            }
        }
        Event::Start(Tag::Item) => {
            state.item_depth += 1;

            // Save current task marker when entering nested item
            if state.item_depth > 1 {
                state.saved_task_markers.push(state.task_list_marker);
                state.task_list_marker = None;
            }

            // Only clear buffers for root-level items
            if state.item_depth == 1 {
                state.paragraph_buffer.clear();
                state.inline_buffer.clear();
                state.item_blocks.clear();
            }
        }
        Event::End(TagEnd::Item) => {
            // Restore saved task marker when exiting nested item
            if state.item_depth > 1 {
                if let Some(saved) = state.saved_task_markers.pop() {
                    state.task_list_marker = saved;
                }
            }

            // Only save items at root level (depth 1)
            if state.item_depth == 1 {
                // Determine content/inline fields:
                // 1. If paragraph_buffer has content (tight list item - no paragraph wrapper), use it
                // 2. Else if item_blocks starts with a Paragraph, extract it
                // 3. Else use empty content
                let (content, inline, remaining_blocks) =
                    if !state.paragraph_buffer.is_empty() {
                        // Tight list item: text was added directly without paragraph wrapper
                        let all_blocks: Vec<Block> = state.item_blocks.drain(..).collect();
                        (
                            state.paragraph_buffer.clone(),
                            state.inline_buffer.clone(),
                            all_blocks,
                        )
                    } else if let Some(Block::Paragraph { content, inline }) =
                        state.item_blocks.first().cloned()
                    {
                        // Loose list item: first block is a paragraph
                        let remaining: Vec<Block> = state.item_blocks.drain(1..).collect();
                        (content, inline, remaining)
                    } else {
                        // Item starts with code block or other non-paragraph block
                        let all_blocks: Vec<Block> = state.item_blocks.drain(..).collect();
                        (String::new(), Vec::new(), all_blocks)
                    };

                state.list_items.push(ListItem {
                    checked: state.task_list_marker,
                    content,
                    inline,
                    blocks: remaining_blocks,
                });
                state.paragraph_buffer.clear();
                state.inline_buffer.clear();
                state.item_blocks.clear();
                state.task_list_marker = None;
            }
            state.item_depth = state.item_depth.saturating_sub(1);
        }
        Event::TaskListMarker(checked) => {
            state.task_list_marker = Some(checked);
            // Checkbox marker will be added when text is encountered (see Text event)
        }
        Event::Start(Tag::BlockQuote(_)) => {
            state.in_blockquote = true;
        }
        Event::End(TagEnd::BlockQuote(_)) => {
            state.flush_blockquote(blocks);
        }
        Event::Start(Tag::Table(alignments)) => {
            state.in_table = true;
            state.table_alignments = alignments
                .iter()
                .map(|a| match a {
                    CmarkAlignment::Left => Alignment::Left,
                    CmarkAlignment::Center => Alignment::Center,
                    CmarkAlignment::Right => Alignment::Right,
                    CmarkAlignment::None => Alignment::None,
                })
                .collect();
        }
        Event::End(TagEnd::Table) => {
            state.flush_table(blocks);
        }
        Event::Start(Tag::TableHead) => {}
        Event::End(TagEnd::TableHead) => {
            state.table_headers = state.current_row.clone();
            state.current_row.clear();
        }
        Event::Start(Tag::TableRow) => {}
        Event::End(TagEnd::TableRow) => {
            state.table_rows.push(state.current_row.clone());
            state.current_row.clear();
        }
        Event::Start(Tag::TableCell) => {
            state.paragraph_buffer.clear();
            state.inline_buffer.clear();
        }
        Event::End(TagEnd::TableCell) => {
            state.current_row.push(state.paragraph_buffer.clone());
            state.paragraph_buffer.clear();
            state.inline_buffer.clear();
        }
        Event::Start(Tag::Strong) => {
            state.in_strong = true;
        }
        Event::End(TagEnd::Strong) => {
            state.in_strong = false;
        }
        Event::Start(Tag::Emphasis) => {
            state.in_emphasis = true;
        }
        Event::End(TagEnd::Emphasis) => {
            state.in_emphasis = false;
        }
        Event::Start(Tag::Strikethrough) => {
            state.in_strikethrough = true;
        }
        Event::End(TagEnd::Strikethrough) => {
            state.in_strikethrough = false;
        }
        Event::Code(text) => {
            state.in_code_inline = true;
            state.add_inline_text(&text);
            state.in_code_inline = false;
        }
        Event::Start(Tag::Link { dest_url, .. }) => {
            state.in_link = true;
            state.link_url = dest_url.to_string();
            state.link_text.clear();
        }
        Event::End(TagEnd::Link) => {
            state.in_link = false;

            // For image-in-link (badge pattern), create a link with the alt text and outer URL
            if state.image_in_link {
                state.inline_buffer.push(InlineElement::Link {
                    text: state.link_text.clone(),
                    url: state.saved_link_url.clone(),
                    title: None,
                });
                state
                    .paragraph_buffer
                    .push_str(&format!("[{}]({})", state.link_text, state.saved_link_url));
            } else {
                // Regular link
                state.inline_buffer.push(InlineElement::Link {
                    text: state.link_text.clone(),
                    url: state.link_url.clone(),
                    title: None,
                });
                state
                    .paragraph_buffer
                    .push_str(&format!("[{}]({})", state.link_text, state.link_url));
            }

            // Reset state
            state.link_text.clear();
            state.link_url.clear();
            state.saved_link_url.clear();
            state.image_in_link = false;
        }
        Event::Start(Tag::Image {
            dest_url, title, ..
        }) => {
            // Check if we're inside a link (badge pattern)
            if state.in_link {
                state.image_in_link = true;
                // Save the outer link URL before it gets overwritten by image src
                state.saved_link_url = state.link_url.clone();
            }
            state.in_image = true;
            state.link_url = dest_url.to_string();
            state.link_text.clear();
            state.paragraph_buffer = title.to_string();
        }
        Event::End(TagEnd::Image) => {
            state.in_image = false;

            // If image is inside a link (badge pattern), don't create image element yet
            // The link end event will use the alt text to create a proper link
            if !state.image_in_link {
                // If image is inside a paragraph, make it inline
                if state.in_paragraph {
                    // Create inline image element
                    state.inline_buffer.push(InlineElement::Image {
                        alt: state.link_text.clone(),
                        src: state.link_url.clone(),
                        title: if state.paragraph_buffer.is_empty() {
                            None
                        } else {
                            Some(state.paragraph_buffer.clone())
                        },
                    });
                    state
                        .paragraph_buffer
                        .push_str(&format!("[{}]", state.link_text));
                } else {
                    // Flush any pending blocks before adding block image
                    state.flush_paragraph(blocks);

                    blocks.push(Block::Image {
                        alt: state.link_text.clone(),
                        src: state.link_url.clone(),
                        title: if state.paragraph_buffer.is_empty() {
                            None
                        } else {
                            Some(state.paragraph_buffer.clone())
                        },
                    });
                }

                state.link_text.clear();
                state.link_url.clear();
                state.paragraph_buffer.clear();
            }
            // For image-in-link, keep link_text for the link element
        }
        Event::Text(text) => {
            if state.in_code {
                state.code_buffer.push_str(&text);
            } else if state.in_blockquote {
                state.blockquote_buffer.push_str(&text);
            } else if state.in_link || state.in_image {
                state.link_text.push_str(&text);
            } else {
                // Add indentation for nested list items
                if state.in_list && state.item_depth > 1 {
                    // Add newline and indentation before nested item text
                    if !state.paragraph_buffer.is_empty() && !state.paragraph_buffer.ends_with('\n')
                    {
                        state.paragraph_buffer.push('\n');
                    }
                    // Add indentation based on depth
                    let indent = "  ".repeat(state.item_depth - 1);
                    state.paragraph_buffer.push_str(&indent);

                    // Add checkbox marker if this is a task list item
                    if let Some(checked) = state.task_list_marker {
                        let marker = if checked { "[x] " } else { "[ ] " };
                        state.paragraph_buffer.push_str(marker);
                        // Clear the marker so it's only added once
                        state.task_list_marker = None;
                    }
                }
                state.add_inline_text(&text);
            }
        }
        Event::SoftBreak => {
            if state.in_paragraph {
                state.paragraph_buffer.push(' ');
                state.inline_buffer.push(InlineElement::Text {
                    value: " ".to_string(),
                });
            }
        }
        Event::HardBreak => {
            if state.in_paragraph {
                state.paragraph_buffer.push('\n');
                state.inline_buffer.push(InlineElement::Text {
                    value: "\n".to_string(),
                });
            }
        }
        Event::Rule => {
            state.flush_paragraph(blocks);
            blocks.push(Block::HorizontalRule);
        }
        Event::Start(Tag::Heading { .. }) => {
            // Clear buffers when entering a heading - we don't include headings in parsed blocks
            state.paragraph_buffer.clear();
            state.inline_buffer.clear();
        }
        Event::End(TagEnd::Heading(_)) => {
            // Clear buffers after heading to prevent text leaking into next paragraph
            state.paragraph_buffer.clear();
            state.inline_buffer.clear();
        }
        _ => {}
    }
}

/// Generate URL-friendly slug from heading text
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod content_tests {
    use super::*;

    #[test]
    fn test_list_with_code_block() {
        // Issue #8: Numbered lists with code blocks should render correctly
        let markdown = r#"1. Test1:
   ```
   test1
   ```

2. Test2:
   test2"#;

        let blocks = parse_content(markdown, 0);

        // Should produce a single List block
        assert_eq!(blocks.len(), 1);

        if let Block::List { ordered, items } = &blocks[0] {
            assert!(ordered, "Should be an ordered list");
            assert_eq!(items.len(), 2, "Should have 2 items");

            // First item: "Test1:" with a code block
            assert_eq!(items[0].content, "Test1:");
            assert_eq!(items[0].blocks.len(), 1, "First item should have 1 nested block");
            if let Block::Code { content, .. } = &items[0].blocks[0] {
                assert_eq!(content, "test1");
            } else {
                panic!("Expected Code block in first item");
            }

            // Second item: "Test2: test2" with no nested blocks
            assert!(items[1].content.contains("Test2:"));
            assert!(items[1].content.contains("test2"));
            assert!(
                items[1].blocks.is_empty(),
                "Second item should have no nested blocks"
            );
        } else {
            panic!("Expected List block");
        }
    }

    #[test]
    fn test_list_starting_with_code_block() {
        // Edge case: list item that starts with a code block (no leading paragraph)
        let markdown = r#"1. ```
   code only
   ```"#;

        let blocks = parse_content(markdown, 0);
        assert_eq!(blocks.len(), 1);

        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items.len(), 1);
            // Content should be empty since item starts with code block
            assert!(items[0].content.is_empty());
            assert_eq!(items[0].blocks.len(), 1);
        } else {
            panic!("Expected List block");
        }
    }

    #[test]
    fn test_list_with_multiple_code_blocks() {
        let markdown = r#"1. First item:
   ```rust
   fn main() {}
   ```
   ```python
   print("hello")
   ```"#;

        let blocks = parse_content(markdown, 0);
        assert_eq!(blocks.len(), 1);

        if let Block::List { items, .. } = &blocks[0] {
            assert_eq!(items.len(), 1);

            // The first text becomes content, code blocks go to blocks field
            assert_eq!(items[0].content, "First item:");
            assert_eq!(items[0].blocks.len(), 2, "Should have 2 code blocks");

            if let Block::Code { language, .. } = &items[0].blocks[0] {
                assert_eq!(language.as_deref(), Some("rust"));
            }
            if let Block::Code { language, .. } = &items[0].blocks[1] {
                assert_eq!(language.as_deref(), Some("python"));
            }
        } else {
            panic!("Expected List block");
        }
    }
}
