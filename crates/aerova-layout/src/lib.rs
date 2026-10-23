//! Aerova layout: block stacking + flex row/column with class/id styles.
//! Grid later.

use aerova_css::{parse_px, specified_styles, Rule};
use aerova_dom::{Document, Node};
use aerova_paint::DecodedImage;
use std::collections::HashMap;
/// A laid-out box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxNode {
    /// Tag or `#text`.
    pub kind: String,
    /// x position.
    pub x: u32,
    /// y position.
    pub y: u32,
    /// width.
    pub w: u32,
    /// height.
    pub h: u32,
    /// Specified style pairs for this box.
    pub style: Vec<(String, String)>,
    /// Snapshot index of the producing element (`None` for anonymous boxes);
    /// text boxes carry their enclosing element's index for click dispatch.
    pub el: Option<usize>,
    /// Children.
    pub children: Vec<BoxNode>,
    /// Text content, already wrapped into lines joined by `\n`. Empty for
    /// element boxes; `#text` boxes carry the lines to paint.
    pub text: String,
}

impl BoxNode {
    /// Get style value by property.
    #[must_use]
    pub fn style_value(&self, property: &str) -> Option<&str> {
        self.style
            .iter()
            .find(|(k, _)| k == property)
            .map(|(_, v)| v.as_str())
    }

    /// Background value from `background` or `background-color`.
    #[must_use]
    pub fn background(&self) -> Option<&str> {
        self.style_value("background")
            .or_else(|| self.style_value("background-color"))
    }

    /// True when `display: flex`.
    #[must_use]
    pub fn is_flex(&self) -> bool {
        self.style_value("display") == Some("flex")
    }
}

/// Lay out document top-to-bottom with CSS rules. Width 800, row 20px default.
/// Element boxes take snapshot indices in pre-order, matching `dom_snapshot`.
#[must_use]
pub fn layout(doc: &Document, rules: &[Rule], images: &HashMap<String, aerova_paint::DecodedImage>) -> Vec<BoxNode> {
    let mut y = 0;
    let mut next = 0usize;
    let mut ancs: Vec<(&str, &[(String, String)])> = Vec::new();
    let mut out = Vec::new();
    for n in &doc.nodes {
        if let Node::Element { tag, .. } = n {
            if tag == "style" || tag == "title" || tag == "head" || tag == "script" {
                continue;
            }
        }
        out.push(layout_node(
            n, 0, &mut y, 800, rules, None, &mut next, &mut ancs, images,
        ));
    }
    out
}

/// Topmost snapshot index at `(x, y)`, or `None` for empty space.
/// Children paint over parents and later siblings over earlier ones.
#[must_use]
pub fn hit_test(boxes: &[BoxNode], x: u32, y: u32) -> Option<usize> {
    for b in boxes.iter().rev() {
        if let Some(hit) = hit_box(b, x, y) {
            return Some(hit);
        }
    }
    None
}

fn hit_box(b: &BoxNode, x: u32, y: u32) -> Option<usize> {
    if x < b.x || y < b.y || x - b.x >= b.w || y - b.y >= b.h {
        return None;
    }
    for c in b.children.iter().rev() {
        if let Some(hit) = hit_box(c, x, y) {
            return Some(hit);
        }
    }
    b.el
}

fn alloc_index(next: &mut usize) -> usize {
    let i = *next;
    *next += 1;
    i
}

type Attrs = Vec<(String, String)>;

fn styled_height(
    tag: &str,
    attrs: &Attrs,
    ancs: &[(&str, &[(String, String)])],
    rules: &[Rule],
) -> Option<u32> {
    for (k, v) in specified_styles(tag, attrs, ancs, rules) {
        if k == "height" {
            if let Some(px) = parse_px(&v) {
                return Some(px);
            }
        }
    }
    None
}

fn has_width_decl(
    tag: &str,
    attrs: &Attrs,
    ancs: &[(&str, &[(String, String)])],
    rules: &[Rule],
) -> Option<u32> {
    for (k, v) in specified_styles(tag, attrs, ancs, rules) {
        if k == "width" {
            if let Some(px) = parse_px(&v) {
                return Some(px);
            }
        }
    }
    None
}

/// First pixel value for `name` in the computed style (`auto`/absent → None).
fn px(style: &[(String, String)], name: &str) -> Option<u32> {
    style
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| parse_px(v))
}

/// Raw value for `name`, for shorthand expansion.
fn raw<'a>(style: &'a [(String, String)], name: &str) -> Option<&'a str> {
    style
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// CSS box sides for `margin`/`padding`: shorthand 1-4 values, then longhand
/// per-side overrides. Bad/`auto` values are 0. `[top, right, bottom, left]`.
fn sides(style: &[(String, String)], name: &str) -> [u32; 4] {
    let mut out = [0u32; 4];
    if let Some(v) = raw(style, name) {
        let parts: Vec<u32> = v.split_whitespace().filter_map(parse_px).collect();
        match parts.len() {
            1 => out = [parts[0]; 4],
            2 => out = [parts[0], parts[1], parts[0], parts[1]],
            3 => out = [parts[0], parts[1], parts[2], parts[1]],
            4.. => out = [parts[0], parts[1], parts[2], parts[3]],
            _ => {}
        }
    }
    for (i, long) in ["-top", "-right", "-bottom", "-left"].iter().enumerate() {
        if let Some(v) = px(style, &format!("{name}{long}")) {
            out[i] = v;
        }
    }
    out
}

/// `margin` sides for a node, for flex item placement.
fn margins_of(n: &Node, ancs: &[(&str, &[(String, String)])], rules: &[Rule]) -> [u32; 4] {
    let (tag, attrs) = match n {
        Node::Element { tag, attrs, .. } => (tag.as_str(), attrs.as_slice()),
        Node::Text(_) => return [0; 4],
    };
    sides(&specified_styles(tag, attrs, ancs, rules), "margin")
}

/// `gap` on a flex row: `gap: <row> <column>`; a row lays out on the column part.
fn gap_h(style: &[(String, String)]) -> u32 {
    match raw(style, "gap").map(|v| {
        v.split_whitespace()
            .filter_map(parse_px)
            .collect::<Vec<_>>()
    }) {
        Some(p) if p.len() >= 2 => p[1],
        Some(p) => p[0],
        _ => px(style, "column-gap").unwrap_or(0),
    }
}

fn styled_width(
    tag: &str,
    attrs: &Attrs,
    avail: u32,
    ancs: &[(&str, &[(String, String)])],
    rules: &[Rule],
) -> u32 {
    if let Some(px) = has_width_decl(tag, attrs, ancs, rules) {
        return px.min(avail);
    }
    avail
}

fn is_flex_tag(
    tag: &str,
    attrs: &Attrs,
    ancs: &[(&str, &[(String, String)])],
    rules: &[Rule],
) -> bool {
    specified_styles(tag, attrs, ancs, rules)
        .iter()
        .any(|(k, v)| k == "display" && v == "flex")
}

fn flex_direction(
    tag: &str,
    attrs: &Attrs,
    ancs: &[(&str, &[(String, String)])],
    rules: &[Rule],
) -> &'static str {
    for (k, v) in specified_styles(tag, attrs, ancs, rules) {
        if k == "flex-direction" {
            if v == "column" {
                return "column";
            }
            if v == "row" {
                return "row";
            }
        }
    }
    "row"
}

/// Elements that are inline by default (don't establish block formatting context).
fn is_inline_tag(tag: &str) -> bool {
    matches!(
        tag,
        "a" | "span" | "strong" | "em" | "b" | "i" | "u" | "code" | "br"
        | "small" | "mark" | "sub" | "sup" | "cite" | "q" | "var" | "samp" | "kbd"
        | "abbr" | "time" | "data" | "bdi" | "bdo" | "wbr"
    )
}

/// Check if all children are inline (text or inline elements) AND there's at least one explicit inline element.
/// Pure text children of a block element should use block layout (anonymous block boxes in CSS).
fn all_children_inline(children: &[Node]) -> bool {
    let mut has_inline_element = false;
    for c in children {
        match c {
            Node::Text(_) => {}
            Node::Element { tag, .. } => {
                if is_inline_tag(tag) {
                    has_inline_element = true;
                } else {
                    return false;
                }
            }
        }
    }
    has_inline_element
}

/// A text run with its style and originating element index (for clicks).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TextRun {
    text: String,
    style: Vec<(String, String)>,
    el: Option<usize>,
}

/// Result of laying out inline content: line boxes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LineBox {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    runs: Vec<TextRun>,
}

/// Lay out inline content into line boxes within available width.
/// Returns the line boxes and the total height they occupy.
fn layout_inline_content<'a>(
    children: &[&Node],
    content_x: u32,
    content_y: u32,
    content_w: u32,
    rules: &[Rule],
    cur: Option<usize>,
    next: &mut usize,
    ancs: &mut Vec<(&'a str, &'a [(String, String)])>,
    pt: u32,
    pb: u32,
) -> (Vec<LineBox>, u32) {
    const LINE_H: u32 = aerova_paint::LINE_H;
    let mut lines = Vec::new();
    let mut current_line = LineBox { x: content_x, y: content_y + pt, ..LineBox::default() };
    let mut current_line_w = 0u32;
    let mut current_line_h = aerova_paint::LINE_H;
    let mut line_x = content_x;

    for child in children {
        let child = *child;
        match child {
            Node::Text(t) => {
                if t.trim().is_empty() {
                    continue;
                }
                let wrapped = aerova_paint::wrap(t, content_w);
                for line_text in wrapped {
                    let run_w = aerova_paint::text_width(&line_text);
                    if line_x + run_w > content_x + content_w && current_line_w > 0 {
                        current_line.w = current_line_w;
                        current_line.h = current_line_h;
                        lines.push(current_line);
                        current_line = LineBox { x: content_x, y: content_y + pt, ..LineBox::default() };
                        current_line_w = 0;
                        current_line_h = aerova_paint::LINE_H;
                        line_x = content_x;
                    }
                    let run = TextRun {
                        text: line_text,
                        style: Vec::new(),
                        el: cur,
                    };
                    current_line.runs.push(run);
                    current_line_w += run_w;
                    line_x += run_w;
                    current_line_h = current_line_h.max(aerova_paint::LINE_H);
                }
            }
            Node::Element { tag, attrs, children } => {
                if is_inline_tag(tag) {
                    let child_mine = Some(alloc_index(next));
                    let child_refs: Vec<&Node> = children.iter().collect();
                    let (inner_lines, _inner_h) = layout_inline_content(
                        &child_refs,
                        content_x + current_line_w,
                        content_y,
                        content_w.saturating_sub(current_line_w),
                        rules,
                        child_mine,
                        next,
                        ancs,
                        pt,
                        pb,
                    );
                    for line in inner_lines {
                        for run in line.runs {
                            let run_w = aerova_paint::text_width(&run.text);
                            if line_x + run_w > content_x + content_w && current_line_w > 0 {
                                current_line.w = current_line_w;
                                current_line.h = current_line_h;
                                lines.push(current_line);
                                current_line = LineBox { x: content_x, y: content_y + pt, ..LineBox::default() };
                                current_line_w = 0;
                                current_line_h = aerova_paint::LINE_H;
                                line_x = content_x;
                            }
                            current_line.runs.push(run);
                            current_line_w += run_w;
                            line_x += run_w;
                            current_line_h = current_line_h.max(line.h);
                        }
                    }
                    current_line_h = current_line_h.max(aerova_paint::LINE_H);
                } else {
                    if current_line_w > 0 {
                        current_line.w = current_line_w;
                        current_line.h = current_line_h;
                        lines.push(current_line);
                        current_line = LineBox { x: content_x, y: content_y + pt, ..LineBox::default() };
                        current_line_w = 0;
                        current_line_h = aerova_paint::LINE_H;
                        line_x = content_x;
                    }
                }
            }
        }
    }
    if current_line_w > 0 {
        current_line.w = current_line_w;
        current_line.h = current_line_h;
        lines.push(current_line);
    }
    let total_h = pt + lines.iter().map(|l| l.h).sum::<u32>() + pb;
    (lines, total_h)
}

#[allow(clippy::too_many_arguments)]
fn layout_node<'a>(
    n: &'a Node,
    x: u32,
    y: &mut u32,
    avail_w: u32,
    rules: &[Rule],
    cur: Option<usize>,
    next: &mut usize,
    ancs: &mut Vec<(&'a str, &'a [(String, String)])>,
    images: &HashMap<String, aerova_paint::DecodedImage>,
) -> BoxNode {
    match n {
        Node::Text(t) => {
            let lines = aerova_paint::wrap(t, avail_w);
            let b = BoxNode {
                kind: "#text".into(),
                x,
                y: *y,
                w: avail_w,
                h: lines.len() as u32 * aerova_paint::LINE_H,
                style: Vec::new(),
                el: cur,
                children: vec![],
                text: lines.join("\n"),
            };
            *y += b.h;
            b
        }
        Node::Element {
            tag,
            attrs,
            children,
        } => {
            if tag == "style" || tag == "title" || tag == "head" || tag == "script" {
                return BoxNode {
                    kind: tag.clone(),
                    x,
                    y: *y,
                    w: 0,
                    h: 0,
                    style: specified_styles(tag, attrs, ancs, rules),
                    el: None,
                    children: vec![],
                    text: String::new(),
                };
            }
            let style = specified_styles(tag, attrs, ancs, rules);
            if is_flex_tag(tag, attrs, ancs, rules)
                && flex_direction(tag, attrs, ancs, rules) == "row"
            {
                return layout_flex_row(tag, attrs, children, x, y, avail_w, rules, next, ancs, images);
            }
            // Handle <img> as inline replaced element
            if tag == "img" {
                let src = attrs.iter().find(|(k, _)| k == "src").map(|(_, v)| v.as_str());
                if let Some(src) = src {
                    if let Some(img) = images.get(src) {
                        let mine = Some(alloc_index(next));
                        let [mt, mr, mb, ml] = sides(&style, "margin");
                        let [pt, pr, pb, pl] = sides(&style, "padding");
                        let border_w = styled_width(tag, attrs, avail_w, ancs, rules).saturating_sub(ml + mr);
                        let content_x = x + ml + pl;
                        let content_w = border_w.saturating_sub(pl + pr);
                        let box_y = *y + mt;
                        let content_top = box_y + pt;
                        
                        // Use intrinsic dimensions from the image, or fallback to CSS width/height
                        let (img_w, img_h) = (img.width, img.height);
                        let css_w = style.iter().find(|(k, _)| k == "width").and_then(|(_, v)| parse_px(v));
                        let css_h = style.iter().find(|(k, _)| k == "height").and_then(|(k, v)| parse_px(v));
                        let w = css_w.unwrap_or(img_w).min(avail_w);
                        let h = css_h.unwrap_or(img_h);
                        
                        let [mt, mr, mb, ml] = sides(&style, "margin");
                        let [pt, pr, pb, pl] = sides(&style, "padding");
                        
                        let box_y = *y + mt;
                        let content_top = box_y + pt;
                        let content_x = x + ml + pl;
                        let content_w = w.saturating_sub(pl + pr);
                        
                        let inner_y = content_top + h;
                        *y = box_y + pt + h + pb + mb;
                        
                        return BoxNode {
                            kind: "img".into(),
                            x: content_x,
                            y: content_top,
                            w,
                            h,
                            style,
                            el: Some(alloc_index(next)),
                            children: vec![],
                            text: String::new(),
                        };
                    }
                }
                // Fallback: treat as inline box with default size
                let mine = Some(alloc_index(next));
                let [mt, mr, mb, ml] = sides(&style, "margin");
                let [pt, pr, pb, pl] = sides(&style, "padding");
                let border_w = styled_width(tag, attrs, avail_w, ancs, rules).saturating_sub(ml + mr);
                let content_x = x + ml + pl;
                let content_w = border_w.saturating_sub(pl + pr);
                let box_y = *y + mt;
                let content_top = box_y + pt;
                let w = 100; // default image width
                let h = 100; // default image height
                let inner_y = box_y + pt + 100;
                *y = box_y + pt + 100 + pb + mb;
                return BoxNode {
                    kind: "img".into(),
                    x: content_x,
                    y: box_y + pt,
                    w,
                    h,
                    style,
                    el: Some(alloc_index(next)),
                    children: vec![],
                    text: String::new(),
                };
            }
            let mine = Some(alloc_index(next));
            let [mt, mr, mb, ml] = sides(&style, "margin");
            let [pt, pr, pb, pl] = sides(&style, "padding");
            let border_w = styled_width(tag, attrs, avail_w, ancs, rules).saturating_sub(ml + mr);
            let content_x = x + ml + pl;
            let content_w = border_w.saturating_sub(pl + pr);
            let box_y = *y + mt;
            let content_top = box_y + pt;
            
            // Check if all children are inline - if so, use inline layout
            if all_children_inline(children) {
                let child_refs: Vec<&Node> = children.iter().collect();
                let (lines, run_h) = layout_inline_content(
                    &child_refs,
                    content_x,
                    box_y,
                    content_w,
                    rules,
                    mine,
                    next,
                    ancs,
                    pt,
                    pb,
                );
                let mut kids = Vec::new();
                for line in lines {
                    let line_box = BoxNode {
                        kind: "#line".into(),
                        x: line.x,
                        y: line.y,
                        w: line.w,
                        h: line.h,
                        style: Vec::new(),
                        el: line.runs.first().and_then(|r| r.el),
                        children: vec![],
                        text: line.runs.iter().map(|r| r.text.clone()).collect::<Vec<_>>().join("\n"),
                    };
                    kids.push(line_box);
                }
                let _ = ancs.pop();
                let wanted = styled_height(tag, attrs, ancs, rules).unwrap_or(0);
                let content_h = run_h.max(wanted);
                let h = pt + content_h + pb;
                *y = box_y + pt + content_h + pb + mb;
                return BoxNode {
                    kind: tag.clone(),
                    x,
                    y: box_y,
                    w: border_w,
                    h: pt + content_h + pb,
                    style,
                    el: mine,
                    children: kids,
                    text: String::new(),
                };
            }
            
            let mut inner_y = content_top;
            let mut kids = Vec::new();
            ancs.push((tag.as_str(), attrs.as_slice()));
            for c in children {
                if let Node::Element { tag: ct, .. } = c {
                    if ct == "style" || ct == "title" || ct == "head" || ct == "script" {
                        continue;
                    }
                }
                kids.push(layout_node(
                    c,
                    content_x,
                    &mut inner_y,
                    content_w,
                    rules,
                    mine,
                    next,
                    ancs,
                    images,
                ));
            }
            let _ = ancs.pop();
            let content_h = inner_y - content_top;
            let wanted = styled_height(tag, attrs, ancs, rules).unwrap_or(0);
            let min_content = if kids.is_empty() {
                wanted.max(20)
            } else {
                wanted
            };
            let h = pt + content_h.max(min_content) + pb;
            *y = box_y + h + mb;
            BoxNode {
                kind: tag.clone(),
                x,
                y: box_y,
                w: border_w,
                h,
                style,
                el: mine,
                children: kids,
                text: String::new(),
            }
        }
    }
}

/// Flex row: children share one line, x advances, y stays at row top.
#[allow(clippy::too_many_arguments)]
fn layout_flex_row<'a>(
    tag: &'a str,
    attrs: &'a Attrs,
    children: &'a [Node],
    x: u32,
    y: &mut u32,
    avail_w: u32,
    rules: &[Rule],
    next: &mut usize,
    ancs: &mut Vec<(&'a str, &'a [(String, String)])>,
    images: &HashMap<String, aerova_paint::DecodedImage>,
) -> BoxNode {
    let mine = Some(alloc_index(next));
    let style = specified_styles(tag, attrs, ancs, rules);
    let [mt, mr, mb, ml] = sides(&style, "margin");
    let [pt, pr, pb, pl] = sides(&style, "padding");
    let border_w = styled_width(tag, attrs, avail_w, ancs, rules).saturating_sub(ml + mr);
    let content_w = border_w.saturating_sub(pl + pr);
    let gap = gap_h(&style);
    let visual: Vec<&Node> = children
        .iter()
        .filter(|c| match c {
            Node::Text(_) => true,
            Node::Element { tag: ct, .. } => {
                ct != "style" && ct != "title" && ct != "head" && ct != "script"
            }
        })
        .collect();
    let n = visual.len().max(1) as u32;
    let share = content_w.saturating_sub(gap * (n - 1)) / n;
    let box_y = *y + mt;
    let content_top = box_y + pt;
    let mut kids = Vec::new();
    let mut cx = x + ml + pl;
    let row_x = x + ml + pl;
    let mut max_h = 0;
    ancs.push((tag, attrs.as_slice()));
    for c in visual {
        let [imt, imr, _, iml] = margins_of(c, ancs, rules);
        let avail = match c {
            Node::Text(_) => share,
            Node::Element {
                tag: ct, attrs: ca, ..
            } => has_width_decl(ct, ca, ancs, rules)
                .unwrap_or(share)
                .min(content_w.saturating_sub(cx.saturating_sub(row_x))),
        };
        let cap = avail.saturating_sub(iml + imr);
        let item = layout_flex_item(c, cx + iml, content_top + imt, cap, rules, mine, next, ancs, images);
        let item_w = item.w;
        let item_h = item.h;
        kids.push(item);
        max_h = max_h.max(imt + item_h + imr);
        cx += iml + item_w + imr + gap;
    }
    let _ = ancs.pop();
    let wanted = styled_height(tag, attrs, ancs, rules).unwrap_or(0);
    let h = (pt + max_h + pb).max(wanted.max(20));
    *y = box_y + h + mb;
    BoxNode {
        kind: tag.to_string(),
        x,
        y: box_y,
        w: border_w,
        h,
        style,
        el: mine,
        children: kids,
        text: String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn layout_flex_item<'a>(
    n: &'a Node,
    x: u32,
    y: u32,
    cap_w: u32,
    rules: &[Rule],
    cur: Option<usize>,
    next: &mut usize,
    ancs: &mut Vec<(&'a str, &'a [(String, String)])>,
    images: &HashMap<String, aerova_paint::DecodedImage>,
) -> BoxNode {
    match n {
        Node::Text(t) => {
            let lines = aerova_paint::wrap(t, cap_w);
            BoxNode {
                kind: "#text".into(),
                x,
                y,
                w: cap_w,
                h: lines.len() as u32 * aerova_paint::LINE_H,
                style: Vec::new(),
                el: cur,
                children: vec![],
                text: lines.join("\n"),
            }
        }
        Node::Element {
            tag,
            attrs,
            children,
        } => {
            let mine = Some(alloc_index(next));
            let style = specified_styles(tag, attrs, ancs, rules);
            let [pt, pr, pb, pl] = sides(&style, "padding");
            let box_w = has_width_decl(tag, attrs, ancs, rules).map_or(cap_w, |px| px.min(cap_w));
            let content_w = box_w.saturating_sub(pl + pr);
            let mut inner_y = y + pt;
            let mut kids = Vec::new();
            ancs.push((tag.as_str(), attrs.as_slice()));
            for c in children {
                if let Node::Element { tag: ct, .. } = c {
                    if ct == "style" || ct == "title" || ct == "head" || ct == "script" {
                        continue;
                    }
                }
                kids.push(layout_node(
                    c,
                    x + pl,
                    &mut inner_y,
                    content_w,
                    rules,
                    mine,
                    next,
                    ancs,
                    images,
                ));
            }
            let _ = ancs.pop();
            let content_h = inner_y - (y + pt);
            let wanted = styled_height(tag, attrs, ancs, rules).unwrap_or(0);
            let min_content = if kids.is_empty() {
                wanted.max(20)
            } else {
                wanted
            };
            let h = pt + content_h.max(min_content) + pb;
            BoxNode {
                kind: tag.clone(),
                x,
                y,
                w: box_w,
                h,
                style,
                el: mine,
                children: kids,
                text: String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aerova_dom::{Document, Node};

    #[test]
    fn stacks_vertically() {
        let doc = Document {
            nodes: vec![
                Node::Element {
                    tag: "p".into(),
                    attrs: vec![],
                    children: vec![Node::Text("a".into())],
                },
                Node::Element {
                    tag: "p".into(),
                    attrs: vec![],
                    children: vec![Node::Text("b".into())],
                },
            ],
        };
        let boxes = layout(&doc, &[], &std::collections::HashMap::new());
        assert_eq!(boxes.len(), 2);
        assert!(boxes[1].y >= boxes[0].y);
    }

    #[test]
    fn respects_height_and_background() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![],
            }],
        };
        let rules = aerova_css::parse("div { height: 60px; background: red; }");
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        assert_eq!(boxes[0].h, 60);
        assert_eq!(boxes[0].background(), Some("red"));
    }

    #[test]
    fn flex_row_places_side_by_side() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let rules = aerova_css::parse("div { display: flex; }");
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        assert_eq!(boxes.len(), 1);
        let row = &boxes[0];
        assert!(row.is_flex());
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[0].y, row.children[1].y);
        assert!(row.children[1].x > row.children[0].x);
    }

    #[test]
    fn flex_column_stacks() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "p".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "p".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let rules = aerova_css::parse("div { display: flex; flex-direction: column; }");
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        let row = &boxes[0];
        assert!(row.children[1].y >= row.children[0].y);
    }

    #[test]
    fn class_flex_applies() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![("class".to_string(), "row".to_string())],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let rules = aerova_css::parse(".row { display: flex; }");
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        assert!(boxes[0].is_flex());
        assert_eq!(boxes[0].children[0].y, boxes[0].children[1].y);
    }

    fn nested_doc() -> Document {
        Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "p".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        }
    }

    fn preorder_els(boxes: &[BoxNode], out: &mut Vec<Option<usize>>) {
        for b in boxes {
            out.push(b.el);
            preorder_els(&b.children, out);
        }
    }

    #[test]
    fn elements_carry_snapshot_indices_in_order() {
        for rules in [vec![], aerova_css::parse("div { display: flex; }")] {
            let boxes = layout(&nested_doc(), &rules, &std::collections::HashMap::new());
            let mut els = Vec::new();
            preorder_els(&boxes, &mut els);
            assert_eq!(
                els,
                vec![Some(0), Some(1), Some(1), Some(2), Some(2)],
                "rules: {rules:?}"
            );
        }
    }

    fn find_kind<'a>(boxes: &'a [BoxNode], kind: &str) -> Option<&'a BoxNode> {
        for b in boxes {
            if b.kind == kind {
                return Some(b);
            }
            if let Some(hit) = find_kind(&b.children, kind) {
                return Some(hit);
            }
        }
        None
    }

    #[test]
    fn hit_test_prefers_topmost_child() {
        let boxes = layout(&nested_doc(), &[], &std::collections::HashMap::new());
        let span = find_kind(&boxes, "span").unwrap();
        assert_eq!(span.el, Some(1));
        assert_eq!(hit_test(&boxes, span.x + 1, span.y + 1), Some(1));
        let text = find_kind(&span.children, "#text").unwrap();
        assert_eq!(hit_test(&boxes, text.x + 1, text.y + 1), Some(1));
        assert_eq!(hit_test(&boxes, 10, 500), None);
    }

    #[test]
    fn sides_expands_shorthand_and_longhand_wins() {
        let style = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        assert_eq!(sides(&style(&[("margin", "10px")]), "margin"), [10; 4]);
        assert_eq!(
            sides(&style(&[("margin", "10px 20px")]), "margin"),
            [10, 20, 10, 20]
        );
        assert_eq!(
            sides(&style(&[("margin", "1px 2px 3px 4px")]), "margin"),
            [1, 2, 3, 4]
        );
        assert_eq!(
            sides(
                &style(&[("margin", "5px"), ("margin-left", "7px")]),
                "margin"
            ),
            [5, 5, 5, 7]
        );
        assert_eq!(sides(&style(&[("margin", "auto")]), "margin"), [0; 4]);
        assert_eq!(sides(&style(&[]), "padding"), [0; 4]);
    }

    fn one_div(child: Node, css: &str) -> (Document, Vec<Rule>) {
        (
            Document {
                nodes: vec![Node::Element {
                    tag: "div".into(),
                    attrs: vec![],
                    children: vec![child],
                }],
            },
            aerova_css::parse(css),
        )
    }

    #[test]
    fn margin_separates_blocks() {
        let doc = Document {
            nodes: vec![
                Node::Element {
                    tag: "p".into(),
                    attrs: vec![],
                    children: vec![Node::Text("a".into())],
                },
                Node::Element {
                    tag: "p".into(),
                    attrs: vec![],
                    children: vec![Node::Text("b".into())],
                },
            ],
        };
        let boxes = layout(&doc, &aerova_css::parse("p { margin: 30px; }"), &std::collections::HashMap::new());
        assert_eq!(boxes[0].y, 30);
        assert_eq!(boxes[0].h, 20);
        assert_eq!(boxes[1].y, 110);
    }

    #[test]
    fn padding_insets_children_and_grows_box() {
        let (doc, rules) = one_div(Node::Text("hi".into()), "div { padding: 25px; }");
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        let div = &boxes[0];
        assert_eq!(div.w, 800);
        assert_eq!(div.h, 70);
        let text = &div.children[0];
        assert_eq!((text.x, text.y, text.w), (25, 25, 750));
    }

    #[test]
    fn padding_adds_to_declared_height() {
        let (doc, rules) = one_div(
            Node::Element {
                tag: "span".into(),
                attrs: vec![],
                children: vec![],
            },
            "div { height: 40px; padding: 10px; }",
        );
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        assert_eq!(boxes[0].h, 60);
    }

    #[test]
    fn horizontal_margin_shrinks_width() {
        let (doc, rules) = one_div(Node::Text("hi".into()), "div { margin: 0 40px; }");
        let boxes = layout(&doc, &rules, &std::collections::HashMap::new());
        assert_eq!(boxes[0].w, 720);
        assert_eq!(boxes[0].children[0].x, 40);
    }

    #[test]
    fn flex_gap_separates_items() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let boxes = layout(
            &doc,
            &aerova_css::parse("div { display: flex; gap: 10px; }"),
            &std::collections::HashMap::new(),
        );
        let row = &boxes[0];
        assert_eq!(row.children[0].x, 0);
        assert_eq!(row.children[0].w, 395);
        assert_eq!(row.children[1].x, 405);
        assert_eq!(row.children[0].y, row.children[1].y);
    }

    #[test]
    fn flex_gap_column_value_wins_for_row() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let boxes = layout(
            &doc,
            &aerova_css::parse("div { display: flex; gap: 2px 30px; }"),
            &std::collections::HashMap::new(),
        );
        assert_eq!(boxes[0].children[1].x, 415);
    }

    #[test]
    fn flex_item_children_sit_inside_their_box() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let boxes = layout(
            &doc,
            &aerova_css::parse(
                "div { display: flex; gap: 20px; padding: 30px; } span { padding: 5px; }",
            ),
            &std::collections::HashMap::new(),
        );
        let row = &boxes[0];
        assert_eq!(row.children[0].x, 30);
        for item in &row.children {
            let text = &item.children[0];
            assert_eq!(text.kind, "#text");
            assert!(
                text.x >= item.x,
                "text x {} left of item x {}",
                text.x,
                item.x
            );
            assert!(text.x < item.x + item.w, "text past item right edge");
            assert!(text.x == item.x + 5);
        }
        assert_eq!(
            row.children[1].x,
            row.children[0].x + row.children[0].w + 20
        );
    }

    #[test]
    fn flex_item_margin_offsets_and_caps() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "div".into(),
                attrs: vec![],
                children: vec![
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("a".into())],
                    },
                    Node::Element {
                        tag: "span".into(),
                        attrs: vec![],
                        children: vec![Node::Text("b".into())],
                    },
                ],
            }],
        };
        let boxes = layout(
            &doc,
            &aerova_css::parse("div { display: flex; } span { margin: 5px 10px; }"),
            &std::collections::HashMap::new(),
        );
        let row = &boxes[0];
        assert_eq!(row.children[0].x, 10);
        assert_eq!(row.children[0].w, 380);
        assert_eq!(row.children[0].y, 5);
        assert_eq!(row.h, 35);
    }
}
