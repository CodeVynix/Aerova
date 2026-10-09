//! Aerova engine: fetch -> parse -> layout -> paint orchestration.

use std::path::Path;

/// Render HTML text to text + boxes + framebuffer stub.
#[must_use]
pub fn render_text(
    html: &str,
) -> (
    String,
    Vec<aerova_layout::BoxNode>,
    aerova_paint::Framebuffer,
) {
    let doc = aerova_html::parse(html);
    let text = doc.text();
    let boxes = aerova_layout::layout(&doc);
    let mut fb = aerova_paint::Framebuffer::new(800, 600);
    fb.rect(0, 0, 800, 40, (230, 240, 250));
    let _css = aerova_css::parse("p { color: black; }");
    (text, boxes, fb)
}

/// Load URL then render.
/// # Errors
/// Returns fetch error string.
pub fn render_url(url: &str) -> Result<(String, aerova_paint::Framebuffer), String> {
    let (bytes, _ct) = aerova_net::fetch(url).map_err(|e| e.to_string())?;
    let html = String::from_utf8_lossy(&bytes).to_string();
    let (text, _boxes, fb) = render_text(&html);
    Ok((text, fb))
}

/// Render file to PPM screenshot.
/// # Errors
/// Returns IO/fetch error string.
pub fn screenshot_file(input: &Path, out_ppm: &Path) -> Result<String, String> {
    let url = format!("file://{}", input.display());
    let (text, fb) = render_url(&url)?;
    fb.save_ppm(out_ppm).map_err(|e| e.to_string())?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders() {
        let (text, _, fb) = render_text("<p>hi</p>");
        assert_eq!(text, "hi");
        assert_eq!((fb.width, fb.height), (800, 600));
    }
}
