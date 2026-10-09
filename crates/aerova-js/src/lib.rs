//! Aerova JS: stub by default, Boa engine behind `boa` feature.
//! Default keeps Lumora port std-only; `--features boa` gives real JS.

/// Evaluate JS source with stub (integers `1+2` only).
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
    Err("js-stub: only `int+int` supported (rebuild with --features boa)".to_string())
}

#[cfg(feature = "boa")]
/// Evaluate with Boa engine. Returns stringified value or error string.
pub fn eval_boa(source: &str) -> Result<String, String> {
    use boa_engine::{Context, Source};
    let mut ctx = Context::default();
    match ctx.eval(Source::from_bytes(source)) {
        Ok(v) => v
            .to_string(&mut ctx)
            .map(|s| s.to_std_string_escaped())
            .map_err(|e| format!("boa-stringify: {e}")),
        Err(e) => Err(format!("boa-eval: {e}")),
    }
}

/// Unified eval: Boa when `boa` feature is on, else stub.
pub fn eval(source: &str) -> Result<String, String> {
    #[cfg(feature = "boa")]
    {
        eval_boa(source)
    }
    #[cfg(not(feature = "boa"))]
    {
        eval_stub(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(eval_stub("1+2;"), Ok("3".to_string()));
    }

    #[test]
    fn unified_eval_adds() {
        let out = eval("1+2;");
        assert!(out.is_ok());
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_handles_closures_and_strings() {
        assert_eq!(eval("1+2;").unwrap(), "3");
        let s = eval("'hi'+' there';").unwrap();
        assert!(s.contains("hi"));
        let f = eval("(function(a,b){return a*b;})(6,7);").unwrap();
        assert_eq!(f, "42");
    }
}
