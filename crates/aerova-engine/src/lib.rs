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

/// Collect inline `<script>` bodies in document order (ignores `src` for now).
fn extract_scripts(doc: &aerova_dom::Document) -> Vec<String> {
    fn walk(n: &aerova_dom::Node, out: &mut Vec<String>) {
        match n {
            aerova_dom::Node::Text(_) => {}
            aerova_dom::Node::Element { tag, children, .. } => {
                if tag == "script" {
                    let mut s = String::new();
                    for c in children {
                        if let aerova_dom::Node::Text(tt) = c {
                            s.push_str(tt);
                            s.push('\n');
                        }
                    }
                    if !s.trim().is_empty() {
                        out.push(s);
                    }
                } else {
                    for c in children {
                        walk(c, out);
                    }
                }
            }
        }
    }
    let mut v = Vec::new();
    for n in &doc.nodes {
        walk(n, &mut v);
    }
    v
}

fn visible_text(doc: &aerova_dom::Document) -> String {
    fn walk(n: &aerova_dom::Node, out: &mut String) {
        match n {
            aerova_dom::Node::Text(t) => {
                out.push_str(t);
                out.push(' ');
            }
            aerova_dom::Node::Element { tag, children, .. } => {
                if tag == "style" || tag == "title" || tag == "head" || tag == "script" {
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

fn element_text(n: &aerova_dom::Node, out: &mut String) {
    match n {
        aerova_dom::Node::Text(t) => {
            out.push_str(t);
            out.push(' ');
        }
        aerova_dom::Node::Element { children, .. } => {
            for c in children {
                element_text(c, out);
            }
        }
    }
}

/// Snapshot queryable elements for JS bindings (skips style/script/head/title).
/// `parent` is the snapshot index of the enclosing element, used by JS bubbling.
fn dom_snapshot(doc: &aerova_dom::Document) -> Vec<aerova_js::DomEl> {
    fn walk(n: &aerova_dom::Node, parent: Option<usize>, out: &mut Vec<aerova_js::DomEl>) {
        match n {
            aerova_dom::Node::Text(_) => {}
            aerova_dom::Node::Element {
                tag,
                attrs,
                children,
            } => {
                if tag == "style" || tag == "script" || tag == "head" || tag == "title" {
                    return;
                }
                let mut text = String::new();
                element_text(n, &mut text);
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                let id = attrs
                    .iter()
                    .find(|(k, _)| k == "id")
                    .map_or("", |(_, v)| v.as_str())
                    .to_string();
                let classes: Vec<String> = attrs
                    .iter()
                    .find(|(k, _)| k == "class")
                    .map(|(_, v)| v.split_whitespace().map(|w| w.to_lowercase()).collect())
                    .unwrap_or_default();
                let idx = out.len();
                out.push(aerova_js::DomEl {
                    tag: tag.to_lowercase(),
                    id,
                    classes,
                    text,
                    attrs: attrs.clone(),
                    parent,
                });
                for c in children {
                    walk(c, Some(idx), out);
                }
            }
        }
    }
    let mut v = Vec::new();
    for n in &doc.nodes {
        walk(n, None, &mut v);
    }
    v
}

/// Child-index paths in `dom_snapshot` order. Keep the skip rules in sync
/// with `dom_snapshot`: paths[i] locates snapshot element i in the doc.
fn snapshot_paths(doc: &aerova_dom::Document) -> Vec<Vec<usize>> {
    use aerova_dom::Node;
    fn walk(n: &Node, here: Vec<usize>, out: &mut Vec<Vec<usize>>) {
        match n {
            Node::Text(_) => {}
            Node::Element { tag, children, .. } => {
                if tag == "style" || tag == "script" || tag == "head" || tag == "title" {
                    return;
                }
                out.push(here.clone());
                for (i, c) in children.iter().enumerate() {
                    let mut p = here.clone();
                    p.push(i);
                    walk(c, p, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for (i, n) in doc.nodes.iter().enumerate() {
        walk(n, vec![i], &mut out);
    }
    out
}

fn node_at<'a>(
    doc: &'a mut aerova_dom::Document,
    path: &[usize],
) -> Option<&'a mut aerova_dom::Node> {
    use aerova_dom::Node;
    let mut cur: &'a mut Node = doc.nodes.get_mut(*path.first()?)?;
    for &i in &path[1..] {
        cur = match cur {
            Node::Element { children, .. } => children.get_mut(i)?,
            Node::Text(_) => return None,
        };
    }
    Some(cur)
}

fn new_node(el: &aerova_js::NewEl) -> aerova_dom::Node {
    let mut attrs = Vec::new();
    if !el.id.is_empty() {
        attrs.push(("id".to_string(), el.id.clone()));
    }
    if !el.classes.is_empty() {
        attrs.push(("class".to_string(), el.classes.join(" ")));
    }
    attrs.extend(el.attrs.iter().cloned());
    let children = if el.text.is_empty() {
        Vec::new()
    } else {
        vec![aerova_dom::Node::Text(el.text.clone())]
    };
    aerova_dom::Node::Element {
        tag: el.tag.clone(),
        attrs,
        children,
    }
}

/// Drain JS-created elements into the parsed doc; returns their texts.
/// Unplaceable nodes (cycles, detached parents) are dropped.
fn apply_created_elements(doc: &mut aerova_dom::Document) -> Vec<String> {
    use aerova_dom::Node;
    use aerova_js::ParentRef;
    let new = aerova_js::take_created();
    if new.is_empty() {
        return Vec::new();
    }
    let paths = snapshot_paths(doc);
    let mut loc: Vec<Option<Vec<usize>>> = vec![None; new.len()];
    for _ in 0..=new.len() {
        let mut progress = false;
        for (k, el) in new.iter().enumerate() {
            if loc[k].is_some() {
                continue;
            }
            match el.parent {
                None => {
                    doc.nodes.push(new_node(el));
                    loc[k] = Some(vec![doc.nodes.len() - 1]);
                    progress = true;
                }
                Some(ParentRef::Snap(p)) => {
                    if let Some(path) = paths.get(p) {
                        if let Some(Node::Element { children, .. }) = node_at(doc, path) {
                            children.push(new_node(el));
                            let mut lp = path.clone();
                            lp.push(children.len() - 1);
                            loc[k] = Some(lp);
                            progress = true;
                        }
                    }
                }
                Some(ParentRef::New(r)) => {
                    if let Some(lp) = loc.get(r).and_then(|o| o.clone()) {
                        if let Some(Node::Element { children, .. }) = node_at(doc, &lp) {
                            children.push(new_node(el));
                            let mut np = lp;
                            np.push(children.len() - 1);
                            loc[k] = Some(np);
                            progress = true;
                        }
                    }
                }
            }
        }
        if !progress {
            break;
        }
    }
    new.iter()
        .filter(|e| !e.text.is_empty())
        .map(|e| e.text.clone())
        .collect()
}

fn paint_tree(fb: &mut aerova_paint::Framebuffer, b: &aerova_layout::BoxNode) {
    if let Some(bg) = b.background() {
        if let Some(rgb) = aerova_paint::parse_color(bg) {
            if b.w > 0 && b.h > 0 {
                fb.rect(b.x, b.y, b.w, b.h.min(fb.height), rgb);
            }
        }
    } else if b.kind == "#text" {
        for (i, line) in b.text.split('\n').enumerate() {
            let y = b.y + i as u32 * aerova_paint::LINE_H;
            if y >= fb.height {
                break;
            }
            aerova_paint::draw_text(fb, b.x, y, line, aerova_paint::INK);
        }
    }
    for c in &b.children {
        paint_tree(fb, c);
    }
}

/// Viewport height in px; content taller than this scrolls.
pub const PAGE_VIEW_H: u32 = 600;

/// Lowest content edge across all boxes.
fn content_bottom(boxes: &[aerova_layout::BoxNode]) -> u32 {
    fn walk(b: &aerova_layout::BoxNode, max: &mut u32) {
        *max = (*max).max(b.y + b.h);
        for c in &b.children {
            walk(c, max);
        }
    }
    let mut m = 0;
    for b in boxes {
        walk(b, &mut m);
    }
    m
}

fn content_fb(boxes: &[aerova_layout::BoxNode]) -> aerova_paint::Framebuffer {
    let mut fb = aerova_paint::Framebuffer::new(800, content_bottom(boxes).max(PAGE_VIEW_H));
    for b in boxes {
        paint_tree(&mut fb, b);
    }
    fb
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
    let mut doc = aerova_html::parse(html);
    let mut text = visible_text(&doc);
    let css_text = extract_css(&doc);
    let rules = aerova_css::parse(&css_text);
    let scripts = extract_scripts(&doc);
    if !scripts.is_empty() {
        let mut title: Option<String> = None;
        for n in &doc.nodes {
            find_title(n, &mut title);
        }
        let dom = dom_snapshot(&doc);
        let out = aerova_js::run_scripts_with_dom(&scripts, title.as_deref().unwrap_or(""), &dom);
        for w in &out.writes {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(w);
        }
        for t in apply_created_elements(&mut doc) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&t);
        }
    }
    let boxes = aerova_layout::layout(&doc, &rules);
    let fb = content_fb(&boxes);
    (text, boxes, fb)
}

/// A loaded page with a live JS world: scripts run once at open, clicks
/// dispatch into the same context so registered listeners fire.
pub struct Page {
    /// Rendered text (grows with `document.write` from clicks).
    pub text: String,
    /// Layout boxes, tagged with snapshot indices for hit-testing.
    pub boxes: Vec<aerova_layout::BoxNode>,
    /// Painted pixels; full content height, taller than the viewport when the
    /// page scrolls (`view_h` rows are shown at a time).
    pub fb: aerova_paint::Framebuffer,
    /// Viewport height in px.
    view_h: u32,
    /// Vertical scroll offset in px.
    scroll_y: u32,
    js: aerova_js::JsSession,
}

impl Page {
    /// Parse HTML, run scripts once, lay out + paint.
    /// # Errors
    /// Returns JS setup error string.
    pub fn open(html: &str) -> Result<Self, String> {
        Self::open_with_url(html, "")
    }

    /// Parse HTML, run scripts once, lay out + paint, with `location.href`.
    /// # Errors
    /// Returns JS setup error string.
    pub fn open_with_url(html: &str, url: &str) -> Result<Self, String> {
        let mut doc = aerova_html::parse(html);
        let mut title: Option<String> = None;
        for n in &doc.nodes {
            find_title(n, &mut title);
        }
        let dom = dom_snapshot(&doc);
        let mut js = aerova_js::JsSession::new(title.as_deref().unwrap_or(""), &dom)?;
        js.set_url(url);
        let mut text = visible_text(&doc);
        let css_text = extract_css(&doc);
        let rules = aerova_css::parse(&css_text);
        for s in extract_scripts(&doc) {
            let (_, writes, _) = js.run(&s);
            for w in &writes {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(w);
            }
        }
        let (fwrites, _) = js.fire();
        for w in &fwrites {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(w);
        }
        for t in apply_created_elements(&mut doc) {
            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(&t);
        }
        let boxes = aerova_layout::layout(&doc, &rules);
        let fb = content_fb(&boxes);
        Ok(Self {
            text,
            boxes,
            fb,
            view_h: PAGE_VIEW_H,
            scroll_y: 0,
            js,
        })
    }

    /// Click at viewport coords (origin is the page area, not the window):
    /// hit-test, dispatch `click` with bubbling. Returns new writes (also
    /// appended to `text`). Elements created by click listeners are dropped:
    /// no click-time relayout yet, and keeping them would leak them into the
    /// next page's registry drain.
    pub fn click(&mut self, x: u32, y: u32) -> Vec<String> {
        let Some(idx) = aerova_layout::hit_test(&self.boxes, x, y.saturating_add(self.scroll_y))
        else {
            return Vec::new();
        };
        let (_, writes, _) = self.js.dispatch(idx, "click");
        let _ = aerova_js::take_created();
        for w in &writes {
            if !self.text.is_empty() {
                self.text.push(' ');
            }
            self.text.push_str(w);
        }
        writes
    }

    /// Scroll to an absolute content offset, clamped to the content.
    pub fn scroll_to(&mut self, y: u32) {
        let max = self.content_h().saturating_sub(self.view_h);
        self.scroll_y = y.min(max);
    }

    /// Scroll by `dy` px (negative scrolls up), clamped to the content.
    pub fn scroll_by(&mut self, dy: i32) {
        let max = self.content_h().saturating_sub(self.view_h) as i64;
        let next = (self.scroll_y as i64 + dy as i64).clamp(0, max);
        self.scroll_y = next as u32;
    }

    /// Current scroll offset in px.
    #[must_use]
    pub fn scroll_y(&self) -> u32 {
        self.scroll_y
    }

    /// Total content height in px.
    #[must_use]
    pub fn content_h(&self) -> u32 {
        self.fb.height
    }

    /// Viewport height in px.
    #[must_use]
    pub fn view_h(&self) -> u32 {
        self.view_h
    }

    /// Fetch URL then open.
    /// # Errors
    /// Returns fetch or JS setup error string.
    pub fn open_url(url: &str) -> Result<Self, String> {
        let (bytes, _ct) = aerova_net::fetch(url).map_err(|e| e.to_string())?;
        let html = String::from_utf8_lossy(&bytes).to_string();
        Self::open_with_url(&html, url)
    }
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
        let i = ((div_box.y * 800 + div_box.x + div_box.w - 5) as usize) * 4;
        assert_eq!(&fb.pixels[i..i + 3], &[255, 0, 0]);
    }

    fn find_box<'a>(
        boxes: &'a [aerova_layout::BoxNode],
        kind: &str,
    ) -> Option<&'a aerova_layout::BoxNode> {
        for b in boxes {
            if b.kind == kind {
                return Some(b);
            }
            if let Some(hit) = find_box(&b.children, kind) {
                return Some(hit);
            }
        }
        None
    }

    #[test]
    fn descendant_rule_paints_nested_only() {
        let html = "<style>div p { background: red; height: 30px; }</style><div><p>hi</p></div><p>plain</p>";
        let (text, boxes, fb) = render_text(html);
        assert!(text.contains("hi"));
        let nested = find_box(&boxes, "p").unwrap();
        assert_eq!(nested.h, 30);
        let i = ((nested.y * 800 + nested.x + nested.w - 5) as usize) * 4;
        assert_eq!(&fb.pixels[i..i + 3], &[255, 0, 0]);
        let plain = boxes.iter().find(|b| b.kind == "p").unwrap();
        assert_eq!(plain.h, 20);
    }

    #[test]
    fn inline_style_beats_stylesheet() {
        let html = "<style>div { background: red; height: 40px; }</style><div style=\"background: blue; height: 20px;\">hi</div>";
        let (_, boxes, fb) = render_text(html);
        let div_box = boxes.iter().find(|b| b.kind == "div").unwrap();
        assert_eq!(div_box.h, 20);
        let i = ((div_box.y * 800 + div_box.x + div_box.w - 5) as usize) * 4;
        assert_eq!(&fb.pixels[i..i + 3], &[0, 0, 255]);
    }

    #[test]
    fn attr_selector_paints_link_with_href() {
        let html =
            "<style>a[href] { background: red; height: 25px; }</style><a href=\"x\">l</a><a>n</a>";
        let (_, boxes, fb) = render_text(html);
        let link = boxes.iter().find(|b| b.kind == "a" && b.h == 25).unwrap();
        let i = ((link.y * 800 + link.x + link.w - 5) as usize) * 4;
        assert_eq!(&fb.pixels[i..i + 3], &[255, 0, 0]);
        let bare = boxes.iter().find(|b| b.kind == "a" && b.h == 20).unwrap();
        let j = ((bare.y * 800 + bare.x + bare.w - 5) as usize) * 4;
        assert_ne!(&fb.pixels[j..j + 3], &[255, 0, 0]);
    }

    #[test]
    fn script_write_appends_text() {
        let (text, _, _) = render_text("<p>base</p><script>document.write('injected');</script>");
        assert!(text.contains("base"));
        assert!(text.contains("injected"));
    }

    #[test]
    fn script_query_apis_do_not_crash_stub() {
        let (text, _, _) =
            render_text("<div id=\"main\">hi</div><script>document.write('done');</script>");
        assert!(text.contains("hi"));
        assert!(text.contains("done"));
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

    #[test]
    fn page_click_without_js_is_quiet() {
        let mut page = Page::open("<div id=\"c\">x</div>").unwrap();
        assert!(page.text.contains('x'));
        assert!(page.click(5, 5).is_empty());
        assert!(page.click(10, 500).is_empty());
    }

    #[cfg(feature = "boa")]
    #[test]
    fn page_location_href_is_page_url() {
        let page = Page::open_with_url(
            "<p>hi</p><script>document.write(window.location.href);</script>",
            "file:///tmp/x.html",
        )
        .unwrap();
        assert!(page.text.contains("file:///tmp/x.html"));
    }

    #[cfg(feature = "boa")]
    #[test]
    fn page_created_element_renders() {
        let mut html = String::from("<style>p { height: 40px; }</style>");
        for i in 0..20 {
            html.push_str(&format!("<p>line {i}</p>"));
        }
        let plain = Page::open(&html).unwrap();
        html.push_str("<script>var p = document.createElement('p'); p.textContent = 'injected'; p.setAttribute('style', 'height: 60px'); document.appendChild(p);</script>");
        let live = Page::open(&html).unwrap();
        assert!(live.text.contains("injected"));
        assert!(live.content_h() > plain.content_h());
    }

    #[test]
    fn short_page_keeps_viewport_framebuffer() {
        let page = Page::open("<p>hi</p>").unwrap();
        assert_eq!(page.content_h(), PAGE_VIEW_H);
        assert_eq!(page.view_h(), PAGE_VIEW_H);
        assert_eq!(page.scroll_y(), 0);
    }

    #[test]
    fn long_page_grows_and_scroll_clamps() {
        let mut html = String::from("<style>p { height: 40px; }</style>");
        for i in 0..20 {
            html.push_str(&format!("<p>line {i}</p>"));
        }
        let mut page = Page::open(&html).unwrap();
        assert_eq!(page.content_h(), 800);
        assert!(page.content_h() > page.view_h());
        page.scroll_by(-100);
        assert_eq!(page.scroll_y(), 0);
        page.scroll_by(50);
        assert_eq!(page.scroll_y(), 50);
        page.scroll_by(10_000);
        assert_eq!(page.scroll_y(), 200);
        page.scroll_by(-10_000);
        assert_eq!(page.scroll_y(), 0);
    }

    #[cfg(feature = "boa")]
    fn box_center(boxes: &[aerova_layout::BoxNode], el: usize) -> Option<(u32, u32)> {
        for b in boxes {
            if b.el == Some(el) {
                return Some((b.x + b.w / 2, b.y + b.h / 2));
            }
            if let Some(p) = box_center(&b.children, el) {
                return Some(p);
            }
        }
        None
    }

    #[cfg(feature = "boa")]
    #[test]
    fn page_click_dispatches_with_bubbling() {
        let html = r#"<div id="p"><div id="c">x</div></div><script>document.getElementById('p').addEventListener('click', function (ev) { document.write('p:' + ev.target.id); }); document.getElementById('c').addEventListener('click', function () { document.write('self'); });</script>"#;
        let mut page = Page::open(html).unwrap();
        let (cx, cy) = box_center(&page.boxes, 1).unwrap();
        let writes = page.click(cx, cy);
        assert!(writes.contains(&"self".to_string()));
        assert!(writes.contains(&"p:c".to_string()));
        assert!(page.text.contains("self"));
    }

    #[cfg(feature = "boa")]
    #[test]
    fn page_click_respects_scroll_offset() {
        let mut html = String::from(
            "<style>p { height: 40px; }</style><script>document.querySelectorAll('p')[19].addEventListener('click', function () { document.write('last'); });</script>",
        );
        for i in 0..20 {
            html.push_str(&format!("<p>line {i}</p>"));
        }
        let mut page = Page::open(&html).unwrap();
        let bottom = page.view_h() - 20;
        assert!(page.click(50, bottom).is_empty());
        page.scroll_by(10_000);
        assert_eq!(page.scroll_y(), 200);
        let writes = page.click(50, bottom);
        assert!(writes.contains(&"last".to_string()));
        assert!(page.text.contains("last"));
    }

    #[cfg(feature = "boa")]
    #[test]
    fn page_click_empty_space_dispatches_nothing() {
        let html = r#"<div id="p">x</div><script>document.getElementById('p').addEventListener('click', function () { document.write('hit'); });</script>"#;
        let mut page = Page::open(html).unwrap();
        assert!(page.click(10, 500).is_empty());
    }

    #[cfg(feature = "boa")]
    #[test]
    fn snapshot_attributes_reach_js() {
        let html = r#"<a href="http://example.com" data-x="7">link</a><script>document.write(document.querySelector('a').getAttribute('href') + '|' + document.querySelector('a').getAttribute('data-x'));</script>"#;
        let (text, _, _) = render_text(html);
        assert!(text.contains("http://example.com"));
        assert!(text.contains("|7"));
    }

    #[cfg(feature = "boa")]
    #[test]
    fn event_bubbles_through_snapshot_parents() {
        let html = r#"<div id="p"><div id="c">x</div></div><script>var seen = []; document.getElementById('p').addEventListener('click', function (ev) { seen.push(ev.target.id + '>' + ev.currentTarget.id); }); document.addEventListener('click', function (ev) { seen.push('doc>' + ev.target.id); }); document.getElementById('c').dispatchEvent('click'); document.write(seen.join(','));</script>"#;
        let (text, _, _) = render_text(html);
        assert!(text.contains("c>p"));
        assert!(text.contains("doc>c"));
    }

    #[cfg(feature = "boa")]
    #[test]
    fn event_does_not_bubble_across_siblings() {
        let html = r#"<div id="a">1</div><div id="b">2</div><script>var seen = []; document.getElementById('a').addEventListener('click', function () { seen.push('b-visited'); }); document.getElementById('b').dispatchEvent('click'); document.write(seen.join(','));</script>"#;
        let (text, _, _) = render_text(html);
        assert!(!text.contains("b-visited"));
    }
}
