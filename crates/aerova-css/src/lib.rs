//! Aerova CSS: rule parser + cascade with compound selectors.
//! Supports `*`, `tag`, `.class`, `#id`, `[attr]`/`[attr="v"]`, compounds
//! (`tag.cls#id[attr]`), comma groups, descendant (`div p`), inline `style`.
//! Specificity: id=100, class-or-attr=10, tag=1, inline=1000, descendants sum.
//! Higher wins; ties break by order. No `>`, `+`, `~`, pseudo-classes.

/// A CSS declaration: `color: red`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// Property name, lowercased.
    pub property: String,
    /// Raw value.
    pub value: String,
}

/// A CSS rule: `p { color: red; }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Single selector text, lowercased (comma groups split at parse).
    pub selector: String,
    /// Declarations.
    pub declarations: Vec<Declaration>,
}

/// Parse `selector { prop: val; ... }` rules. Splits comma groups. Never panics.
#[must_use]
pub fn parse(css: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    for block in css.split('}') {
        let Some((sel, body)) = block.split_once('{') else {
            continue;
        };
        let mut declarations = Vec::new();
        for decl in body.split(';') {
            let Some((k, v)) = decl.split_once(':') else {
                continue;
            };
            let property = k.trim().to_lowercase();
            let value = v.trim().to_string();
            if !property.is_empty() && !value.is_empty() {
                declarations.push(Declaration { property, value });
            }
        }
        if declarations.is_empty() {
            continue;
        }
        for part in sel.split(',') {
            let selector = part.trim().to_lowercase();
            if selector.is_empty() {
                continue;
            }
            rules.push(Rule {
                selector,
                declarations: declarations.clone(),
            });
        }
    }
    rules
}

fn attr_value<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn class_list(attrs: &[(String, String)]) -> Vec<String> {
    attr_value(attrs, "class").map_or_else(Vec::new, |c| {
        c.split_whitespace().map(|w| w.to_lowercase()).collect()
    })
}

/// One compound selector: `div#id.cls[a="v"]`.
struct Compound {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attrs: Vec<(String, Option<String>)>,
}

fn take_name(s: &str, i: usize) -> (String, usize) {
    let b = s.as_bytes();
    let mut j = i;
    while j < b.len() && b[j] != b'#' && b[j] != b'.' && b[j] != b'[' {
        j += 1;
    }
    (s[i..j].to_string(), j)
}

fn parse_compound(s: &str) -> Option<Compound> {
    if s.is_empty() {
        return None;
    }
    if s == "*" {
        return Some(Compound {
            tag: None,
            id: None,
            classes: Vec::new(),
            attrs: Vec::new(),
        });
    }
    let mut c = Compound {
        tag: None,
        id: None,
        classes: Vec::new(),
        attrs: Vec::new(),
    };
    let b = s.as_bytes();
    let mut j = 0;
    while j < b.len() && b[j] != b'#' && b[j] != b'.' && b[j] != b'[' {
        j += 1;
    }
    let head = &s[..j];
    if head == "*" {
        if j < b.len() {
            return None;
        }
    } else if !head.is_empty() {
        c.tag = Some(head.to_string());
    }
    let mut i = j;
    while i < b.len() {
        match b[i] {
            b'#' => {
                let (v, ni) = take_name(s, i + 1);
                if v.is_empty() || c.id.is_some() {
                    return None;
                }
                c.id = Some(v);
                i = ni;
            }
            b'.' => {
                let (v, ni) = take_name(s, i + 1);
                if v.is_empty() {
                    return None;
                }
                c.classes.push(v);
                i = ni;
            }
            b'[' => {
                let rel = s[i..].find(']')?;
                let close = i + rel;
                let inner = s[i + 1..close].trim();
                let (name, val) = match inner.split_once('=') {
                    None => (inner.trim().to_string(), None),
                    Some((k, v)) => {
                        let v = v.trim();
                        let v = v
                            .strip_prefix('"')
                            .and_then(|x| x.strip_suffix('"'))
                            .or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
                            .unwrap_or(v);
                        (k.trim().to_string(), Some(v.to_string()))
                    }
                };
                if name.is_empty() {
                    return None;
                }
                c.attrs.push((name, val));
                i = close + 1;
            }
            _ => return None,
        }
    }
    if c.tag.is_none() && c.id.is_none() && c.classes.is_empty() && c.attrs.is_empty() {
        return None;
    }
    Some(c)
}

fn match_compound(c: &Compound, tag: &str, attrs: &[(String, String)]) -> Option<u32> {
    if let Some(t) = &c.tag {
        if t != tag {
            return None;
        }
    }
    if let Some(id) = &c.id {
        let mine = attr_value(attrs, "id")?.to_lowercase();
        if mine != *id {
            return None;
        }
    }
    if !c.classes.is_empty() {
        let have = class_list(attrs);
        if !c.classes.iter().all(|w| have.iter().any(|h| h == w)) {
            return None;
        }
    }
    for (name, want) in &c.attrs {
        let have = attr_value(attrs, name)?;
        if let Some(v) = want {
            if have.to_lowercase() != *v {
                return None;
            }
        }
    }
    let mut spec = 0u32;
    if c.tag.is_some() {
        spec += 1;
    }
    if c.id.is_some() {
        spec += 100;
    }
    spec += 10 * (c.classes.len() + c.attrs.len()) as u32;
    Some(spec)
}

/// Match selector against `(tag, attrs)` with ancestor chain for descendants.
/// Returns specificity on match; ancestor parts sum into it.
fn matches(
    selector: &str,
    tag: &str,
    attrs: &[(String, String)],
    ancestors: &[(&str, &[(String, String)])],
) -> Option<u32> {
    let parts: Vec<&str> = selector.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let mut spec = match_compound(&parse_compound(parts[parts.len() - 1])?, tag, attrs)?;
    let mut ai = ancestors.len();
    for part in parts[..parts.len() - 1].iter().rev() {
        let c = parse_compound(part)?;
        let mut found = None;
        while ai > 0 {
            ai -= 1;
            let (atag, aattrs) = ancestors[ai];
            if let Some(s) = match_compound(&c, atag, aattrs) {
                found = Some(s);
                break;
            }
        }
        spec += found?;
    }
    Some(spec)
}

/// Compute specified style for `(tag, attrs)`: specificity then order, later wins ties.
/// `ancestors` is the parent chain (root first) for descendant selectors.
/// An inline `style` attribute beats every rule.
#[must_use]
pub fn specified_styles(
    tag: &str,
    attrs: &[(String, String)],
    ancestors: &[(&str, &[(String, String)])],
    rules: &[Rule],
) -> Vec<(String, String)> {
    // UA default margins for block elements (like browsers).
    let mut hits: Vec<(u32, usize, String, String)> = Vec::new();
    let tag_lc = tag.to_lowercase();
    let ua_margin = match tag_lc.as_str() {
        "p" => Some("20px"),
        "h1" => Some("13px"),
        "h2" => Some("17px"),
        "h3" => Some("20px"),
        "h4" => Some("27px"),
        "h5" => Some("33px"),
        "h6" => Some("47px"),
        "ul" | "ol" => Some("20px"),
        "body" => Some("8px"),
        "blockquote" => Some("20px"),
        "pre" => Some("20px"),
        "hr" => Some("10px"),
        "figure" => Some("20px"),
        _ => None,
    };
    if let Some(m) = ua_margin {
        hits.push((0, usize::MAX, "margin".to_string(), m.to_string()));
    }
    for (idx, r) in rules.iter().enumerate() {
        if let Some(spec) = matches(&r.selector, tag, attrs, ancestors) {
            for d in &r.declarations {
                hits.push((spec, idx, d.property.clone(), d.value.clone()));
            }
        }
    }
    if let Some(inline) = attr_value(attrs, "style") {
        for decl in inline.split(';') {
            let Some((k, v)) = decl.split_once(':') else {
                continue;
            };
            let property = k.trim().to_lowercase();
            let value = v.trim().to_string();
            if !property.is_empty() && !value.is_empty() {
                hits.push((1000, usize::MAX, property, value));
            }
        }
    }
    hits.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut out: Vec<(String, String)> = Vec::new();
    for (_, _, k, v) in hits {
        if let Some(slot) = out.iter_mut().find(|(ek, _)| *ek == k) {
            slot.1 = v;
        } else {
            out.push((k, v));
        }
    }
    out
}

/// Parse `40px` or `40` to pixels.
#[must_use]
pub fn parse_px(value: &str) -> Option<u32> {
    let v = value
        .trim()
        .to_lowercase()
        .trim_end_matches("px")
        .trim()
        .to_string();
    v.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rule() {
        let r = parse("p { color: red; margin: 0; }");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].selector, "p");
        assert_eq!(r[0].declarations.len(), 2);
    }

    #[test]
    fn splits_comma_groups() {
        let r = parse("p, div { color: red; }");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn cascade_later_wins_and_star_applies() {
        let rules = parse("* { color: black; } p { color: red; } p { color: blue; }");
        let s = specified_styles("p", &[], &[], &rules);
        let color = s.iter().find(|(k, _)| k == "color").unwrap();
        assert_eq!(color.1, "blue");
        let div = specified_styles("div", &[], &[], &rules);
        assert_eq!(div.iter().find(|(k, _)| k == "color").unwrap().1, "black");
    }

    #[test]
    fn matches_classes_and_ids() {
        let rules = parse(".row { display: flex; } #main { color: red; } div.box { height: 9px; }");
        let cls = vec![("class".to_string(), "row".to_string())];
        assert!(specified_styles("div", &cls, &[], &rules)
            .iter()
            .any(|(k, v)| k == "display" && v == "flex"));
        let id = vec![("id".to_string(), "main".to_string())];
        assert_eq!(
            specified_styles("p", &id, &[], &rules)
                .iter()
                .find(|(k, _)| k == "color")
                .unwrap()
                .1,
            "red"
        );
        let both = vec![("class".to_string(), "box".to_string())];
        assert!(specified_styles("div", &both, &[], &rules)
            .iter()
            .any(|(k, _)| k == "height"));
        assert!(specified_styles("span", &both, &[], &rules)
            .iter()
            .all(|(k, _)| k != "height"));
    }

    #[test]
    fn specificity_id_beats_class_beats_tag() {
        let rules = parse("div { color: black; } .x { color: green; } #y { color: red; }");
        let attrs = vec![
            ("class".to_string(), "x".to_string()),
            ("id".to_string(), "y".to_string()),
        ];
        let s = specified_styles("div", &attrs, &[], &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "red");
    }

    fn attr(n: &str, v: &str) -> (String, String) {
        (n.to_string(), v.to_string())
    }

    #[test]
    fn matches_attr_selectors() {
        let rules = parse("[href] { color: blue; } a[href=\"x\"] { color: red; }");
        let link = vec![attr("href", "x")];
        let s = specified_styles("a", &link, &[], &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "red");
        let other = vec![attr("href", "y")];
        let s = specified_styles("a", &other, &[], &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "blue");
        let bare: Vec<(String, String)> = vec![];
        assert!(specified_styles("a", &bare, &[], &rules)
            .iter()
            .all(|(k, _)| k != "color"));
        let wrong_tag = vec![attr("href", "x")];
        assert_eq!(
            specified_styles("p", &wrong_tag, &[], &rules)
                .iter()
                .find(|(k, _)| k == "color")
                .unwrap()
                .1,
            "blue"
        );
    }

    #[test]
    fn matches_compound_class_and_attr() {
        let rules = parse(".btn[disabled] { color: gray; }");
        let both = vec![attr("class", "btn"), attr("disabled", "")];
        assert!(specified_styles("button", &both, &[], &rules)
            .iter()
            .any(|(k, _)| k == "color"));
        let no_attr = vec![attr("class", "btn")];
        assert!(specified_styles("button", &no_attr, &[], &rules)
            .iter()
            .all(|(k, _)| k != "color"));
    }

    #[test]
    fn matches_descendants_not_siblings() {
        let rules = parse("div p { color: red; }");
        let div: Vec<(String, String)> = vec![];
        let ancs = [("div", &div[..])];
        let s = specified_styles("p", &[], &ancs, &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "red");
        let s = specified_styles("p", &[], &[], &rules);
        assert!(s.iter().all(|(k, _)| k != "color"));
        let span_ancs: Vec<(String, String)> = vec![];
        let ancs = [("span", &span_ancs[..])];
        let s = specified_styles("p", &[], &ancs, &rules);
        assert!(s.iter().all(|(k, _)| k != "color"));
    }

    #[test]
    fn descendant_specificity_sums() {
        let rules = parse("p { color: black; } div p { color: red; } .x p { color: green; }");
        let div: Vec<(String, String)> = vec![];
        let ancs = [("div", &div[..])];
        let s = specified_styles("p", &[], &ancs, &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "red");
        let cls = [attr("class", "x")];
        let ancs = [("div", &cls[..])];
        let s = specified_styles("p", &[], &ancs, &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "green");
    }

    #[test]
    fn inline_style_beats_id_rule() {
        let rules = parse("#y { color: red; }");
        let attrs = vec![attr("id", "y"), attr("style", "color: blue; height: 9px")];
        let s = specified_styles("div", &attrs, &[], &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "blue");
        assert!(specified_styles("div", &attrs, &[], &rules)
            .iter()
            .any(|(k, v)| k == "height" && v == "9px"));
    }

    #[test]
    fn parses_px() {
        assert_eq!(parse_px("40px"), Some(40));
        assert_eq!(parse_px("12"), Some(12));
        assert_eq!(parse_px("auto"), None);
    }
}
