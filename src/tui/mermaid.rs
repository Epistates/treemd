use image::DynamicImage;
use std::sync::Arc;
use std::sync::OnceLock;

/// Largest raster a diagram is drawn at, in pixels. Scaling to the pane's
/// width alone has no bound: a fifty-node chain is 2000x77000 at a wide pane,
/// over a gigabyte of RGBA, and the raster is redrawn on every width change.
/// 16 MP is a 4000x4000 square, well past what a terminal pane shows.
const MAX_RASTER_PIXELS: f32 = 16_000_000.0;

/// How far a small diagram is enlarged to fill the pane. Past this, it only
/// costs pixels without adding detail.
const MAX_UPSCALE: f32 = 2.0;

/// Largest diagram source treemd will lay out. Layout cost grows much faster
/// than the source: a few kilobytes of densely connected nodes takes minutes.
/// Diagrams run on [`MermaidWorker`], so that no longer freezes the UI, but
/// past this size it is not worth the CPU.
const MAX_SOURCE_BYTES: usize = 32 * 1024;

/// A finished diagram, for the pane width it was drawn at.
pub struct MermaidResult {
    pub hash: u64,
    pub width: u32,
    pub image: Result<DynamicImage, String>,
}

struct MermaidJob {
    hash: u64,
    width: u32,
    source: String,
}

/// Lays out and rasterizes diagrams on a thread of its own.
///
/// Layout cannot be interrupted, and a dense diagram of a few kilobytes
/// takes minutes, so it ran inside the frame and froze the UI with raw mode
/// on, from one block in a README. Off the UI thread, the worst case is one
/// diagram that never finishes while the rest of treemd carries on.
pub struct MermaidWorker {
    jobs: std::sync::mpsc::Sender<MermaidJob>,
    results: std::sync::mpsc::Receiver<MermaidResult>,
}

impl MermaidWorker {
    pub fn new() -> Self {
        let (jobs, job_rx) = std::sync::mpsc::channel::<MermaidJob>();
        let (result_tx, results) = std::sync::mpsc::channel();
        // The thread ends when the worker, and with it the job sender, drops.
        let _ = std::thread::Builder::new()
            .name("treemd-mermaid".to_string())
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    let image = render_mermaid_to_image(&job.source, job.width);
                    let result = MermaidResult {
                        hash: job.hash,
                        width: job.width,
                        image,
                    };
                    if result_tx.send(result).is_err() {
                        break;
                    }
                }
            });
        Self { jobs, results }
    }

    /// Queue a diagram, identified by `hash`, for drawing at `width` pixels.
    pub fn request(&self, hash: u64, source: &str, width: u32) {
        let _ = self.jobs.send(MermaidJob {
            hash,
            width,
            source: source.to_string(),
        });
    }

    /// Every diagram finished since the last call.
    pub fn drain(&self) -> Vec<MermaidResult> {
        self.results.try_iter().collect()
    }
}

/// Render mermaid source text to a raster image.
///
/// Pipeline: mermaid source → SVG (via mermaid-rs-renderer) → raster (via resvg)
///
/// Both stages are third-party code that can panic on input it did not
/// expect, so a panic becomes an error for this one diagram.
pub fn render_mermaid_to_image(source: &str, target_width: u32) -> Result<DynamicImage, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "diagram is {} KB, over the {} KB limit",
            source.len() / 1024,
            MAX_SOURCE_BYTES / 1024
        ));
    }
    crate::tui::tty::catch_isolated(|| {
        let svg = render_to_svg(source)?;
        rasterize_svg(&svg, target_width)
    })
    .unwrap_or_else(|_| Err("mermaid renderer panicked".to_string()))
}

/// Generate SVG from mermaid diagram source.
fn render_to_svg(source: &str) -> Result<String, String> {
    let opts = mermaid_rs_renderer::RenderOptions::default();
    mermaid_rs_renderer::render_with_options(source, opts)
        .map(|svg| fix_svg_font_families(&svg))
        .map_err(|e| format!("mermaid render error: {e}"))
}

/// Fix malformed font-family attributes produced by mermaid-rs-renderer.
///
/// The renderer emits unescaped inner quotes like:
///   font-family="Inter, "Segoe UI", sans-serif"
/// This replaces inner `"` with `'` so the XML is valid:
///   font-family="Inter, 'Segoe UI', sans-serif"
fn fix_svg_font_families(svg: &str) -> String {
    let marker = "font-family=\"";
    let mut result = String::with_capacity(svg.len());
    let mut remaining = svg;

    while let Some(start) = remaining.find(marker) {
        result.push_str(&remaining[..start + marker.len()]);
        remaining = &remaining[start + marker.len()..];

        // Process the attribute value character by character, only as far
        // as its closing quote. Collecting the rest of the document into a
        // Vec for every attribute made this quadratic in the SVG's size.
        let mut found_close = false;
        let mut chars = remaining.char_indices().peekable();

        while let Some((at, c)) = chars.next() {
            if c == '"' {
                let next = chars.peek().map_or('>', |&(_, next)| next);
                if next == '>' || next == ' ' || next == '/' {
                    // Real closing quote
                    result.push('"');
                    // Advance remaining past this quote
                    remaining = &remaining[at + 1..];
                    found_close = true;
                    break;
                }
                // Inner quote, replaced with a single quote
                result.push('\'');
            } else {
                result.push(c);
            }
        }

        if !found_close {
            // No closing quote found, consume rest
            remaining = "";
        }
    }

    result.push_str(remaining);
    result
}

/// Cached system font database for resvg.
fn font_database() -> Arc<resvg::usvg::fontdb::Database> {
    static DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        Arc::new(db)
    })
    .clone()
}

/// The scale that fits a `w`x`h` diagram to `target_width`, bounded by
/// [`MAX_UPSCALE`] and by [`MAX_RASTER_PIXELS`] of area. A diagram that would
/// exceed the area is drawn narrower than the pane rather than not at all.
fn raster_scale(w: f32, h: f32, target_width: u32) -> f32 {
    if !(w > 0.0 && h > 0.0) {
        return 1.0;
    }
    let mut scale = (target_width as f32 / w).min(MAX_UPSCALE);
    let area = w * h * scale * scale;
    if area > MAX_RASTER_PIXELS {
        scale *= (MAX_RASTER_PIXELS / area).sqrt();
    }
    scale
}

/// Rasterize an SVG string to a pixel image at the given target width.
fn rasterize_svg(svg: &str, target_width: u32) -> Result<DynamicImage, String> {
    let db = font_database();

    let opts = resvg::usvg::Options {
        fontdb: db,
        ..Default::default()
    };

    let tree = resvg::usvg::Tree::from_str(svg, &opts).map_err(|e| format!("SVG parse: {e}"))?;

    let svg_size = tree.size();
    let scale = raster_scale(svg_size.width(), svg_size.height(), target_width);
    let width = ((svg_size.width() * scale).ceil() as u32).max(1);
    let height = ((svg_size.height() * scale).ceil() as u32).max(1);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "failed to create pixmap".to_string())?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba = image::RgbaImage::from_raw(width, height, pixmap.take())
        .ok_or_else(|| "failed to create image buffer".to_string())?;

    Ok(DynamicImage::ImageRgba8(rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_flowchart() {
        let source = "graph TD\n    A --> B";
        let result = render_mermaid_to_image(source, 400);
        assert!(result.is_ok(), "flowchart failed: {:?}", result.err());
        let img = result.unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn test_flowchart_keyword() {
        let source = "flowchart TD\n    A[Start] --> B{Decision?}\n    B -->|Yes| C[Ok]\n    B -->|No| D[Fix]";
        let result = render_mermaid_to_image(source, 800);
        assert!(
            result.is_ok(),
            "flowchart keyword failed: {:?}",
            result.err()
        );
        let img = result.unwrap();
        // Fits the pane, or stops at MAX_UPSCALE for a diagram this small.
        assert!(img.width() > 0 && img.width() <= 800, "{}", img.width());
        assert!(img.height() > 100);
    }

    #[test]
    fn raster_scale_fits_the_width_within_its_bounds() {
        // Fits a wide diagram to the pane.
        assert_eq!(raster_scale(1600.0, 400.0, 800), 0.5);
        // Enlarges a small one, but only so far.
        assert_eq!(raster_scale(100.0, 100.0, 800), MAX_UPSCALE);
        // A degenerate size is left alone rather than dividing by zero.
        assert_eq!(raster_scale(0.0, 100.0, 800), 1.0);
        assert_eq!(raster_scale(f32::NAN, 100.0, 800), 1.0);

        // A very tall one is held to the pixel budget, drawn narrower.
        let (w, h) = (1000.0, 40_000.0);
        let s = raster_scale(w, h, 2000);
        assert!(s < 2.0);
        assert!((w * s) * (h * s) <= MAX_RASTER_PIXELS * 1.001);
    }

    /// A long chain used to rasterize at full pane width with no bound, which
    /// at a wide pane is gigabytes. It has to come back inside the budget.
    /// A diagram comes back from the worker tagged with what was asked for,
    /// so a result for a width the pane has since left can be told apart.
    #[test]
    fn the_worker_returns_each_diagram_with_its_hash_and_width() {
        let worker = MermaidWorker::new();
        worker.request(7, "graph TD\n    A --> B", 400);
        worker.request(8, &"x".repeat(MAX_SOURCE_BYTES + 1), 400);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut results = Vec::new();
        while results.len() < 2 && std::time::Instant::now() < deadline {
            results.extend(worker.drain());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        results.sort_by_key(|r| r.hash);

        assert_eq!(results.len(), 2);
        assert_eq!((results[0].hash, results[0].width), (7, 400));
        assert!(results[0].image.is_ok());
        // Too large to lay out, refused rather than attempted.
        let error = results[1].image.as_ref().unwrap_err();
        assert!(error.contains("limit"), "{error}");
    }

    #[test]
    fn a_long_chain_is_rasterized_within_the_pixel_budget() {
        let mut source = String::from("graph TD\n");
        for i in 0..60 {
            source.push_str(&format!("    N{i} --> N{}\n", i + 1));
        }
        let img = render_mermaid_to_image(&source, 2400).expect("renders");
        let area = img.width() as f32 * img.height() as f32;
        assert!(
            area <= MAX_RASTER_PIXELS * 1.01,
            "{}x{}",
            img.width(),
            img.height()
        );
    }

    #[test]
    fn test_invalid_input() {
        // The renderer may accept some invalid inputs gracefully.
        // At minimum, it should not panic.
        let result = render_mermaid_to_image("not a valid diagram $$$$", 400);
        // Either succeeds or returns an error — both are acceptable
        let _ = result;
    }

    #[test]
    fn test_fix_font_families() {
        let input = r#"<text font-family="Inter, "Segoe UI", sans-serif">hello</text>"#;
        let fixed = fix_svg_font_families(input);
        assert!(
            !fixed.contains(r#""Segoe UI""#),
            "inner quotes should be replaced"
        );
        assert!(fixed.contains("'Segoe UI'"), "should use single quotes");
    }
}
