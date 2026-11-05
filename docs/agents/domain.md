# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

**Layout: single-context** — one `GLOSSARY.md` at the repo root, with architecture decisions in `docs/adr/` at the repo root. There is no `GLOSSARY-MAP.md` and no per-subproject `GLOSSARY.md` in this repo; multi-context does not apply.

## Before exploring, read these

- **`GLOSSARY.md`** at the repo root — the project's vocabulary.
- **`docs/adr/`** — read the ADRs that touch the area you're about to work in.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

Single-context repo (this one):

```
/
├── GLOSSARY.md          ← project vocabulary, created with the first real term
└── docs/
    ├── adr/             ← architecture decision records
    └── agents/          ← this setup: tracker, labels, domain rules
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `GLOSSARY.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders), but worth reopening because…_
