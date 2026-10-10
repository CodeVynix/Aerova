//! Aerova JS: stub by default, Boa engine behind `boa` feature.
//! Phase 6b adds `document.write` / `console.log` bindings + `run_scripts`.
//! Default keeps Lumora port std-only; `--features boa` gives real JS.

use std::cell::RefCell;

thread_local! {
    static STUB_WRITES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Evaluate JS source with stub (integers `1+2` only).
pub fn eval_stub(source: &str) -> Result<String, String> {
    let s = source.trim().trim_end_matches(';').trim();
    if let Some((a, b)) = s.split_once('+') {
        // Only treat as addition when both sides look like plain integers.
        // Otherwise fall through to document.write scan below.
        let a_trim = a.trim();
        let b_trim = b.trim();
        let a_clean = a_trim.trim_matches('"').trim_matches('\'');
        let b_clean = b_trim.trim_matches('"').trim_matches('\'');
        let a_is_int = !a_clean.is_empty()
            && (a_clean.bytes().all(|c| c.is_ascii_digit())
                || (a_clean.starts_with('-')
                    && a_clean.len() > 1
                    && a_clean[1..].bytes().all(|c| c.is_ascii_digit())));
        let b_is_int = !b_clean.is_empty()
            && (b_clean.bytes().all(|c| c.is_ascii_digit())
                || (b_clean.starts_with('-')
                    && b_clean.len() > 1
                    && b_clean[1..].bytes().all(|c| c.is_ascii_digit())));
        if a_is_int && b_is_int {
            let x: i64 = a_clean
                .parse()
                .map_err(|_| "js-stub: left is not int".to_string())?;
            let y: i64 = b_clean
                .parse()
                .map_err(|_| "js-stub: right is not int".to_string())?;
            return Ok((x + y).to_string());
        }
    }
    // Fall back to write-scan so `<script>document.write('hi')</script>` works without Boa.
    let writes = scan_writes(source);
    if !writes.is_empty() {
        STUB_WRITES.with(|w| w.borrow_mut().extend(writes.clone()));
        return Ok(writes.join(""));
    }
    Err(
        "js-stub: only `int+int` or `document.write` supported (rebuild with --features boa)"
            .to_string(),
    )
}

/// Scan `document.write('...')` / `document.write("...")` calls, return args.
fn scan_writes(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(i) = rest.find("document.write") {
        let after = &rest[i + "document.write".len()..];
        let open = after.find('(');
        let Some(o) = open else { break };
        let args = &after[o + 1..];
        let args = args.trim_start();
        let quote = args.chars().next();
        let Some(q) = quote else { break };
        if q != '"' && q != '\'' {
            rest = &args[1..];
            continue;
        }
        let mut s = String::new();
        let mut esc = false;
        let mut end = None;
        for (idx, ch) in args[1..].char_indices() {
            if esc {
                s.push(ch);
                esc = false;
            } else if ch == '\\' {
                esc = true;
            } else if ch == q {
                end = Some(idx);
                break;
            } else {
                s.push(ch);
            }
        }
        if end.is_some() {
            out.push(s);
            rest = &args[1..];
        } else {
            break;
        }
    }
    out
}

#[cfg(feature = "boa")]
mod boa_impl {
    use super::STUB_WRITES;
    use boa_engine::{
        js_string, native_function::NativeFunction, Context, JsResult, JsValue, Source,
    };
    use std::cell::RefCell;

    thread_local! {
        static WRITES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static LOGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn document_write(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let s = args
            .first()
            .map(|v| v.to_string(ctx))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        WRITES.with(|w| w.borrow_mut().push(s));
        Ok(JsValue::undefined())
    }

    fn console_log(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
        let mut parts = Vec::new();
        for a in args {
            parts.push(
                a.to_string(ctx)
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default(),
            );
        }
        LOGS.with(|l| l.borrow_mut().push(parts.join(" ")));
        Ok(JsValue::undefined())
    }

    /// Evaluate with Boa engine. Returns stringified value or error string.
    pub fn eval_boa(source: &str) -> Result<String, String> {
        let mut ctx = Context::default();
        match ctx.eval(Source::from_bytes(source)) {
            Ok(v) => v
                .to_string(&mut ctx)
                .map(|s| s.to_std_string_escaped())
                .map_err(|e| format!("boa-stringify: {e}")),
            Err(e) => Err(format!("boa-eval: {e}")),
        }
    }

    /// Install `document`/`console`/`window` via JS so we avoid typed-key API churn.
    fn install_bindings(ctx: &mut Context, title: &str) -> Result<(), String> {
        ctx.register_global_callable(
            js_string!("__aerova_write"),
            1,
            NativeFunction::from_fn_ptr(document_write),
        )
        .map_err(|e| format!("boa-bind: {e}"))?;
        ctx.register_global_callable(
            js_string!("__aerova_log"),
            1,
            NativeFunction::from_fn_ptr(console_log),
        )
        .map_err(|e| format!("boa-bind: {e}"))?;
        let setup = format!(
            "var document = {{ title: {:?}, write: __aerova_write }}; var window = globalThis; var console = {{ log: __aerova_log }};",
            title
        );
        ctx.eval(Source::from_bytes(setup.as_str()))
            .map_err(|e| format!("boa-bind: {e}"))?;
        Ok(())
    }

    /// Run script with `document`/`console`/`window` bindings.
    /// Returns `(result, writes, logs)`.
    pub fn run_with_bindings(
        source: &str,
        title: &str,
    ) -> (Result<String, String>, Vec<String>, Vec<String>) {
        WRITES.with(|w| w.borrow_mut().clear());
        LOGS.with(|l| l.borrow_mut().clear());
        let mut ctx = Context::default();
        if let Err(e) = install_bindings(&mut ctx, title) {
            return (Err(e), vec![], vec![]);
        }
        let result = match ctx.eval(Source::from_bytes(source)) {
            Ok(v) => v
                .to_string(&mut ctx)
                .map(|s| s.to_std_string_escaped())
                .map_err(|e| format!("boa-stringify: {e}")),
            Err(e) => Err(format!("boa-eval: {e}")),
        };
        let writes = WRITES.with(|w| std::mem::take(&mut *w.borrow_mut()));
        let logs = LOGS.with(|l| std::mem::take(&mut *l.borrow_mut()));
        // Mirror Boa writes into stub store so mixed callers see them.
        STUB_WRITES.with(|w| w.borrow_mut().extend(writes.clone()));
        (result, writes, logs)
    }
}

#[cfg(feature = "boa")]
/// Evaluate with Boa engine. Returns stringified value or error string.
pub fn eval_boa(source: &str) -> Result<String, String> {
    boa_impl::eval_boa(source)
}

/// Output of a script run with DOM bindings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScriptOutput {
    /// Stringified completion value or error.
    pub result: String,
    /// `document.write` captures in order.
    pub writes: Vec<String>,
    /// `console.log` lines in order.
    pub logs: Vec<String>,
    /// True if evaluation failed.
    pub failed: bool,
}

/// Run one script with `document`/`console` bindings, capturing writes.
/// Works with and without `boa`: stub scans `document.write`, Boa really runs JS.
pub fn run_script(source: &str, title: &str) -> ScriptOutput {
    #[cfg(feature = "boa")]
    {
        let (r, writes, logs) = boa_impl::run_with_bindings(source, title);
        match r {
            Ok(v) => ScriptOutput {
                result: v,
                writes,
                logs,
                failed: false,
            },
            Err(e) => ScriptOutput {
                result: e,
                writes,
                logs,
                failed: true,
            },
        }
    }
    #[cfg(not(feature = "boa"))]
    {
        let _ = title;
        let writes = scan_writes(source);
        if writes.is_empty() {
            match eval_stub(source) {
                Ok(v) => ScriptOutput {
                    result: v,
                    writes: vec![],
                    logs: vec![],
                    failed: false,
                },
                Err(e) => ScriptOutput {
                    result: e,
                    writes: vec![],
                    logs: vec![],
                    failed: true,
                },
            }
        } else {
            ScriptOutput {
                result: writes.join(""),
                writes,
                logs: vec![],
                failed: false,
            }
        }
    }
}

/// Run many scripts in order, concatenating writes/logs.
#[must_use]
pub fn run_scripts(sources: &[String], title: &str) -> ScriptOutput {
    let mut all_writes = Vec::new();
    let mut all_logs = Vec::new();
    let mut last = String::new();
    let mut failed = false;
    for s in sources {
        let o = run_script(s, title);
        if !o.result.is_empty() {
            last = o.result.clone();
        }
        all_writes.extend(o.writes);
        all_logs.extend(o.logs);
        if o.failed {
            failed = true;
        }
    }
    ScriptOutput {
        result: last,
        writes: all_writes,
        logs: all_logs,
        failed,
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

    #[test]
    fn captures_document_write_stub_or_boa() {
        let o = run_script("document.write('hello');", "T");
        assert!(!o.failed);
        assert_eq!(o.writes, vec!["hello".to_string()]);
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

    #[cfg(feature = "boa")]
    #[test]
    fn boa_bindings_title_and_log() {
        let (r, writes, logs) = boa_impl::run_with_bindings(
            "document.write(document.title + '!'); console.log('a','b');",
            "Page",
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["Page!".to_string()]);
        assert_eq!(logs, vec!["a b".to_string()]);
    }
}
