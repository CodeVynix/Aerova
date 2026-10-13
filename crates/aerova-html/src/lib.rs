//! Aerova HTML: tiny tokenizer + tree builder (Phase 2).
//! Supports: <p> <div> <a href> <title> <b> <i> text. Rest becomes text.

use aerova_dom::{Document, Node};

/// Open element on the stack.
type OpenElement = (String, Vec<(String, String)>, Vec<Node>);

fn parse_tag(s: &str) -> (String, Vec<(String, String)>) {
    let s = s.trim().trim_end_matches('/').trim();
    let mut chars = s.chars().peekable();
    let mut tag = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            break;
        }
        tag.push(c);
        chars.next();
    }
    let tag = if tag.is_empty() {
        "div".to_string()
    } else {
        tag.to_lowercase()
    };
    let mut attrs = Vec::new();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek() == Some(&'=') {
            chars.next();
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
            let mut val = String::new();
            if chars.peek() == Some(&'"') || chars.peek() == Some(&'\'') {
                let q = chars.next().unwrap_or('"');
                for c in chars.by_ref() {
                    if c == q {
                        break;
                    }
                    val.push(c);
                }
            } else {
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    val.push(c);
                    chars.next();
                }
            }
            if !key.is_empty() {
                attrs.push((key.to_lowercase(), val));
            }
        } else if !key.is_empty() {
            attrs.push((key.to_lowercase(), String::new()));
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

    fn element_attrs(html: &str) -> Vec<(String, String)> {
        match &parse(html).nodes[..] {
            [Node::Element { attrs, .. }] => attrs.clone(),
            _ => panic!("want one element: {html}"),
        }
    }

    #[test]
    fn quoted_attr_values_keep_spaces() {
        let attrs = element_attrs("<div style=\"background: blue; height: 20px;\">hi</div>");
        assert_eq!(
            attrs,
            vec![(
                "style".to_string(),
                "background: blue; height: 20px;".to_string()
            )]
        );
        let attrs = element_attrs("<a href='x y'>l</a>");
        assert_eq!(attrs, vec![("href".to_string(), "x y".to_string())]);
    }

    #[test]
    fn unquoted_and_bare_attrs_behave() {
        let attrs = element_attrs("<a href=http://x>l</a>");
        assert_eq!(attrs, vec![("href".to_string(), "http://x".to_string())]);
        let attrs = element_attrs("<button disabled>x</button>");
        assert_eq!(attrs, vec![("disabled".to_string(), String::new())]);
    }
}
