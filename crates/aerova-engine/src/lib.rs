//! Aerova engine: fetch -> parse -> style -> layout -> paint.
//! Phase 3 wires CSS cascade. Phase 4 adds tabs + per-tab isolation stub.
//! Real OS processes later (Lumora); threads now give crash containment.

use std::collections::HashMap;
use std::path::Path;

/// Document id re-export for shell convenience.
pub use aerova_search::DocId;

/// Tab identifier.
pub type TabId = usize;

/// One browser tab with isolated history.
#[derive(Debug, Clone)]
pub struct Tab {
    /// Tab id.
    pub id: TabId,
    history: Vec<String>,
    pos: usize,
}

impl Tab {
    /// Current URL, if any.
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.history.get(self.pos).map(String::as_str)
    }
}

/// Manager for independent tabs. Each tab owns its history; no shared mutable state.
#[derive(Debug, Default)]
pub struct TabManager {
    tabs: HashMap<TabId, Tab>,
    next: TabId,
}

impl TabManager {
    /// Create empty manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a new tab at `url`, returns id.
    pub fn new_tab(&mut self, url: &str) -> TabId {
        let id = self.next;
        self.next += 1;
        self.tabs.insert(
            id,
            Tab {
                id,
                history: vec![url.to_string()],
                pos: 0,
            },
        );
        id
    }

    /// Navigate tab to `url`. Returns false if tab is missing.
    pub fn navigate(&mut self, id: TabId, url: &str) -> bool {
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.history.truncate(tab.pos + 1);
            tab.history.push(url.to_string());
            tab.pos = tab.history.len() - 1;
            true
        } else {
            false
        }
    }

    /// Go back. Returns new current URL or `None`.
    pub fn back(&mut self, id: TabId) -> Option<String> {
        if let Some(tab) = self.tabs.get_mut(&id) {
            if tab.pos > 0 {
                tab.pos -= 1;
                return tab.current().map(ToString::to_string);
            }
        }
        None
    }

    /// Go forward. Returns new current URL or `None`.
    pub fn forward(&mut self, id: TabId) -> Option<String> {
        if let Some(tab) = self.tabs.get_mut(&id) {
            if tab.pos + 1 < tab.history.len() {
                tab.pos += 1;
                return tab.current().map(ToString::to_string);
            }
        }
        None
    }

    /// Current URL for tab.
    #[must_use]
    pub fn current(&self, id: TabId) -> Option<String> {
        self.tabs
            .get(&id)
            .and_then(|t| t.current().map(ToString::to_string))
    }

    /// List `(id, current_url)` sorted by id.
    #[must_use]
    pub fn list(&self) -> Vec<(TabId, String)> {
        let mut v: Vec<(TabId, String)> = self
            .tabs
            .iter()
            .map(|(id, t)| (*id, t.current().unwrap_or("").to_string()))
            .collect();
        v.sort_by_key(|(id, _)| *id);
        v
    }

    /// Close tab. Returns true if it existed.
    pub fn close(&mut self, id: TabId) -> bool {
        self.tabs.remove(&id).is_some()
    }

    /// Number of open tabs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// True if no tabs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

/// How a tab render is isolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsolationMode {
    /// Same thread (fastest, no containment).
    #[default]
    InProcess,
    /// Spawned thread per render; panic in one tab cannot kill the shell.
    /// Lumora later maps this to per-tab processes.
    PerTabThread,
}

/// Policy wrapper (extensible to sandbox/CSP later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IsolationPolicy {
    /// Mode.
    pub mode: IsolationMode,
}

impl IsolationPolicy {
    /// Per-tab thread containment.
    #[must_use]
    pub fn per_tab_thread() -> Self {
        Self {
            mode: IsolationMode::PerTabThread,
        }
    }
}

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

/// Render with isolation policy. `PerTabThread` contains panics per tab.
/// # Errors
/// Returns fetch error or `tab crashed` panic containment.
pub fn render_isolated(
    url: &str,
    policy: &IsolationPolicy,
) -> Result<(String, aerova_paint::Framebuffer), String> {
    match policy.mode {
        IsolationMode::InProcess => render_url(url),
        IsolationMode::PerTabThread => {
            let owned = url.to_string();
            match std::thread::spawn(move || render_url(&owned)).join() {
                Ok(r) => r,
                Err(_) => Err("tab crashed (panic isolated)".to_string()),
            }
        }
    }
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

    #[test]
    fn tabs_navigate_back_forward() {
        let mut m = TabManager::new();
        let a = m.new_tab("aerova://home");
        let b = m.new_tab("aerova://home");
        assert_eq!(m.len(), 2);
        assert!(m.navigate(a, "file:///tmp/x.html"));
        assert_eq!(m.current(a).unwrap(), "file:///tmp/x.html");
        assert_eq!(m.back(a).unwrap(), "aerova://home");
        assert_eq!(m.forward(a).unwrap(), "file:///tmp/x.html");
        assert_eq!(m.current(b).unwrap(), "aerova://home");
        assert!(m.close(a));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn isolation_contains_bad_url() {
        let policy = IsolationPolicy::per_tab_thread();
        assert!(render_isolated("aerova://home", &policy).is_ok());
        assert!(render_isolated("bogus://nope", &policy).is_err());
        // Other tab still works after a failure.
        assert!(render_isolated("aerova://home", &policy).is_ok());
    }
}
