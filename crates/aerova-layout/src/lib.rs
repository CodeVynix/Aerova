//! Aerova layout: block stacking + CSS heights/widths (Phase 4).
//! Full flex/grid later.

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
}

/// Lay out document top-to-bottom with CSS rules. Width 800, row 20px default.
#[must_use]
pub fn layout(doc: &Document, rules: &[Rule]) -> Vec<BoxNode> {
    let mut y = 0;
    let mut out = Vec::new();
    for n in &doc.nodes {
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

fn styled_width(tag: &str, x: u32, rules: &[Rule]) -> u32 {
    let base = 800u32.saturating_sub(x);
    for (k, v) in specified_styles(tag, rules) {
        if k == "width" {
            if let Some(px) = parse_px(&v) {
                return px.min(base);
            }
        }
    }
    base
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
            let start = *y;
            let style = specified_styles(tag, rules);
            let w = styled_width(tag, x, rules);
            let mut kids = Vec::new();
            for c in children {
                // Skip style/title/head content from visual flow.
                if tag == "style" || tag == "title" || tag == "head" {
                    continue;
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
}
