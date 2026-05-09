# Screen-Map Phase 3 — Brainstorm Decisions

**Date:** 2026-05-08
**Author:** Martin Janci (brainstorming with Claude)
**Status:** Approved for plan-writing
**Builds on:** [`2026-05-07-screen-map-system-design.md`](./2026-05-07-screen-map-system-design.md) — system spec covers Phase 3 (Sections 5.2, 5.6, 5.7, 9). [`2026-05-07-screen-map-phase-2-brainstorm.md`](./2026-05-07-screen-map-phase-2-brainstorm.md) — pattern reference.

## 1. Phase 3 Scope: split into 3a + 3b

**Decision:** Two separate plans.

- **Phase 3a** (this brainstorm + plan): the 3 remaining skills (`update`, `render`, `query`), CLAUDE.md addenda for the agent self-management protocol, and Phase 2-deferred polish items.
- **Phase 3b** (later, separate plan): bootstrap runs — actually invoke `/screens init --product=ppt` and `/screens init --product=reality` against the live repo, generating real `docs/screens/<product>/` markdown files. Phase 3b is content generation, not new code.

Single-plan was rejected because the bootstrap step generates hundreds of markdown files that would mix with new code in one giant PR — much harder to review.

## 2. `screen-map-update` — interactive drift detection

**Decision:** Interactive (mirrors `screen-map-init` UX), not report-only.

Drift checks:

1. Sitemap entry has no screen-map referencing it → propose new screen-map or attach to existing.
2. Screen-map references endpoint not in sitemap → flag.
3. `sharedComponents` not exported anywhere → flag.
4. `useCases`/`epics` referenced in screen-map but absent from `docs/use-cases.md` / `docs/epics/` → flag.
5. Screen-map present but no sitemap entry maps to it (orphan) → flag for deletion or re-bind.

Implementation: new `src/scan-drift.ts` returns `DriftIssue[]`; CLI subcommand `update` + skill manifest. CLI groups issues, prompts user, applies approved patches.

## 3. `screen-render` — three mermaid outputs

**Decision:** Three outputs written to `docs/screens/_diagrams/`:

- **Site graph** — `<scope>-site-graph.mmd` (Mermaid `graph TD`, nodes = screens, edges = `relatedScreens.rel`).
- **Endpoint matrix** — `<scope>-endpoint-matrix.md` (markdown table screens × endpoints, `✓` cells).
- **Status dashboard** — `<scope>-status.mmd` (Mermaid `pie` per platform showing build / redesign / api counts).

Scope: `--scope=product|all` (no per-epic — `screen-query` covers that case).

Implementation: new `src/render.ts` (pure: `(screens) => { siteGraph, endpointMatrix, statusDashboard }`); CLI subcommand `render` writes them to `_diagrams/`.

## 4. `screen-query` — reuse Phase 2 `parseFilter`

**Decision:** Reuse `parseFilter` from Phase 2 (comma-AND, dotted paths, already exported in cleanup commit). No upgrade to full boolean DSL — Phase 2's syntax is enough for the documented use cases.

Output formats: `--format=table` (default, terminal-friendly), `--format=json`, `--format=md`.

Implementation: new `src/query.ts` (`discoverScreenMaps` + `parseScreenMap` + `parseFilter` + format); CLI subcommand `query <expr>`.

## 5. CLAUDE.md addenda — root + 4 subprojects

**Decision:** Edit root `CLAUDE.md` plus four subproject CLAUDE.md files.

- **Root**: short "Screen-Map Self-Management Protocol" section pointing to the spec. Lists the A/B/C/D protocol rules verbatim from spec Section 9.
- **`frontend/apps/ppt-web/CLAUDE.md`**: when implementing a PPT-web route, prefix with `/screens edit <id>`; on completion run `/screens validate`.
- **`frontend/apps/reality-web/CLAUDE.md`**: same pattern for Reality.
- **`frontend/apps/mobile/CLAUDE.md`**: analog for React Native screens.
- **`mobile-native/CLAUDE.md`**: KMP-side equivalent.

Why per-subproject: Claude Code merges nested CLAUDE.md files based on the agent's working directory, so per-app rules fire only when the agent is actually working in that app.

## 6. Phase 2 deferred items folded into Phase 3a polish (T1-3)

**Included:**

- **T1**: `parseFilter` `:` split → `split(':', 2)` so values containing `:` (URL routes like `/buildings/:id`) work; add unit tests.
- **T2**: `appendAgentLog` insertion position — anchor on first `- ` line, not blank-line heuristic; add test asserting new entry sits directly above first existing entry.
- **T3**: ESM SRI hashing for `esm.sh` imports in `client/app.tsx` (pin SRI hashes, document upgrade procedure in a comment).

**Deferred out of Phase 3a:**

- SPA `app.tsx` automated tests (Vitest + happy-dom or Playwright — separate task, bigger lift).
- `--preview=design` SPA rendering (UI work, currently CLI accepts the flag but SPA only renders local/staging).
- Playwright integration in `loadScreenContext` (real I/O, hard to test cleanly).
- Worktree pre-commit hook ROOT_DIR fix (pre-existing repo issue, not screen-map specific).

These four can be addressed in Phase 3b or a separate polish pass after 3a + 3b ship.

## 7. File / skill inventory

```
frontend/packages/screen-map/src/
├── scan-drift.ts                # NEW (T4-5: drift detection)
├── render.ts                    # NEW (T6-7: mermaid generators)
├── query.ts                     # NEW (T8-9: query + format)
└── cli.ts                       # MODIFY (add update, render, query subcommands)

frontend/packages/screen-map/tests/
├── scan-drift.test.ts           # NEW
├── render.test.ts               # NEW
├── query.test.ts                # NEW
└── parse-filter.test.ts         # MODIFY (T1: add `:` split tests)

.claude/skills/
├── screen-map-update/SKILL.md   # NEW (T10)
├── screen-render/SKILL.md       # NEW (T11)
└── screen-query/SKILL.md        # NEW (T12)

.claude/commands/screens.md      # MODIFY (extend dispatcher to all 7 subcommands)

CLAUDE.md                        # MODIFY root (add self-management section)
frontend/apps/ppt-web/CLAUDE.md  # NEW or MODIFY
frontend/apps/reality-web/CLAUDE.md   # NEW or MODIFY
frontend/apps/mobile/CLAUDE.md   # NEW or MODIFY
mobile-native/CLAUDE.md          # NEW or MODIFY

frontend/packages/screen-map/src/review-server/
├── api.ts                       # MODIFY (T2: appendAgentLog anchor)
└── client/app.tsx + index.html  # MODIFY (T3: ESM SRI hashes)
```

## 8. Estimated task count

~15 tasks:

- T1-3: Phase 2 polish (parseFilter `:`, appendAgentLog anchor, ESM SRI)
- T4-5: scan-drift + tests
- T6-7: render + tests
- T8-9: query + tests
- T10-12: 3 skill manifests
- T13: CLAUDE.md addenda (root + 4 subprojects)
- T14: `/screens` dispatcher full extension (7 subcommands: validate, init, edit, review, update, render, query)
- T15: Phase 3a ship checkpoint

## 9. Open Risks Carried into Phase 3a

| # | Risk | Mitigation |
|---|------|------------|
| 1 | `screen-map-update`'s "interactive prompt" pattern doesn't translate to CI mode | Add `--report-only` flag that skips prompts and just outputs drift report; CI uses that. |
| 2 | `screen-render` mermaid syntax may render differently in different viewers (GitHub vs Mermaid Live) | Generate plain `graph TD` / `pie` syntax, test that `npx -y @mermaid-js/mermaid-cli` parses the output (lazily — only when easy). |
| 3 | `parseFilter` boolean-OR not supported, users may want it | Document in skill manifest; if a real workflow needs OR, add it in a follow-up after Phase 3b. |
| 4 | Per-subproject CLAUDE.md files may already exist with unrelated content | Append to existing files rather than overwriting; check current state during T13 before writing. |
| 5 | ESM SRI hashes need to be regenerated on every `esm.sh` upstream change | Document upgrade procedure inline + script if needed. |

## 10. Out of Scope (Phase 3b or later)

- Bootstrap runs against PPT and Reality (Phase 3b).
- SPA `app.tsx` automated tests.
- `--preview=design` SPA rendering.
- Playwright integration in `screen-edit`.
- Worktree pre-commit hook ROOT_DIR fix.
- Boolean-OR query syntax.
- Vendor `esm.sh` modules into `client/vendor/` (alternative to SRI hashing).
- Cron / autonomous-loop driven periodic queries.
