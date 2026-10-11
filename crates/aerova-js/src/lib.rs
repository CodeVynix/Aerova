//! Aerova JS: stub by default, Boa engine behind `boa` feature.
//! Phase 6d makes timers delay-ordered via a logical clock (Aerova has no
//! event loop, so delays model ordering only) and events dispatchable with
//! real event objects, per-element listeners and one `DOMContentLoaded`.
//! Default keeps Lumora port std-only; `--features boa` gives real JS.

use std::cell::RefCell;

thread_local! {
    static STUB_WRITES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// One queryable DOM element snapshot for bindings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DomEl {
    /// Lowercased tag.
    pub tag: String,
    /// Raw id (may be empty).
    pub id: String,
    /// Lowercased class list.
    pub classes: Vec<String>,
    /// Visible text inside the element.
    pub text: String,
}

#[cfg(feature = "boa")]
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(feature = "boa")]
fn dom_to_json(dom: &[DomEl]) -> String {
    let mut out = String::from("[");
    for (i, e) in dom.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"tag\":\"{}\",\"id\":\"{}\",\"classes\":[{}],\"text\":\"{}\"}}",
            escape_json(&e.tag),
            escape_json(&e.id),
            e.classes
                .iter()
                .map(|c| format!("\"{}\"", escape_json(c)))
                .collect::<Vec<_>>()
                .join(","),
            escape_json(&e.text),
        ));
    }
    out.push(']');
    out
}

/// Evaluate JS source with stub (integers `1+2` or `document.write` scan).
pub fn eval_stub(source: &str) -> Result<String, String> {
    let s = source.trim().trim_end_matches(';').trim();
    if let Some((a, b)) = s.split_once('+') {
        let a_clean = a.trim().trim_matches('"').trim_matches('\'');
        let b_clean = b.trim().trim_matches('"').trim_matches('\'');
        let int_like = |v: &str| {
            !v.is_empty()
                && (v.bytes().all(|c| c.is_ascii_digit())
                    || (v.starts_with('-')
                        && v.len() > 1
                        && v[1..].bytes().all(|c| c.is_ascii_digit())))
        };
        if int_like(a_clean) && int_like(b_clean) {
            let x: i64 = a_clean
                .parse()
                .map_err(|_| "js-stub: left is not int".to_string())?;
            let y: i64 = b_clean
                .parse()
                .map_err(|_| "js-stub: right is not int".to_string())?;
            return Ok((x + y).to_string());
        }
    }
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
    use super::{dom_to_json, DomEl, STUB_WRITES};
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

    /// JS runtime shim: DOM query, element listeners, dispatch, timers.
    /// Needs `__DOM`, `__TITLE`, `__aerova_write`, `__aerova_log` from Rust.
    const SHIM: &str = r##"
var __wrappers = [];
var __timerId = 1;
var __now = 0;
var __queue = [];

function __match(e, sel) {
  if (sel === "*") return true;
  if (sel[0] === "#") return e.id === sel.slice(1);
  if (sel[0] === ".") {
    var parts = sel.slice(1).split(".");
    for (var i = 0; i < parts.length; i++) if (e.classes.indexOf(parts[i]) < 0) return false;
    return true;
  }
  var hi = sel.indexOf("#");
  if (hi > 0) return e.tag === sel.slice(0, hi) && e.id === sel.slice(hi + 1);
  var di = sel.indexOf(".");
  if (di > 0) {
    var t = sel.slice(0, di), cs = sel.slice(di + 1).split(".");
    if (e.tag !== t) return false;
    for (var j = 0; j < cs.length; j++) if (e.classes.indexOf(cs[j]) < 0) return false;
    return true;
  }
  return e.tag === sel;
}

function __wrap(i) {
  if (i === null || i === undefined || i < 0 || i >= __DOM.length) return null;
  if (__wrappers[i]) return __wrappers[i];
  var e = __DOM[i];
  var o = { tag: e.tag, id: e.id, textContent: e.text, classes: e.classes.slice() };
  o.__listeners = [];
  o.addEventListener = function (t, fn) { this.__listeners.push({ type: t, fn: fn }); };
  o.removeEventListener = function (t, fn) {
    var ls = this.__listeners;
    for (var k = 0; k < ls.length; k++) {
      if (ls[k].type === t && ls[k].fn === fn) { ls.splice(k, 1); return true; }
    }
    return false;
  };
  o.dispatchEvent = function (ev) { return __dispatch(this, ev); };
  __wrappers[i] = o;
  return o;
}

function __dispatch(target, ev) {
  if (typeof ev === 'string') ev = { type: ev };
  if (!ev) ev = {};
  var e = {
    type: ev.type || '',
    target: target,
    currentTarget: target,
    bubbles: ev.bubbles === undefined ? true : !!ev.bubbles,
    defaultPrevented: false,
    __stopped: false
  };
  e.preventDefault = function () { e.defaultPrevented = true; };
  e.stopPropagation = function () { e.__stopped = true; };
  var ls = target.__listeners || [];
  for (var k = 0; k < ls.length; k++) {
    if (ls[k].type !== e.type) continue;
    e.currentTarget = target;
    ls[k].fn(e);
    if (e.__stopped) break;
  }
  return !e.defaultPrevented;
}

var document = {
  __listeners: [],
  title: __TITLE,
  write: __aerova_write,
  getElementById: function (id) {
    for (var i = 0; i < __DOM.length; i++) if (__DOM[i].id === id) return __wrap(i);
    return null;
  },
  querySelector: function (sel) {
    for (var i = 0; i < __DOM.length; i++) if (__match(__DOM[i], sel)) return __wrap(i);
    return null;
  },
  querySelectorAll: function (sel) {
    var out = [];
    for (var i = 0; i < __DOM.length; i++) if (__match(__DOM[i], sel)) out.push(__wrap(i));
    return out;
  },
  addEventListener: function (t, fn) { this.__listeners.push({ type: t, fn: fn }); },
  removeEventListener: function (t, fn) {
    var ls = this.__listeners;
    for (var k = 0; k < ls.length; k++) if (ls[k].type === t && ls[k].fn === fn) { ls.splice(k, 1); return true; }
    return false;
  },
  dispatchEvent: function (ev) { return __dispatch(this, ev); }
};

var window = globalThis;
var console = { log: __aerova_log };

function setTimeout(fn, ms) {
  var id = __timerId++;
  __queue.push({ id: id, fn: fn, due: __now + (ms || 0) });
  return id;
}

function clearTimeout(id) {
  for (var i = 0; i < __queue.length; i++) {
    if (__queue[i].id === id) { __queue.splice(i, 1); return; }
  }
}
"##;

    /// Drains the timer queue after the main script, plus one DOMContentLoaded.
    /// `__now` advances to each due time so delays model ordering only, and a
    /// callback cap keeps the drain finite without an event loop.
    const FIRE: &str = r##"
var __fired = 0;
try { __dispatch(document, { type: 'DOMContentLoaded' }); } catch (e) {}
while (__queue.length > 0 && __fired < 10000) {
  var __bi = -1;
  for (var i = 0; i < __queue.length; i++) {
    if (__bi < 0 || __queue[i].due < __queue[__bi].due) __bi = i;
  }
  if (__bi < 0) break;
  var t = __queue.splice(__bi, 1)[0];
  if (t.due > __now) __now = t.due;
  __fired++;
  try {
    if (typeof t.fn === 'function') { t.fn(); } else { eval(t.fn); }
  } catch (e) {}
}
"##;

    /// Install bindings + DOM snapshot, then eval main script, timers, listeners.
    pub fn run_with_bindings(
        source: &str,
        title: &str,
        dom: &[DomEl],
    ) -> (Result<String, String>, Vec<String>, Vec<String>) {
        WRITES.with(|w| w.borrow_mut().clear());
        LOGS.with(|l| l.borrow_mut().clear());
        let mut ctx = Context::default();
        let setup = |ctx: &mut Context| -> Result<(), String> {
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
            let setup_src = format!(
                "var __DOM = {}; var __TITLE = {:?};",
                dom_to_json(dom),
                title
            );
            ctx.eval(Source::from_bytes(setup_src.as_str()))
                .map_err(|e| format!("boa-bind: {e}"))?;
            ctx.eval(Source::from_bytes(SHIM))
                .map_err(|e| format!("boa-bind: {e}"))?;
            Ok(())
        };
        if let Err(e) = setup(&mut ctx) {
            return (Err(e), vec![], vec![]);
        }
        let result = match ctx.eval(Source::from_bytes(source)) {
            Ok(v) => v
                .to_string(&mut ctx)
                .map(|s| s.to_std_string_escaped())
                .map_err(|e| format!("boa-stringify: {e}")),
            Err(e) => Err(format!("boa-eval: {e}")),
        };
        // Drain timers (and the single DOMContentLoaded) in the same context,
        // so callbacks registered by scripts run with the same bindings.
        let _ = ctx.eval(Source::from_bytes(FIRE));
        let writes = WRITES.with(|w| std::mem::take(&mut *w.borrow_mut()));
        let logs = LOGS.with(|l| std::mem::take(&mut *l.borrow_mut()));
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

/// Run one script with bindings and DOM snapshot.
pub fn run_script_with_dom(source: &str, title: &str, dom: &[DomEl]) -> ScriptOutput {
    #[cfg(feature = "boa")]
    {
        let (r, writes, logs) = boa_impl::run_with_bindings(source, title, dom);
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
        let _ = (title, dom);
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

/// Run one script with `document`/`console` bindings, capturing writes.
pub fn run_script(source: &str, title: &str) -> ScriptOutput {
    run_script_with_dom(source, title, &[])
}

/// Run many scripts in order with DOM snapshot, concatenating writes/logs.
#[must_use]
pub fn run_scripts_with_dom(sources: &[String], title: &str, dom: &[DomEl]) -> ScriptOutput {
    let mut all_writes = Vec::new();
    let mut all_logs = Vec::new();
    let mut last = String::new();
    let mut failed = false;
    for s in sources {
        let o = run_script_with_dom(s, title, dom);
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

/// Run many scripts in order, concatenating writes/logs.
#[must_use]
pub fn run_scripts(sources: &[String], title: &str) -> ScriptOutput {
    run_scripts_with_dom(sources, title, &[])
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

    #[cfg(feature = "boa")]
    fn fixture_dom() -> Vec<DomEl> {
        vec![
            DomEl {
                tag: "div".into(),
                id: "main".into(),
                classes: vec!["row".into()],
                text: "hello box".into(),
            },
            DomEl {
                tag: "span".into(),
                id: "".into(),
                classes: vec!["row".into()],
                text: "side note".into(),
            },
        ]
    }

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
            &[],
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["Page!".to_string()]);
        assert_eq!(logs, vec!["a b".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_queries_dom() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "document.write(document.getElementById('main').textContent + '|' + document.querySelector('.row').textContent + '|' + document.querySelectorAll('span').length);",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["hello box|hello box|1".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_timers_respect_delay_order() {
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "setTimeout(function(){ document.write('slow'); }, 5); setTimeout(function(){ document.write('fast'); }, 0); setTimeout(function(){ document.write('mid'); }, 2);",
            "T",
            &[],
        );
        assert_eq!(
            writes,
            vec!["fast".to_string(), "mid".to_string(), "slow".to_string()]
        );
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_clear_timeout_cancels() {
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "var id = setTimeout(function(){ document.write('never'); }, 0); clearTimeout(id); setTimeout(function(){ document.write('ran'); }, 1);",
            "T",
            &[],
        );
        assert_eq!(writes, vec!["ran".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_timer_chained_from_timer_fires_after() {
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "setTimeout(function(){ document.write('one'); setTimeout(function(){ document.write('two'); }, 0); }, 0);",
            "T",
            &[],
        );
        assert_eq!(writes, vec!["one".to_string(), "two".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_timers_fire_after_domcontentloaded() {
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "document.addEventListener('DOMContentLoaded', function(){ document.write('dcl'); }); setTimeout(function(){ document.write('timer'); }, 0);",
            "T",
            &[],
        );
        assert_eq!(writes, vec!["dcl".to_string(), "timer".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_dispatch_event_target_and_type() {
        let dom = fixture_dom();
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "var el = document.getElementById('main'); el.addEventListener('click', function(ev){ document.write(ev.target.id + ':' + ev.type + ':' + (ev.target === el)); }); el.dispatchEvent({ type: 'click' });",
            "T",
            &dom,
        );
        assert_eq!(writes, vec!["main:click:true".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_dispatch_prevent_default_and_return() {
        let dom = fixture_dom();
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "var el = document.querySelector('#main'); el.addEventListener('submit', function(ev){ ev.preventDefault(); document.write('dp:' + ev.defaultPrevented); }); var ok = el.dispatchEvent({ type: 'submit' }); document.write('ret:' + ok);",
            "T",
            &dom,
        );
        assert_eq!(writes, vec!["dp:true".to_string(), "ret:false".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_remove_event_listener_stops_calls() {
        let dom = fixture_dom();
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "var el = document.getElementById('main'); var h = function(){ document.write('hit'); }; el.addEventListener('click', h); el.dispatchEvent('click'); el.removeEventListener('click', h); el.dispatchEvent('click');",
            "T",
            &dom,
        );
        assert_eq!(writes, vec!["hit".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_element_identity_stable_across_queries() {
        let dom = fixture_dom();
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "var a = document.getElementById('main'); var b = document.querySelector('#main'); document.write('same:' + (a === b)); a.addEventListener('click', function(){ document.write('via-b'); }); b.dispatchEvent('click');",
            "T",
            &dom,
        );
        assert_eq!(writes, vec!["same:true".to_string(), "via-b".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_undispatched_listener_never_fires() {
        let (_r, writes, _) = boa_impl::run_with_bindings(
            "document.addEventListener('click', function(){ document.write('clicked'); });",
            "T",
            &[],
        );
        assert!(writes.is_empty());
    }
}
