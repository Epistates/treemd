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
use std::time::{Duration, Instant};

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
                    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        job.protocol.resize_encode(&job.resize, job.size);
                        job.protocol
                            .last_encoding_result()
                            .and_then(Result::err)
                            .map(|error| error.to_string())
                    }))
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

/// Namespace for image loading static methods.
pub struct ImageCache;

/// GIF frame with timing information
#[derive(Clone)]
pub struct GifFrame {
    pub image: image::DynamicImage,
    /// Delay in milliseconds
    pub delay_ms: u32,
}

/// Result of advancing a streaming inline GIF.
pub(crate) enum GifAdvance {
    Waiting,
    Frame(image::DynamicImage),
    Finished,
    Failed(String),
}

/// A memory-bounded GIF player for inline images.
///
/// The modal viewer deliberately keeps every frame for random access. Inline
/// images only need forward playback, so retaining a compositing canvas and
/// decoder avoids expanding a long README recording into hundreds of full-size
/// RGBA frames.
pub(crate) struct StreamingGif {
    path: PathBuf,
    decoder: gif::Decoder<BufReader<File>>,
    canvas: RgbaImage,
    previous_disposal: gif::DisposalMethod,
    previous_rect: (u32, u32, u32, u32),
    restore_previous: Option<RgbaImage>,
    repeat: gif::Repeat,
    completed_repeats: u16,
    frames_in_first_pass: usize,
    current_delay: Duration,
    last_frame_update: Instant,
    active: bool,
}

impl StreamingGif {
    /// Open a GIF and return its composited first frame.
    pub(crate) fn open(path: &Path) -> Result<(Self, image::DynamicImage), ImageError> {
        let decoder = Self::open_decoder(path)?;
        let width = u32::from(decoder.width());
        let height = u32::from(decoder.height());
        let repeat = decoder.repeat();
        let mut animation = Self {
            path: path.to_path_buf(),
            decoder,
            canvas: RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0])),
            previous_disposal: gif::DisposalMethod::Keep,
            previous_rect: (0, 0, 0, 0),
            restore_previous: None,
            repeat,
            completed_repeats: 0,
            frames_in_first_pass: 0,
            current_delay: Duration::from_millis(20),
            last_frame_update: Instant::now(),
            active: false,
        };

        let Some((image, delay)) = animation.decode_next()? else {
            return Err(ImageError::InvalidFormat(
                "GIF contains no image frames".to_string(),
            ));
        };
        animation.frames_in_first_pass = 1;
        animation.current_delay = delay;
        Ok((animation, image))
    }

    fn open_decoder(path: &Path) -> Result<gif::Decoder<BufReader<File>>, ImageError> {
        let file = File::open(path).map_err(|_| ImageError::NotFound)?;
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        options
            .read_info(BufReader::new(file))
            .map_err(|error| ImageError::InvalidFormat(error.to_string()))
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

    fn decode_next(&mut self) -> Result<Option<(image::DynamicImage, Duration)>, ImageError> {
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

        let delay =
            Duration::from_millis(u64::from(frame.delay) * 10).max(Duration::from_millis(20));
        self.previous_disposal = frame.dispose;
        self.previous_rect = (
            frame.left.into(),
            frame.top.into(),
            frame.width.into(),
            frame.height.into(),
        );
        self.restore_previous = restore_for_this_frame;

        Ok(Some((
            image::DynamicImage::ImageRgba8(self.canvas.clone()),
            delay,
        )))
    }

    /// Start or stop playback without decoding a frame.
    ///
    /// GIFs are deliberately inactive when opened so the composited first frame
    /// remains a still. A viewer can explicitly activate the frame clock.
    pub(crate) fn set_active(&mut self, active: bool) {
        if active && !self.active {
            self.last_frame_update = Instant::now();
        }
        self.active = active;
    }

    pub(crate) fn time_until_next_frame(&self) -> Option<Duration> {
        if !self.active {
            return None;
        }
        Some(
            self.current_delay
                .saturating_sub(self.last_frame_update.elapsed())
                .max(Duration::from_millis(1)),
        )
    }

    pub(crate) fn advance(&mut self) -> GifAdvance {
        if !self.active || self.last_frame_update.elapsed() < self.current_delay {
            return GifAdvance::Waiting;
        }

        match self.decode_next() {
            Ok(Some((image, delay))) => {
                self.frames_in_first_pass = self.frames_in_first_pass.saturating_add(1);
                self.current_delay = delay;
                self.last_frame_update = Instant::now();
                GifAdvance::Frame(image)
            }
            Ok(None) if self.frames_in_first_pass <= 1 => GifAdvance::Finished,
            Ok(None) => {
                let should_repeat = match self.repeat {
                    gif::Repeat::Infinite => true,
                    gif::Repeat::Finite(repeats) => self.completed_repeats < repeats,
                };
                if !should_repeat {
                    return GifAdvance::Finished;
                }

                self.completed_repeats = self.completed_repeats.saturating_add(1);
                match self.reset_decoder().and_then(|()| self.decode_next()) {
                    Ok(Some((image, delay))) => {
                        self.current_delay = delay;
                        self.last_frame_update = Instant::now();
                        GifAdvance::Frame(image)
                    }
                    Ok(None) => GifAdvance::Finished,
                    Err(error) => GifAdvance::Failed(error.to_string()),
                }
            }
            Err(error) => GifAdvance::Failed(error.to_string()),
        }
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

    /// Read GIF timing without retaining decoded RGBA canvases.
    ///
    /// The decoder reuses its frame buffer, so memory remains bounded even for
    /// long terminal recordings that a capable terminal will animate itself.
    pub fn extract_gif_frame_delays(path: &Path) -> Result<Vec<u32>, ImageError> {
        let file = File::open(path).map_err(|_| ImageError::NotFound)?;
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::Indexed);
        let mut decoder = options
            .read_info(BufReader::new(file))
            .map_err(|error| ImageError::InvalidFormat(error.to_string()))?;
        let mut delays = Vec::new();

        while let Some(frame) = decoder
            .read_next_frame()
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

    /// Decode one composited GIF frame with bounded memory.
    ///
    /// Used only for deliberate pause/step actions during terminal-native
    /// playback; normal playback never expands all frames into memory.
    pub fn extract_gif_frame(
        path: &Path,
        target_index: usize,
    ) -> Result<image::DynamicImage, ImageError> {
        let file = File::open(path).map_err(|_| ImageError::NotFound)?;
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options
            .read_info(BufReader::new(file))
            .map_err(|error| ImageError::InvalidFormat(error.to_string()))?;
        let width = u32::from(decoder.width());
        let height = u32::from(decoder.height());
        let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
        let mut previous_disposal = gif::DisposalMethod::Keep;
        let mut previous_rect = (0, 0, 0, 0);
        let mut restore_previous: Option<RgbaImage> = None;
        let mut index = 0;

        while let Some(frame) = decoder
            .read_next_frame()
            .map_err(|error| ImageError::InvalidFormat(error.to_string()))?
        {
            match previous_disposal {
                gif::DisposalMethod::Background => {
                    Self::clear_gif_rect(&mut canvas, previous_rect);
                }
                gif::DisposalMethod::Previous => {
                    if let Some(previous) = restore_previous.take() {
                        canvas = previous;
                    }
                }
                gif::DisposalMethod::Any | gif::DisposalMethod::Keep => {}
            }

            let restore_for_this_frame =
                (frame.dispose == gif::DisposalMethod::Previous).then(|| canvas.clone());
            Self::composite_gif_frame(&mut canvas, frame, width, height);
            if index == target_index {
                return Ok(image::DynamicImage::ImageRgba8(canvas));
            }

            previous_disposal = frame.dispose;
            previous_rect = (
                frame.left.into(),
                frame.top.into(),
                frame.width.into(),
                frame.height.into(),
            );
            restore_previous = restore_for_this_frame;
            index += 1;
        }

        Err(ImageError::InvalidFormat(format!(
            "GIF frame {target_index} is out of range"
        )))
    }

    fn clear_gif_rect(canvas: &mut RgbaImage, rect: (u32, u32, u32, u32)) {
        let (left, top, width, height) = rect;
        for y in top..top.saturating_add(height).min(canvas.height()) {
            for x in left..left.saturating_add(width).min(canvas.width()) {
                canvas.put_pixel(x, y, Rgba([0, 0, 0, 0]));
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
                let mut previous_disposal = gif::DisposalMethod::Keep;
                let mut previous_rect = (0, 0, 0, 0);
                let mut restore_previous: Option<RgbaImage> = None;

                while let Ok(Some(frame)) = decoder.read_next_frame() {
                    // Disposal applies immediately before the following frame.
                    match previous_disposal {
                        gif::DisposalMethod::Background => {
                            Self::clear_gif_rect(&mut canvas, previous_rect);
                        }
                        gif::DisposalMethod::Previous => {
                            if let Some(previous) = restore_previous.take() {
                                canvas = previous;
                            }
                        }
                        gif::DisposalMethod::Any | gif::DisposalMethod::Keep => {}
                    }

                    let restore_for_this_frame =
                        (frame.dispose == gif::DisposalMethod::Previous).then(|| canvas.clone());

                    // Composite this frame onto the persistent canvas
                    Self::composite_gif_frame(&mut canvas, frame, width, height);

                    // Clone the current canvas state as this frame
                    let delay_ms = (frame.delay as u32) * 10;
                    frames.push(GifFrame {
                        image: image::DynamicImage::ImageRgba8(canvas.clone()),
                        delay_ms: delay_ms.max(20), // Min 20ms (50fps cap)
                    });

                    previous_disposal = frame.dispose;
                    previous_rect = (
                        frame.left.into(),
                        frame.top.into(),
                        frame.width.into(),
                        frame.height.into(),
                    );
                    restore_previous = restore_for_this_frame;
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
    fn streaming_gif_advances_and_loops_without_retaining_all_frames() {
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

        animation.last_frame_update = Instant::now() - Duration::from_millis(21);
        assert!(animation.time_until_next_frame().is_none());
        assert!(matches!(animation.advance(), GifAdvance::Waiting));

        animation.set_active(true);
        animation.last_frame_update = Instant::now() - Duration::from_millis(21);
        let GifAdvance::Frame(second) = animation.advance() else {
            panic!("second frame should be due");
        };
        assert_eq!(second.to_rgba8().get_pixel(0, 0), &Rgba([0, 255, 0, 255]));

        animation.last_frame_update = Instant::now() - Duration::from_millis(21);
        let GifAdvance::Frame(looped) = animation.advance() else {
            panic!("infinite GIF should loop to its first frame");
        };
        assert_eq!(looped.to_rgba8().get_pixel(0, 0), &Rgba([255, 0, 0, 255]));

        animation.set_active(false);
        animation.last_frame_update = Instant::now() - Duration::from_millis(21);
        assert!(animation.time_until_next_frame().is_none());
        assert!(matches!(animation.advance(), GifAdvance::Waiting));
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
        let second = ImageCache::extract_gif_frame(file.path(), 1).unwrap();
        assert_eq!(second.to_rgba8().get_pixel(0, 0), &Rgba([0, 255, 0, 255]));
    }
}
