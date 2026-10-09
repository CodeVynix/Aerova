//! Aerova HTML: tiny tokenizer + tree builder (Phase 2).
//! Supports: <p> <div> <a href> <title> <b> <i> text. Rest becomes text.

use aerova_dom::{Document, Node};

/// Open element on the stack.
type OpenElement = (String, Vec<(String, String)>, Vec<Node>);

fn parse_tag(s: &str) -> (String, Vec<(String, String)>) {
    let s = s.trim().trim_end_matches('/').trim();
    let mut parts = s.split_whitespace();
    let tag = parts.next().unwrap_or("div").to_lowercase();
    let mut attrs = Vec::new();
    for p in parts {
        if let Some((k, v)) = p.split_once('=') {
            let v = v.trim_matches('"').trim_matches('\'').to_string();
            attrs.push((k.to_lowercase(), v));
        }
    }
    (tag, attrs)
}

fn flush_text(buf: &mut String, stack: &mut [OpenElement], root: &mut Vec<Node>) {
    let t = buf.trim();
    if !t.is_empty() {
        let n = Node::Text(t.to_string());
        if let Some((_, _, kids)) = stack.last_mut() {
            kids.push(n);
        } else {
            root.push(n);
        }
    }
    buf.clear();
}

/// Parse HTML text into a [`Document`]. Never panics on malformed input.
#[must_use]
pub fn parse(html: &str) -> Document {
    let mut root: Vec<Node> = Vec::new();
    let mut stack: Vec<OpenElement> = Vec::new();
    let mut buf = String::new();
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag_buf = String::new();
            for ch in chars.by_ref() {
                if ch == '>' {
                    break;
                }
                tag_buf.push(ch);
            }
            flush_text(&mut buf, &mut stack, &mut root);
            let tb = tag_buf.trim();
            if tb.starts_with('!') || tb.starts_with('?') {
                continue;
            }
            if let Some(after) = tb.strip_prefix('/') {
                let name = after.split_whitespace().next().unwrap_or("").to_lowercase();
                if let Some((open, attrs, kids)) = stack.pop() {
                    let node = Node::Element {
                        tag: open,
                        attrs,
                        children: kids,
                    };
                    if let Some((_, _, parent_kids)) = stack.last_mut() {
                        parent_kids.push(node);
                    } else {
                        root.push(node);
                    }
                    let _ = name;
                }
            } else {
                let (tag, attrs) = parse_tag(&tag_buf);
                let self_closing = tag_buf.trim_end().ends_with('/')
                    || tag == "br"
                    || tag == "img"
                    || tag == "meta"
                    || tag == "link";
                if self_closing {
                    let node = Node::Element {
                        tag,
                        attrs,
                        children: vec![],
                    };
                    if let Some((_, _, kids)) = stack.last_mut() {
                        kids.push(node);
                    } else {
                        root.push(node);
                    }
                } else {
                    stack.push((tag, attrs, Vec::new()));
                }
            }
        } else {
            buf.push(c);
        }
    }
    flush_text(&mut buf, &mut stack, &mut root);
    while let Some((tag, attrs, kids)) = stack.pop() {
        let node = Node::Element {
            tag,
            attrs,
            children: kids,
        };
        if let Some((_, _, parent_kids)) = stack.last_mut() {
            parent_kids.push(node);
        } else {
            root.push(node);
        }
    }
    Document { nodes: root }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paragraph() {
        let d = parse("<p>hello <b>world</b></p>");
        assert_eq!(d.count(), 4);
        assert_eq!(d.text(), "hello world");
    }

    #[test]
    fn malformed_never_panics() {
        let d = parse("<div><p>oops");
        assert!(d.count() >= 2);
    }
}
