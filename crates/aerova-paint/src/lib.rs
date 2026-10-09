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
}
