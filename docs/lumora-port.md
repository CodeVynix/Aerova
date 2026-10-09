# Lumora port

Blocked on Lumora PROJECT.md steps 11-13: NIC/TCP-IP/DNS/TLS, libc/POSIX (threads, mmap, sockets, poll), GPU/compositor.

Aerova needs only: files, sockets, threads, mmap, blit, fonts.
Chromium/Firefox need: full POSIX + NSS + Skia + V8 + sandbox + GPU + build toolchain port.

Strategy: finish native Linux/Windows CLI render first, then swap `aerova-shell` winit/wgpu for `lumora-ui` + compositor blit. Keep engine std-only.
