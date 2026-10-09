//! Aerova CSS: rule parser + cascade (Phase 3).
//! Matches `*` and tag selectors; later rules win.

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
    /// Selector text, lowercased tag or `*`.
    pub selector: String,
    /// Declarations.
    pub declarations: Vec<Declaration>,
}

/// Parse `selector { prop: val; ... }` rules. Skips malformed parts, never panics.
#[must_use]
pub fn parse(css: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    for block in css.split('}') {
        let Some((sel, body)) = block.split_once('{') else {
            continue;
        };
        let selector = sel.trim().to_lowercase();
        if selector.is_empty() {
            continue;
        }
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
        rules.push(Rule {
            selector,
            declarations,
        });
    }
    rules
}

/// Compute specified style for a tag: merges `*` + exact-tag rules in order.
#[must_use]
pub fn specified_styles(tag: &str, rules: &[Rule]) -> Vec<(String, String)> {
    let tag = tag.to_lowercase();
    let mut out: Vec<(String, String)> = Vec::new();
    for r in rules {
        if r.selector == "*" || r.selector == tag {
            for d in &r.declarations {
                if let Some(slot) = out.iter_mut().find(|(k, _)| *k == d.property) {
                    slot.1 = d.value.clone();
                } else {
                    out.push((d.property.clone(), d.value.clone()));
                }
            }
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
    fn cascade_later_wins_and_star_applies() {
        let rules = parse("* { color: black; } p { color: red; } p { color: blue; }");
        let s = specified_styles("p", &rules);
        let color = s.iter().find(|(k, _)| k == "color").unwrap();
        assert_eq!(color.1, "blue");
        let div = specified_styles("div", &rules);
        assert_eq!(div.iter().find(|(k, _)| k == "color").unwrap().1, "black");
    }

    #[test]
    fn parses_px() {
        assert_eq!(parse_px("40px"), Some(40));
        assert_eq!(parse_px("12"), Some(12));
        assert_eq!(parse_px("auto"), None);
    }
}
