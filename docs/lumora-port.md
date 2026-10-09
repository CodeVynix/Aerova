# Lumora port (contract frozen)

Engine renders RGBA `Framebuffer` on Linux/Windows today. Lumora implements `BlitTarget`.

- `aerova-lumora::BlitTarget`: `size()` + `blit_rgba(dx,dy,w,h,bytes)` with clipping.
- `MemorySurface`: test double used by `aerova lumora-blit`.
- `blit_framebuffer(src, dst, dx, dy)`: what `engine` calls; no code change when OS swaps in.
- `blit_to_raw_framebuffer(ptr, w, h, pitch, src, dx, dy)`: unsafe VRAM helper with `// SAFETY:` per Lumora rules. Assumes 32bpp; driver swizzles if XRGB/BGRA.

Still gated on Lumora OS: libc/POSIX (threads, mmap, sockets, poll), NIC/TCP-IP/DNS/TLS, framebuffer driver + compositor + `lumora-ui` fonts. Chromium/Firefox need all that plus NSS/Skia/V8; Aerova needs only blit + sockets + files.
