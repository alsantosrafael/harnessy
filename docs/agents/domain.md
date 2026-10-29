# Domain docs

This is a single-context repository. Domain vocabulary and architectural decisions are documented at the repository root and in `docs/adr/`.

## Before domain exploration

- Read root `GLOSSARY.md` when it exists; use its terms in issue titles, plans, code, and tests.
- Read ADRs under `docs/adr/` that concern the modules or decisions being changed.
- If a needed term is absent, avoid silently introducing a synonym. Identify the terminology gap and resolve it when the domain decision is clear.
- If a proposed change conflicts with an accepted ADR, surface the conflict and the evidence before changing the decision.

These documents are created when domain language or an architectural decision has actually been established. Do not create placeholder glossary entries or ADRs to satisfy a checklist.

## Layout

```text
GLOSSARY.md       # Shared domain vocabulary
docs/adr/         # Accepted architectural decisions
```
