use image::DynamicImage;
use std::sync::Arc;
use std::sync::OnceLock;

/// Render mermaid source text to a raster image.
///
/// Pipeline: mermaid source → SVG (via mermaid-rs-renderer) → raster (via resvg)
pub fn render_mermaid_to_image(source: &str, target_width: u32) -> Result<DynamicImage, String> {
    let svg = render_to_svg(source)?;
    rasterize_svg(&svg, target_width)
}

/// Generate SVG from mermaid diagram source.
fn render_to_svg(source: &str) -> Result<String, String> {
    let opts = mermaid_rs_renderer::RenderOptions::default();

    // Wrap in catch_unwind — the renderer can panic on malformed input
    let result = std::panic::catch_unwind(|| {
        mermaid_rs_renderer::render_with_options(source, opts)
    });

    match result {
        Ok(Ok(svg)) => Ok(fix_svg_font_families(&svg)),
        Ok(Err(e)) => Err(format!("mermaid render error: {e}")),
        Err(_) => Err("mermaid renderer panicked".to_string()),
    }
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

/// Rasterize an SVG string to a pixel image at the given target width.
fn rasterize_svg(svg: &str, target_width: u32) -> Result<DynamicImage, String> {
    let db = font_database();

    let opts = resvg::usvg::Options {
        fontdb: db,
        ..Default::default()
    };

    let tree = resvg::usvg::Tree::from_str(svg, &opts).map_err(|e| format!("SVG parse: {e}"))?;

    let svg_size = tree.size();
    let scale = target_width as f32 / svg_size.width();
    let width = target_width;
    let height = (svg_size.height() * scale).ceil() as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| "failed to create pixmap".to_string())?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let rgba = image::RgbaImage::from_raw(width, height, pixmap.data().to_vec())
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
        assert!(result.is_ok(), "flowchart keyword failed: {:?}", result.err());
        let img = result.unwrap();
        assert!(img.width() == 800);
        assert!(img.height() > 100);
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
