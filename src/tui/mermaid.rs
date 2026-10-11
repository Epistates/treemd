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

/// Render mermaid source text to a raster image.
///
/// Pipeline: mermaid source → SVG (via mermaid-rs-renderer) → raster (via resvg)
///
/// Both stages are third-party code that can panic on input it did not
/// expect, so a panic becomes an error for this one diagram.
pub fn render_mermaid_to_image(source: &str, target_width: u32) -> Result<DynamicImage, String> {
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

        // Process the attribute value character by character
        let mut found_close = false;
        let chars: Vec<char> = remaining.chars().collect();
        let mut char_idx = 0;

        while char_idx < chars.len() {
            if chars[char_idx] == '"' {
                let next = chars.get(char_idx + 1).copied().unwrap_or('>');
                if next == '>' || next == ' ' || next == '/' {
                    // Real closing quote
                    result.push('"');
                    // Advance remaining past this quote
                    let byte_offset: usize =
                        chars[..char_idx + 1].iter().map(|c| c.len_utf8()).sum();
                    remaining = &remaining[byte_offset..];
                    found_close = true;
                    break;
                } else {
                    // Inner quote — replace with single quote
                    result.push('\'');
                }
            } else {
                result.push(chars[char_idx]);
            }
            char_idx += 1;
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
