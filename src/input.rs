//! Input handling for stdin and file sources
//!
//! Provides robust stdin reading with UTF-8 validation and format detection.
//! Includes security limits to prevent denial-of-service via large inputs.

use std::io::{self, IsTerminal, Read};
use std::path::Path;

/// Maximum input size (100 MB) - prevents memory exhaustion attacks
const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Maximum line size (10 MB) - prevents single-line attacks
const MAX_LINE_SIZE: usize = 10 * 1024 * 1024;

/// Input source for treemd
#[derive(Debug)]
pub enum InputSource {
    File(String),
    Stdin(String),
}

/// Errors that can occur during input reading
#[derive(Debug)]
pub enum InputError {
    Io(io::Error),
    Utf8Error,
    EmptyInput,
    NoTty,
    InputTooLarge(usize),
    LineTooLong(usize),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::Io(e) => write!(f, "I/O error: {}", e),
            InputError::Utf8Error => write!(f, "Invalid UTF-8 in input"),
            InputError::EmptyInput => write!(f, "Empty input provided"),
            InputError::NoTty => {
                write!(f, "No file specified and stdin is not being piped")
            }
            InputError::InputTooLarge(_) => {
                write!(
                    f,
                    "Input too large: over the {} MB limit",
                    MAX_INPUT_SIZE / (1024 * 1024)
                )
            }
            InputError::LineTooLong(size) => {
                write!(
                    f,
                    "Line too long: {} bytes (max {} MB)",
                    size,
                    MAX_LINE_SIZE / (1024 * 1024)
                )
            }
        }
    }
}

impl std::error::Error for InputError {}

impl From<io::Error> for InputError {
    fn from(e: io::Error) -> Self {
        InputError::Io(e)
    }
}

/// Whether `spec` names a remote document rather than a path: an http(s)
/// URL, scheme in any case as URLs allow, or `github:owner/repo`.
pub fn is_remote_spec(spec: &str) -> bool {
    let lower = spec.get(..8).unwrap_or(spec).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || spec.starts_with("github:")
}

/// Check if stdin is being piped (not a TTY)
pub fn is_stdin_piped() -> bool {
    !io::stdin().is_terminal()
}

/// Read all of `reader` as text, refusing more than [`MAX_INPUT_SIZE`] bytes.
///
/// The size is only known by reading: stdin has none, and `/dev/zero`, a
/// FIFO or `<(command)` report 0 in their metadata. Reading at most one byte
/// past the limit is what keeps any of them from filling memory first. Line
/// by line it did not: a line was read whole before its length was checked,
/// so a stream without newlines was never stopped.
fn read_capped(reader: impl Read) -> Result<String, InputError> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_INPUT_SIZE as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_INPUT_SIZE {
        return Err(InputError::InputTooLarge(bytes.len()));
    }
    String::from_utf8(bytes).map_err(|_| InputError::Utf8Error)
}

/// Read a markdown file, held to the same limit as stdin.
///
/// Everything that opens a document reads it through here: the file named
/// on the command line, and in the TUI a followed link, the file picker
/// and a reload.
pub fn read_file(path: &Path) -> Result<String, InputError> {
    // A regular file says how big it is, so a huge one is refused without
    // reading any of it.
    let size = std::fs::metadata(path)?.len();
    if size > MAX_INPUT_SIZE as u64 {
        return Err(InputError::InputTooLarge(
            usize::try_from(size).unwrap_or(usize::MAX),
        ));
    }
    read_capped(std::fs::File::open(path)?)
}

/// Read input from stdin, within the size limits.
pub fn read_stdin() -> Result<String, InputError> {
    let buffer = read_capped(io::stdin().lock())?;
    if let Some(longest) = buffer.split('\n').map(str::len).max()
        && longest > MAX_LINE_SIZE
    {
        return Err(InputError::LineTooLong(longest));
    }
    if buffer.is_empty() {
        return Err(InputError::EmptyInput);
    }
    Ok(buffer)
}

/// Determine input source based on arguments and stdin state
///
/// Priority:
/// 1. If file path is exactly "-", read from stdin
/// 2. If file path is provided, use file
/// 3. If no file and stdin is piped, read from stdin
/// 4. Otherwise, error (no input available)
pub fn determine_input_source(file_path: Option<&Path>) -> Result<InputSource, InputError> {
    match file_path {
        Some(path) if path == Path::new("-") => {
            // Explicit stdin via "-"
            let content = read_stdin()?;
            Ok(InputSource::Stdin(content))
        }
        Some(path) => {
            // Held to the same limit as stdin: it is read whole, so a
            // multi-gigabyte file named by mistake (or a device) would
            // otherwise be read into memory before anything is shown.
            Ok(InputSource::File(read_file(path)?))
        }
        None if is_stdin_piped() => {
            // No file, but stdin is piped
            let content = read_stdin()?;
            Ok(InputSource::Stdin(content))
        }
        None => {
            // No file and stdin is TTY (user error)
            Err(InputError::NoTty)
        }
    }
}

/// Process input and return content ready for markdown parsing
///
/// Supports:
/// - Raw markdown (passed through)
/// - Plain text (wrapped in markdown heading)
pub fn process_input(source: InputSource) -> Result<String, Box<dyn std::error::Error>> {
    // A file is kept exactly as it is on disk: it can be edited and saved,
    // and a save checks that the file still holds what was loaded, so a
    // heading added here made every save fail as a change made on disk. The
    // outline gives a document without headings an overview entry anyway.
    let content = match source {
        InputSource::File(c) => return Ok(c),
        InputSource::Stdin(c) => c,
    };

    // Decide whether to wrap based on whether the markdown *parser* finds any
    // headings — not a naive `#` text scan. A `#` scan both misfires (e.g. a
    // leading `#` inside a code fence) and misses setext headings
    // (`Title\n=====`), which have no `#` at all but are real headings. The
    // document parser also reads past a byte order mark, which hid the
    // first heading from the bare heading parser.
    if crate::parser::parse_markdown(&content).headings.is_empty() {
        // No headings at all: wrap in a phantom heading for basic viewing.
        let mut markdown = String::from("# Input\n\n");
        markdown.push_str(&content);
        Ok(markdown)
    } else {
        // Document has structure already; pass through unchanged.
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_markdown_input() {
        let markdown = "# Title\n\nContent here\n\n## Section\n";
        let source = InputSource::Stdin(markdown.to_string());

        let result = process_input(source).unwrap();
        assert_eq!(result, markdown);
    }

    #[test]
    fn test_process_plain_text() {
        let text = "Just some plain text\nwith multiple lines";
        let source = InputSource::Stdin(text.to_string());

        let result = process_input(source).unwrap();
        assert!(result.starts_with("# Input\n\n"));
        assert!(result.contains("Just some plain text"));
    }

    /// A stream that never ends (`/dev/zero`, a FIFO, stdin) has no size to
    /// check up front, and reading it line by line never stopped one with
    /// no newlines. It is cut off just past the limit.
    #[test]
    fn an_endless_stream_stops_at_the_limit() {
        assert!(matches!(
            read_capped(io::repeat(b'a')),
            Err(InputError::InputTooLarge(_))
        ));
    }

    /// A file has to stay byte for byte what is on disk, or saving it is
    /// refused as a change made elsewhere.
    #[test]
    fn a_file_is_never_wrapped() {
        let text = "- [ ] a\n- [ ] b\n";
        let result = process_input(InputSource::File(text.to_string())).unwrap();
        assert_eq!(result, text);
    }

    /// The byte order mark used to hide the heading, so a heading-less
    /// wrapper went on top and the real heading stopped being one.
    #[test]
    fn a_byte_order_mark_does_not_hide_the_first_heading() {
        let text = "\u{feff}# T\n\n- [ ] a\n";
        let result = process_input(InputSource::Stdin(text.to_string())).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn test_setext_markdown_not_wrapped() {
        // Setext headings have no '#' but are real headings — must pass through.
        let md = "Title\n=====\n\nbody text\n";
        let source = InputSource::Stdin(md.to_string());
        let result = process_input(source).unwrap();
        assert_eq!(result, md);
        assert!(!result.starts_with("# Input"));
    }

    #[test]
    fn test_hash_inside_code_fence_only_is_wrapped() {
        // A leading '#' that lives inside a fenced code block is not a heading;
        // such content has no real headings and should be wrapped.
        let md = "```\n# not a heading\n```\n";
        let source = InputSource::Stdin(md.to_string());
        let result = process_input(source).unwrap();
        assert!(result.starts_with("# Input\n\n"));
    }
}
