//! Aerova layout: block stacking + flex row/column (Phase 7).
//! Grid later.

use aerova_css::{parse_px, specified_styles, Rule};
use aerova_dom::{Document, Node};

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
    /// Children.
    pub children: Vec<BoxNode>,
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
#[must_use]
pub fn layout(doc: &Document, rules: &[Rule]) -> Vec<BoxNode> {
    let mut y = 0;
    let mut out = Vec::new();
    for n in &doc.nodes {
        // Skip head/style/title/script at top level (not visual).
        if let Node::Element { tag, .. } = n {
            if tag == "style" || tag == "title" || tag == "head" || tag == "script" {
                continue;
            }
        }
        out.push(layout_node(n, 0, &mut y, rules));
    }
    out
}

fn styled_height(tag: &str, rules: &[Rule]) -> Option<u32> {
    for (k, v) in specified_styles(tag, rules) {
        if k == "height" {
            if let Some(px) = parse_px(&v) {
                return Some(px);
            }
        }
    }
    None
}

fn has_width_decl(tag: &str, rules: &[Rule]) -> Option<u32> {
    for (k, v) in specified_styles(tag, rules) {
        if k == "width" {
            if let Some(px) = parse_px(&v) {
                return Some(px);
            }
        }
    }
    None
}

fn styled_width(tag: &str, x: u32, rules: &[Rule]) -> u32 {
    let base = 800u32.saturating_sub(x);
    if let Some(px) = has_width_decl(tag, rules) {
        return px.min(base);
    }
    base
}

fn is_flex_tag(tag: &str, rules: &[Rule]) -> bool {
    specified_styles(tag, rules)
        .iter()
        .any(|(k, v)| k == "display" && v == "flex")
}

fn flex_direction(tag: &str, rules: &[Rule]) -> &'static str {
    for (k, v) in specified_styles(tag, rules) {
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

fn layout_node(n: &Node, x: u32, y: &mut u32, rules: &[Rule]) -> BoxNode {
    match n {
        Node::Text(t) => {
            let b = BoxNode {
                kind: "#text".into(),
                x,
                y: *y,
                w: 800u32.saturating_sub(x),
                h: 20,
                style: Vec::new(),
                children: vec![],
            };
            *y += 20;
            let _ = t;
            b
        }
        Node::Element { tag, children, .. } => {
            if tag == "style" || tag == "title" || tag == "head" || tag == "script" {
                return BoxNode {
                    kind: tag.clone(),
                    x,
                    y: *y,
                    w: 0,
                    h: 0,
                    style: specified_styles(tag, rules),
                    children: vec![],
                };
            }
            if is_flex_tag(tag, rules) && flex_direction(tag, rules) == "row" {
                return layout_flex_row(tag, children, x, y, rules);
            }
            let start = *y;
            let style = specified_styles(tag, rules);
            let w = styled_width(tag, x, rules);
            let mut kids = Vec::new();
            for c in children {
                // Skip non-visual elements inside flow.
                if let Node::Element { tag: ct, .. } = c {
                    if ct == "style" || ct == "title" || ct == "head" || ct == "script" {
                        continue;
                    }
                }
                kids.push(layout_node(c, x + 10, y, rules));
            }
            let content_h = *y - start;
            let wanted = styled_height(tag, rules).unwrap_or(0);
            let h = content_h.max(if kids.is_empty() {
                wanted.max(20)
            } else {
                wanted
            });
            if h > content_h {
                *y = start + h;
            }
            BoxNode {
                kind: tag.clone(),
                x,
                y: start,
                w,
                h,
                style,
                children: kids,
            }
        }
    }
}

/// Flex row: children share one line, x advances, y stays at row top.
/// Unspecified child widths split the remaining space equally.
fn layout_flex_row(tag: &str, children: &[Node], x: u32, y: &mut u32, rules: &[Rule]) -> BoxNode {
    let start_y = *y;
    let style = specified_styles(tag, rules);
    let w = styled_width(tag, x, rules);
    // Collect visual children only.
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
    let share = w / n;
    let mut kids = Vec::new();
    let mut cx = x;
    let mut max_h = 0;
    for c in visual {
        // Fixed width for this item if its own tag declares one.
        let item_w = match c {
            Node::Text(_) => share,
            Node::Element { tag: ct, .. } => has_width_decl(ct, rules)
                .unwrap_or(share)
                .min(w.saturating_sub(cx - x)),
        };
        let mut row_y = start_y;
        let mut item = layout_flex_item(c, cx, &mut row_y, item_w, rules);
        // Force item into row slot.
        item.x = cx;
        item.y = start_y;
        if item.w == 0 || item.w > item_w {
            item.w = item_w;
        }
        max_h = max_h.max(item.h);
        kids.push(item);
        cx += item_w;
    }
    let wanted = styled_height(tag, rules).unwrap_or(0);
    let h = max_h.max(wanted).max(if kids.is_empty() { 20 } else { 0 });
    *y = start_y + h;
    BoxNode {
        kind: tag.to_string(),
        x,
        y: start_y,
        w,
        h,
        style,
        children: kids,
    }
}

/// Lay out a single flex item with a capped width, without advancing outer `y`.
fn layout_flex_item(n: &Node, x: u32, _row_y: &mut u32, cap_w: u32, rules: &[Rule]) -> BoxNode {
    match n {
        Node::Text(_) => BoxNode {
            kind: "#text".into(),
            x,
            y: 0,
            w: cap_w,
            h: 20,
            style: Vec::new(),
            children: vec![],
        },
        Node::Element { tag, children, .. } => {
            let style = specified_styles(tag, rules);
            let mut inner_y = 0;
            let mut kids = Vec::new();
            for c in children {
                if let Node::Element { tag: ct, .. } = c {
                    if ct == "style" || ct == "title" || ct == "head" || ct == "script" {
                        continue;
                    }
                }
                kids.push(layout_node(c, 0, &mut inner_y, rules));
            }
            let content_h = inner_y;
            let wanted = styled_height(tag, rules).unwrap_or(0);
            let h = content_h.max(if kids.is_empty() {
                wanted.max(20)
            } else {
                wanted
            });
            let w = has_width_decl(tag, rules).unwrap_or(cap_w).min(cap_w);
            BoxNode {
                kind: tag.clone(),
                x,
                y: 0,
                w,
                h,
                style,
                children: kids,
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
        let boxes = layout(&doc, &[]);
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
        let boxes = layout(&doc, &rules);
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
        let boxes = layout(&doc, &rules);
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
        let boxes = layout(&doc, &rules);
        let row = &boxes[0];
        assert!(row.children[1].y >= row.children[0].y);
    }
}
