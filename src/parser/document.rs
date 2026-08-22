//! Document model for markdown files.
//!
//! This module defines the core data structures for representing
//! markdown documents and their heading hierarchy.

use serde::Serialize;
use std::collections::HashMap;

/// A markdown document with its content and structure.
///
/// Contains the original markdown content and a list of extracted headings.
#[derive(Debug, Clone)]
pub struct Document {
    pub content: String,
    pub headings: Vec<Heading>,
    /// Lowercased heading text, parallel to `headings`. Used for
    /// case-insensitive search without re-allocating per comparison.
    heading_text_lc: Vec<String>,
}

/// A heading in a markdown document.
///
/// Represents a single heading with its level (1-6), text content, and byte position.
#[derive(Debug, Clone, Serialize)]
pub struct Heading {
    /// Heading level (1 for #, 2 for ##, etc.)
    pub level: usize,
    /// Heading text content (stripped of inline markdown formatting)
    pub text: String,
    /// Byte offset where the heading starts in the source document
    #[serde(skip_serializing)]
    pub offset: usize,
    /// Byte length of the entire heading construct, as reported by the parser.
    ///
    /// For ATX headings this spans the marker line (including its line
    /// terminator when present); for setext headings it spans both the text
    /// line and the underline. `offset + source_len` therefore lands at the
    /// first byte of the section body, even across CRLF or setext underlines.
    #[serde(skip_serializing)]
    pub source_len: usize,
}

/// A node in the heading tree.
///
/// Represents a heading and its child headings in a hierarchical structure.
#[derive(Debug, Clone)]
pub struct HeadingNode {
    pub heading: Heading,
    pub children: Vec<HeadingNode>,
    pub index: usize,
}

impl Document {
    pub fn new(content: String, headings: Vec<Heading>) -> Self {
        let heading_text_lc = headings.iter().map(|h| h.text.to_lowercase()).collect();
        Self {
            content,
            headings,
            heading_text_lc,
        }
    }

    /// Build a hierarchical tree from the flat heading list.
    ///
    /// Walks the headings once with an explicit stack of `(level, &mut Vec<HeadingNode>)`
    /// pointers; child arrays are filled in place. No intermediate arena, no
    /// extra clones beyond the one Heading copy each node owns.
    pub fn build_tree(&self) -> Vec<HeadingNode> {
        // Build into raw indices first so we can mutate parent nodes safely.
        let mut roots: Vec<HeadingNode> = Vec::new();
        // `stack` stores indices describing how to navigate from the root
        // down to the current parent: each entry is the index into the
        // parent's `children` Vec. Walking the path on demand avoids
        // borrow-checker issues from holding mutable references on the stack.
        let mut path: Vec<(usize, usize)> = Vec::new(); // (level, child_idx)

        for (idx, heading) in self.headings.iter().enumerate() {
            let node = HeadingNode {
                heading: heading.clone(),
                children: Vec::new(),
                index: idx,
            };

            // Pop until current heading is deeper than top of stack.
            while let Some(&(parent_level, _)) = path.last() {
                if parent_level < heading.level {
                    break;
                }
                path.pop();
            }

            // Walk down the path to the parent's children Vec, push, and
            // record the new node's index for descendants.
            if path.is_empty() {
                roots.push(node);
                let idx = roots.len() - 1;
                path.push((heading.level, idx));
            } else {
                let mut cursor: &mut Vec<HeadingNode> = &mut roots;
                let last = path.len() - 1;
                for (i, &(_, child_idx)) in path.iter().enumerate() {
                    if i == last {
                        cursor[child_idx].children.push(node);
                        let new_idx = cursor[child_idx].children.len() - 1;
                        let parent_level = heading.level;
                        path.push((parent_level, new_idx));
                        break;
                    } else {
                        cursor = &mut cursor[child_idx].children;
                    }
                }
            }
        }

        roots
    }

    /// Get headings at a specific level
    pub fn headings_at_level(&self, level: usize) -> Vec<&Heading> {
        self.headings.iter().filter(|h| h.level == level).collect()
    }

    /// Find heading by text (case-insensitive)
    pub fn find_heading(&self, text: &str) -> Option<&Heading> {
        let search = text.to_lowercase();
        self.heading_text_lc
            .iter()
            .position(|lc| *lc == search)
            .map(|i| &self.headings[i])
    }

    /// Get all headings matching a filter
    pub fn filter_headings(&self, filter: &str) -> Vec<&Heading> {
        let search = filter.to_lowercase();
        self.headings
            .iter()
            .zip(self.heading_text_lc.iter())
            .filter(|(_, lc)| lc.contains(&search))
            .map(|(h, _)| h)
            .collect()
    }

    /// Extract the content of a section by heading text.
    ///
    /// Uses stored byte offsets for fast, accurate extraction without string searching.
    pub fn extract_section(&self, heading_text: &str) -> Option<String> {
        // Find the heading (O(n) scan of headings list)
        let search = heading_text.to_lowercase();
        let heading_idx = self.heading_text_lc.iter().position(|lc| *lc == search)?;
        self.extract_section_at_index(heading_idx)
    }

    /// Extract the content of a section by heading index.
    pub fn extract_section_at_index(&self, heading_idx: usize) -> Option<String> {
        if heading_idx >= self.headings.len() {
            return None;
        }
        let content_start = self.body_start(heading_idx);
        let end = self.section_end(heading_idx);
        Some(self.content[content_start..end].trim().to_string())
    }

    /// Byte offset where the body of section `idx` begins, i.e. the first byte
    /// after the heading construct, normalized to the start of the next line.
    ///
    /// The parser reports `offset + source_len` already pointing at the start of
    /// the body in the common case (the construct length includes its line
    /// terminator, handling CRLF and setext underlines uniformly). The extra
    /// `find('\n')` is a safety net for the rare case where the reported range
    /// stops before the terminator; it never crosses into the next line's text.
    pub fn body_start(&self, idx: usize) -> usize {
        let heading = &self.headings[idx];
        let after = (heading.offset + heading.source_len).min(self.content.len());

        // If `after` already sits at the start of a line (or EOF), it is the
        // body start. Otherwise advance to just past the next newline.
        if after >= self.content.len() {
            return self.content.len();
        }
        let preceding_is_newline = self.content[..after]
            .chars()
            .next_back()
            .map(|c| c == '\n')
            .unwrap_or(true);
        if preceding_is_newline {
            after
        } else {
            self.content[after..]
                .find('\n')
                .map(|i| after + i + 1)
                .unwrap_or(self.content.len())
        }
    }

    /// Byte offset where section `idx` ends: the offset of the next heading at a
    /// level less than or equal to this heading's level, or end of content.
    ///
    /// Used for "section" extraction where deeper subsections stay within the
    /// parent.
    pub fn section_end(&self, idx: usize) -> usize {
        let level = self.headings[idx].level;
        self.headings
            .iter()
            .skip(idx + 1)
            .find(|h| h.level <= level)
            .map(|h| h.offset)
            .unwrap_or(self.content.len())
    }

    /// Byte offset where section `idx` ends when bounding at *any* following
    /// heading (regardless of level). Used by the nested JSON builder, where
    /// child sections are emitted separately, so the parent's own body must
    /// stop at the very next heading.
    pub fn section_end_any(&self, idx: usize) -> usize {
        self.headings
            .get(idx + 1)
            .map(|h| h.offset)
            .unwrap_or(self.content.len())
    }

    /// Total number of source lines. A trailing newline does not open a new line.
    fn total_lines(&self) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let newlines = self.content.matches('\n').count();
        if self.content.ends_with('\n') {
            newlines
        } else {
            newlines + 1
        }
    }

    /// 1-indexed source line of each heading, parallel to `self.headings`.
    ///
    /// Counts newlines between consecutive heading offsets in a single pass so
    /// the whole call is O(content_len) rather than O(headings * content_len),
    /// matching how `--at-line` walks the document.
    fn heading_lines(&self) -> Vec<usize> {
        let mut lines = Vec::with_capacity(self.headings.len());
        let mut line = 1usize;
        let mut cursor = 0usize;

        for heading in &self.headings {
            // Headings arrive in ascending offset order. Clamping keeps this
            // total for hand-built documents (e.g. the filtered tree, which
            // pairs real headings with empty content).
            let target = heading.offset.min(self.content.len());
            if target >= cursor {
                line += self.content[cursor..target].matches('\n').count();
                cursor = target;
            }
            lines.push(line);
        }

        lines
    }

    /// 1-indexed inclusive `(start, end)` line range of every heading's section,
    /// parallel to `self.headings`.
    ///
    /// `start` is the line the heading itself sits on. `end` is the line before
    /// the next heading at the same or a higher level, so a parent's range spans
    /// its subsections; the last such heading runs to the end of the document.
    /// Back-to-back headings collapse to a single line (`end == start`).
    pub fn heading_line_ranges(&self) -> Vec<(usize, usize)> {
        let starts = self.heading_lines();
        let total = self.total_lines();
        let mut ranges = vec![(0, 0); self.headings.len()];

        // Walk backwards tracking, per level, the start line of the nearest
        // following heading that closes a section at that level.
        const MAX_LEVEL: usize = 6;
        let mut next_closing = [usize::MAX; MAX_LEVEL + 1];

        for idx in (0..self.headings.len()).rev() {
            let level = self.headings[idx].level.clamp(1, MAX_LEVEL);
            let start = starts[idx];
            let end = if next_closing[level] == usize::MAX {
                total
            } else {
                next_closing[level].saturating_sub(1)
            };
            ranges[idx] = (start, end.max(start));

            // This heading closes any section at its level or deeper.
            for slot in next_closing.iter_mut().take(MAX_LEVEL + 1).skip(level) {
                *slot = start;
            }
        }

        ranges
    }
}

impl HeadingNode {
    /// Render as tree with box-drawing characters
    /// If compact is true, uses gapless box characters without trailing spaces
    pub fn render_box_tree(&self, prefix: &str, is_last: bool) -> String {
        self.render_box_tree_styled(prefix, is_last, false)
    }

    /// Render as tree with box-drawing characters, with optional compact style
    pub fn render_box_tree_styled(&self, prefix: &str, is_last: bool, compact: bool) -> String {
        self.render_box_tree_annotated(prefix, is_last, compact, None)
    }

    /// Render as tree with box-drawing characters, optionally appending each
    /// heading's `[start-end]` source line range.
    ///
    /// `ranges` maps a heading's byte offset to its line range (see
    /// [`Document::heading_line_ranges`]). Offsets are stable across the
    /// filtered-tree rebuild, so the map is keyed on them rather than on
    /// position within a possibly-filtered heading list.
    pub fn render_box_tree_annotated(
        &self,
        prefix: &str,
        is_last: bool,
        compact: bool,
        ranges: Option<&HashMap<usize, (usize, usize)>>,
    ) -> String {
        let mut result = String::new();

        let (connector, space, continuation) = if compact {
            // Compact/gapless style: no trailing space after connector
            if is_last {
                ("└──", "", "   ")
            } else {
                ("├──", "", "│  ")
            }
        } else {
            // Spaced style (default): space after connector for readability
            if is_last {
                ("└─ ", "", "    ")
            } else {
                ("├─ ", "", "│   ")
            }
        };

        let marker = "#".repeat(self.heading.level);
        let suffix = ranges
            .and_then(|r| r.get(&self.heading.offset))
            .map(|(start, end)| format!(" [{}-{}]", start, end))
            .unwrap_or_default();
        result.push_str(&format!(
            "{}{}{}{} {}{}\n",
            prefix, connector, space, marker, self.heading.text, suffix
        ));

        let child_prefix = format!("{}{}", prefix, continuation);

        for (i, child) in self.children.iter().enumerate() {
            let is_last_child = i == self.children.len() - 1;
            result.push_str(&child.render_box_tree_annotated(
                &child_prefix,
                is_last_child,
                compact,
                ranges,
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(level: usize, text: &str, offset: usize) -> Heading {
        Heading {
            level,
            text: text.to_string(),
            offset,
            source_len: 0,
        }
    }

    /// Build a heading whose `source_len` is derived from `content`: the span
    /// from `offset` through the end of that line (including the newline). This
    /// matches what the parser reports for ATX headings and lets the
    /// extraction tests exercise the real `body_start` path.
    fn h_in(content: &str, level: usize, text: &str, offset: usize) -> Heading {
        let line_end = content[offset..]
            .find('\n')
            .map(|i| offset + i + 1)
            .unwrap_or(content.len());
        Heading {
            level,
            text: text.to_string(),
            offset,
            source_len: line_end - offset,
        }
    }

    fn doc(content: &str, headings: Vec<Heading>) -> Document {
        Document::new(content.to_string(), headings)
    }

    // ---------- build_tree ----------

    #[test]
    fn build_tree_empty() {
        let d = doc("", vec![]);
        assert!(d.build_tree().is_empty());
    }

    #[test]
    fn build_tree_single_root() {
        let d = doc("# A\n", vec![h(1, "A", 0)]);
        let tree = d.build_tree();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].heading.text, "A");
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn build_tree_simple_nesting() {
        // # A
        //   ## B
        //     ### C
        //   ## D
        let d = doc(
            "",
            vec![h(1, "A", 0), h(2, "B", 1), h(3, "C", 2), h(2, "D", 3)],
        );
        let tree = d.build_tree();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].heading.text, "A");
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].heading.text, "B");
        assert_eq!(tree[0].children[0].children.len(), 1);
        assert_eq!(tree[0].children[0].children[0].heading.text, "C");
        assert_eq!(tree[0].children[1].heading.text, "D");
        assert!(tree[0].children[1].children.is_empty());
    }

    #[test]
    fn build_tree_multiple_roots() {
        let d = doc(
            "",
            vec![h(1, "A", 0), h(2, "A1", 1), h(1, "B", 2), h(2, "B1", 3)],
        );
        let tree = d.build_tree();
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].heading.text, "A");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].heading.text, "A1");
        assert_eq!(tree[1].heading.text, "B");
        assert_eq!(tree[1].children.len(), 1);
        assert_eq!(tree[1].children[0].heading.text, "B1");
    }

    #[test]
    fn build_tree_skipped_levels() {
        // # A
        //     ### C   (skips level 2 — should still nest under A)
        let d = doc("", vec![h(1, "A", 0), h(3, "C", 1)]);
        let tree = d.build_tree();
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].heading.text, "C");
        assert_eq!(tree[0].children[0].heading.level, 3);
    }

    #[test]
    fn build_tree_jump_back_to_root() {
        // ### deep
        // # root  (jumps back; should be a sibling root, not a child)
        let d = doc("", vec![h(3, "deep", 0), h(1, "root", 1)]);
        let tree = d.build_tree();
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].heading.text, "deep");
        assert_eq!(tree[1].heading.text, "root");
        assert!(tree[1].children.is_empty());
    }

    #[test]
    fn build_tree_same_level_siblings() {
        let d = doc(
            "",
            vec![h(2, "A", 0), h(2, "B", 1), h(2, "C", 2), h(3, "C1", 3)],
        );
        let tree = d.build_tree();
        assert_eq!(tree.len(), 3);
        assert!(tree[0].children.is_empty());
        assert!(tree[1].children.is_empty());
        assert_eq!(tree[2].children.len(), 1);
        assert_eq!(tree[2].children[0].heading.text, "C1");
    }

    #[test]
    fn build_tree_deep_chain() {
        // # / ## / ### / #### / ##### / ######
        let headings: Vec<Heading> = (1..=6)
            .map(|lvl| h(lvl, &format!("L{}", lvl), lvl))
            .collect();
        let d = doc("", headings);
        let tree = d.build_tree();
        let mut node = &tree[0];
        for lvl in 1..=6 {
            assert_eq!(node.heading.level, lvl);
            if lvl == 6 {
                assert!(node.children.is_empty());
            } else {
                assert_eq!(node.children.len(), 1, "expected single child at L{}", lvl);
                node = &node.children[0];
            }
        }
    }

    #[test]
    fn build_tree_pop_to_grandparent() {
        // # A
        //   ## B
        //     ### C
        //   ## D     (pops both C and B; D is child of A)
        let d = doc(
            "",
            vec![h(1, "A", 0), h(2, "B", 1), h(3, "C", 2), h(2, "D", 3)],
        );
        let tree = d.build_tree();
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[1].heading.text, "D");
    }

    // ---------- find_heading ----------

    #[test]
    fn find_heading_case_insensitive() {
        let d = doc("", vec![h(1, "Hello World", 0), h(2, "Other", 1)]);
        assert_eq!(d.find_heading("hello world").unwrap().text, "Hello World");
        assert_eq!(d.find_heading("HELLO WORLD").unwrap().text, "Hello World");
        assert_eq!(d.find_heading("Hello World").unwrap().text, "Hello World");
    }

    #[test]
    fn find_heading_no_match() {
        let d = doc("", vec![h(1, "Foo", 0)]);
        assert!(d.find_heading("Bar").is_none());
        // Substring should NOT match (find_heading is exact equality).
        assert!(d.find_heading("Fo").is_none());
    }

    #[test]
    fn find_heading_returns_first_on_duplicate() {
        let d = doc("", vec![h(1, "Dup", 0), h(2, "Dup", 5)]);
        let found = d.find_heading("dup").unwrap();
        assert_eq!(found.level, 1);
        assert_eq!(found.offset, 0);
    }

    // ---------- filter_headings ----------

    #[test]
    fn filter_headings_substring_case_insensitive() {
        let d = doc(
            "",
            vec![
                h(1, "Introduction", 0),
                h(2, "Setup intro", 1),
                h(2, "Conclusion", 2),
            ],
        );
        let matches: Vec<_> = d
            .filter_headings("INTRO")
            .into_iter()
            .map(|h| h.text.clone())
            .collect();
        assert_eq!(matches, vec!["Introduction", "Setup intro"]);
    }

    #[test]
    fn filter_headings_empty_query_matches_all() {
        let d = doc("", vec![h(1, "A", 0), h(2, "B", 1)]);
        assert_eq!(d.filter_headings("").len(), 2);
    }

    #[test]
    fn filter_headings_no_match() {
        let d = doc("", vec![h(1, "A", 0)]);
        assert!(d.filter_headings("zzz").is_empty());
    }

    // ---------- headings_at_level ----------

    #[test]
    fn headings_at_level_filters_correctly() {
        let d = doc(
            "",
            vec![h(1, "A", 0), h(2, "B", 1), h(2, "C", 2), h(3, "D", 3)],
        );
        let l2: Vec<_> = d
            .headings_at_level(2)
            .into_iter()
            .map(|h| h.text.clone())
            .collect();
        assert_eq!(l2, vec!["B", "C"]);
        assert_eq!(d.headings_at_level(99).len(), 0);
    }

    // ---------- extract_section ----------
    // (parser/mod.rs already covers happy paths via parse_markdown — these
    // exercise the document layer directly, with hand-built offsets.)

    #[test]
    fn extract_section_case_insensitive_lookup() {
        // Both at level 1 so Alpha is bounded by Beta.
        let content = "# Alpha\nbody alpha\n\n# Beta\nbody beta\n";
        let alpha = content.find("# Alpha").unwrap();
        let beta = content.find("# Beta").unwrap();
        let d = doc(
            content,
            vec![
                h_in(content, 1, "Alpha", alpha),
                h_in(content, 1, "Beta", beta),
            ],
        );
        let section = d.extract_section("ALPHA").expect("found");
        assert!(section.contains("body alpha"));
        assert!(!section.contains("body beta"));
    }

    #[test]
    fn extract_section_stops_at_same_or_higher_level() {
        // Section ## A ends at the next ## or # — not at deeper ###.
        let content = "# Top\nintro\n\n## A\na-body\n\n### A1\na1-body\n\n## B\nb-body\n";
        let top = content.find("# Top").unwrap();
        let a = content.find("## A").unwrap();
        let a1 = content.find("### A1").unwrap();
        let b = content.find("## B").unwrap();
        let d = doc(
            content,
            vec![
                h_in(content, 1, "Top", top),
                h_in(content, 2, "A", a),
                h_in(content, 3, "A1", a1),
                h_in(content, 2, "B", b),
            ],
        );
        let section = d.extract_section("A").expect("found");
        assert!(section.contains("a-body"));
        assert!(section.contains("A1"), "deeper subsection stays in A");
        assert!(section.contains("a1-body"));
        assert!(!section.contains("b-body"));
    }

    #[test]
    fn extract_section_missing_returns_none() {
        let d = doc("# A\n", vec![h(1, "A", 0)]);
        assert!(d.extract_section("missing").is_none());
    }

    // ---------- extract_section_at_index ----------

    #[test]
    fn extract_section_at_index_duplicate_sub_headings() {
        let content = "# heading\nfirst body\n\n## sub heading\nfirst sub\n\n# heading 2\nsecond body\n\n## sub heading\nsecond sub\n";
        let h1 = content.find("# heading\n").unwrap();
        let s1 = content.find("## sub heading\nfirst").unwrap();
        let h2 = content.find("# heading 2").unwrap();
        let s2 = content.find("## sub heading\nsecond").unwrap();
        let d = doc(
            content,
            vec![
                h_in(content, 1, "heading", h1),
                h_in(content, 2, "sub heading", s1),
                h_in(content, 1, "heading 2", h2),
                h_in(content, 2, "sub heading", s2),
            ],
        );

        let first = d.extract_section_at_index(1).unwrap();
        assert!(first.contains("first sub"));
        assert!(!first.contains("second sub"));

        let second = d.extract_section_at_index(3).unwrap();
        assert!(second.contains("second sub"));
        assert!(!second.contains("first sub"));
    }

    #[test]
    fn extract_section_at_index_duplicate_at_eof() {
        let content =
            "# First\nFirst body.\n\n# Second\nMiddle.\n\n# First\nLast body.\nEOF line.\n";
        let f1 = content.find("# First\nFirst").unwrap();
        let sec = content.find("# Second").unwrap();
        let f2 = content.find("# First\nLast").unwrap();
        let d = doc(
            content,
            vec![
                h_in(content, 1, "First", f1),
                h_in(content, 1, "Second", sec),
                h_in(content, 1, "First", f2),
            ],
        );

        let first = d.extract_section_at_index(0).unwrap();
        assert!(first.contains("First body"));
        assert!(!first.contains("Last body"));

        let last = d.extract_section_at_index(2).unwrap();
        assert!(last.contains("Last body"));
        assert!(!last.contains("First body"));
    }

    #[test]
    fn extract_section_at_index_out_of_bounds() {
        let d = doc("# A\n", vec![h(1, "A", 0)]);
        assert!(d.extract_section_at_index(999).is_none());
    }

    // ---------- regressions via parse_markdown (real source_len) ----------

    #[test]
    fn extract_section_crlf_does_not_panic_and_excludes_heading() {
        // Mid-multibyte-char on a CRLF line previously risked a byte-slice
        // panic in the section extractor.
        let d = crate::parser::parse_markdown("# Top\r\naaa\r\nccé\r\n# Next\r\n");
        let top = d.extract_section_at_index(0).unwrap();
        assert!(top.contains("aaa"));
        assert!(top.contains("ccé"));
        assert!(!top.contains("# Top"));
        assert!(!top.contains("# Next"));
    }

    #[test]
    fn extract_section_setext_excludes_underline() {
        // The setext underline ("-----") must not appear in the body.
        let d = crate::parser::parse_markdown("Title\n=====\nbody\n\nSub\n-----\nsub body\n");
        // Two headings: Title (h1), Sub (h2).
        let sub_idx = d.headings.iter().position(|h| h.text == "Sub").unwrap();
        let body = d.extract_section_at_index(sub_idx).unwrap();
        assert_eq!(body.trim(), "sub body");
        assert!(!body.contains("-----"));
    }

    #[test]
    fn extract_section_solo_heading_at_eof_is_empty() {
        // A heading with no trailing newline and no body should yield "" — not
        // its own heading line.
        let d = crate::parser::parse_markdown("# First\nbody\n\n# Solo");
        let solo_idx = d.headings.iter().position(|h| h.text == "Solo").unwrap();
        let body = d.extract_section_at_index(solo_idx).unwrap();
        assert_eq!(body, "");
    }

    // ---------- heading_line_ranges ----------

    #[test]
    fn line_ranges_empty_document() {
        let d = doc("", vec![]);
        assert!(d.heading_line_ranges().is_empty());
    }

    #[test]
    fn line_ranges_parent_spans_its_subsections() {
        // A parent's range covers its children; each h2 stops at the next h2.
        let d =
            crate::parser::parse_markdown("# A\nintro\n## B\nb body\n## C\nc body\n# D\nd body\n");
        assert_eq!(
            d.heading_line_ranges(),
            vec![(1, 6), (3, 4), (5, 6), (7, 8)]
        );
    }

    #[test]
    fn line_ranges_deeper_heading_stops_at_shallower_one() {
        // The h3 must end where the following h2 begins, not run to EOF.
        let d = crate::parser::parse_markdown("# A\n## B\n### C\ntext\n## D\n");
        assert_eq!(
            d.heading_line_ranges(),
            vec![(1, 5), (2, 4), (3, 4), (5, 5)]
        );
    }

    #[test]
    fn line_ranges_back_to_back_headings_collapse_to_one_line() {
        let d = crate::parser::parse_markdown("# A\n# B\n");
        assert_eq!(d.heading_line_ranges(), vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn line_ranges_last_section_runs_to_eof_without_trailing_newline() {
        // A missing final newline must not lose the last line.
        let d = crate::parser::parse_markdown("# A\nbody");
        assert_eq!(d.heading_line_ranges(), vec![(1, 2)]);
    }

    #[test]
    fn line_ranges_ignore_headings_inside_code_blocks() {
        // The fenced "# fake" must neither become a heading nor shift the
        // line numbers of the real one that follows.
        let d = crate::parser::parse_markdown("# A\n```\n# fake\n```\n## B\nbody\n");
        assert_eq!(d.heading_line_ranges(), vec![(1, 6), (5, 6)]);
    }

    #[test]
    fn line_ranges_count_crlf_lines_once() {
        let d = crate::parser::parse_markdown("# A\r\n## B\r\ntext\r\n");
        assert_eq!(d.heading_line_ranges(), vec![(1, 3), (2, 3)]);
    }

    #[test]
    fn line_ranges_span_setext_underlines() {
        // The h1's range must cover the setext underline rows too.
        let d = crate::parser::parse_markdown("Title\n=====\nbody\n\nSub\n-----\nsub body\n");
        assert_eq!(d.heading_line_ranges(), vec![(1, 7), (5, 7)]);
    }

    #[test]
    fn line_ranges_tolerate_offsets_past_content() {
        // The filtered `--tree` path pairs real headings with empty content;
        // this must clamp rather than panic on an out-of-bounds slice.
        let d = doc("", vec![h(1, "A", 40), h(2, "B", 80)]);
        assert_eq!(d.heading_line_ranges(), vec![(1, 1), (1, 1)]);
    }
}
