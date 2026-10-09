//! Aerova engine: fetch -> parse -> style -> layout -> paint.
//! Phase 3 wires CSS cascade into boxes + framebuffer.

use std::path::Path;

/// Document id re-export for shell convenience.
pub use aerova_search::DocId;

/// Collect CSS text from `<style>` elements.
fn extract_css(doc: &aerova_dom::Document) -> String {
    fn walk(n: &aerova_dom::Node, out: &mut String) {
        match n {
            aerova_dom::Node::Text(_) => {}
            aerova_dom::Node::Element { tag, children, .. } => {
                if tag == "style" {
                    for c in children {
                        if let aerova_dom::Node::Text(t) = c {
                            out.push_str(t);
                            out.push('\n');
                        }
                    }
                } else {
                    for c in children {
                        walk(c, out);
                    }
                }
            }
        }
    }
    let mut s = String::new();
    for n in &doc.nodes {
        walk(n, &mut s);
    }
    s
}

fn visible_text(doc: &aerova_dom::Document) -> String {
    fn walk(n: &aerova_dom::Node, out: &mut String) {
        match n {
            aerova_dom::Node::Text(t) => {
                out.push_str(t);
                out.push(' ');
            }
            aerova_dom::Node::Element { tag, children, .. } => {
                if tag == "style" || tag == "title" || tag == "head" {
                    return;
                }
                for c in children {
                    walk(c, out);
                }
            }
        }
    }
    let mut s = String::new();
    for n in &doc.nodes {
        walk(n, &mut s);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn paint_tree(fb: &mut aerova_paint::Framebuffer, b: &aerova_layout::BoxNode) {
    if let Some(bg) = b.background() {
        if let Some(rgb) = aerova_paint::parse_color(bg) {
            if b.w > 0 && b.h > 0 {
                fb.rect(b.x, b.y, b.w, b.h.min(600), rgb);
            }
        }
    } else if b.kind == "#text" {
        fb.rect(b.x, b.y, b.w.min(800), 2, (220, 220, 220));
    }
    for c in &b.children {
        paint_tree(fb, c);
    }
}

/// Render HTML text to text + boxes + framebuffer.
#[must_use]
pub fn render_text(
    html: &str,
) -> (
    String,
    Vec<aerova_layout::BoxNode>,
    aerova_paint::Framebuffer,
) {
    let doc = aerova_html::parse(html);
    let text = visible_text(&doc);
    let css_text = extract_css(&doc);
    let rules = aerova_css::parse(&css_text);
    let boxes = aerova_layout::layout(&doc, &rules);
    let mut fb = aerova_paint::Framebuffer::new(800, 600);
    for b in &boxes {
        paint_tree(&mut fb, b);
    }
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

fn find_title(node: &aerova_dom::Node, out: &mut Option<String>) {
    if out.is_some() {
        return;
    }
    match node {
        aerova_dom::Node::Text(_) => {}
        aerova_dom::Node::Element { tag, children, .. } => {
            if tag == "title" {
                let mut s = String::new();
                for c in children {
                    if let aerova_dom::Node::Text(t) = c {
                        s.push_str(t);
                        s.push(' ');
                    }
                }
                let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
                if !s.is_empty() {
                    *out = Some(s);
                    return;
                }
            }
            for c in children {
                find_title(c, out);
            }
        }
    }
}

/// Fetch URL, parse, extract title + text, add to index.
/// Returns new document id.
/// # Errors
/// Returns fetch error string.
pub fn index_url(index: &mut aerova_search::Index, url: &str) -> Result<DocId, String> {
    let (bytes, _ct) = aerova_net::fetch(url).map_err(|e| e.to_string())?;
    let html = String::from_utf8_lossy(&bytes).to_string();
    let doc = aerova_html::parse(&html);
    let mut title: Option<String> = None;
    for n in &doc.nodes {
        find_title(n, &mut title);
    }
    let mut body = doc.text();
    if body.len() > 8000 {
        body.truncate(8000);
    }
    let title = title.unwrap_or_else(|| {
        if url.len() > 80 {
            url[..80].to_string()
        } else {
            url.to_string()
        }
    });
    Ok(index.add(&title, &format!("{url} {body}")))
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

    #[test]
    fn styled_background_paints() {
        let html = "<style>div { background: red; height: 40px; }</style><div>hi</div>";
        let (text, boxes, fb) = render_text(html);
        assert!(text.contains("hi"));
        assert!(!boxes.is_empty());
        // Box with red background exists and top-left of its rect is red.
        let div_box = boxes.iter().find(|b| b.kind == "div").unwrap();
        assert_eq!(div_box.h, 40);
        let i = ((div_box.y * 800 + div_box.x) as usize) * 4;
        assert_eq!(&fb.pixels[i..i + 3], &[255, 0, 0]);
    }

    #[test]
    fn indexes_builtin_url() {
        let mut idx = aerova_search::Index::new();
        let id = index_url(&mut idx, "aerova://home").unwrap();
        assert_eq!(id, 0);
        assert_eq!(idx.search("aerova", 10).len(), 1);
    }
}
