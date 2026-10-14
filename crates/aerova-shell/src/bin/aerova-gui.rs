//! Aerova GUI v1 (`gui` feature): minifb window on the engine framebuffer.
//! URL bar + status line in a 5x7 bitmap font; clicks dispatch into the live page.

use aerova_engine::{Page, TabId, TabManager};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use std::collections::HashMap;

const PAGE_W: usize = 800;
const PAGE_H: usize = 600;
const BAR: usize = 16;
const TAB_BAR: usize = 16;
const PAGE_Y: usize = TAB_BAR + BAR;
const STATUS_Y: usize = PAGE_Y + PAGE_H;
const WIN_W: usize = PAGE_W;
const WIN_H: usize = STATUS_Y + BAR;
const TAB_MIN_W: usize = 48;
const BLACK: u32 = 0x000000;
const DARK: u32 = 0x333333;
const WHITE: u32 = 0xFFFFFF;
const GRAY: u32 = 0xE8E8E8;

/// 5x7 glyph rows; bit 4 (0x10) is the leftmost column. Lowercase echoes uppercase.
fn glyph(c: u8) -> Option<[u8; 7]> {
    Some(match c {
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        b'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        b'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        b'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        b'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x19, 0x19, 0x15, 0x13, 0x13, 0x11],
        b'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        b'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        b'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        b'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        b'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        b'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        b'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        b'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        b'0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        b'1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        b'3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        b'4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        b'5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        b'6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        b'7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        b'8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        b'9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        b' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        b':' => [0x00, 0x04, 0x00, 0x00, 0x00, 0x04, 0x00],
        b'/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        b'?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        b'&' => [0x0C, 0x12, 0x14, 0x08, 0x15, 0x12, 0x0D],
        b'=' => [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00],
        b'#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        b'%' => [0x19, 0x1A, 0x02, 0x04, 0x08, 0x0B, 0x13],
        b'+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        b'-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        b'~' => [0x00, 0x00, 0x0A, 0x15, 0x00, 0x00, 0x00],
        b'@' => [0x0E, 0x11, 0x17, 0x15, 0x17, 0x10, 0x0E],
        b'(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        b')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        b',' => [0x00, 0x00, 0x00, 0x00, 0x0C, 0x04, 0x08],
        b'\'' => [0x04, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00],
        b'!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        b'>' => [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08],
        b'<' => [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02],
        b'*' => [0x00, 0x04, 0x15, 0x0E, 0x15, 0x04, 0x00],
        b';' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x04, 0x08],
        b'|' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'"' => [0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'\\' => [0x10, 0x08, 0x08, 0x04, 0x02, 0x02, 0x01],
        b'[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        b']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        b'`' => [0x04, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00],
        b'^' => [0x04, 0x0A, 0x11, 0x00, 0x00, 0x00, 0x00],
        b'$' => [0x04, 0x0F, 0x14, 0x0E, 0x05, 0x1E, 0x04],
        b'{' => [0x02, 0x04, 0x04, 0x08, 0x04, 0x04, 0x02],
        b'}' => [0x08, 0x04, 0x04, 0x02, 0x04, 0x04, 0x08],
        _ => return None,
    })
}

/// Map a pressed key to a typed char. Letters honor shift; unknown keys are ignored.
fn key_to_char(key: Key, shift: bool) -> Option<char> {
    let n = key as usize;
    if (Key::A as usize..=Key::Z as usize).contains(&n) {
        let c = (b'a' + (n - Key::A as usize) as u8) as char;
        return Some(if shift { c.to_ascii_uppercase() } else { c });
    }
    if (Key::Key0 as usize..=Key::Key9 as usize).contains(&n) {
        let d = n - Key::Key0 as usize;
        return Some(if shift {
            b")!@#$%^&*("[d] as char
        } else {
            (b'0' + d as u8) as char
        });
    }
    Some(match key {
        Key::Space => ' ',
        Key::Minus => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        Key::Equal => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        Key::Slash => {
            if shift {
                '?'
            } else {
                '/'
            }
        }
        Key::Period => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        Key::Comma => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        Key::Semicolon => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        Key::Apostrophe => {
            if shift {
                '"'
            } else {
                '\''
            }
        }
        Key::LeftBracket => {
            if shift {
                '{'
            } else {
                '['
            }
        }
        Key::RightBracket => {
            if shift {
                '}'
            } else {
                ']'
            }
        }
        Key::Backslash => {
            if shift {
                '|'
            } else {
                '\\'
            }
        }
        Key::Backquote => {
            if shift {
                '~'
            } else {
                '`'
            }
        }
        Key::NumPad0 => '0',
        Key::NumPad1 => '1',
        Key::NumPad2 => '2',
        Key::NumPad3 => '3',
        Key::NumPad4 => '4',
        Key::NumPad5 => '5',
        Key::NumPad6 => '6',
        Key::NumPad7 => '7',
        Key::NumPad8 => '8',
        Key::NumPad9 => '9',
        Key::NumPadDot => '.',
        Key::NumPadSlash => '/',
        Key::NumPadAsterisk => '*',
        Key::NumPadMinus => '-',
        Key::NumPadPlus => '+',
        _ => return None,
    })
}

/// Single-line URL editor: append + backspace only.
struct Editor {
    buf: String,
}

impl Editor {
    fn new(s: String) -> Self {
        Self { buf: s }
    }

    fn push(&mut self, c: char) {
        if self.buf.len() < 200 {
            self.buf.push(c);
        }
    }

    fn backspace(&mut self) {
        self.buf.pop();
    }
}

fn normalize_url(s: &str) -> String {
    let s = s.trim();
    if s.contains("://") {
        s.to_string()
    } else {
        format!("file://{s}")
    }
}

/// Keyboard navigation gesture, resolved from pressed keys + modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nav {
    Back,
    Forward,
    Reload,
    NewTab,
    CloseTab,
    NextTab,
}

fn nav_action(pressed: &[Key], alt: bool, ctrl: bool) -> Option<Nav> {
    if alt && pressed.contains(&Key::Left) {
        return Some(Nav::Back);
    }
    if alt && pressed.contains(&Key::Right) {
        return Some(Nav::Forward);
    }
    if pressed.contains(&Key::F5) {
        return Some(Nav::Reload);
    }
    if ctrl && pressed.contains(&Key::T) {
        return Some(Nav::NewTab);
    }
    if ctrl && pressed.contains(&Key::W) {
        return Some(Nav::CloseTab);
    }
    if ctrl && pressed.contains(&Key::Tab) {
        return Some(Nav::NextTab);
    }
    None
}

/// One live page per tab; the manager owns history, the map owns JS worlds.
struct Tabs {
    mgr: TabManager,
    pages: HashMap<TabId, Page>,
    current: TabId,
}

fn blank() -> Page {
    Page::open("").expect("blank page")
}

impl Tabs {
    fn open(start: &str) -> Self {
        let page = Page::open_url(start).unwrap_or_else(|_| blank());
        Self::from_html(start, page)
    }

    /// Open a tab straight from HTML: test seam for draw/scroll geometry.
    #[cfg(test)]
    fn open_html(html: &str) -> Self {
        Self::from_html("about:blank", Page::open(html).unwrap_or_else(|_| blank()))
    }

    fn from_html(url: &str, page: Page) -> Self {
        let mut mgr = TabManager::new();
        let current = mgr.new_tab(url);
        let mut pages = HashMap::new();
        pages.insert(current, page);
        Self {
            mgr,
            pages,
            current,
        }
    }

    fn page(&self) -> &Page {
        self.pages.get(&self.current).expect("current tab page")
    }

    fn page_mut(&mut self) -> &mut Page {
        self.pages.get_mut(&self.current).expect("current tab page")
    }

    fn url(&self) -> String {
        self.mgr.current(self.current).unwrap_or_default()
    }

    fn count(&self) -> usize {
        self.mgr.len()
    }

    fn status_line(&self) -> String {
        let list = self.mgr.list();
        let pos = list
            .iter()
            .position(|(id, _)| *id == self.current)
            .map_or(0, |i| i + 1);
        format!("tab {}/{} {}", pos, list.len(), self.url())
    }

    fn load_id(&mut self, id: TabId, url: &str) {
        self.pages
            .insert(id, Page::open_url(url).unwrap_or_else(|_| blank()));
        self.current = id;
    }

    fn go(&mut self, url: &str) {
        self.mgr.navigate(self.current, url);
        self.load_id(self.current, url);
    }

    fn back(&mut self) -> Option<String> {
        let url = self.mgr.back(self.current)?;
        self.load_id(self.current, &url);
        Some(url)
    }

    fn forward(&mut self) -> Option<String> {
        let url = self.mgr.forward(self.current)?;
        self.load_id(self.current, &url);
        Some(url)
    }

    fn reload(&mut self) -> String {
        let url = self.url();
        self.load_id(self.current, &url);
        url
    }

    fn new_tab(&mut self, url: &str) {
        let id = self.mgr.new_tab(url);
        self.load_id(id, url);
    }

    fn close_current(&mut self) -> bool {
        if self.mgr.len() <= 1 {
            return false;
        }
        self.mgr.close(self.current);
        self.pages.remove(&self.current);
        self.current = self.mgr.list().first().map(|(id, _)| *id).unwrap_or(0);
        true
    }

    fn switch(&mut self, idx: usize) {
        if let Some((id, _)) = self.mgr.list().get(idx) {
            self.current = *id;
        }
    }

    fn next(&mut self) {
        let list = self.mgr.list();
        if list.is_empty() {
            return;
        }
        let pos = list
            .iter()
            .position(|(id, _)| *id == self.current)
            .unwrap_or(0);
        self.current = list[(pos + 1) % list.len()].0;
    }

    fn click(&mut self, x: u32, y: u32) -> Vec<String> {
        self.page_mut().click(x, y)
    }
}

/// Tab cell width: even split like Chrome, floored so cells stay clickable.
fn tab_cell_w(count: usize) -> usize {
    (WIN_W / count.max(1)).max(TAB_MIN_W)
}

/// Scrollbar thumb at `(x, y, h)`, or `None` when content fits the viewport.
fn scrollbar(tabs: &Tabs) -> Option<(usize, usize, usize)> {
    let page = tabs.page();
    let content = page.content_h() as usize;
    if content <= PAGE_H {
        return None;
    }
    let thumb_h = (PAGE_H * PAGE_H / content).max(24);
    let max_off = page.content_h() - page.view_h();
    let travel = PAGE_H - thumb_h;
    let top = (page.scroll_y() as usize * travel) / max_off as usize;
    Some((WIN_W - 4, PAGE_Y + top, thumb_h))
}

/// Strip cell index at x. Every in-window x maps to a tab; empty set has none.
fn tab_at(x: usize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some((x / tab_cell_w(count)).min(count - 1))
}

fn fb_to_u32(fb: &aerova_paint::Framebuffer) -> Vec<u32> {
    fb.pixels
        .chunks_exact(4)
        .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
        .collect()
}

fn set_px(buf: &mut [u32], x: usize, y: usize, color: u32) {
    if x < WIN_W && y < WIN_H {
        buf[y * WIN_W + x] = color;
    }
}

fn fill_rect(buf: &mut [u32], x: usize, y: usize, w: usize, h: usize, color: u32) {
    for dy in 0..h {
        for dx in 0..w {
            set_px(buf, x + dx, y + dy, color);
        }
    }
}

fn draw_glyph(buf: &mut [u32], x: usize, y: usize, rows: [u8; 7], color: u32) {
    for (r, row) in rows.iter().enumerate() {
        for c in 0..5 {
            if row & (0x10 >> c) != 0 {
                set_px(buf, x + c, y + r, color);
            }
        }
    }
}

fn draw_text(buf: &mut [u32], x: usize, y: usize, s: &str, color: u32) {
    for (i, b) in s.bytes().enumerate() {
        let up = if b.is_ascii_lowercase() {
            b.to_ascii_uppercase()
        } else {
            b
        };
        if let Some(rows) = glyph(up) {
            draw_glyph(buf, x + i * 6, y, rows, color);
        }
    }
}

fn draw(buf: &mut [u32], tabs: &Tabs, url: &str, status: &str) {
    fill_rect(buf, 0, 0, WIN_W, TAB_BAR, GRAY);
    let list = tabs.mgr.list();
    let cell = tab_cell_w(list.len());
    let max_chars = (cell.saturating_sub(8) / 6).max(1);
    for (i, (id, u)) in list.iter().enumerate() {
        let short: String = u.chars().take(max_chars).collect();
        if *id == tabs.current {
            fill_rect(buf, i * cell, 0, cell, TAB_BAR, BLACK);
            draw_text(buf, 4 + i * cell, 4, &short, WHITE);
        } else {
            draw_text(buf, 4 + i * cell, 4, &short, DARK);
        }
    }
    fill_rect(buf, 0, TAB_BAR, WIN_W, BAR, WHITE);
    fill_rect(buf, 0, STATUS_Y, WIN_W, BAR, GRAY);
    let page = tabs.page();
    let top = page.scroll_y() as usize;
    let src = fb_to_u32(&page.fb);
    for y in 0..PAGE_H {
        let row = (top + y).min(src.len() / PAGE_W - 1) * PAGE_W;
        for x in 0..PAGE_W {
            buf[(y + PAGE_Y) * WIN_W + x] = src[row + x];
        }
    }
    if let Some((sx, sy, sh)) = scrollbar(tabs) {
        fill_rect(buf, sx, sy, 4, sh, GRAY);
    }
    draw_text(buf, 4, TAB_BAR + 4, url, BLACK);
    fill_rect(buf, 4 + url.len() * 6, TAB_BAR + 12, 5, 1, BLACK);
    draw_text(buf, 4, STATUS_Y + 4, status, DARK);
}

fn sync_bar(tabs: &Tabs, editor: &mut Editor, status: &mut String) {
    editor.buf = tabs.url();
    *status = tabs.status_line();
}

/// Scroll keys: arrows nudge, Page keys jump a viewport, Home/End jump ends.
fn scroll_keys(tabs: &mut Tabs, pressed: &[Key]) {
    let view = tabs.page().view_h() as i32;
    for k in pressed {
        let dy = match k {
            Key::Down => 40,
            Key::Up => -40,
            Key::PageDown => view,
            Key::PageUp => -view,
            _ => continue,
        };
        tabs.page_mut().scroll_by(dy);
    }
    if pressed.contains(&Key::Home) {
        tabs.page_mut().scroll_to(0);
    }
    if pressed.contains(&Key::End) {
        tabs.page_mut().scroll_to(u32::MAX);
    }
}

fn main() {
    let start = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "aerova://home".to_string());
    let mut window =
        Window::new("Aerova", WIN_W, WIN_H, WindowOptions::default()).unwrap_or_else(|e| {
            eprintln!("gui: cannot open window: {e}");
            std::process::exit(1);
        });
    window.set_target_fps(30);
    let mut tabs = Tabs::open(&start);
    println!("{}", tabs.page().text);
    let mut editor = Editor::new(start);
    let mut status = String::from(
        "esc quits | enter loads | alt+arrows history | f5 reload | ctrl+t/w/tab tabs",
    );
    let mut buf = vec![0u32; WIN_W * WIN_H];
    let mut was_down = false;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let alt = window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt);
        let ctrl = window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl);
        let pressed = window.get_keys_pressed(KeyRepeat::Yes);
        let gesture = window.get_keys_pressed(KeyRepeat::No);
        match nav_action(&gesture, alt, ctrl) {
            Some(Nav::Back) => match tabs.back() {
                Some(_) => sync_bar(&tabs, &mut editor, &mut status),
                None => status = String::from("no back history"),
            },
            Some(Nav::Forward) => match tabs.forward() {
                Some(_) => sync_bar(&tabs, &mut editor, &mut status),
                None => status = String::from("no forward history"),
            },
            Some(Nav::Reload) => {
                tabs.reload();
                sync_bar(&tabs, &mut editor, &mut status);
            }
            Some(Nav::NewTab) => {
                tabs.new_tab("aerova://home");
                sync_bar(&tabs, &mut editor, &mut status);
            }
            Some(Nav::CloseTab) => {
                if tabs.count() <= 1 {
                    break;
                }
                tabs.close_current();
                sync_bar(&tabs, &mut editor, &mut status);
            }
            Some(Nav::NextTab) => {
                tabs.next();
                sync_bar(&tabs, &mut editor, &mut status);
            }
            None => {}
        }
        let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        scroll_keys(&mut tabs, &pressed);
        if let Some((_, wy)) = window.get_scroll_wheel() {
            if wy.abs() > 0.5 {
                let steps = (wy * 3.0) as i32;
                tabs.page_mut().scroll_by(-steps * 20);
            }
        }
        if !(ctrl || alt) {
            for k in pressed {
                match k {
                    Key::Backspace => editor.backspace(),
                    Key::Enter | Key::NumPadEnter => {
                        let url = normalize_url(&editor.buf);
                        tabs.go(&url);
                        println!("{}", tabs.page().text);
                        sync_bar(&tabs, &mut editor, &mut status);
                    }
                    _ => {
                        if let Some(c) = key_to_char(k, shift) {
                            editor.push(c);
                        }
                    }
                }
            }
        }
        let down = window.get_mouse_down(MouseButton::Left);
        if down && !was_down {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Discard) {
                let (mx, my) = (mx as usize, my as usize);
                if my < TAB_BAR {
                    if let Some(i) = tab_at(mx, tabs.count()) {
                        tabs.switch(i);
                        sync_bar(&tabs, &mut editor, &mut status);
                    }
                } else {
                    let (px, py) = (mx as i32, my as i32 - PAGE_Y as i32);
                    if px >= 0 && py >= 0 {
                        let writes = tabs.click(px as u32, py as u32);
                        if !writes.is_empty() {
                            status = writes.join(" ");
                        }
                    }
                }
            }
        }
        was_down = down;
        draw(&mut buf, &tabs, &editor.buf, &status);
        window
            .update_with_buffer(&buf, WIN_W, WIN_H)
            .unwrap_or_else(|e| {
                eprintln!("gui: blit failed: {e}");
                std::process::exit(1);
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_discriminants_match_assumption() {
        assert_eq!(Key::A as usize, 10);
        assert_eq!(Key::Z as usize, 35);
        assert_eq!(Key::Key0 as usize, 0);
        assert_eq!(Key::Key9 as usize, 9);
    }

    #[test]
    fn keys_map_to_chars() {
        assert_eq!(key_to_char(Key::A, false), Some('a'));
        assert_eq!(key_to_char(Key::A, true), Some('A'));
        assert_eq!(key_to_char(Key::Key1, false), Some('1'));
        assert_eq!(key_to_char(Key::Key1, true), Some('!'));
        assert_eq!(key_to_char(Key::Slash, false), Some('/'));
        assert_eq!(key_to_char(Key::Slash, true), Some('?'));
        assert_eq!(key_to_char(Key::Semicolon, true), Some(':'));
        assert_eq!(key_to_char(Key::Space, false), Some(' '));
        assert_eq!(key_to_char(Key::F5, false), None);
        assert_eq!(key_to_char(Key::Enter, false), None);
    }

    fn long_html() -> String {
        let mut h = String::new();
        for i in 0..30 {
            h.push_str(&format!("<p style=\"height:60px\">line {i}</p>"));
        }
        h
    }

    #[test]
    fn scrollbar_geometry_tracks_offset() {
        let short = Tabs::open_html("<p>hi</p>");
        assert_eq!(short.page().content_h(), PAGE_H as u32);
        assert!(scrollbar(&short).is_none());

        let mut long = Tabs::open_html(&long_html());
        assert_eq!(long.page().content_h(), 1800);
        let (x, y, h) = scrollbar(&long).unwrap();
        assert_eq!(x, WIN_W - 4);
        assert_eq!(y, PAGE_Y);
        assert!(h < PAGE_H);
        long.page_mut().scroll_to(600);
        let (_, y2, _) = scrollbar(&long).unwrap();
        assert!(y2 > y);
    }

    #[test]
    fn draw_paints_thumb_only_for_tall_pages() {
        let x = WIN_W - 2;
        let y = PAGE_Y + 2;
        let mut tall = vec![0u32; WIN_W * WIN_H];
        let tabs = Tabs::open_html(&long_html());
        draw(&mut tall, &tabs, "u", "s");
        assert_eq!(tall[y * WIN_W + x], GRAY);
        let mut short = vec![0u32; WIN_W * WIN_H];
        let tabs = Tabs::open_html("<p>hi</p>");
        draw(&mut short, &tabs, "u", "s");
        assert_ne!(short[y * WIN_W + x], GRAY);
    }

    #[test]
    fn scroll_keys_move_and_clamp() {
        let mut t = Tabs::open_html(&long_html());
        scroll_keys(&mut t, &[]);
        assert_eq!(t.page().scroll_y(), 0);
        scroll_keys(&mut t, &[Key::Down]);
        assert_eq!(t.page().scroll_y(), 40);
        scroll_keys(&mut t, &[Key::Up, Key::Up]);
        assert_eq!(t.page().scroll_y(), 0);
        scroll_keys(&mut t, &[Key::PageDown]);
        assert_eq!(t.page().scroll_y(), PAGE_H as u32);
        scroll_keys(&mut t, &[Key::PageUp, Key::PageUp]);
        assert_eq!(t.page().scroll_y(), 0);
        scroll_keys(&mut t, &[Key::End]);
        assert_eq!(t.page().scroll_y(), 1200);
        scroll_keys(&mut t, &[Key::Home]);
        assert_eq!(t.page().scroll_y(), 0);
    }

    #[test]
    fn nav_gestures_resolve() {
        assert_eq!(nav_action(&[Key::Left], true, false), Some(Nav::Back));
        assert_eq!(nav_action(&[Key::Right], true, false), Some(Nav::Forward));
        assert_eq!(nav_action(&[Key::F5], false, false), Some(Nav::Reload));
        assert_eq!(nav_action(&[Key::T], false, true), Some(Nav::NewTab));
        assert_eq!(nav_action(&[Key::W], false, true), Some(Nav::CloseTab));
        assert_eq!(nav_action(&[Key::Tab], false, true), Some(Nav::NextTab));
        assert_eq!(nav_action(&[Key::Left], false, false), None);
        assert_eq!(nav_action(&[Key::T], false, false), None);
        assert_eq!(nav_action(&[], true, true), None);
    }

    #[test]
    fn tabs_open_switch_close() {
        let mut t = Tabs::open("aerova://home");
        assert_eq!(t.count(), 1);
        assert!(t.page().text.contains("Welcome"));
        t.new_tab("bogus://nope");
        assert_eq!(t.count(), 2);
        assert_eq!(t.url(), "bogus://nope");
        t.switch(0);
        assert_eq!(t.url(), "aerova://home");
        assert!(t.page().text.contains("Welcome"));
        t.next();
        assert_eq!(t.url(), "bogus://nope");
        t.next();
        assert_eq!(t.url(), "aerova://home");
        t.switch(1);
        assert!(t.close_current());
        assert_eq!(t.count(), 1);
        assert!(!t.close_current());
        assert_eq!(t.count(), 1);
    }

    #[test]
    fn tabs_go_back_forward() {
        let mut t = Tabs::open("aerova://home");
        t.go("bogus://nope");
        assert_eq!(t.url(), "bogus://nope");
        assert_eq!(t.back(), Some("aerova://home".to_string()));
        assert_eq!(t.url(), "aerova://home");
        assert!(t.page().text.contains("Welcome"));
        assert_eq!(t.forward(), Some("bogus://nope".to_string()));
        assert_eq!(t.back(), Some("aerova://home".to_string()));
        assert_eq!(t.back(), None);
    }

    #[test]
    fn tab_cells_shrink_like_chrome() {
        assert_eq!(tab_cell_w(1), 800);
        assert_eq!(tab_cell_w(5), 160);
        assert_eq!(tab_cell_w(6), 133);
        assert_eq!(tab_cell_w(100), 48);
    }

    #[test]
    fn tab_strip_cells() {
        assert_eq!(tab_at(0, 3), Some(0));
        assert_eq!(tab_at(265, 3), Some(0));
        assert_eq!(tab_at(266, 3), Some(1));
        assert_eq!(tab_at(799, 3), Some(2));
        assert_eq!(tab_at(400, 6), Some(3));
        assert_eq!(tab_at(0, 0), None);
    }

    #[test]
    fn editor_appends_backspaces_and_caps() {
        let mut e = Editor::new(String::new());
        e.push('a');
        e.push('b');
        e.backspace();
        assert_eq!(e.buf, "a");
        for _ in 0..300 {
            e.push('x');
        }
        assert_eq!(e.buf.len(), 200);
    }

    #[test]
    fn urls_gain_file_scheme() {
        assert_eq!(normalize_url("aerova://home"), "aerova://home");
        assert_eq!(normalize_url("/tmp/x.html"), "file:///tmp/x.html");
    }

    #[test]
    fn framebuffer_converts_to_u32() {
        let mut fb = aerova_paint::Framebuffer::new(2, 1);
        fb.rect(0, 0, 1, 1, (255, 0, 0));
        let buf = fb_to_u32(&fb);
        assert_eq!(buf, vec![0xFF0000, 0xFFFFFF]);
    }

    #[test]
    fn glyph_a_lights_expected_pixels() {
        let mut buf = vec![0u32; WIN_W * WIN_H];
        draw_glyph(&mut buf, 0, 0, glyph(b'A').unwrap(), BLACK);
        assert_eq!(buf[0], 0);
        assert_eq!(buf[1], BLACK);
        assert_eq!(buf[3], BLACK);
        assert_eq!(buf[4], 0);
    }

    #[test]
    fn every_typable_char_has_a_glyph() {
        let keys = [
            Key::A,
            Key::Z,
            Key::Key0,
            Key::Key9,
            Key::Space,
            Key::Minus,
            Key::Equal,
            Key::Slash,
            Key::Period,
            Key::Comma,
            Key::Semicolon,
            Key::Apostrophe,
            Key::LeftBracket,
            Key::RightBracket,
            Key::Backslash,
            Key::Backquote,
            Key::NumPad0,
            Key::NumPadDot,
            Key::NumPadSlash,
            Key::NumPadAsterisk,
            Key::NumPadMinus,
            Key::NumPadPlus,
        ];
        for k in keys {
            for shift in [false, true] {
                if let Some(c) = key_to_char(k, shift) {
                    let b = (c as u8).to_ascii_uppercase();
                    assert!(glyph(b).is_some(), "missing glyph for {c:?}");
                }
            }
        }
    }

    #[test]
    fn unknown_glyph_is_none() {
        assert!(glyph(0x00).is_none());
        assert!(glyph(b'a').is_none());
    }
}
