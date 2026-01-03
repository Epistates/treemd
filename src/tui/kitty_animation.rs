//! Native Kitty terminal animation support.
//!
//! Uses the Kitty graphics protocol animation features for smooth, flicker-free
//! GIF playback. The terminal handles frame timing, eliminating client-side flicker.
//!
//! Protocol reference: https://sw.kovidgoyal.net/kitty/graphics-protocol/#animation

use image::{DynamicImage, GenericImageView};
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};

/// Counter for generating unique image IDs
static IMAGE_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Generate a unique image ID for Kitty protocol
fn next_image_id() -> u32 {
    IMAGE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Represents an active Kitty animation
#[derive(Debug)]
pub struct KittyAnimation {
    /// The image ID used in the Kitty protocol
    pub image_id: u32,
    /// Number of frames in the animation
    pub frame_count: usize,
    /// Whether the animation is currently playing
    pub is_playing: bool,
}

/// Check if the terminal supports Kitty graphics protocol
pub fn is_kitty_terminal() -> bool {
    // Check TERM and TERM_PROGRAM environment variables
    if std::env::var("TERM")
        .map(|t| t.contains("kitty"))
        .unwrap_or(false)
    {
        return true;
    }
    if std::env::var("TERM_PROGRAM")
        .map(|t| t.to_lowercase() == "kitty")
        .unwrap_or(false)
    {
        return true;
    }
    // Also check KITTY_WINDOW_ID which is set by Kitty
    std::env::var("KITTY_WINDOW_ID").is_ok()
}


/// Simple base64 encoding
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Encode a DynamicImage to raw RGBA bytes
fn image_to_rgba(img: &DynamicImage) -> Vec<u8> {
    img.to_rgba8().into_raw()
}

/// Send a Kitty graphics protocol command
fn send_kitty_command<W: Write>(writer: &mut W, params: &str, payload: Option<&str>) -> io::Result<()> {
    write!(writer, "\x1b_G{}", params)?;
    if let Some(data) = payload {
        write!(writer, ";{}", data)?;
    }
    write!(writer, "\x1b\\")?;
    writer.flush()
}

/// Transmit animation frames to Kitty terminal.
///
/// This uses Kitty's native animation protocol where:
/// 1. Base image is transmitted with `a=T` (transmit and display)
/// 2. Additional frames are added with `a=f` (frame)
/// 3. Animation is started with `a=a` (animate)
///
/// The terminal handles all frame timing internally - no flicker!
pub fn transmit_animation<W: Write>(
    writer: &mut W,
    frames: &[(DynamicImage, u32)], // (image, delay_ms)
    col: u16,
    row: u16,
) -> io::Result<Option<KittyAnimation>> {
    if frames.is_empty() {
        return Ok(None);
    }

    let image_id = next_image_id();
    let first_frame = &frames[0].0;
    let (width, height) = first_frame.dimensions();

    // Step 1: Transmit base image (first frame)
    // a=T means transmit and display
    // f=32 means RGBA format
    // i=image_id for referencing
    // s=width, v=height
    // C=1 means don't move cursor
    let rgba_data = image_to_rgba(first_frame);
    let encoded = base64_encode(&rgba_data);

    // Send in chunks if data is large (max ~4096 bytes per chunk recommended)
    const CHUNK_SIZE: usize = 4096;
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(CHUNK_SIZE)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        let m = if is_last { 0 } else { 1 };

        if i == 0 {
            // First chunk includes all parameters
            let params = format!(
                "a=T,f=32,i={},s={},v={},c={},r={},C=1,q=2,m={}",
                image_id, width, height, col, row, m
            );
            send_kitty_command(writer, &params, Some(chunk))?;
        } else {
            // Subsequent chunks just continue the transmission
            let params = format!("m={}", m);
            send_kitty_command(writer, &params, Some(chunk))?;
        }
    }

    // Step 2: Add additional frames with a=f
    for (_frame_idx, (frame_img, delay_ms)) in frames.iter().enumerate().skip(1) {
        let rgba_data = image_to_rgba(frame_img);
        let encoded = base64_encode(&rgba_data);
        let chunks: Vec<&str> = encoded
            .as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect();

        // z=delay_ms sets frame timing
        // r=1 means this frame replaces the entire image (not a delta)
        for (i, chunk) in chunks.iter().enumerate() {
            let is_last = i == chunks.len() - 1;
            let m = if is_last { 0 } else { 1 };

            if i == 0 {
                let params = format!(
                    "a=f,i={},z={},r=1,f=32,s={},v={},q=2,m={}",
                    image_id, delay_ms, width, height, m
                );
                send_kitty_command(writer, &params, Some(chunk))?;
            } else {
                let params = format!("a=f,m={}", m);
                send_kitty_command(writer, &params, Some(chunk))?;
            }
        }
    }

    // Step 3: Start animation with a=a
    // s=3 means loop forever
    // v=0 means use default loop count
    let params = format!("a=a,i={},s=3,v=0,q=2", image_id);
    send_kitty_command(writer, &params, None)?;

    Ok(Some(KittyAnimation {
        image_id,
        frame_count: frames.len(),
        is_playing: true,
    }))
}

/// Stop and delete a Kitty animation
pub fn delete_animation<W: Write>(writer: &mut W, animation: &KittyAnimation) -> io::Result<()> {
    // a=d means delete, d=I means delete by image id and free memory
    let params = format!("a=d,d=I,i={},q=2", animation.image_id);
    send_kitty_command(writer, &params, None)
}

/// Pause a Kitty animation
pub fn pause_animation<W: Write>(writer: &mut W, animation: &KittyAnimation) -> io::Result<()> {
    // s=1 means stop
    let params = format!("a=a,i={},s=1,q=2", animation.image_id);
    send_kitty_command(writer, &params, None)
}

/// Resume a Kitty animation
pub fn resume_animation<W: Write>(writer: &mut W, animation: &KittyAnimation) -> io::Result<()> {
    // s=3 means run in loop mode
    let params = format!("a=a,i={},s=3,q=2", animation.image_id);
    send_kitty_command(writer, &params, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_kitty_detection() {
        // Just ensure it doesn't panic
        let _ = is_kitty_terminal();
    }

    #[test]
    fn test_base64_encoding() {
        let data = b"Hello, World!";
        let encoded = base64_encode(data);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_image_id_generation() {
        let id1 = next_image_id();
        let id2 = next_image_id();
        assert!(id2 > id1);
    }
}
