//! Aerova paint: RGBA framebuffer + PPM dump + color parsing.
//! No GPU yet; Lumora framebuffer reuse planned.

/// RGBA framebuffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framebuffer {
    /// Width in px.
    pub width: u32,
    /// Height in px.
    pub height: u32,
    /// RGBA bytes.
    pub pixels: Vec<u8>,
}

/// Parse CSS color: names + `#rrggbb` + `#rgb`.
#[must_use]
pub fn parse_color(value: &str) -> Option<(u8, u8, u8)> {
    let v = value.trim().to_lowercase();
    match v.as_str() {
        "black" => Some((0, 0, 0)),
        "white" => Some((255, 255, 255)),
        "red" => Some((255, 0, 0)),
        "green" => Some((0, 128, 0)),
        "lime" => Some((0, 255, 0)),
        "blue" => Some((0, 0, 255)),
        "yellow" => Some((255, 255, 0)),
        "gray" | "grey" => Some((128, 128, 128)),
        "lightgray" | "lightgrey" => Some((211, 211, 211)),
        "lightblue" => Some((173, 216, 230)),
        _ => {
            let hex = v.strip_prefix('#')?;
            let (r, g, b) = match hex.len() {
                6 => (
                    u8::from_str_radix(&hex[0..2], 16).ok()?,
                    u8::from_str_radix(&hex[2..4], 16).ok()?,
                    u8::from_str_radix(&hex[4..6], 16).ok()?,
                ),
                3 => {
                    let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                    let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                    let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                    (r * 17, g * 17, b * 17)
                }
                _ => return None,
            };
            Some((r, g, b))
        }
    }
}

impl Framebuffer {
    /// Create filled with white.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![255; n],
        }
    }

    /// Fill rectangle with RGB color.
    pub fn rect(&mut self, x: u32, y: u32, w: u32, h: u32, rgb: (u8, u8, u8)) {
        for dy in 0..h {
            for dx in 0..w {
                let px = x + dx;
                let py = y + dy;
                if px < self.width && py < self.height {
                    let i = ((py * self.width + px) as usize) * 4;
                    self.pixels[i] = rgb.0;
                    self.pixels[i + 1] = rgb.1;
                    self.pixels[i + 2] = rgb.2;
                    self.pixels[i + 3] = 255;
                }
            }
        }
    }

    /// Save as binary PPM (P6), easy without image deps.
    /// # Errors
    /// Returns IO error if file cannot be written.
    pub fn save_ppm(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        write!(f, "P6\n{} {}\n255\n", self.width, self.height)?;
        for chunk in self.pixels.chunks_exact(4) {
            f.write_all(&chunk[0..3])?;
        }
        Ok(())
    }
}

/// Glyph cell width in px.
pub const GLYPH_W: u32 = 5;
/// Glyph cell height in px.
pub const GLYPH_H: u32 = 7;
/// Horizontal px between glyph origins.
pub const ADVANCE: u32 = 6;
/// Line height in px; a wrapped text line occupies one of these.
pub const LINE_H: u32 = 20;
/// Text ink color.
pub const INK: (u8, u8, u8) = (0, 0, 0);

/// 5x7 glyph rows; bit 4 (0x10) is the leftmost column. ASCII only; callers
/// uppercase before lookup.
#[must_use]
pub fn glyph(c: u8) -> Option<[u8; 7]> {
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

/// Glyph cell width in px.
#[must_use]
pub fn cell_w() -> u32 {
    GLYPH_W
}

/// Draw one glyph's set pixels at `(x, y)`.
pub fn draw_glyph(fb: &mut Framebuffer, x: u32, y: u32, rows: [u8; 7], rgb: (u8, u8, u8)) {
    for (r, row) in rows.iter().enumerate() {
        for c in 0..GLYPH_W as usize {
            if row & (0x10 >> c) != 0 {
                fb.rect(x + c as u32, y + r as u32, 1, 1, rgb);
            }
        }
    }
}

/// Draw one line of text from `(x, y)`; lowercase renders uppercase.
pub fn draw_text(fb: &mut Framebuffer, x: u32, y: u32, s: &str, rgb: (u8, u8, u8)) {
    for (i, b) in s.bytes().enumerate() {
        let up = if b.is_ascii_lowercase() {
            b.to_ascii_uppercase()
        } else {
            b
        };
        if let Some(rows) = glyph(up) {
            draw_glyph(fb, x + i as u32 * ADVANCE, y, rows, rgb);
        }
    }
}

/// Width in px of `s` on one line.
#[must_use]
pub fn text_width(s: &str) -> u32 {
    s.chars().count() as u32 * ADVANCE
}

/// Glyphs that fit in `w` px, at least one.
#[must_use]
pub fn chars_per_line(w: u32) -> usize {
    (w / ADVANCE).max(1) as usize
}

/// Greedy wrap into lines of at most `w` px. Words stay whole when they fit; a
/// word longer than one line is split. Newlines in `s` start a new line.
#[must_use]
pub fn wrap(s: &str, w: u32) -> Vec<String> {
    let per = chars_per_line(w);
    let mut out: Vec<String> = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() {
                if line.chars().count() + 1 + word.chars().count() <= per {
                    line.push(' ');
                    line.push_str(word);
                    continue;
                }
                out.push(std::mem::take(&mut line));
            }
            let mut rest: Vec<char> = word.chars().collect();
            while rest.len() > per {
                let chunk: String = rest.drain(..per).collect();
                out.push(chunk);
            }
            line = rest.into_iter().collect();
        }
        out.push(line);
    }
    if s.ends_with('\n') {
        out.pop();
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paints_rect() {
        let mut fb = Framebuffer::new(10, 10);
        fb.rect(0, 0, 5, 5, (255, 0, 0));
        assert_eq!(&fb.pixels[0..3], &[255, 0, 0]);
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("red"), Some((255, 0, 0)));
        assert_eq!(parse_color("#00ff00"), Some((0, 255, 0)));
        assert_eq!(parse_color("#f00"), Some((255, 0, 0)));
        assert_eq!(parse_color("nope"), None);
    }

    #[test]
    fn draw_text_lights_glyph_pixels() {
        let mut fb = Framebuffer::new(40, 12);
        draw_text(&mut fb, 0, 0, "A", (0, 0, 0));
        let px = |x: usize| {
            let i = x * 4;
            &fb.pixels[i..i + 3]
        };
        // 'A' row 0 is 0b01110: off, on, on, on, off.
        assert_eq!(px(0), &[255, 255, 255]);
        assert_eq!(px(1), &[0, 0, 0]);
        assert_eq!(px(2), &[0, 0, 0]);
        assert_eq!(px(4), &[255, 255, 255]);
    }

    #[test]
    fn text_width_and_fit() {
        assert_eq!(text_width("ab"), 12);
        assert_eq!(text_width(""), 0);
        assert_eq!(chars_per_line(0), 1);
        assert_eq!(chars_per_line(133), 22);
    }

    #[test]
    fn wrap_keeps_words_whole_and_splits_long_ones() {
        let w = 60; // 10 glyphs per line
        assert_eq!(wrap("one two", w), vec!["one two".to_string()]);
        assert_eq!(
            wrap("aaa bbb ccc ddd", w),
            vec!["aaa bbb".to_string(), "ccc ddd".to_string()]
        );
        assert_eq!(
            wrap("abcdefghijklmno", w),
            vec!["abcdefghij".to_string(), "klmno".to_string()]
        );
        assert_eq!(wrap("", w), vec![String::new()]);
        assert_eq!(wrap("a\n\nb", w), vec!["a".to_string(), String::new(), "b".to_string()]);
        assert_eq!(wrap("a\n", w), vec!["a".to_string()]);
    }
}

use image::{DynamicImage, GenericImageView, ImageFormat};

/// Decoded image data ready for painting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA
}

/// Decode image bytes (PNG/JPEG) into RGBA pixels.
/// Returns (width, height, RGBA pixels).
pub fn decode_image(bytes: &[u8]) -> Option<DecodedImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(DecodedImage {
        width,
        height,
        pixels: rgba.into_raw(),
    })
}

/// Draw decoded image onto framebuffer at (x, y) with optional scaling.
pub fn draw_image(fb: &mut Framebuffer, img: &DecodedImage, x: u32, y: u32, w: u32, h: u32) {
    let src_w = img.width;
    let src_h = img.height;
    let scale_x = src_w as f32 / w as f32;
    let scale_y = src_h as f32 / h as f32;
    for dy in 0..h {
        for dx in 0..w {
            let sx = ((dx as f32) * scale_x) as u32;
            let sy = ((dy as f32) * scale_y) as u32;
            if sx >= img.width || sy >= img.height {
                continue;
            }
            let src_idx = ((sy * img.width + sx) * 4) as usize;
            let dst_x = x + dx;
            let dst_y = y + dy;
            if dst_x >= fb.width || dst_y >= fb.height {
                continue;
            }
            let dst_idx = ((dst_y * fb.width + dst_x) * 4) as usize;
            fb.pixels[dst_idx] = img.pixels[src_idx];
            fb.pixels[dst_idx + 1] = img.pixels[src_idx + 1];
            fb.pixels[dst_idx + 2] = img.pixels[src_idx + 2];
            fb.pixels[dst_idx + 3] = img.pixels[src_idx + 3];
        }
    }
}
