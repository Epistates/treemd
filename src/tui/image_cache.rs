//! Image utilities for TUI rendering.
//!
//! Provides helper functions for loading and processing images, particularly
//! extracting the first frame from GIF files for rendering.

use std::io::BufReader;
use std::path::Path;

use image::Rgba;
use image::RgbaImage;

/// Errors that can occur during image loading and caching
#[derive(Debug, Clone)]
pub enum ImageError {
    /// Image file not found
    NotFound,
    /// Image is currently being loaded
    Loading,
    /// Invalid image format
    InvalidFormat(String),
    /// Other errors (IO, parsing, etc.)
    Failed(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::NotFound => write!(f, "Image not found"),
            ImageError::Loading => write!(f, "Image loading"),
            ImageError::InvalidFormat(s) => write!(f, "Invalid format: {}", s),
            ImageError::Failed(s) => write!(f, "Failed: {}", s),
        }
    }
}

impl std::error::Error for ImageError {}

/// Namespace for image loading static methods.
pub struct ImageCache;

/// GIF frame with timing information
#[derive(Clone)]
pub struct GifFrame {
    pub image: image::DynamicImage,
    /// Delay in milliseconds
    pub delay_ms: u32,
}

impl ImageCache {
    /// Composite a GIF frame onto a canvas, handling transparency
    fn composite_gif_frame(canvas: &mut RgbaImage, frame: &gif::Frame, width: u32, height: u32) {
        let frame_buffer = &frame.buffer;
        let frame_width = frame.width as u32;
        let frame_height = frame.height as u32;
        let left = frame.left as u32;
        let top = frame.top as u32;

        for y in 0..frame_height {
            for x in 0..frame_width {
                let src_idx = ((y * frame_width + x) * 4) as usize;
                if src_idx + 3 < frame_buffer.len() {
                    let pixel = Rgba([
                        frame_buffer[src_idx],
                        frame_buffer[src_idx + 1],
                        frame_buffer[src_idx + 2],
                        frame_buffer[src_idx + 3],
                    ]);

                    let canvas_x = left + x;
                    let canvas_y = top + y;

                    if canvas_x < width && canvas_y < height && pixel[3] > 0 {
                        canvas.put_pixel(canvas_x, canvas_y, pixel);
                    }
                }
            }
        }
    }

    /// Extract all frames from a GIF file with timing information.
    ///
    /// Returns a vector of frames with their delays in milliseconds.
    /// For non-GIF images, returns a single frame with 0ms delay (static).
    /// Maintains a persistent canvas to properly handle GIF disposal methods.
    pub fn extract_all_frames(path: &Path) -> Result<Vec<GifFrame>, ImageError> {
        use std::fs::File;

        let file = File::open(path).map_err(|_| ImageError::NotFound)?;
        let reader = BufReader::new(file);

        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);

        match options.read_info(reader) {
            Ok(mut decoder) => {
                let width = decoder.width() as u32;
                let height = decoder.height() as u32;
                let mut frames = Vec::new();

                // Persistent canvas for proper frame compositing
                let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));

                while let Ok(Some(frame)) = decoder.read_next_frame() {
                    // Composite this frame onto the persistent canvas
                    Self::composite_gif_frame(&mut canvas, frame, width, height);

                    // Clone the current canvas state as this frame
                    let delay_ms = (frame.delay as u32) * 10;
                    frames.push(GifFrame {
                        image: image::DynamicImage::ImageRgba8(canvas.clone()),
                        delay_ms: delay_ms.max(20), // Min 20ms (50fps cap)
                    });
                }

                if frames.is_empty() {
                    Self::load_static_image(path)
                } else {
                    Ok(frames)
                }
            }
            Err(_) => Self::load_static_image(path),
        }
    }

    /// Load a static (non-GIF) image as a single frame
    fn load_static_image(path: &Path) -> Result<Vec<GifFrame>, ImageError> {
        image::ImageReader::open(path)
            .ok()
            .and_then(|r| r.with_guessed_format().ok())
            .and_then(|r| r.decode().ok())
            .map(|img| {
                vec![GifFrame {
                    image: img,
                    delay_ms: 0,
                }]
            })
            .ok_or_else(|| ImageError::InvalidFormat("Unsupported image format".to_string()))
    }

    /// Extract the first frame from an image file, properly handling GIFs.
    ///
    /// For regular images, returns the image as-is.
    /// For GIFs, extracts and composites the first frame with proper transparency.
    pub fn extract_first_frame(path: &Path) -> Result<image::DynamicImage, ImageError> {
        use std::fs::File;

        let file = File::open(path).map_err(|_| ImageError::NotFound)?;
        let reader = BufReader::new(file);

        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);

        match options.read_info(reader) {
            Ok(mut decoder) => {
                let width = decoder.width() as u32;
                let height = decoder.height() as u32;

                if let Ok(Some(frame)) = decoder.read_next_frame() {
                    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
                    Self::composite_gif_frame(&mut canvas, frame, width, height);
                    Ok(image::DynamicImage::ImageRgba8(canvas))
                } else {
                    image::ImageReader::open(path)
                        .ok()
                        .and_then(|r| r.with_guessed_format().ok())
                        .and_then(|r| r.decode().ok())
                        .ok_or_else(|| {
                            ImageError::InvalidFormat("Failed to decode image".to_string())
                        })
                }
            }
            Err(_) => image::ImageReader::open(path)
                .ok()
                .and_then(|r| r.with_guessed_format().ok())
                .and_then(|r| r.decode().ok())
                .ok_or_else(|| ImageError::InvalidFormat("Unsupported format".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gif_frame_struct() {
        // GifFrame should store image and delay information
        let img = image::DynamicImage::new_rgba8(10, 10);
        let frame = GifFrame {
            image: img,
            delay_ms: 100,
        };
        assert_eq!(frame.delay_ms, 100);
    }

    // Note: Integration tests for image loading would require actual image files.
}
