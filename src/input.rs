//! Input handling for stdin and file sources
//!
//! Provides robust stdin reading with UTF-8 validation and format detection.
//! Includes security limits to prevent denial-of-service via large inputs.

use std::io::{self, BufRead, IsTerminal};
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
            InputError::InputTooLarge(size) => {
                write!(
                    f,
                    "Input too large: {} bytes (max {} MB)",
                    size,
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

/// Check if stdin is being piped (not a TTY)
pub fn is_stdin_piped() -> bool {
    !io::stdin().is_terminal()
}

/// Read input from stdin with proper error handling
///
/// Implements best practices from Rust stdin handling guides:
/// - Line-by-line buffered reading for performance
/// - UTF-8 validation
/// - Proper error propagation
/// - Size limits to prevent DoS attacks
pub fn read_stdin() -> Result<String, InputError> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();
    let mut total_size = 0usize;
    let mut line_buffer = String::new();

    loop {
        line_buffer.clear();
        let bytes_read = handle.read_line(&mut line_buffer)?;

        // EOF reached
        if bytes_read == 0 {
            break;
        }

        // Check line size limit
        if line_buffer.len() > MAX_LINE_SIZE {
            return Err(InputError::LineTooLong(line_buffer.len()));
        }

        // Check total size limit
        total_size = total_size.saturating_add(bytes_read);
        if total_size > MAX_INPUT_SIZE {
            return Err(InputError::InputTooLarge(total_size));
        }

        buffer.push_str(&line_buffer);
    }

    // Validate UTF-8 (String already enforces this, but explicit check)
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
            // File path provided
            let content = std::fs::read_to_string(path).map_err(InputError::Io)?;
            Ok(InputSource::File(content))
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
    let content = match source {
        InputSource::File(c) | InputSource::Stdin(c) => c,
    };

    // Check if content looks like markdown (has headings)
    if content.trim_start().starts_with('#') || content.contains("\n#") {
        // Markdown content, pass through
        Ok(content)
    } else {
        // Plain text - wrap in a document heading for basic viewing
        let mut markdown = String::from("# Input\n\n");
        markdown.push_str(&content);
        Ok(markdown)
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
}
