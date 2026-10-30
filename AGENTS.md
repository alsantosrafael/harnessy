# Harnessy — agent orientation

Harnessy is a local-first macOS desktop application built with Tauri 2, Rust, React, TypeScript, Bun, Vite, and Vitest. The product specification is in `SPEC.md`; user/developer instructions are in `README.md` and `docs/`.

## Working loop

- Start with `git status --short`; preserve existing edits and untracked orientation docs.
- Install with `bun install --frozen-lockfile` (Bun 1.4.3; Node 22+ for Vitest).
- Run `bun run check:quick` for frontend edits; `bun run verify` adds the production build. Native edits require `bun run verify:rust` and desktop smoke checks when IPC/lifecycle changes.
- Keep privileged operations in Rust and IPC narrow. Local tasks (state, history, blocks), runs and card state deduced from run results are real (the agent never touches cards); Codex review and publishing (push/PR) are not. Read `docs/development.md` before persistence, IPC or lifecycle changes.
- Reuse existing `.claude/skills` procedures. Codex reads this file and subtree `AGENTS.md`; OpenCode can use the same context.
- Repository harness work: read `docs/harness.md` for scope, checks and limitations. This tooling is distinct from app orchestration planned in `SPEC.md`.
- Versioning: every change evaluates front (`package.json`) and back (`src-tauri/Cargo.toml`) independently and bumps the affected side(s) plus `CHANGELOG.md`, without asking. See `.claude/rules/versionamento.md`.
- Keep credentials out of source, prompts and reports. Gitignore is not access control. Publishing changes or issues and changing personal configuration require explicit task authorization.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues for `alsantosrafael/harnessy`; use the authenticated `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix` for the five canonical triage roles. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: root `GLOSSARY.md` and ADRs under `docs/adr/`. Read relevant files when domain terms or architectural decisions affect the task. See `docs/agents/domain.md`.
