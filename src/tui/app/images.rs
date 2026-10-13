//! Inline images, mermaid diagrams, and the image viewer.

use super::*;

/// Modal image viewer state.
#[derive(Default)]
pub struct ImageModalState {
    pub path: Option<std::path::PathBuf>,
    pub(crate) state: Option<AsyncProtocol>,
    /// Decoder for a multi-frame GIF, positioned after `stream_index`. One
    /// canvas rather than every frame, so a long recording stays small.
    pub(crate) stream: Option<StreamingGif>,
    pub(crate) stream_index: usize,
    /// A walk to a frame other than the next, running off the UI thread.
    pub(crate) seek: Option<std::sync::mpsc::Receiver<ModalSeek>>,
    /// Bumped by each seek and on close; a walk stops once it is stale.
    pub(crate) seek_generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// One entry per frame; a still image has one, of zero.
    pub frame_delays_ms: Vec<u32>,
    pub frame_index: usize,
    pub last_rendered_frame: Option<usize>,
    pub last_frame_update: Option<Instant>,
    pub animation_paused: bool,
}

/// A GIF decoder walked to a frame by [`App::modal_frame`]'s seek thread.
pub(crate) struct ModalSeek {
    stream: StreamingGif,
    index: usize,
    frame: image::DynamicImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItermAnimationSurface {
    Modal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItermAnimationTarget {
    path: PathBuf,
    surface: ItermAnimationSurface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ItermAnimationPlacement {
    target: ItermAnimationTarget,
    area: Rect,
}

impl App {
    /// Initialize graphics protocol picker with stdio protocol detection.
    ///
    /// Ratatui-image's stdio query is authoritative because terminal environment
    /// variables can be stale in tmux or remote sessions. Environment hints are
    /// only used when the query returns no capabilities at all.
    pub(super) fn init_picker() -> Option<ratatui_image::picker::Picker> {
        use ratatui_image::picker::Picker;

        // Before any picker is made: in tmux, every one of them, halfblocks
        // included, turns the pane's passthrough on.
        crate::tui::tty::remember_tmux_passthrough();

        // ratatui-image 11 reads the probe's answer on a thread it leaves
        // blocked on stdin when the terminal never answers, and from then on
        // that thread swallows every key, `q` included. Ask first with a
        // query crossterm times out without leaving a reader behind, and
        // keeps any keys it reads for the event loop. Fixed upstream in
        // ratatui-image 12, which polls instead.
        if crossterm::cursor::position().is_err() {
            return Some(Picker::halfblocks());
        }
        // Inside tmux that answer comes from tmux itself. The probe goes on
        // to the terminal tmux runs in, and a session with none attached (a
        // script, a detached session) never answers, with the same result.
        if Self::tmux_session_detached() {
            return Some(Picker::halfblocks());
        }

        let mut picker = match Picker::from_query_stdio() {
            Ok(picker) => {
                let font_size = picker.font_size();
                // Reject malformed or implausible terminal responses before
                // they can become enormous image allocations.
                if font_size.width < 4
                    || font_size.height < 4
                    || font_size.width > 128
                    || font_size.height > 256
                {
                    Picker::halfblocks()
                } else {
                    picker
                }
            }
            Err(_) => Picker::halfblocks(),
        };

        // iTerm2 3.6 answers the Kitty capability probe, but its Kitty
        // implementation is not compatible with ratatui-image's Unicode
        // placeholder renderer. ratatui-image's own compatibility matrix
        // specifies the iTerm2 protocol for iTerm, so prefer it over the
        // otherwise-authoritative probe result.
        //
        // Not inside a multiplexer, though: LC_TERMINAL survives ssh and
        // lingers in a tmux environment attached later from a different
        // terminal, and there the probe is what knows. Where the probe got
        // no answer, ratatui-image already falls back from the environment,
        // and keeps halfblocks when it cannot learn the font size, which is
        // what a protocol needs to place an image in its cells; guessing a
        // protocol here on top of that drew images outside them.
        if Self::is_iterm2_terminal() && !Self::in_multiplexer(Some(&picker)) {
            picker.set_protocol_type(ratatui_image::picker::ProtocolType::Iterm2);
        }

        Some(picker)
    }

    /// Whether this is a tmux session no terminal is attached to.
    pub(super) fn tmux_session_detached() -> bool {
        if std::env::var_os("TMUX").is_none() {
            return false;
        }
        std::process::Command::new("tmux")
            .args(["display-message", "-p", "#{session_attached}"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .is_ok_and(|output| output.status.success() && output.stdout.trim_ascii() == b"0")
    }

    pub(super) fn is_iterm2_terminal() -> bool {
        let term_program = std::env::var("TERM_PROGRAM").ok();
        let lc_terminal = std::env::var("LC_TERMINAL").ok();
        Self::is_iterm2_environment(term_program.as_deref(), lc_terminal.as_deref())
    }

    /// Whether treemd is running inside tmux, zellij or screen, where escape
    /// sequences are passed through, rewritten or dropped depending on how
    /// the multiplexer is configured.
    pub(super) fn in_multiplexer(picker: Option<&ratatui_image::picker::Picker>) -> bool {
        Self::is_multiplexer_environment(
            picker.is_some_and(ratatui_image::picker::Picker::tmux_detected),
            |name| std::env::var_os(name).is_some_and(|value| !value.is_empty()),
        )
    }

    pub(super) fn is_multiplexer_environment(
        tmux_detected: bool,
        is_set: impl Fn(&str) -> bool,
    ) -> bool {
        tmux_detected || ["TMUX", "ZELLIJ", "STY"].into_iter().any(is_set)
    }

    pub(super) fn is_iterm2_environment(
        term_program: Option<&str>,
        lc_terminal: Option<&str>,
    ) -> bool {
        term_program == Some("iTerm.app") || lc_terminal == Some("iTerm2")
    }

    pub(super) fn can_use_native_iterm_animation(&self) -> bool {
        self.images_enabled
            && !self.iterm_animation_failed
            && Self::is_iterm2_terminal()
            && !Self::in_multiplexer(self.picker.as_ref())
    }

    /// Populate the image protocol cache from indexed interactive elements.
    ///
    /// Queues any new images to be decoded on the image worker, keeping
    /// previously cached images so navigation back and forth doesn't
    /// re-decode. Decoded images arrive in [`Self::process_image_results`].
    /// Bounded by `IMAGE_CACHE_LIMIT`; least-recently-used entries are evicted.
    pub fn populate_image_cache(&mut self) {
        if self.picker.is_none() || !self.images_enabled {
            return;
        }

        // Unique image paths, in the order the section shows them.
        let mut seen = HashSet::new();
        let paths: Vec<PathBuf> = self
            .interactive_state
            .elements
            .iter()
            .filter_map(|elem| match &elem.element_type {
                ElementType::Image { src, .. } => self.resolve_image_path(src).ok(),
                _ => None,
            })
            .filter(|path| seen.insert(path.clone()))
            .collect();

        // Queued decodes for the section left behind are not wanted any
        // more; skipped, they leave the worker to this section's images.
        let section: HashSet<PathBuf> = paths.iter().cloned().collect();
        if section != self.image_section {
            if let Some(worker) = &self.image_worker {
                worker.supersede_decodes();
            }
            self.image_section = section;
        }

        for path in paths {
            self.request_image(path);
        }
    }

    /// Make sure `path` is cached as the file now on disk, or on its way.
    pub(super) fn request_image(&mut self, path: PathBuf) {
        let stamp = file_stamp(&path);
        if self.image_failures.get(&path) == Some(&stamp)
            || self.image_decoding.get(&path) == Some(&stamp)
        {
            return;
        }
        // Touch existing entries so index 0 remains the least recently used.
        if self.image_stamps.get(&path) == Some(&stamp)
            && let Some(protocol) = self.image_protocol_cache.shift_remove(&path)
        {
            self.image_protocol_cache.insert(path, protocol);
            return;
        }
        if let Some(worker) = &self.image_worker {
            worker.request_decode(path.clone(), stamp);
            self.image_decoding.insert(path, stamp);
        }
    }

    /// Cache an image the worker decoded, or note why it was not.
    pub(super) fn receive_decoded_image(&mut self, decoded: DecodedStill) {
        const IMAGE_CACHE_LIMIT: usize = 32;

        let DecodedStill { path, stamp, image } = decoded;
        // Asked for again since, for a newer file; that answer is to come.
        if self.image_decoding.get(&path) != Some(&stamp) {
            return;
        }
        self.image_decoding.remove(&path);

        let image = match image {
            // Skipped for a section that was left. Ask again if this one
            // shows it too.
            None => {
                if self.image_section.contains(&path) {
                    self.request_image(path);
                }
                return;
            }
            Some(Err(_)) => {
                self.image_failures.insert(path, stamp);
                return;
            }
            Some(Ok(image)) => image,
        };
        self.image_failures.remove(&path);
        let (Some(picker), Some(worker)) = (self.picker.as_mut(), &self.image_worker) else {
            return;
        };
        let protocol = worker.wrap(picker.new_resize_protocol(image));
        self.image_protocol_cache.shift_remove(&path);
        self.image_protocol_cache.insert(path.clone(), protocol);
        self.image_stamps.insert(path, stamp);

        // Evict the least recently used entry when over capacity, but never
        // one this section shows. The oldest entry used to go whatever it
        // was, so a section with more than the limit evicted its own first
        // images as it decoded the last, and they were never drawn.
        while self.image_protocol_cache.len() > IMAGE_CACHE_LIMIT {
            let Some(oldest) = self
                .image_protocol_cache
                .keys()
                .position(|cached| !self.image_section.contains(cached))
            else {
                break;
            };
            if let Some((evicted, _)) = self.image_protocol_cache.shift_remove_index(oldest) {
                self.image_stamps.remove(&evicted);
            }
        }
    }

    /// Synchronize native GIF playback with the open image viewer.
    ///
    /// Merely selecting an embedded GIF is intentionally a first-frame still.
    /// Playback begins only after the user opens the image viewer.
    pub(crate) fn sync_native_animation_activation(&mut self) -> bool {
        // iTerm2 animates the original GIF without clearing between frames.
        // Avoid this direct-output path inside ordinary tmux sessions, where
        // passthrough wrapping varies by configuration.
        let use_native_iterm = self.can_use_native_iterm_animation();

        // The whole file is written to the terminal on the UI thread, and
        // again whenever the image moves. Past this size that is a freeze,
        // over ssh especially, so a larger GIF plays through the streaming
        // decoder like any other.
        const MAX_NATIVE_GIF_BYTES: u64 = 16 * 1024 * 1024;
        let native_target = if use_native_iterm
            && self.is_image_modal_open()
            && !self.image_modal.animation_paused
            && self.image_modal_frame_count() > 1
            && self
                .image_modal
                .path
                .as_ref()
                .and_then(|path| std::fs::metadata(path).ok())
                .is_some_and(|meta| meta.len() <= MAX_NATIVE_GIF_BYTES)
        {
            self.image_modal
                .path
                .as_ref()
                .map(|path| ItermAnimationTarget {
                    path: path.clone(),
                    surface: ItermAnimationSurface::Modal,
                })
        } else {
            None
        };

        let target_changed = self.iterm_animation_target != native_target;
        if target_changed {
            self.iterm_animation_target = native_target;
            self.iterm_animation_staged = None;
            self.iterm_animation_sent = None;
        }

        target_changed
    }

    /// Reset layout information before a Ratatui frame computes image areas.
    pub(crate) fn prepare_iterm_animation_frame(&mut self) {
        self.iterm_animation_staged = None;
    }

    /// Record the exact cell rectangle where a native GIF should be anchored.
    pub(crate) fn stage_iterm_animation(
        &mut self,
        path: &std::path::Path,
        surface: ItermAnimationSurface,
        area: Rect,
    ) {
        let Some(target) = self.iterm_animation_target.as_ref() else {
            return;
        };
        if target.path == path && target.surface == surface && !area.is_empty() {
            self.iterm_animation_staged = Some(ItermAnimationPlacement {
                target: target.clone(),
                area,
            });
        }
    }

    /// Transfer a staged GIF after Ratatui has flushed its frame.
    pub(crate) fn transmit_staged_iterm_animation<W: Write>(
        &mut self,
        writer: &mut W,
    ) -> io::Result<bool> {
        let Some(placement) = self.iterm_animation_staged.clone() else {
            return Ok(false);
        };
        if self.iterm_animation_sent.as_ref() == Some(&placement) {
            return Ok(false);
        }

        iterm_animation::transmit_gif(writer, &placement.target.path, placement.area)?;
        self.iterm_animation_sent = Some(placement);
        // Native playback begins at the first frame when FileEnd is received.
        // Start the lightweight UI counter from the same point.
        self.image_modal.frame_index = 0;
        self.image_modal.last_frame_update = Some(Instant::now());
        Ok(true)
    }

    pub(crate) fn disable_iterm_animation(&mut self, error: &io::Error) {
        self.iterm_animation_failed = true;
        self.iterm_animation_target = None;
        self.iterm_animation_staged = None;
        self.iterm_animation_sent = None;
        self.image_modal.animation_paused = true;
        self.image_modal.frame_index = 0;
        self.status_message = Some(format!(
            "⚠ Native iTerm GIF playback failed; showing a still frame: {error}"
        ));
        self.status_message_time = None;
    }

    pub(crate) fn has_native_iterm_modal_animation(&self) -> bool {
        self.iterm_animation_target
            .as_ref()
            .is_some_and(|target| target.surface == ItermAnimationSurface::Modal)
    }

    pub(crate) fn image_modal_frame_count(&self) -> usize {
        self.image_modal.frame_delays_ms.len()
    }

    pub(super) fn image_modal_frame_delay_ms(&self, index: usize) -> Option<u32> {
        self.image_modal.frame_delays_ms.get(index).copied()
    }

    /// Render a mermaid diagram and cache the result as a StatefulProtocol.
    /// Returns true if a cached protocol is available (either fresh or from prior render).
    #[cfg(all(feature = "mermaid", unix))]
    pub fn render_mermaid_if_needed(&mut self, source: &str, width: u16) -> bool {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = hasher.finish();

        // Use picker's actual font width for accurate pixel calculation
        let font_width = self.picker.as_ref().map_or(8u16, |p| {
            let font_size = p.font_size();
            if font_size.width < 1 {
                8
            } else {
                font_size.width
            }
        });
        let target_px = (width as u32) * (font_width as u32);

        // Clear caches on render width change (re-rasterize at new size)
        if target_px != self.mermaid_last_render_width {
            self.mermaid_protocol_cache.clear();
            self.mermaid_render_errors.clear();
            // Anything still queued was for the old width, and its result is
            // dropped when it arrives.
            self.mermaid_pending.clear();
            self.mermaid_image_dims.clear();
            self.mermaid_placeholder_rows.clear();
            self.mermaid_needs_reindex = false;
            self.mermaid_last_render_width = target_px;
        }

        // Already cached (success or failure)
        if self.mermaid_protocol_cache.contains_key(&hash) {
            return true;
        }
        if self.mermaid_render_errors.contains_key(&hash)
            || self.mermaid_pending.contains(&hash)
            || self.picker.is_none()
            || self.image_worker.is_none()
        {
            return false;
        }

        // Laid out on the worker; the frame shows a placeholder until
        // process_mermaid_results picks the image up.
        self.mermaid_worker
            .get_or_insert_with(crate::tui::mermaid::MermaidWorker::new)
            .request(hash, source, target_px);
        self.mermaid_pending.insert(hash);
        false
    }

    /// Take in diagrams the worker has finished. Returns whether any arrived,
    /// so the frame is redrawn with them.
    #[cfg(all(feature = "mermaid", unix))]
    pub(super) fn process_mermaid_results(&mut self) -> bool {
        let Some(results) = self.mermaid_worker.as_ref().map(|w| w.drain()) else {
            return false;
        };
        let arrived = !results.is_empty();

        for result in results {
            // Drawn for a width the pane no longer has; the current width's
            // request is already queued behind it.
            if result.width != self.mermaid_last_render_width
                || !self.mermaid_pending.remove(&result.hash)
            {
                continue;
            }
            let img = match result.image {
                Ok(img) => img,
                Err(e) => {
                    self.mermaid_render_errors.insert(result.hash, e);
                    continue;
                }
            };
            let (Some(picker), Some(worker)) = (self.picker.as_mut(), &self.image_worker) else {
                continue;
            };
            let dims = (img.width(), img.height());
            let font_h = match picker.font_size().height {
                0 => 16u32,
                h => u32::from(h),
            };
            let rows = dims.1.div_ceil(font_h) as usize;
            let protocol = picker.new_resize_protocol(img);
            self.mermaid_protocol_cache
                .insert(result.hash, worker.wrap(protocol));
            self.mermaid_image_dims.insert(result.hash, dims);
            self.mermaid_placeholder_rows
                .insert(result.hash, rows.max(1));
            self.mermaid_needs_reindex = true;

            // Up to 64 MB each, and kept for the whole session at one
            // width, so bounded like the image cache: the oldest go first,
            // and a diagram still on screen is drawn again if it was one.
            const MERMAID_CACHE_LIMIT: usize = 32;
            while self.mermaid_protocol_cache.len() > MERMAID_CACHE_LIMIT {
                if let Some((evicted, _)) = self.mermaid_protocol_cache.shift_remove_index(0) {
                    self.mermaid_image_dims.remove(&evicted);
                    self.mermaid_placeholder_rows.remove(&evicted);
                }
            }
        }
        arrived
    }

    /// Re-index interactive elements using pixel-accurate placeholder rows if new dims arrived.
    ///
    /// Called once per frame (before rendering). On the frame after a mermaid diagram is first
    /// rendered, this replaces the heuristic placeholder sizes with the real image heights so
    /// the blank-line reservation matches the displayed image.
    #[cfg(all(feature = "mermaid", unix))]
    pub fn reindex_mermaid_if_needed(&mut self) {
        if !self.mermaid_needs_reindex {
            return;
        }
        self.mermaid_needs_reindex = false;
        let content_text = self.displayed_section_content().to_string();
        use crate::parser::content::parse_content;
        let blocks = parse_content(&content_text, 0);
        let rows = self.mermaid_placeholder_rows.clone();
        self.interactive_state.index_elements(&blocks, &rows);
    }

    /// Get the hash for a mermaid source string.
    #[cfg(all(feature = "mermaid", unix))]
    pub fn mermaid_source_hash(source: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    /// Open image modal for a given image path
    pub fn open_image_modal(&mut self, image_src: &str) {
        // Skip if images are disabled
        if !self.images_enabled {
            return;
        }

        use crate::tui::image_cache::ImageCache;

        let Ok(path) = self.resolve_image_path(image_src) else {
            return;
        };

        // A GIF keeps its delays and a streaming decoder, never every frame:
        // a screen recording expands to gigabytes of RGBA. Frames are decoded
        // as playback reaches them, and iTerm2 is sent the original file.
        let delays = ImageCache::extract_gif_frame_delays(&path)
            .ok()
            .filter(|delays| delays.len() > 1);
        let (first_frame, stream, delays) = match delays {
            Some(delays) => match StreamingGif::open(&path) {
                Ok((stream, first)) => (first, Some(stream), delays),
                Err(_) => return,
            },
            None => match ImageCache::extract_first_frame(&path) {
                Ok(still) => (still, None, vec![0]),
                Err(_) => return,
            },
        };

        self.cancel_modal_seek();
        let (Some(picker), Some(worker)) = (&mut self.picker, &self.image_worker) else {
            return;
        };
        let initial_protocol = picker.new_resize_protocol(first_frame);
        self.image_modal.path = Some(path);
        self.image_modal.state = Some(worker.wrap(initial_protocol));
        self.image_modal.stream = stream;
        self.image_modal.stream_index = 0;
        self.image_modal.frame_delays_ms = delays;
        self.image_modal.frame_index = 0;
        self.image_modal.last_rendered_frame = Some(0);
        self.image_modal.last_frame_update = Some(Instant::now());
    }

    /// Close the image modal
    pub fn close_image_modal(&mut self) {
        self.cancel_modal_seek();
        self.image_modal.path = None;
        self.image_modal.state = None;
        self.image_modal.stream = None;
        self.image_modal.stream_index = 0;
        self.image_modal.frame_delays_ms.clear();
        self.image_modal.frame_index = 0;
        self.image_modal.last_rendered_frame = None;
        self.image_modal.last_frame_update = None;
        self.image_modal.animation_paused = false;
    }

    /// Go to previous frame in GIF animation
    pub fn modal_prev_frame(&mut self) {
        let len = self.image_modal_frame_count();
        if len == 0 {
            return;
        }
        // Pause animation when manually stepping
        self.image_modal.animation_paused = true;
        self.image_modal.frame_index = (self.image_modal.frame_index + len - 1) % len;
        self.replace_current_modal_frame();
    }

    /// Go to next frame in GIF animation
    pub fn modal_next_frame(&mut self) {
        let len = self.image_modal_frame_count();
        if len == 0 {
            return;
        }
        // Pause animation when manually stepping
        self.image_modal.animation_paused = true;
        self.image_modal.frame_index = (self.image_modal.frame_index + 1) % len;
        self.replace_current_modal_frame();
    }

    /// Toggle animation play/pause
    pub fn modal_toggle_animation(&mut self) {
        let was_native_iterm = self.has_native_iterm_modal_animation();
        self.image_modal.animation_paused = !self.image_modal.animation_paused;

        if self.image_modal.animation_paused && was_native_iterm {
            // The terminal-owned animation is cleared on the next update.
            // Prepare the corresponding still frame for paused display.
            self.replace_current_modal_frame();
        } else if !self.image_modal.animation_paused {
            // Reset the timer when resuming
            self.image_modal.last_frame_update = Some(Instant::now());
        }
    }

    /// Check if image modal is open
    pub fn is_image_modal_open(&self) -> bool {
        self.image_modal.path.is_some()
    }

    /// The modal's frame at `frame_index`, from its streaming decoder.
    ///
    /// Playback asks for the frame after the last one, which is one decode.
    /// Any other frame means walking the GIF from its start, which for a long
    /// recording took tens of seconds on the UI thread, with every key
    /// waiting behind it. That walk runs on its own thread instead, and
    /// returns `None` here; [`Self::process_modal_seek`] shows the frame when
    /// it arrives, and playback carries on from there one frame at a time.
    pub(super) fn modal_frame(&mut self) -> Option<image::DynamicImage> {
        use std::sync::atomic::Ordering;

        let modal = &mut self.image_modal;
        let target = modal.frame_index;
        let len = modal.frame_delays_ms.len().max(1);
        if let Some(stream) = modal.stream.as_mut()
            && target == (modal.stream_index + 1) % len
        {
            let frame = stream.next_frame_looping().ok()?;
            modal.stream_index = target;
            return Some(frame);
        }

        let path = modal.path.clone()?;
        let generation = modal.seek_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let current = std::sync::Arc::clone(&modal.seek_generation);
        let (sender, receiver) = std::sync::mpsc::channel();
        modal.seek = Some(receiver);
        modal.stream = None;
        std::thread::spawn(move || {
            let _ = crate::tui::tty::catch_isolated(|| -> Option<()> {
                let (mut stream, mut frame) = StreamingGif::open(&path).ok()?;
                for _ in 0..target {
                    if current.load(Ordering::Relaxed) != generation {
                        return None;
                    }
                    frame = stream.next_frame_looping().ok()?;
                }
                sender
                    .send(ModalSeek {
                        stream,
                        index: target,
                        frame,
                    })
                    .ok()
            });
        });
        None
    }

    /// Stop any walk in progress; its frame is no longer wanted.
    pub(super) fn cancel_modal_seek(&mut self) {
        self.image_modal
            .seek_generation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.image_modal.seek = None;
    }

    /// Show the frame a seek walked to, once it arrives. Returns whether one
    /// did.
    pub(super) fn process_modal_seek(&mut self) -> bool {
        use std::sync::mpsc::TryRecvError;

        let Some(receiver) = &self.image_modal.seek else {
            return false;
        };
        let seek = match receiver.try_recv() {
            Ok(seek) => seek,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                self.image_modal.seek = None;
                return false;
            }
        };
        self.image_modal.seek = None;
        self.image_modal.stream = Some(seek.stream);
        self.image_modal.stream_index = seek.index;
        self.show_modal_frame(seek.frame, seek.index);
        true
    }

    pub(super) fn replace_current_modal_frame(&mut self) {
        let Some(image) = self.modal_frame() else {
            return;
        };
        self.show_modal_frame(image, self.image_modal.frame_index);
    }

    pub(super) fn show_modal_frame(&mut self, image: image::DynamicImage, index: usize) {
        let Some(picker) = self.picker.as_ref() else {
            return;
        };
        let replacement = picker.new_resize_protocol(image);
        if let Some(state) = self.image_modal.state.as_mut() {
            state.replace_protocol(replacement);
            self.image_modal.last_rendered_frame = Some(index);
        }
    }

    /// Advance modal GIF state in the update loop, never from the view.
    pub(crate) fn advance_modal_animation(&mut self) -> bool {
        if !self.is_image_modal_open()
            || self.image_modal.animation_paused
            || self.image_modal_frame_count() <= 1
            || self.image_modal.seek.is_some()
        {
            return false;
        }

        let native_iterm = self.has_native_iterm_modal_animation();
        if !native_iterm {
            let Some(state) = self.image_modal.state.as_ref() else {
                return false;
            };
            if state.has_in_flight_work() || !state.has_completed_frame() {
                return false;
            }
        }

        let Some(last_update) = self.image_modal.last_frame_update else {
            return false;
        };
        let Some(delay_ms) = self.image_modal_frame_delay_ms(self.image_modal.frame_index) else {
            return false;
        };
        let delay = Duration::from_millis(delay_ms.into());
        if last_update.elapsed() < delay {
            return false;
        }

        self.image_modal.frame_index =
            (self.image_modal.frame_index + 1) % self.image_modal_frame_count();
        self.image_modal.last_frame_update = Some(Instant::now());
        if !native_iterm {
            self.replace_current_modal_frame();
        }
        true
    }

    /// Apply completed resize/encode work to its originating image state.
    ///
    /// Results may arrive after a resize or GIF frame has superseded them;
    /// `AsyncProtocol` generation checks discard those stale completions.
    pub fn process_image_results(&mut self) -> bool {
        #[cfg(all(feature = "mermaid", unix))]
        let diagrams_arrived = self.process_mermaid_results();
        #[cfg(not(all(feature = "mermaid", unix)))]
        let diagrams_arrived = false;
        let diagrams_arrived = self.process_modal_seek() | diagrams_arrived;

        let Some(worker) = &self.image_worker else {
            return diagrams_arrived;
        };
        let decoded = worker.drain_decoded();
        let images_arrived = !decoded.is_empty();
        for still in decoded {
            self.receive_decoded_image(still);
        }

        let Some(worker) = &self.image_worker else {
            return diagrams_arrived;
        };
        let results = worker.drain();
        let had_results = diagrams_arrived || images_arrived || !results.is_empty();

        for result in results {
            let mut result = Some(result);
            let mut error = None;

            if self
                .image_modal
                .state
                .as_ref()
                .is_some_and(|state| state.matches(result.as_ref().expect("result is present")))
            {
                error = self
                    .image_modal
                    .state
                    .as_mut()
                    .and_then(|state| state.update(result.take().expect("result is present")));
            }

            if result.is_some()
                && let Some(state) = self
                    .image_protocol_cache
                    .values_mut()
                    .find(|state| state.matches(result.as_ref().expect("result is present")))
            {
                error = state.update(result.take().expect("result is present"));
            }

            #[cfg(all(feature = "mermaid", unix))]
            if result.is_some()
                && let Some(state) = self
                    .mermaid_protocol_cache
                    .values_mut()
                    .find(|state| state.matches(result.as_ref().expect("result is present")))
            {
                error = state.update(result.take().expect("result is present"));
            }

            if let Some(error) = error {
                self.status_message = Some(format!("✗ Image encoding failed: {error}"));
                self.status_message_time = None;
            }
        }

        had_results
    }

    pub fn has_pending_image_work(&self) -> bool {
        if self.image_modal.seek.is_some() {
            return true;
        }
        #[cfg(all(feature = "mermaid", unix))]
        if !self.mermaid_pending.is_empty() {
            return true;
        }
        self.image_worker
            .as_ref()
            .is_some_and(ImageWorker::has_pending)
    }

    /// Get time until next GIF frame should be displayed.
    /// Returns None if not animating, Some(Duration) otherwise.
    /// Used by the event loop to optimize poll timeout for smooth animation.
    pub fn time_until_next_frame(&self) -> Option<std::time::Duration> {
        // iTerm2 handles pixels internally, but the UI still follows the GIF's
        // frame delays so its progress counter remains useful.
        if !self.is_image_modal_open()
            || self.image_modal_frame_count() <= 1
            || self.image_modal.animation_paused
        {
            return None;
        }
        self.image_modal.last_frame_update.and_then(|last_update| {
            let frame_delay = std::time::Duration::from_millis(u64::from(
                self.image_modal_frame_delay_ms(self.image_modal.frame_index)?,
            ));
            Some(
                frame_delay
                    .saturating_sub(last_update.elapsed())
                    .max(std::time::Duration::from_millis(1)),
            )
        })
    }

    /// Resolve an image path relative to the current markdown file.
    ///
    /// Supports both relative and absolute paths:
    /// - Relative paths are resolved against the current file's directory
    /// - Absolute paths are returned as-is
    ///
    /// A document fetched from the web loads no local images.
    ///
    /// # Examples
    ///
    /// If current file is `/docs/file.md`:
    /// - `./images/photo.png` → `/docs/images/photo.png`
    /// - `../assets/logo.png` → `/assets/logo.png`
    /// - `/etc/hosts` → `/etc/hosts`
    pub fn resolve_image_path(&self, src: &str) -> Result<std::path::PathBuf, String> {
        // A document fetched from the web names its images relative to where
        // it came from, not to this machine. Resolved locally, a remote page
        // could display any image file on disk by its absolute path.
        if self.is_remote_document() {
            return Err(
                "Images in a fetched document are not loaded from this machine".to_string(),
            );
        }
        let path = std::path::Path::new(src);

        // A piped document may come from anywhere (`curl ... | treemd`), so
        // unless the config trusts it, its images are held to the working
        // directory the way its links are. It could otherwise show any
        // image on disk by absolute path.
        if self.is_untrusted_piped_document() {
            if path.has_root() {
                return Err(
                    "A piped document loads images from the working directory only; \
                     set security.trust_piped_documents to allow absolute paths"
                        .to_string(),
                );
            }
            return self.confine_link_path(path);
        }

        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        // Resolve relative to markdown file's directory
        let base_dir = self
            .current_file_path
            .parent()
            .ok_or_else(|| "No parent directory for current file".to_string())?;

        Ok(base_dir.join(src))
    }
}

#[cfg(test)]
mod image_picker_tests {
    use super::*;

    /// tmux, zellij and screen each make the iTerm2 override and native
    /// iTerm playback unsafe.
    #[test]
    fn multiplexers_are_recognized() {
        let only = |set: &'static str| move |name: &str| name == set;
        assert!(App::is_multiplexer_environment(true, |_| false));
        assert!(App::is_multiplexer_environment(false, only("TMUX")));
        assert!(App::is_multiplexer_environment(false, only("ZELLIJ")));
        assert!(App::is_multiplexer_environment(false, only("STY")));
        assert!(!App::is_multiplexer_environment(
            false,
            only("KITTY_WINDOW_ID")
        ));
    }

    #[test]
    fn iterm_environment_overrides_partial_kitty_capability() {
        assert!(App::is_iterm2_environment(Some("iTerm.app"), None));
        assert!(App::is_iterm2_environment(None, Some("iTerm2")));
        assert!(!App::is_iterm2_environment(Some("kitty"), None));
    }
}

#[cfg(test)]
mod image_modal_tests {
    use super::*;

    /// The cache held 32 images and evicted the oldest as it decoded each
    /// new one, so in a section with more, the first ones were evicted by
    /// the last and never drawn.
    #[test]
    fn a_section_with_many_images_keeps_them_all() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let mut markdown = String::from("# Gallery\n\n");
        for i in 0..40 {
            image::RgbaImage::from_pixel(2, 2, image::Rgba([i, 0, 0, 255]))
                .save(root.join(format!("{i}.png")))
                .unwrap();
            markdown.push_str(&format!("![{i}]({i}.png)\n\n"));
        }
        let path = root.join("gallery.md");
        std::fs::write(&path, &markdown).unwrap();

        let mut app = App::new(
            crate::parser::parse_markdown(&markdown),
            "gallery.md".into(),
            path,
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        app.picker = Some(ratatui_image::picker::Picker::halfblocks());
        app.image_worker = Some(ImageWorker::new());
        app.images_enabled = true;
        app.select_outline_index(0);
        app.enter_interactive_mode();
        settle_images(&mut app);

        assert_eq!(app.image_protocol_cache.len(), 40);
    }

    /// Wait for the image worker to finish what it was asked to decode.
    fn settle_images(app: &mut App) {
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        while !app.image_decoding.is_empty() {
            assert!(Instant::now() < deadline, "images never decoded");
            app.process_image_results();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Images are decoded on the image worker, so entering a section with
    /// images returns before any is read; they arrive with the results. A
    /// file that is not an image is remembered as failed.
    #[test]
    fn images_decode_off_the_ui_thread() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]))
            .save(root.join("ok.png"))
            .unwrap();
        std::fs::write(root.join("bad.png"), "not a png").unwrap();
        let markdown = "# A\n\n![ok](ok.png)\n\n![bad](bad.png)\n\n# B\n\ntext\n";
        let path = root.join("doc.md");
        std::fs::write(&path, markdown).unwrap();

        let mut app = App::new(
            crate::parser::parse_markdown(markdown),
            "doc.md".into(),
            path,
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        app.picker = Some(ratatui_image::picker::Picker::halfblocks());
        app.image_worker = Some(ImageWorker::new());
        app.images_enabled = true;
        app.select_outline_index(0);
        app.enter_interactive_mode();

        assert_eq!(app.image_decoding.len(), 2);
        assert!(app.has_pending_image_work());
        settle_images(&mut app);
        assert!(app.image_protocol_cache.contains_key(&root.join("ok.png")));
        assert!(app.image_failures.contains_key(&root.join("bad.png")));

        // Asked again, neither is decoded twice.
        app.populate_image_cache();
        assert!(app.image_decoding.is_empty());
    }

    /// A three-frame GIF whose frames are red, green and blue.
    fn three_frame_gif() -> tempfile::NamedTempFile {
        use std::borrow::Cow;

        let mut file = tempfile::NamedTempFile::new().expect("temporary gif");
        let palette = [255, 0, 0, 0, 255, 0, 0, 0, 255];
        let mut encoder =
            gif::Encoder::new(file.as_file_mut(), 1, 1, &palette).expect("gif encoder");
        for color in 0..3u8 {
            let frame = gif::Frame {
                width: 1,
                height: 1,
                delay: 2,
                buffer: Cow::Owned(vec![color]),
                ..gif::Frame::default()
            };
            encoder.write_frame(&frame).expect("frame");
        }
        drop(encoder);
        file
    }

    fn red(image: &image::DynamicImage) -> u8 {
        image.to_rgba8().get_pixel(0, 0).0[0]
    }
    fn green(image: &image::DynamicImage) -> u8 {
        image.to_rgba8().get_pixel(0, 0).0[1]
    }
    fn blue(image: &image::DynamicImage) -> u8 {
        image.to_rgba8().get_pixel(0, 0).0[2]
    }

    /// The viewer streams a GIF rather than holding every frame, so each
    /// frame it shows has to be the one its index names, whether playback got
    /// there by advancing, by stepping back, or by wrapping past the end.
    #[test]
    fn the_modal_frame_follows_its_index_in_any_order() {
        let gif = three_frame_gif();
        let doc = crate::parser::parse_markdown("# t\n");
        let mut app = App::new(
            doc,
            "t.md".into(),
            gif.path().with_file_name("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        let (stream, _) = StreamingGif::open(gif.path()).expect("stream");
        app.image_modal.path = Some(gif.path().to_path_buf());
        app.image_modal.stream = Some(stream);
        app.image_modal.stream_index = 0;
        app.image_modal.frame_delays_ms = vec![20, 20, 20];

        let mut show = |index: usize| {
            app.image_modal.frame_index = index;
            if let Some(frame) = app.modal_frame() {
                return frame;
            }
            // Anything but the next frame is walked to off the UI thread.
            let seek = app
                .image_modal
                .seek
                .take()
                .expect("a seek")
                .recv()
                .expect("frame");
            app.image_modal.stream = Some(seek.stream);
            app.image_modal.stream_index = seek.index;
            seek.frame
        };
        assert_eq!(green(&show(1)), 255, "advance");
        assert_eq!(blue(&show(2)), 255, "advance");
        assert_eq!(red(&show(0)), 255, "wrap past the end");
        assert_eq!(green(&show(1)), 255, "advance after wrapping");
        assert_eq!(red(&show(0)), 255, "step back");
        assert_eq!(blue(&show(2)), 255, "jump forward");
        assert_eq!(blue(&show(2)), 255, "the same frame again");
    }

    /// The viewer's keys were wired into the event loop past the keymap, so
    /// they could not be rebound and Ctrl+C did nothing there. They are a
    /// keybinding mode like any other now.
    #[test]
    fn the_image_viewer_is_a_keybinding_mode() {
        use crossterm::event::{KeyCode, KeyModifiers};

        let gif = three_frame_gif();
        let mut app = App::new(
            crate::parser::parse_markdown("# t\n"),
            "t.md".into(),
            gif.path().with_file_name("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        app.image_modal.path = Some(gif.path().to_path_buf());
        app.image_modal.frame_delays_ms = vec![20, 20, 20];
        assert_eq!(app.current_keybinding_mode(), KeybindingMode::ImageViewer);

        let space = app.get_action_for_key(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(space, Some(Action::TogglePlayback));
        app.execute_action(Action::TogglePlayback);
        assert!(app.image_modal.animation_paused);

        // Typed text does not reach a search behind the viewer.
        assert!(!app.apply_text_input_edit(TextInputEdit::Insert('x')));

        let ctrl_c = app.get_action_for_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(matches!(
            app.execute_action(ctrl_c.unwrap()),
            ActionResult::Continue
        ));
        assert!(!app.is_image_modal_open());
    }
}
