//! Aerova JS: stub trait now, Boa integration in Phase 6 (behind `boa` feature).
//! This keeps v0.1 std-only and Lumora-portable; Boa pulls many deps.

/// Evaluate JS source, return stringified result or error.
/// v0.1: handles `1+1`-style integers only; full engine is TODO(boa).
pub fn eval_stub(source: &str) -> Result<String, String> {
    let s = source.trim().trim_end_matches(';').trim();
    if let Some((a, b)) = s.split_once('+') {
        let x: i64 = a
            .trim()
            .parse()
            .map_err(|_| "js-stub: left is not int".to_string())?;
        let y: i64 = b
            .trim()
            .parse()
            .map_err(|_| "js-stub: right is not int".to_string())?;
        return Ok((x + y).to_string());
    }
    Err("js-stub: only `int+int` supported until Boa lands".to_string())
    // TODO(phase-6): add `boa` feature with `boa_engine::Context` and DOM bindings.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(eval_stub("1+2;"), Ok("3".to_string()));
    }
}
