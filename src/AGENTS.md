# Frontend

Read `../docs/development.md` for the source map and foundation invariants.
`lib/drafts.ts` owns draft session lifetime and persistence; verify through `../tests/drafts.test.ts` with `bun run test` (Vitest, not Bun's runner). Preserve retry/recovery and unsaved text across navigation.
Use existing CSS tokens and preserve keyboard focus and reduced motion. Keep desktop calls behind `lib/desktop.ts`; browser rendering does not verify native IPC.
Complete edits with `bun run check:quick`; build when changing imports, dependencies or build configuration.
