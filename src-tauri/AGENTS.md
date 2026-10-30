# Native macOS layer

Rust owns lifecycle, tray and privileged operations. The WebView only reaches it through typed IPC commands.

- Rules for any edit here: `.claude/rules/rust-architecture.md` (path-scoped, loaded automatically by Claude Code; Codex reads it from here).
- Architecture map and rationale: `docs/architecture-rust.md`.
- For IPC or window changes, inspect `src/lib.rs`, `capabilities/main.json` and `tauri.conf.json` together; grant only required permissions.
- Product version comes from the root manifest; the crate version is intentionally internal.
- Run `bun run verify:rust` and `cargo test` from the root. Lifecycle, tray or IPC changes also need the desktop smoke checks in `../docs/development.md`.
