//! End-to-end integration tests for the `treemd` binary.
//!
//! These tests invoke the actual built binary and assert on its
//! stdout/stderr/exit code, exercising the full CLI surface.
//!
//! No assert_cmd / predicates dependency — uses the built-in
//! `CARGO_BIN_EXE_treemd` env var that Cargo sets for integration tests.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Path to the built `treemd` binary.
fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_treemd"))
}

/// A small fixture markdown document used across most tests.
const FIXTURE: &str = "\
# Title

Intro paragraph.

## Installation

Install steps here.

```rust
// `# fake heading` inside a code block must be ignored by parsing.
let x = 1;
```

## Usage

Some usage notes.

### Advanced

Deep section.

## Conclusion

End of doc.
";

/// Write the fixture to a tempfile and return its path.
/// Caller is responsible for cleanup (we leak — test process exits anyway).
fn fixture_file() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "treemd-it-{}-{}",
        std::process::id(),
        // Add a tiny salt so concurrent tests don't collide on the same fixture.
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("doc.md");
    std::fs::write(&path, FIXTURE).expect("write fixture");
    path
}

/// Run treemd with args, return (stdout, stderr, exit code).
fn run(args: &[&str]) -> (String, String, i32) {
    let out = Command::new(bin())
        .args(args)
        .output()
        .expect("spawn treemd");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// Run treemd with stdin piped from `input` and the given args.
fn run_with_stdin(args: &[&str], input: &str) -> (String, String, i32) {
    let mut child = Command::new(bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn treemd");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

// ------------------------------------------------------------------
// --version / --help — basic CLI plumbing
// ------------------------------------------------------------------

#[test]
fn version_flag_prints_version_and_exits_zero() {
    let (stdout, _, code) = run(&["--version"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("treemd"),
        "version output should mention treemd, got: {stdout}"
    );
}

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    let (stdout, _, code) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage"), "help output should contain Usage");
    assert!(
        stdout.contains("--list") && stdout.contains("--tree"),
        "help should mention --list and --tree"
    );
}

#[test]
fn query_help_prints_query_docs_and_exits_zero() {
    let (stdout, _, code) = run(&["--query-help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Query Language") || stdout.contains("ELEMENT SELECTORS"));
}

// ------------------------------------------------------------------
// --list (plain / json) and filtering
// ------------------------------------------------------------------

#[test]
fn list_plain_emits_all_headings() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["-l", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    // Exactly one line per heading in the fixture (5 of them).
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        5,
        "expected 5 heading lines, got: {:?}",
        lines
    );
    assert!(lines[0].starts_with("# Title"));
    assert!(lines[1].starts_with("## Installation"));
    assert!(lines[2].starts_with("## Usage"));
    assert!(lines[3].starts_with("### Advanced"));
    assert!(lines[4].starts_with("## Conclusion"));
}

#[test]
fn list_with_level_filter_keeps_only_that_level() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["-l", "-L", "2", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    for line in stdout.lines() {
        assert!(line.starts_with("## "), "non-h2 line leaked: {line}");
    }
    assert_eq!(stdout.lines().count(), 3); // Installation, Usage, Conclusion
}

#[test]
fn list_with_text_filter_is_case_insensitive() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["-l", "--filter", "INSTALL", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "## Installation");
}

#[test]
fn list_json_output_is_valid_and_has_expected_shape() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["-l", "-o", "json", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let doc = &v["document"];
    assert_eq!(doc["metadata"]["headingCount"], 5);
    assert_eq!(doc["metadata"]["maxDepth"], 3);
    let sections = doc["sections"].as_array().expect("sections array");
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0]["title"], "Title");
    let children = sections[0]["children"].as_array().expect("children");
    assert_eq!(children.len(), 3); // Installation, Usage, Conclusion
}

// ------------------------------------------------------------------
// --tree
// ------------------------------------------------------------------

#[test]
fn tree_output_uses_box_drawing_chars() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["--tree", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    // At least one branch and one terminal connector should appear.
    assert!(stdout.contains("├") || stdout.contains("└"), "no branches");
    assert!(stdout.contains("Title"));
    assert!(stdout.contains("Advanced"));
}

// ------------------------------------------------------------------
// -s / --section
// ------------------------------------------------------------------

#[test]
fn section_extracts_named_section_only() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["-s", "Installation", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Install steps here"));
    assert!(!stdout.contains("Some usage notes"));
}

#[test]
fn section_missing_exits_nonzero() {
    let f = fixture_file();
    let (_, stderr, code) = run(&["-s", "Nonexistent", f.to_str().unwrap()]);
    assert_ne!(code, 0, "missing section should exit nonzero");
    assert!(stderr.contains("not found"));
}

// ------------------------------------------------------------------
// --count
// ------------------------------------------------------------------

#[test]
fn count_reports_per_level_and_total() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["--count", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("#: 1"), "expected one h1");
    assert!(stdout.contains("##: 3"), "expected three h2");
    assert!(stdout.contains("###: 1"), "expected one h3");
    assert!(stdout.contains("Total: 5"));
}

// ------------------------------------------------------------------
// -q / query mode
// ------------------------------------------------------------------

#[test]
fn query_h2_returns_only_h2_headings() {
    let f = fixture_file();
    let (stdout, _, code) = run(&["-q", ".h2", f.to_str().unwrap()]);
    assert_eq!(code, 0, "stdout: {stdout}");
    // Should reference all three h2 headings.
    assert!(stdout.contains("Installation"));
    assert!(stdout.contains("Usage"));
    assert!(stdout.contains("Conclusion"));
    assert!(!stdout.contains("Advanced"), "h3 leaked into h2 query");
}

#[test]
fn query_invalid_syntax_exits_nonzero() {
    let f = fixture_file();
    let (_, stderr, code) = run(&["-q", ".h2 | nonexistent_fn(((", f.to_str().unwrap()]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty(), "expected an error message on stderr");
}

// ------------------------------------------------------------------
// stdin piping
// ------------------------------------------------------------------

#[test]
fn list_reads_markdown_from_stdin() {
    let input = "# Alpha\n## Beta\n";
    let (stdout, _, code) = run_with_stdin(&["-l", "-"], input);
    assert_eq!(code, 0, "stdout: {stdout}");
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "# Alpha");
    assert_eq!(lines[1], "## Beta");
}

// ------------------------------------------------------------------
// --at-line
// ------------------------------------------------------------------

#[test]
fn at_line_finds_enclosing_heading() {
    let f = fixture_file();
    // 1-indexed line number of "## Usage" in FIXTURE.
    let usage_line = FIXTURE
        .lines()
        .position(|l| l.starts_with("## Usage"))
        .expect("Usage heading present")
        + 1;
    // A line *inside* the Usage section — should resolve to "## Usage".
    let target = usage_line + 1;
    let (stdout, stderr, code) = run(&[
        "--at-line",
        &target.to_string(),
        f.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert_eq!(stdout.trim(), "## Usage");
}

#[test]
fn at_line_on_heading_line_returns_that_heading() {
    let f = fixture_file();
    let install_line = FIXTURE
        .lines()
        .position(|l| l.starts_with("## Installation"))
        .expect("Installation heading present")
        + 1;
    let (stdout, _, code) = run(&[
        "--at-line",
        &install_line.to_string(),
        f.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "## Installation");
}

#[test]
fn at_line_before_first_heading_exits_nonzero() {
    // First heading is on line 5; line 2 has no heading at or before it.
    let dir = std::env::temp_dir().join(format!("treemd-it-noheading-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doc.md");
    std::fs::write(&path, "lorem\nipsum\ndolor\nsit\n# Hello\nbody\n").unwrap();
    let (_, stderr, code) = run(&["--at-line", "2", path.to_str().unwrap()]);
    assert_ne!(code, 0);
    assert!(stderr.contains("No heading"));
}

#[test]
fn at_line_zero_is_rejected() {
    let f = fixture_file();
    let (_, stderr, code) = run(&["--at-line", "0", f.to_str().unwrap()]);
    assert_ne!(code, 0);
    assert!(stderr.contains(">= 1"));
}

// ------------------------------------------------------------------
// -s with formatted heading text (regression: previously broken because
// main.rs::extract_section did string-search on heading.text, which is
// the stripped form, so `## **Bold** Section` was never found).
// ------------------------------------------------------------------

#[test]
fn section_with_inline_markdown_in_heading() {
    let dir = std::env::temp_dir().join(format!("treemd-it-fmt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doc.md");
    std::fs::write(
        &path,
        "# Top\n\n## **Bold** Section\nbody-of-bold\n\n## Next\nbody-of-next\n",
    )
    .unwrap();

    let (stdout, stderr, code) = run(&["-s", "Bold Section", path.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(
        stdout.contains("body-of-bold"),
        "expected body of formatted-heading section, got: {stdout:?}"
    );
    assert!(!stdout.contains("body-of-next"));
}
