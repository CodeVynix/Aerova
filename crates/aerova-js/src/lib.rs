//! Aerova JS: stub by default, Boa engine behind `boa` feature.
//! Phase 6e extends the element wrapper (`getAttribute`/`setAttribute`, `className`,
//! `classList`) and makes dispatch target-scoped: events walk the parent chain
//! and terminate at `document`, so delegation works. Default keeps Lumora
//! port std-only; `--features boa` gives real JS.

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
    /// `(name, value)` attributes, names lowercased by the parser.
    pub attrs: Vec<(String, String)>,
    /// Snapshot index of the parent element; `None` for top-level elements.
    pub parent: Option<usize>,
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
            "{{\"tag\":\"{}\",\"id\":\"{}\",\"classes\":[{}],\"text\":\"{}\",\"attrs\":[{}],\"parent\":{}}}",
            escape_json(&e.tag),
            escape_json(&e.id),
            e.classes
                .iter()
                .map(|c| format!("\"{}\"", escape_json(c)))
                .collect::<Vec<_>>()
                .join(","),
            escape_json(&e.text),
            e.attrs
                .iter()
                .map(|(k, v)| format!("[\"{}\",\"{}\"]", escape_json(k), escape_json(v)))
                .collect::<Vec<_>>()
                .join(","),
            match e.parent {
                Some(p) => p.to_string(),
                None => "null".to_string(),
            },
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

function __attrVal(o, k) {
  for (var j = 0; j < o.attrs.length; j++) if (o.attrs[j][0] === k) return o.attrs[j][1];
  return null;
}

function __wrap(i) {
  if (i === null || i === undefined || i < 0 || i >= __DOM.length) return null;
  if (__wrappers[i]) return __wrappers[i];
  var e = __DOM[i];
  var o = { tag: e.tag, id: e.id, textContent: e.text, classes: e.classes.slice(), attrs: (e.attrs || []).slice() };
  o.__listeners = [];
  o.__parent = e.parent === null || e.parent === undefined ? null : e.parent;
  Object.defineProperty(o, 'className', {
    get: function () { return o.classes.join(' '); },
    set: function (v) {
      var parts = String(v).split(' ');
      o.classes = [];
      for (var j = 0; j < parts.length; j++) if (parts[j].length > 0) o.classes.push(parts[j]);
    }
  });
  o.getAttribute = function (k) { return __attrVal(o, k); };
  o.setAttribute = function (k, v) {
    for (var j = 0; j < o.attrs.length; j++) if (o.attrs[j][0] === k) { o.attrs[j][1] = String(v); return; }
    o.attrs.push([k, String(v)]);
  };
  o.removeAttribute = function (k) {
    for (var j = 0; j < o.attrs.length; j++) if (o.attrs[j][0] === k) { o.attrs.splice(j, 1); return; }
  };
  o.hasAttribute = function (k) { return __attrVal(o, k) !== null; };
  o.classList = {
    add: function (c) { if (o.classes.indexOf(c) < 0) o.classes.push(c); },
    remove: function (c) { var i = o.classes.indexOf(c); if (i >= 0) o.classes.splice(i, 1); },
    contains: function (c) { return o.classes.indexOf(c) >= 0; },
    toggle: function (c) {
      if (o.classList.contains(c)) { o.classList.remove(c); return false; }
      o.classList.add(c);
      return true;
    }
  };
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

function __parentOf(node) {
  if (node === document) return null;
  var p = node.__parent;
  if (p === null || p === undefined) return document;
  return __wrap(p);
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
    __stopBubble: false,
    __stopNow: false
  };
  e.preventDefault = function () { e.defaultPrevented = true; };
  e.stopPropagation = function () { e.__stopBubble = true; };
  e.stopImmediatePropagation = function () { e.__stopBubble = true; e.__stopNow = true; };
  var node = target;
  var guard = 0;
  while (node && guard++ < 1000) {
    e.currentTarget = node;
    var ls = node.__listeners || [];
    for (var k = 0; k < ls.length; k++) {
      if (ls[k].type !== e.type) continue;
      ls[k].fn(e);
      if (e.__stopNow) return !e.defaultPrevented;
    }
    if (!e.bubbles || e.__stopBubble) break;
    node = __parentOf(node);
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

    /// A live JS world: one context kept across runs so clicks dispatch into
    /// listeners registered by page scripts.
    pub struct JsSession {
        ctx: Context,
    }

    impl JsSession {
        /// Install bindings + DOM snapshot.
        pub fn new(title: &str, dom: &[DomEl]) -> Result<Self, String> {
            WRITES.with(|w| w.borrow_mut().clear());
            LOGS.with(|l| l.borrow_mut().clear());
            let mut ctx = Context::default();
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
            Ok(Self { ctx })
        }

        fn take_output(&mut self) -> (Vec<String>, Vec<String>) {
            let writes = WRITES.with(|w| std::mem::take(&mut *w.borrow_mut()));
            let logs = LOGS.with(|l| std::mem::take(&mut *l.borrow_mut()));
            STUB_WRITES.with(|w| w.borrow_mut().extend(writes.clone()));
            (writes, logs)
        }

        /// Eval one script, returning value + writes/logs since the last take.
        pub fn run(&mut self, source: &str) -> (Result<String, String>, Vec<String>, Vec<String>) {
            let result = match self.ctx.eval(Source::from_bytes(source)) {
                Ok(v) => v
                    .to_string(&mut self.ctx)
                    .map(|s| s.to_std_string_escaped())
                    .map_err(|e| format!("boa-stringify: {e}")),
                Err(e) => Err(format!("boa-eval: {e}")),
            };
            let (writes, logs) = self.take_output();
            (result, writes, logs)
        }

        /// Fire `DOMContentLoaded` + drain timers in this context.
        pub fn fire(&mut self) -> (Vec<String>, Vec<String>) {
            let _ = self.ctx.eval(Source::from_bytes(FIRE));
            self.take_output()
        }

        /// Dispatch `typ` on snapshot element `idx` with bubbling.
        /// Returns (default-not-prevented, writes, logs).
        pub fn dispatch(&mut self, idx: usize, typ: &str) -> (bool, Vec<String>, Vec<String>) {
            let src = format!("__wrap({idx}).dispatchEvent({{type:{typ:?}}})");
            let ok = match self.ctx.eval(Source::from_bytes(src.as_str())) {
                Ok(v) => v.to_boolean(),
                Err(_) => false,
            };
            let (writes, logs) = self.take_output();
            (ok, writes, logs)
        }
    }

    /// Install bindings + DOM snapshot, then eval main script, timers, listeners.
    pub fn run_with_bindings(
        source: &str,
        title: &str,
        dom: &[DomEl],
    ) -> (Result<String, String>, Vec<String>, Vec<String>) {
        let mut s = match JsSession::new(title, dom) {
            Ok(s) => s,
            Err(e) => return (Err(e), vec![], vec![]),
        };
        let (result, mut writes, mut logs) = s.run(source);
        let (fw, fl) = s.fire();
        writes.extend(fw);
        logs.extend(fl);
        (result, writes, logs)
    }
}

#[cfg(feature = "boa")]
/// Evaluate with Boa engine. Returns stringified value or error string.
pub fn eval_boa(source: &str) -> Result<String, String> {
    boa_impl::eval_boa(source)
}

#[cfg(feature = "boa")]
pub use boa_impl::JsSession;

#[cfg(not(feature = "boa"))]
/// Stub session: no live context; dispatch degrades to a no-op.
pub struct JsSession;

#[cfg(not(feature = "boa"))]
impl JsSession {
    /// Always succeeds; kept for call-shape parity with Boa.
    pub fn new(_title: &str, _dom: &[DomEl]) -> Result<Self, String> {
        Ok(Self)
    }

    /// Scan `document.write` / `int+int`, like `run_script_with_dom`.
    pub fn run(&mut self, source: &str) -> (Result<String, String>, Vec<String>, Vec<String>) {
        let o = run_script_with_dom(source, "", &[]);
        let r = if o.failed {
            Err(o.result.clone())
        } else {
            Ok(o.result.clone())
        };
        (r, o.writes, o.logs)
    }

    /// No timers without Boa.
    pub fn fire(&mut self) -> (Vec<String>, Vec<String>) {
        (Vec::new(), Vec::new())
    }

    /// No listeners without Boa.
    pub fn dispatch(&mut self, _idx: usize, _typ: &str) -> (bool, Vec<String>, Vec<String>) {
        (false, Vec::new(), Vec::new())
    }
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
                attrs: vec![("data-x".into(), "1".into())],
                parent: None,
            },
            DomEl {
                tag: "span".into(),
                id: "inner".into(),
                classes: vec!["row".into()],
                text: "side note".into(),
                attrs: vec![],
                parent: Some(0),
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
    fn boa_element_attribute_api() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "var el = document.getElementById('main'); \
             document.write('g:' + el.getAttribute('data-x')); \
             document.write('h:' + el.hasAttribute('href')); \
             el.setAttribute('href', 'http://a'); \
             document.write('s:' + el.getAttribute('href')); \
             el.removeAttribute('data-x'); \
             document.write('r:' + el.getAttribute('data-x'));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(
            writes,
            vec![
                "g:1".to_string(),
                "h:false".to_string(),
                "s:http://a".to_string(),
                "r:null".to_string()
            ]
        );
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_class_name_and_class_list() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "var el = document.querySelector('span'); \
             document.write('cn:' + el.className + ':' + el.classList.contains('row')); \
             el.classList.add('hot'); \
             document.write('a:' + el.classList.contains('hot') + ':' + el.className); \
             el.classList.remove('row'); \
             document.write('rm:' + el.classList.contains('row')); \
             document.write('t:' + el.classList.toggle('hot') + ':' + el.classList.toggle('new'));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(
            writes,
            vec![
                "cn:row:true".to_string(),
                "a:true:row hot".to_string(),
                "rm:false".to_string(),
                "t:false:true".to_string()
            ]
        );
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_event_bubbles_to_parent_and_document() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "var seen = []; \
             document.addEventListener('click', function (ev) { seen.push('doc:' + ev.target.id + ':' + (ev.currentTarget === document)); }); \
             document.addEventListener('tick', function () { seen.push('no-match'); }); \
             document.getElementById('main').addEventListener('click', function (ev) { seen.push('p:' + ev.target.id + ':' + ev.currentTarget.id); }); \
             document.querySelector('span').addEventListener('click', function () { seen.push('t'); }); \
             document.querySelector('span').dispatchEvent({ type: 'click' }); \
             document.write(seen.join('|'));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["t|p:inner:main|doc:inner:true".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_event_stop_propagation_halts_bubble_only() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "var seen = []; \
             document.addEventListener('click', function () { seen.push('doc'); }); \
             document.getElementById('main').addEventListener('click', function (ev) { \
               seen.push('p1'); ev.stopPropagation(); }); \
             document.getElementById('main').addEventListener('click', function () { seen.push('p2'); }); \
             document.querySelector('span').dispatchEvent('click'); \
             document.write(seen.join(','));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["p1,p2".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_event_stop_immediate_halts_same_node() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "var seen = []; \
             document.addEventListener('click', function () { seen.push('doc'); }); \
             document.getElementById('main').addEventListener('click', function (ev) { \
               seen.push('p1'); ev.stopImmediatePropagation(); }); \
             document.getElementById('main').addEventListener('click', function () { seen.push('p2'); }); \
             document.querySelector('span').dispatchEvent('click'); \
             document.write(seen.join(','));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["p1".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_event_bubbles_false_stops_at_target() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "var seen = []; \
             document.addEventListener('click', function () { seen.push('doc'); }); \
             document.getElementById('main').addEventListener('click', function () { seen.push('p'); }); \
             document.querySelector('span').addEventListener('click', function () { seen.push('t'); }); \
             document.querySelector('span').dispatchEvent({ type: 'click', bubbles: false }); \
             document.write(seen.join('|'));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["t".to_string()]);
    }

    #[cfg(feature = "boa")]
    #[test]
    fn boa_document_has_no_parent_bubble_path() {
        let dom = fixture_dom();
        let (r, writes, _) = boa_impl::run_with_bindings(
            "document.dispatchEvent({ type: 'x' }); document.write('ret:' + document.dispatchEvent('x'));",
            "T",
            &dom,
        );
        assert!(r.is_ok());
        assert_eq!(writes, vec!["ret:true".to_string()]);
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
