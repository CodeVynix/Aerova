//! Aerova Lumora blit contract (Phase 5).
//! Engine renders [`aerova_paint::Framebuffer`] (RGBA) on Linux/Windows today.
//! Lumora compositor later implements [`BlitTarget`]; engine code does not change.
//!
//! Blocked on Lumora OS Build 1 steps 11-13 (net/libc/GPU/compositor) for full
//! boot, but the blit ABI below is frozen so the port is a drop-in.

use aerova_paint::Framebuffer;

/// Destination surface in Lumora compositor (or test memory surface).
pub trait BlitTarget {
    /// Width and height in pixels.
    fn size(&self) -> (u32, u32);
    /// Blit tightly-packed RGBA bytes at `(dx, dy)`. Must clip safely.
    fn blit_rgba(&mut self, dx: u32, dy: u32, src_w: u32, src_h: u32, rgba: &[u8]);
}

/// In-memory surface for Linux/Windows tests. Lumora replaces with real VRAM window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySurface {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl MemorySurface {
    /// Create filled opaque black.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Raw RGBA pixels.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Convert to [`Framebuffer`] for PPM dump / tests.
    #[must_use]
    pub fn to_framebuffer(&self) -> Framebuffer {
        let mut fb = Framebuffer::new(self.width, self.height);
        fb.pixels.clone_from_slice(&self.pixels);
        fb
    }
}

impl BlitTarget for MemorySurface {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn blit_rgba(&mut self, dx: u32, dy: u32, src_w: u32, src_h: u32, rgba: &[u8]) {
        for sy in 0..src_h {
            for sx in 0..src_w {
                let tx = dx + sx;
                let ty = dy + sy;
                if tx < self.width && ty < self.height {
                    let si = ((sy * src_w + sx) as usize) * 4;
                    let di = ((ty * self.width + tx) as usize) * 4;
                    if si + 3 < rgba.len() && di + 3 < self.pixels.len() {
                        self.pixels[di..di + 4].copy_from_slice(&rgba[si..si + 4]);
                    }
                }
            }
        }
    }
}

/// Blit engine framebuffer onto any Lumora surface with clipping.
pub fn blit_framebuffer(src: &Framebuffer, dst: &mut impl BlitTarget, dx: u32, dy: u32) {
    dst.blit_rgba(dx, dy, src.width, src.height, &src.pixels);
}

/// Blit RGBA framebuffer into a raw Lumora VRAM pointer (pitch in bytes).
/// Lumora framebuffer driver is assumed 32bpp; byte order swizzle happens in driver.
///
/// # Safety
/// `raw` must point to at least `raw_h * pitch_bytes` writable bytes for the
/// duration of this call, with 32bpp rows. Caller guarantees no concurrent
/// scanout write aliasing. No Lumora hardware touched here; pure memory copy.
// SAFETY: bounds-checked copy below; see contract above.
pub unsafe fn blit_to_raw_framebuffer(
    raw: *mut u8,
    raw_w: u32,
    raw_h: u32,
    pitch_bytes: usize,
    src: &Framebuffer,
    dx: u32,
    dy: u32,
) {
    if raw.is_null() {
        return;
    }
    for sy in 0..src.height {
        for sx in 0..src.width {
            let tx = dx + sx;
            let ty = dy + sy;
            if tx < raw_w && ty < raw_h {
                let si = ((sy * src.width + sx) as usize) * 4;
                let di = (ty as usize) * pitch_bytes + (tx as usize) * 4;
                // SAFETY: di..di+4 is inside raw_h*pitch per checks above; caller upholds allocation.
                unsafe {
                    std::ptr::copy_nonoverlapping(src.pixels.as_ptr().add(si), raw.add(di), 4);
                }
            }
        }
    }
}

/// Port readiness string (contract frozen, OS boot gated).
#[must_use]
pub fn status() -> &'static str {
    "blit ABI frozen (BlitTarget/RGBA); waiting on Lumora libc/compositor/net"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_reports() {
        assert!(!status().is_empty());
    }

    #[test]
    fn blits_and_clips() {
        let mut src = Framebuffer::new(4, 4);
        src.rect(0, 0, 4, 4, (255, 0, 0));
        let mut dst = MemorySurface::new(2, 2);
        blit_framebuffer(&src, &mut dst, 0, 0);
        assert_eq!(&dst.pixels()[0..3], &[255, 0, 0]);
        // Offset fully outside still safe (clip, no panic).
        blit_framebuffer(&src, &mut dst, 10, 10);
        assert_eq!(&dst.pixels()[0..3], &[255, 0, 0]);
    }

    #[test]
    fn raw_ptr_blit() {
        let mut src = Framebuffer::new(2, 1);
        src.rect(0, 0, 2, 1, (0, 255, 0));
        let mut vram = vec![0u8; 2 * 8];
        // SAFETY: vram is 2 rows x 8-byte pitch, valid for this call.
        unsafe {
            blit_to_raw_framebuffer(vram.as_mut_ptr(), 2, 2, 8, &src, 0, 1);
        }
        assert_eq!(&vram[8..11], &[0, 255, 0]);
    }
}
