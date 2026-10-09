//! Aerova CSS: minimal rule parser + cascade stub (Phase 3).

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
}
