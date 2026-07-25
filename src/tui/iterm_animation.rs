//! Flicker-free animated GIF delivery for iTerm2.
//!
//! iTerm2 can animate a GIF itself. Sending the original file avoids the
//! clear-then-redraw behavior required by ratatui-image's per-frame iTerm2
//! encoder. Multipart transfer keeps both the OSC sequences and allocations
//! bounded for large recordings.

use ratatui::layout::Rect;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

// Divisible by three so every non-final base64 part is independently aligned.
// The encoded OSC payload is 64 KiB, well below iTerm2's 1 MiB limit.
const RAW_CHUNK_SIZE: usize = 48 * 1024;

pub(crate) fn transmit_gif<W: Write>(writer: &mut W, path: &Path, area: Rect) -> io::Result<()> {
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut raw = vec![0; RAW_CHUNK_SIZE];

    // Position the image at the same cell ratatui-image reserved. iTerm's
    // doNotMoveCursor extension prevents the transfer from disturbing the TUI
    // cursor position.
    write!(
        writer,
        "\x1b[{};{}H\x1b]1337;MultipartFile=inline=1;size={file_size};width={};height={};preserveAspectRatio=1;doNotMoveCursor=1\x07",
        area.y.saturating_add(1),
        area.x.saturating_add(1),
        area.width,
        area.height,
    )?;

    loop {
        let mut filled = 0;
        while filled < raw.len() {
            let read = reader.read(&mut raw[filled..])?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        if filled == 0 {
            break;
        }

        writer.write_all(b"\x1b]1337;FilePart=")?;
        write_base64(writer, &raw[..filled])?;
        writer.write_all(b"\x07")?;

        if filled < raw.len() {
            break;
        }
    }

    writer.write_all(b"\x1b]1337;FileEnd\x07")?;
    writer.flush()
}

fn write_base64<W: Write>(writer: &mut W, data: &[u8]) -> io::Result<()> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = Vec::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);

        encoded.push(ALPHABET[(first >> 2) as usize]);
        encoded.push(ALPHABET[(((first & 0x03) << 4) | (second >> 4)) as usize]);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize]
        } else {
            b'='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(third & 0x3f) as usize]
        } else {
            b'='
        });
    }

    writer.write_all(&encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(Default)]
    struct CountingWriter {
        writes: usize,
        bytes: usize,
    }

    impl Write for CountingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            self.bytes += buffer.len();
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn base64_matches_known_vectors() {
        for (input, expected) in [
            (&b""[..], ""),
            (&b"f"[..], "Zg=="),
            (&b"fo"[..], "Zm8="),
            (&b"foo"[..], "Zm9v"),
            (&b"foobar"[..], "Zm9vYmFy"),
        ] {
            let mut output = Vec::new();
            write_base64(&mut output, input).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), expected);
        }
    }

    #[test]
    fn multipart_transfer_is_positioned_and_bounded() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"GIF89a").unwrap();
        let mut output = Cursor::new(Vec::new());

        transmit_gif(&mut output, file.path(), Rect::new(4, 7, 80, 20)).unwrap();

        let output = String::from_utf8(output.into_inner()).unwrap();
        assert!(output.starts_with("\u{1b}[8;5H\u{1b}]1337;MultipartFile="));
        assert!(output.contains("size=6;width=80;height=20"));
        assert!(output.contains("\u{1b}]1337;FilePart=R0lGODlh\u{7}"));
        assert!(output.ends_with("\u{1b}]1337;FileEnd\u{7}"));
    }

    #[test]
    fn base64_chunk_is_emitted_in_one_write() {
        let mut output = CountingWriter::default();
        write_base64(&mut output, &vec![42; RAW_CHUNK_SIZE]).unwrap();
        assert_eq!(output.writes, 1);
        assert_eq!(output.bytes, RAW_CHUNK_SIZE / 3 * 4);
    }
}
