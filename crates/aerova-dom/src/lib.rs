//! Aerova DOM: arena-free minimal document tree (Phase 2).
//! Keeps Lumora port easy: std-only, no GUI deps.

/// A DOM node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// Element with tag, attributes, children.
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<Node>,
    },
    /// Text content.
    Text(String),
}

/// A parsed document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// Top-level nodes.
    pub nodes: Vec<Node>,
}

impl Document {
    /// Count all nodes recursively.
    #[must_use]
    pub fn count(&self) -> usize {
        fn walk(n: &Node, acc: &mut usize) {
            *acc += 1;
            if let Node::Element { children, .. } = n {
                for c in children {
                    walk(c, acc);
                }
            }
        }
        let mut total = 0;
        for n in &self.nodes {
            walk(n, &mut total);
        }
        total
    }

    /// Collect visible text.
    #[must_use]
    pub fn text(&self) -> String {
        fn walk(n: &Node, out: &mut String) {
            match n {
                Node::Text(t) => {
                    out.push_str(t);
                    out.push(' ');
                }
                Node::Element { children, .. } => {
                    for c in children {
                        walk(c, out);
                    }
                }
            }
        }
        let mut s = String::new();
        for n in &self.nodes {
            walk(n, &mut s);
        }
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_and_text() {
        let doc = Document {
            nodes: vec![Node::Element {
                tag: "p".into(),
                attrs: vec![],
                children: vec![Node::Text("hello world".into())],
            }],
        };
        assert_eq!(doc.count(), 2);
        assert_eq!(doc.text(), "hello world");
    }
}
