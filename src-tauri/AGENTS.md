# Native macOS layer

Rust owns lifecycle, tray and privileged operations. Inspect `src/lib.rs`, `capabilities/main.json` and `tauri.conf.json` together for IPC or window changes; grant only required permissions.
Product version comes from the root manifest; the crate version is intentionally internal.
Run `bun run verify:rust` from the root. For lifecycle, tray or IPC changes, run Tauri and follow desktop smoke checks in `../docs/development.md`. Browser tests cannot establish native correctness.
