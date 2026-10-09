//! Aerova layout: block stacking stub (Phase 4). Full flex/grid later.

use aerova_dom::{Document, Node};

/// A laid-out box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxNode {
    /// Tag or `#text`.
    pub kind: String,
    /// x, y, width, height in px.
    pub x: u32,
    /// y position.
    pub y: u32,
    /// width.
    pub w: u32,
    /// height.
    pub h: u32,
    /// Children.
    pub children: Vec<BoxNode>,
}

/// Lay out document top-to-bottom, each block 20px tall, width 800.
#[must_use]
pub fn layout(doc: &Document) -> Vec<BoxNode> {
    let mut y = 0;
    let mut out = Vec::new();
    for n in &doc.nodes {
        out.push(layout_node(n, 0, &mut y));
    }
    out
}

fn layout_node(n: &Node, x: u32, y: &mut u32) -> BoxNode {
    match n {
        Node::Text(t) => {
            let b = BoxNode {
                kind: "#text".into(),
                x,
                y: *y,
                w: 800,
                h: 20,
                children: vec![],
            };
            *y += 20;
            let _ = t;
            b
        }
        Node::Element { tag, children, .. } => {
            let start = *y;
            let mut kids = Vec::new();
            for c in children {
                kids.push(layout_node(c, x + 10, y));
            }
            if kids.is_empty() {
                *y += 20;
            }
            BoxNode {
                kind: tag.clone(),
                x,
                y: start,
                w: 800,
                h: *y - start,
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
        let boxes = layout(&doc);
        assert_eq!(boxes.len(), 2);
        assert!(boxes[1].y >= boxes[0].y);
    }
}
