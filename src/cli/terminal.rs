//! Printing document text to a terminal.
//!
//! A document is untrusted input, and its text reaches stdout verbatim in
//! `-l`, `--tree`, `-s` and `-q`. Printed to a terminal, an escape sequence in
//! a heading can retitle the window, clear the screen, move the cursor to
//! rewrite earlier output, or ask for a clipboard write. The TUI is not
//! exposed, because ratatui drops control characters from its buffer; this is
//! the same guard for the CLI.

use std::borrow::Cow;
use std::io::IsTerminal;
use std::sync::OnceLock;

/// `text`, made safe to print if stdout is a terminal, unchanged otherwise.
///
/// Piped output keeps every byte, since the next program may want them and
/// cannot be fooled by them the way a terminal can.
pub fn display(text: &str) -> Cow<'_, str> {
    static STDOUT_IS_TERMINAL: OnceLock<bool> = OnceLock::new();
    if *STDOUT_IS_TERMINAL.get_or_init(|| std::io::stdout().is_terminal()) {
        escape_controls(text)
    } else {
        Cow::Borrowed(text)
    }
}

/// Control characters made visible, the way `cat -v` shows them: ESC as `^[`,
/// DEL as `^?`, and a C1 control as its code point, so nothing reaches the
/// terminal as a command. Newlines, tabs and the CR of a CRLF pair pass
/// through, so ordinary text, including a CRLF file, prints as it is.
///
/// The bidirectional overrides and isolates are shown as code points too.
/// They are not commands, but they reorder how the rest of the line is
/// drawn, so a heading could display as something other than what it says.
/// The plain direction marks used in right-to-left text are left alone.
pub fn escape_controls(text: &str) -> Cow<'_, str> {
    let is_kept = |c: char, next: Option<char>| {
        let reorders = matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
        (!c.is_control() && !reorders)
            || c == '\n'
            || c == '\t'
            || (c == '\r' && next == Some('\n'))
    };
    let mut chars = text.chars().peekable();
    let mut clean = true;
    while let Some(c) = chars.next() {
        if !is_kept(c, chars.peek().copied()) {
            clean = false;
            break;
        }
    }
    if clean {
        return Cow::Borrowed(text);
    }

    let mut out = String::with_capacity(text.len() + 8);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if is_kept(c, chars.peek().copied()) {
            out.push(c);
        } else if (c as u32) < 0x20 {
            out.push('^');
            out.push(char::from(b'@' + c as u8));
        } else if c == '\u{7f}' {
            out.push_str("^?");
        } else {
            out.push_str(&format!("<U+{:04X}>", c as u32));
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bidi_overrides_are_shown_not_applied() {
        assert_eq!(
            escape_controls("admin\u{202e}txt.exe"),
            "admin<U+202E>txt.exe"
        );
        assert_eq!(escape_controls("a\u{2067}b\u{2069}"), "a<U+2067>b<U+2069>");
        // A right-to-left mark is ordinary text.
        assert!(matches!(escape_controls("שלום\u{200f}"), Cow::Borrowed(_)));
    }

    #[test]
    fn ordinary_text_is_untouched() {
        for text in [
            "plain",
            "tab\there",
            "two\nlines",
            "crlf\r\nline",
            "",
            "日本語 ✓",
        ] {
            assert!(matches!(escape_controls(text), Cow::Borrowed(t) if t == text));
        }
    }

    #[test]
    fn escape_sequences_are_shown_rather_than_sent() {
        assert_eq!(
            escape_controls("Title \u{1b}]0;PWNED\u{7} \u{1b}[2J end"),
            "Title ^[]0;PWNED^G ^[[2J end"
        );
        assert_eq!(escape_controls("a\u{7f}b"), "a^?b");
        assert_eq!(escape_controls("csi\u{9b}2J"), "csi<U+009B>2J");
        // A bare CR could overwrite the line it is on; only CRLF keeps it.
        assert_eq!(escape_controls("fake\roverwrite"), "fake^Moverwrite");
    }
}
