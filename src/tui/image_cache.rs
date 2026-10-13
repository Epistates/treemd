//! Image utilities for TUI rendering.
//!
//! Provides non-blocking terminal image encoding, static image loading, and
//! memory-bounded streaming GIF compositing for inline rendering.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

use image::Rgba;
use image::RgbaImage;
use ratatui::buffer::Buffer;
use ratatui::layout::{Rect, Size};
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::{Resize, ResizeEncodeRender};

/// How often the event loop checks for completed image work while an encode is pending.
pub const IMAGE_WORK_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

struct ResizeJob {
    protocol: StatefulProtocol,
    resize: Resize,
    size: Size,
    protocol_id: u64,
    generation: u64,
}

pub(crate) struct ResizeResult {
    protocol: StatefulProtocol,
    protocol_id: u64,
    generation: u64,
    size: Size,
    error: Option<String>,
}

/// A single background worker shared by all images in an [`App`](crate::tui::App).
///
/// `ratatui-image` deliberately separates resize/encode from rendering because
/// graphics protocols can be expensive to prepare. Keeping one ordered worker
/// avoids both frame stalls and an unbounded thread per cached image.
pub(crate) struct ImageWorker {
    request_tx: mpsc::Sender<ResizeJob>,
    result_rx: mpsc::Receiver<ResizeResult>,
    pending: Arc<AtomicUsize>,
    next_protocol_id: AtomicU64,
}

impl ImageWorker {
    pub(crate) fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<ResizeJob>();
        let (result_tx, result_rx) = mpsc::channel::<ResizeResult>();
        let pending = Arc::new(AtomicUsize::new(0));

        let _ = thread::Builder::new()
            .name("treemd-image-resize".to_string())
            .spawn(move || {
                while let Ok(mut job) = request_rx.recv() {
                    let error = crate::tui::tty::catch_isolated(|| {
                        job.protocol.resize_encode(&job.resize, job.size);
                        job.protocol
                            .last_encoding_result()
                            .and_then(Result::err)
                            .map(|error| error.to_string())
                    })
                    .unwrap_or_else(|_| Some("image encoder panicked".to_string()));
                    if result_tx
                        .send(ResizeResult {
                            protocol: job.protocol,
                            protocol_id: job.protocol_id,
                            generation: job.generation,
                            size: job.size,
                            error,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });

        Self {
            request_tx,
            result_rx,
            pending,
            next_protocol_id: AtomicU64::new(1),
        }
    }

    pub(crate) fn wrap(&self, protocol: StatefulProtocol) -> AsyncProtocol {
        AsyncProtocol {
            inner: Some(protocol),
            fallback: None,
            request_tx: self.request_tx.clone(),
            pending: Arc::clone(&self.pending),
            protocol_id: self.next_protocol_id.fetch_add(1, Ordering::Relaxed),
            generation: 0,
            in_flight_generation: None,
            last_size: Size::default(),
            failed_size: None,
            has_completed_frame: false,
        }
    }

    pub(crate) fn drain(&self) -> Vec<ResizeResult> {
        let results: Vec<_> = self.result_rx.try_iter().collect();
        if !results.is_empty() {
            self.pending.fetch_sub(results.len(), Ordering::AcqRel);
        }
        results
    }

    pub(crate) fn has_pending(&self) -> bool {
        self.pending.load(Ordering::Acquire) > 0
    }
}

/// Non-blocking state for `ratatui_image::StatefulImage`.
///
/// Rendering submits resize/encode work and continues with the last completed
/// frame. A generation counter discards stale responses after a newer image or
/// terminal size supersedes them.
pub(crate) struct AsyncProtocol {
    inner: Option<StatefulProtocol>,
    /// Last successfully encoded frame, retained while its replacement is prepared.
    fallback: Option<StatefulProtocol>,
    request_tx: mpsc::Sender<ResizeJob>,
    pending: Arc<AtomicUsize>,
    protocol_id: u64,
    generation: u64,
    in_flight_generation: Option<u64>,
    last_size: Size,
    failed_size: Option<Size>,
    has_completed_frame: bool,
}

impl AsyncProtocol {
    pub(crate) fn size_for(&mut self, resize: Resize, available: Size) -> Size {
        if let Some(protocol) = self.inner.as_ref().or(self.fallback.as_ref()) {
            self.last_size = protocol.size_for(resize, available);
        }
        Size {
            width: self.last_size.width.min(available.width),
            height: self.last_size.height.min(available.height),
        }
    }

    pub(crate) fn replace_protocol(&mut self, protocol: StatefulProtocol) {
        // Keep the completed frame visible until the replacement has finished
        // resize/encode. This is especially important for animated images:
        // dropping it here produces a blank flash on every GIF frame.
        if let Some(previous) = self.inner.take() {
            self.fallback = Some(previous);
        }
        self.inner = Some(protocol);
        self.generation = self.generation.wrapping_add(1);
        self.last_size = Size::default();
        self.failed_size = None;
    }

    /// Whether resize/encode work is currently outstanding for this protocol.
    pub(crate) fn has_in_flight_work(&self) -> bool {
        self.in_flight_generation.is_some()
    }

    /// Whether at least one frame has completed encoding successfully.
    pub(crate) fn has_completed_frame(&self) -> bool {
        self.has_completed_frame
    }

    pub(crate) fn matches(&self, result: &ResizeResult) -> bool {
        self.protocol_id == result.protocol_id
    }

    pub(crate) fn update(&mut self, result: ResizeResult) -> Option<String> {
        if self.in_flight_generation == Some(result.generation) {
            self.in_flight_generation = None;
        }

        if self.generation == result.generation {
            if result.error.is_some() && self.fallback.is_some() {
                // Preserve the previously completed frame on an encode error.
                self.inner = None;
            } else {
                self.inner = Some(result.protocol);
            }
            self.failed_size = result.error.as_ref().map(|_| result.size);
            if result.error.is_none() {
                self.fallback = None;
                self.has_completed_frame = true;
            }
            result.error
        } else {
            None
        }
    }
}

impl ResizeEncodeRender for AsyncProtocol {
    fn needs_resize(&self, resize: &Resize, size: Size) -> Option<Size> {
        self.inner
            .as_ref()
            .and_then(|protocol| protocol.needs_resize(resize, size))
            .filter(|requested| Some(*requested) != self.failed_size)
    }

    fn resize_encode(&mut self, resize: &Resize, size: Size) {
        let Some(protocol) = self.inner.take() else {
            return;
        };

        self.generation = self.generation.wrapping_add(1);
        self.in_flight_generation = Some(self.generation);
        self.pending.fetch_add(1, Ordering::AcqRel);
        let job = ResizeJob {
            protocol,
            resize: resize.clone(),
            size,
            protocol_id: self.protocol_id,
            generation: self.generation,
        };

        if let Err(error) = self.request_tx.send(job) {
            self.pending.fetch_sub(1, Ordering::AcqRel);
            let mut job = error.0;
            job.protocol.resize_encode(&job.resize, job.size);
            let failed = job
                .protocol
                .last_encoding_result()
                .and_then(Result::err)
                .is_some();
            self.failed_size = failed.then_some(job.size);
            self.has_completed_frame |= !failed;
            self.in_flight_generation = None;
            self.inner = Some(job.protocol);
            if !failed {
                self.fallback = None;
            }
        }
    }

    fn render(&mut self, area: Rect, buffer: &mut Buffer) {
        if let Some(protocol) = self.inner.as_mut().or(self.fallback.as_mut()) {
            protocol.render(area, buffer);
        }
    }
}

/// Errors that can occur during image loading and caching
#[derive(Debug, Clone)]
pub enum ImageError {
    /// Image file not found
    NotFound,
    /// Invalid image format
    InvalidFormat(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::NotFound => write!(f, "Image not found"),
            ImageError::InvalidFormat(s) => write!(f, "Invalid format: {}", s),
        }
    }
}

impl std::error::Error for ImageError {}

/// Largest image file treemd will open. A screen recording of a few minutes
/// fits comfortably; past this the file is almost certainly not meant for a
/// terminal pane.
const MAX_IMAGE_FILE_BYTES: u64 = 128 * 1024 * 1024;

/// Largest GIF canvas treemd will allocate, in pixels, which is 64 MB of
/// RGBA. A GIF declares its canvas size in its header, before any frame, and
/// a 62-byte file can declare 65535x65535: 17 GB, allocated the moment the
/// document is opened.
const MAX_GIF_CANVAS_PIXELS: u64 = 16 * 1024 * 1024;

/// Open an image the document names, if it is a regular file of a size worth
/// drawing.
///
/// Checked before opening, because opening a FIFO blocks until something
/// writes to it, and this runs on the UI thread with raw mode on, so the
/// session freezes with no way to interrupt it. A terminal or a device is
/// refused for the same reason.
pub(crate) fn open_image_file(path: &Path) -> Result<File, ImageError> {
    let meta = std::fs::metadata(path).map_err(|_| ImageError::NotFound)?;
    if !meta.is_file() {
        return Err(ImageError::InvalidFormat("not a regular file".to_string()));
    }
    if meta.len() > MAX_IMAGE_FILE_BYTES {
        return Err(ImageError::InvalidFormat(format!(
            "{} MB, over the {} MB limit",
            meta.len() / (1024 * 1024),
            MAX_IMAGE_FILE_BYTES / (1024 * 1024)
        )));
    }
    File::open(path).map_err(|_| ImageError::NotFound)
}

/// A GIF decoder for `path`, or `None` when the file is not a GIF.
///
/// Refuses a canvas over [`MAX_GIF_CANVAS_PIXELS`] before anything is
/// allocated for it.
fn read_gif(
    path: &Path,
    color: gif::ColorOutput,
) -> Result<Option<gif::Decoder<BufReader<File>>>, ImageError> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(color);
    read_gif_with(path, options)
}

/// [`read_gif`] with the decoder configured by the caller.
fn read_gif_with(
    path: &Path,
    options: gif::DecodeOptions,
) -> Result<Option<gif::Decoder<BufReader<File>>>, ImageError> {
    let file = open_image_file(path)?;
    let Ok(decoder) = options.read_info(BufReader::new(file)) else {
        return Ok(None);
    };
    let (width, height) = (decoder.width(), decoder.height());
    if u64::from(width) * u64::from(height) > MAX_GIF_CANVAS_PIXELS {
        return Err(ImageError::InvalidFormat(format!(
            "GIF canvas {width}x{height} is too large"
        )));
    }
    Ok(Some(decoder))
}

/// [`read_gif`] for a path that has to be a GIF.
fn require_gif(
    path: &Path,
    color: gif::ColorOutput,
) -> Result<gif::Decoder<BufReader<File>>, ImageError> {
    read_gif(path, color)?.ok_or_else(|| ImageError::InvalidFormat("not a GIF".to_string()))
}

/// Largest still image treemd decodes, in pixels: a 48-megapixel phone
/// photo fits.
///
/// The `image` crate's default limit is on memory, 512 MB, which an
/// 11000x11000 PNG of 470 KB stays under: three seconds of decoding with
/// input frozen, and 840 MB held, for a file anyone can drop in a folder.
const MAX_STILL_PIXELS: u64 = 64 * 1024 * 1024;

/// Decode a still image in any supported format, refusing one over
/// [`MAX_STILL_PIXELS`] from the size in its header, before decoding it.
fn decode_still(path: &Path) -> Result<image::DynamicImage, ImageError> {
    let invalid = |error: &dyn std::fmt::Display| ImageError::InvalidFormat(error.to_string());
    let reader = || -> Result<_, ImageError> {
        image::ImageReader::new(BufReader::new(open_image_file(path)?))
            .with_guessed_format()
            .map_err(|e| invalid(&e))
    };
    let (width, height) = reader()?.into_dimensions().map_err(|e| invalid(&e))?;
    if u64::from(width) * u64::from(height) > MAX_STILL_PIXELS {
        return Err(ImageError::InvalidFormat(format!(
            "{width}x{height} is too large to draw"
        )));
    }
    reader()?.decode().map_err(|e| invalid(&e))
}

/// Longest side an inline image is kept at, in pixels.
const MAX_INLINE_SIDE: u32 = 2048;

/// An image scaled down, if need be, to a size an inline image can use.
///
/// Inline images are drawn into a pane a few hundred cells wide, so a photo
/// kept at its full resolution, tens of megabytes of RGBA, held far more than
/// could ever be shown, and the cache holds up to 32 of them. The image
/// viewer decodes its own copy at full resolution.
pub(crate) fn fit_for_inline(image: image::DynamicImage) -> image::DynamicImage {
    if image.width().max(image.height()) <= MAX_INLINE_SIDE {
        image
    } else {
        image.resize(
            MAX_INLINE_SIDE,
            MAX_INLINE_SIDE,
            image::imageops::FilterType::Triangle,
        )
    }
}

/// Namespace for image loading static methods.
pub struct ImageCache;

/// A memory-bounded GIF player for the image viewer.
///
/// Retaining one compositing canvas and the decoder, rather than every frame,
/// keeps a long README recording from expanding into hundreds of full-size
/// RGBA frames. Playback is forward only; the viewer reopens it to step back.
/// Inline, a GIF is a still of its first frame.
pub(crate) struct StreamingGif {
    path: PathBuf,
    decoder: gif::Decoder<BufReader<File>>,
    canvas: RgbaImage,
    previous_disposal: gif::DisposalMethod,
    previous_rect: (u32, u32, u32, u32),
    restore_previous: Option<RgbaImage>,
}

impl StreamingGif {
    /// Open a GIF and return its composited first frame.
    pub(crate) fn open(path: &Path) -> Result<(Self, image::DynamicImage), ImageError> {
        let decoder = Self::open_decoder(path)?;
        let width = u32::from(decoder.width());
        let height = u32::from(decoder.height());
        let mut animation = Self {
            path: path.to_path_buf(),
            decoder,
            canvas: RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0])),
            previous_disposal: gif::DisposalMethod::Keep,
            previous_rect: (0, 0, 0, 0),
            restore_previous: None,
        };

        let Some(image) = animation.decode_next()? else {
            return Err(ImageError::InvalidFormat(
                "GIF contains no image frames".to_string(),
            ));
        };
        Ok((animation, image))
    }

    fn open_decoder(path: &Path) -> Result<gif::Decoder<BufReader<File>>, ImageError> {
        require_gif(path, gif::ColorOutput::RGBA)
    }

    fn reset_decoder(&mut self) -> Result<(), ImageError> {
        self.decoder = Self::open_decoder(&self.path)?;
        self.canvas = RgbaImage::from_pixel(
            u32::from(self.decoder.width()),
            u32::from(self.decoder.height()),
            Rgba([0, 0, 0, 0]),
        );
        self.previous_disposal = gif::DisposalMethod::Keep;
        self.previous_rect = (0, 0, 0, 0);
        self.restore_previous = None;
        Ok(())
    }

    fn decode_next(&mut self) -> Result<Option<image::DynamicImage>, ImageError> {
        match self.previous_disposal {
            gif::DisposalMethod::Background => {
                ImageCache::clear_gif_rect(&mut self.canvas, self.previous_rect);
            }
            gif::DisposalMethod::Previous => {
                if let Some(previous) = self.restore_previous.take() {
                    self.canvas = previous;
                }
            }
            gif::DisposalMethod::Any | gif::DisposalMethod::Keep => {}
        }

        let frame = self
            .decoder
            .read_next_frame()
            .map_err(|error| ImageError::InvalidFormat(error.to_string()))?;
        let Some(frame) = frame else {
            return Ok(None);
        };

        let restore_for_this_frame =
            (frame.dispose == gif::DisposalMethod::Previous).then(|| self.canvas.clone());
        let width = self.canvas.width();
        let height = self.canvas.height();
        ImageCache::composite_gif_frame(&mut self.canvas, frame, width, height);

        self.previous_disposal = frame.dispose;
        self.previous_rect = (
            frame.left.into(),
            frame.top.into(),
            frame.width.into(),
            frame.height.into(),
        );
        self.restore_previous = restore_for_this_frame;

        Ok(Some(image::DynamicImage::ImageRgba8(self.canvas.clone())))
    }

    /// The next frame regardless of the frame clock, wrapping to the first
    /// after the last. For the image viewer, which keeps its own timing and
    /// its own frame index.
    pub(crate) fn next_frame_looping(&mut self) -> Result<image::DynamicImage, ImageError> {
        if let Some(image) = self.decode_next()? {
            return Ok(image);
        }
        self.reset_decoder()?;
        self.decode_next()?
            .ok_or_else(|| ImageError::InvalidFormat("GIF contains no image frames".to_string()))
    }
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

    /// Read GIF timing without decoding any frame.
    ///
    /// Only the frames' headers are read: decompressing each frame just to
    /// learn its delay cost about half a second on a 66 MB recording, on the
    /// UI thread, as the viewer opened.
    pub fn extract_gif_frame_delays(path: &Path) -> Result<Vec<u32>, ImageError> {
        let mut options = gif::DecodeOptions::new();
        options.skip_frame_decoding(true);
        let mut decoder = read_gif_with(path, options)?
            .ok_or_else(|| ImageError::InvalidFormat("not a GIF".to_string()))?;
        let mut delays = Vec::new();

        while let Some(frame) = decoder
            .next_frame_info()
            .map_err(|error| ImageError::InvalidFormat(error.to_string()))?
        {
            delays.push((u32::from(frame.delay) * 10).max(20));
        }

        if delays.is_empty() {
            Err(ImageError::InvalidFormat(
                "GIF contains no image frames".to_string(),
            ))
        } else {
            Ok(delays)
        }
    }

    fn clear_gif_rect(canvas: &mut RgbaImage, rect: (u32, u32, u32, u32)) {
        let (left, top, width, height) = rect;
        for y in top..top.saturating_add(height).min(canvas.height()) {
            for x in left..left.saturating_add(width).min(canvas.width()) {
                canvas.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }

    /// Extract the first frame from an image file, properly handling GIFs.
    ///
    /// For regular images, returns the image as-is.
    /// For GIFs, extracts and composites the first frame with proper transparency.
    pub fn extract_first_frame(path: &Path) -> Result<image::DynamicImage, ImageError> {
        if let Some(mut decoder) = read_gif(path, gif::ColorOutput::RGBA)? {
            let width = decoder.width() as u32;
            let height = decoder.height() as u32;
            if let Ok(Some(frame)) = decoder.read_next_frame() {
                let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
                Self::composite_gif_frame(&mut canvas, frame, width, height);
                return Ok(image::DynamicImage::ImageRgba8(canvas));
            }
        }
        decode_still(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BMP header declaring 10000x10000, with no pixels behind it. The
    /// size in the header is enough to refuse it, before any decoding.
    #[test]
    fn a_huge_still_is_refused_from_its_header() {
        let mut bmp = Vec::new();
        bmp.extend_from_slice(b"BM");
        bmp.extend_from_slice(&54u32.to_le_bytes()); // file size
        bmp.extend_from_slice(&0u32.to_le_bytes()); // reserved
        bmp.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset
        bmp.extend_from_slice(&40u32.to_le_bytes()); // header size
        bmp.extend_from_slice(&10_000i32.to_le_bytes()); // width
        bmp.extend_from_slice(&10_000i32.to_le_bytes()); // height
        bmp.extend_from_slice(&1u16.to_le_bytes()); // planes
        bmp.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
        bmp.extend_from_slice(&[0; 24]); // compression and the rest
        let file = tempfile::Builder::new().suffix(".bmp").tempfile().unwrap();
        std::fs::write(file.path(), &bmp).unwrap();

        let error = decode_still(file.path()).unwrap_err();
        assert!(error.to_string().contains("too large"), "{error}");
    }

    #[test]
    fn clear_gif_rect_only_clears_disposal_region() {
        let opaque = Rgba([10, 20, 30, 255]);
        let mut canvas = RgbaImage::from_pixel(4, 4, opaque);

        ImageCache::clear_gif_rect(&mut canvas, (1, 1, 2, 2));

        assert_eq!(*canvas.get_pixel(0, 0), opaque);
        assert_eq!(*canvas.get_pixel(1, 1), Rgba([0, 0, 0, 0]));
        assert_eq!(*canvas.get_pixel(2, 2), Rgba([0, 0, 0, 0]));
        assert_eq!(*canvas.get_pixel(3, 3), opaque);
    }

    #[test]
    fn async_protocol_encodes_off_thread_and_reuses_completed_size() {
        let worker = ImageWorker::new();
        let picker = ratatui_image::picker::Picker::halfblocks();
        let image = image::DynamicImage::new_rgba8(8, 8);
        let mut protocol = worker.wrap(picker.new_resize_protocol(image));
        let resize = Resize::Scale(None);
        let size = Size::new(4, 3);

        let requested = protocol
            .needs_resize(&resize, size)
            .expect("new protocol needs encoding");
        protocol.resize_encode(&resize, requested);
        assert!(worker.has_pending());

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let result = loop {
            if let Some(result) = worker.drain().into_iter().next() {
                break result;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "image worker timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        };

        assert!(protocol.matches(&result));
        assert_eq!(protocol.update(result), None);
        assert!(!worker.has_pending());
        assert!(protocol.has_completed_frame());
        assert!(protocol.needs_resize(&resize, size).is_none());

        protocol.replace_protocol(picker.new_resize_protocol(image::DynamicImage::new_rgba8(8, 8)));
        assert!(
            protocol.fallback.is_some(),
            "the completed frame should remain available during replacement"
        );
        let requested = protocol
            .needs_resize(&resize, size)
            .expect("replacement needs encoding");
        protocol.resize_encode(&resize, requested);
        assert!(protocol.inner.is_none());
        assert!(protocol.fallback.is_some());
        assert!(protocol.has_in_flight_work());
    }

    #[test]
    fn an_inline_image_is_kept_no_larger_than_it_can_be_shown() {
        let big = image::DynamicImage::new_rgba8(8000, 2000);
        let fitted = fit_for_inline(big);
        assert_eq!((fitted.width(), fitted.height()), (2048, 512));

        let small = image::DynamicImage::new_rgba8(300, 200);
        let fitted = fit_for_inline(small);
        assert_eq!((fitted.width(), fitted.height()), (300, 200));
    }

    #[test]
    fn streaming_gif_steps_and_loops_without_retaining_all_frames() {
        use std::borrow::Cow;

        let mut file = tempfile::NamedTempFile::new().expect("temporary gif");
        {
            let palette = [255, 0, 0, 0, 255, 0];
            let mut encoder =
                gif::Encoder::new(file.as_file_mut(), 2, 1, &palette).expect("gif encoder");
            encoder
                .set_repeat(gif::Repeat::Infinite)
                .expect("set repeat");

            let first = gif::Frame {
                width: 2,
                height: 1,
                delay: 2,
                buffer: Cow::Owned(vec![0, 0]),
                ..gif::Frame::default()
            };
            encoder.write_frame(&first).expect("first frame");

            let second = gif::Frame {
                width: 2,
                height: 1,
                delay: 2,
                buffer: Cow::Owned(vec![1, 1]),
                ..gif::Frame::default()
            };
            encoder.write_frame(&second).expect("second frame");
        }

        let (mut animation, first) = StreamingGif::open(file.path()).expect("streaming decoder");
        assert_eq!(first.to_rgba8().get_pixel(0, 0), &Rgba([255, 0, 0, 255]));

        let second = animation.next_frame_looping().expect("second frame");
        assert_eq!(second.to_rgba8().get_pixel(0, 0), &Rgba([0, 255, 0, 255]));

        let looped = animation.next_frame_looping().expect("looped frame");
        assert_eq!(looped.to_rgba8().get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn gif_metadata_and_random_frame_decode_are_memory_bounded() {
        use std::borrow::Cow;

        let mut file = tempfile::NamedTempFile::new().expect("temporary gif");
        {
            let palette = [255, 0, 0, 0, 255, 0];
            let mut encoder =
                gif::Encoder::new(file.as_file_mut(), 2, 1, &palette).expect("gif encoder");
            for (color, delay) in [(0, 2), (1, 7)] {
                let frame = gif::Frame {
                    width: 2,
                    height: 1,
                    delay,
                    buffer: Cow::Owned(vec![color, color]),
                    ..gif::Frame::default()
                };
                encoder.write_frame(&frame).expect("frame");
            }
        }

        assert_eq!(
            ImageCache::extract_gif_frame_delays(file.path()).unwrap(),
            vec![20, 70]
        );
        let (mut stream, first) = StreamingGif::open(file.path()).unwrap();
        assert_eq!(first.to_rgba8().get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        let second = stream.next_frame_looping().unwrap();
        assert_eq!(second.to_rgba8().get_pixel(0, 0), &Rgba([0, 255, 0, 255]));
        // The viewer loops by asking past the end.
        let wrapped = stream.next_frame_looping().unwrap();
        assert_eq!(wrapped.to_rgba8().get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
    }

    /// A GIF's canvas size comes from its header, so a file of a few dozen
    /// bytes can ask for gigabytes. Every path that composites a GIF has to
    /// refuse it before allocating.
    #[test]
    fn a_gif_declaring_a_huge_canvas_is_refused_before_allocating() {
        use std::borrow::Cow;

        let mut file = tempfile::NamedTempFile::new().expect("temporary gif");
        {
            let palette = [255, 0, 0, 0, 255, 0];
            let mut encoder = gif::Encoder::new(file.as_file_mut(), 60_000, 60_000, &palette)
                .expect("gif encoder");
            let frame = gif::Frame {
                width: 1,
                height: 1,
                buffer: Cow::Owned(vec![0]),
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).expect("frame");
        }
        let path = file.path();
        assert!(std::fs::metadata(path).unwrap().len() < 100);

        assert!(StreamingGif::open(path).is_err());
        assert!(ImageCache::extract_first_frame(path).is_err());
        assert!(ImageCache::extract_gif_frame_delays(path).is_err());
    }

    /// Opening a FIFO blocks until something writes to it, which on the UI
    /// thread freezes the session with raw mode on. An image path that is not
    /// a regular file is refused without opening it.
    #[test]
    fn an_image_path_that_is_not_a_regular_file_is_refused() {
        let dir = tempfile::tempdir().expect("temporary dir");
        assert!(open_image_file(dir.path()).is_err());
        assert!(ImageCache::extract_first_frame(dir.path()).is_err());

        #[cfg(unix)]
        {
            let fifo = dir.path().join("pipe.png");
            let c_path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
            // SAFETY: `c_path` is a valid NUL-terminated path for the call's duration.
            assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
            assert!(open_image_file(&fifo).is_err());
            assert!(ImageCache::extract_first_frame(&fifo).is_err());
            assert!(StreamingGif::open(&fifo).is_err());
        }
    }
}
