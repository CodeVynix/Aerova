//! Aerova engine: fetch -> parse -> layout -> paint orchestration.
//! Phase 2 adds `index_url` crawl hook into `aerova-search`.

use std::path::Path;

/// Document id re-export for shell convenience.
pub use aerova_search::DocId;

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
    fn indexes_builtin_url() {
        let mut idx = aerova_search::Index::new();
        let id = index_url(&mut idx, "aerova://home").unwrap();
        assert_eq!(id, 0);
        assert_eq!(idx.search("aerova", 10).len(), 1);
    }
}
