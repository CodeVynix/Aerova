//! Aerova CSS: rule parser + cascade with class/id selectors.
//! Supports `*`, `tag`, `.class`, `#id`, `tag.class`, `tag#id`, comma groups.
//! Specificity: id=100, class=10, tag=1. Higher wins; ties break by order.

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

/// Match selector against `(tag, attrs)`. Returns specificity on match.
fn matches(selector: &str, tag: &str, attrs: &[(String, String)]) -> Option<u32> {
    if selector == "*" {
        return Some(0);
    }
    // id-only: #main
    if let Some(id) = selector.strip_prefix('#') {
        let mine = attr_value(attrs, "id")?.to_lowercase();
        return if mine == id { Some(100) } else { None };
    }
    // class-only: .row (single class for v1; .a.b checks all parts)
    if let Some(stripped) = selector.strip_prefix('.') {
        let want: Vec<&str> = stripped.split('.').collect();
        if want.is_empty() || want.iter().any(|w| w.is_empty()) {
            return None;
        }
        let have = class_list(attrs);
        return if want.iter().all(|w| have.iter().any(|h| h == w)) {
            Some(10 * want.len() as u32)
        } else {
            None
        };
    }
    // tag#id
    if let Some((t, id)) = selector.split_once('#') {
        if t != tag {
            return None;
        }
        let mine = attr_value(attrs, "id")?.to_lowercase();
        return if mine == id { Some(101) } else { None };
    }
    // tag.class(.class2...)
    if selector.contains('.') {
        let mut parts = selector.split('.');
        let t = parts.next().unwrap_or("");
        if t != tag {
            return None;
        }
        let want: Vec<&str> = parts.collect();
        if want.is_empty() || want.iter().any(|w| w.is_empty()) {
            return None;
        }
        let have = class_list(attrs);
        return if want.iter().all(|w| have.iter().any(|h| h == w)) {
            Some(1 + 10 * want.len() as u32)
        } else {
            None
        };
    }
    // plain tag
    if selector == tag {
        return Some(1);
    }
    None
}

/// Compute specified style for `(tag, attrs)`: specificity then order, later wins ties.
#[must_use]
pub fn specified_styles(
    tag: &str,
    attrs: &[(String, String)],
    rules: &[Rule],
) -> Vec<(String, String)> {
    let tag = tag.to_lowercase();
    let mut hits: Vec<(u32, usize, String, String)> = Vec::new();
    for (idx, r) in rules.iter().enumerate() {
        if let Some(spec) = matches(&r.selector, &tag, attrs) {
            for d in &r.declarations {
                hits.push((spec, idx, d.property.clone(), d.value.clone()));
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
        let s = specified_styles("p", &[], &rules);
        let color = s.iter().find(|(k, _)| k == "color").unwrap();
        assert_eq!(color.1, "blue");
        let div = specified_styles("div", &[], &rules);
        assert_eq!(div.iter().find(|(k, _)| k == "color").unwrap().1, "black");
    }

    #[test]
    fn matches_classes_and_ids() {
        let rules = parse(".row { display: flex; } #main { color: red; } div.box { height: 9px; }");
        let cls = vec![("class".to_string(), "row".to_string())];
        assert!(specified_styles("div", &cls, &rules)
            .iter()
            .any(|(k, v)| k == "display" && v == "flex"));
        let id = vec![("id".to_string(), "main".to_string())];
        assert_eq!(
            specified_styles("p", &id, &rules)
                .iter()
                .find(|(k, _)| k == "color")
                .unwrap()
                .1,
            "red"
        );
        let both = vec![("class".to_string(), "box".to_string())];
        assert!(specified_styles("div", &both, &rules)
            .iter()
            .any(|(k, _)| k == "height"));
        assert!(specified_styles("span", &both, &rules)
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
        let s = specified_styles("div", &attrs, &rules);
        assert_eq!(s.iter().find(|(k, _)| k == "color").unwrap().1, "red");
    }

    #[test]
    fn parses_px() {
        assert_eq!(parse_px("40px"), Some(40));
        assert_eq!(parse_px("12"), Some(12));
        assert_eq!(parse_px("auto"), None);
    }
}
