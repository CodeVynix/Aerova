# Aerova agent rules

Source of truth: README + docs/arch.md + docs/lumora-port.md.
Repo: ~/aerova (never /mnt/c). Toolchain: $HOME/.cargo/bin.

Commands: cargo xtask build/test/lint/fmt. GUI later; CLI now.

Rules: small steps, build+clippy+fmt+test green before next step. Every unsafe needs // SAFETY:. Never invent specs. docs/ per crate touched. Honest logs only.
