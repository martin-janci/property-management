# Screen-Map Phase 2 — Brainstorm Decisions

**Date:** 2026-05-07
**Author:** Martin Janci (brainstorming with Claude)
**Status:** Approved for plan-writing
**Builds on:** [`2026-05-07-screen-map-system-design.md`](./2026-05-07-screen-map-system-design.md) — the system spec covers Phase 2 (Sections 5.1, 5.3, 5.5, 6, 7). This brainstorm captures the additional Phase-2-specific decisions made before writing the plan.

## 1. Phase 2 Scope

**Decision:** Single Phase 2 plan covering all five Phase-2 features in one go.

- DesignSource (interface + ZipAdapter + ClaudeDesignAdapter stub)
- Visual Review server (hono backend + Preact ESM SPA)
- `screen-map-init` skill
- `screen-map-review` skill
- `screen-edit` skill
- Plus Phase-1 polish items (slugify diacritics, ENOENT narrowing, body `\r?\n`, etc.) folded as the first three tasks.

Estimated 23 tasks total. Bigger than Phase 1, but coherent. Splitting into 2a + 2b was considered and rejected — DesignSource is only meaningfully consumed by the review server, and `screen-map-review` depends on the server, so separating them produces a 2a that delivers no end-user value beyond what `screen-map-init` provides on its own.

## 2. `scan.ts` is multi-source

**Decision:** Candidates come from any combination of:

1. `@ppt/sitemap` IDs (`pptWebRoutes`, `realityWebRoutes`, `mobileScreens`) — existing code.
2. `docs/use-cases.md` regex pulls (`UC-NN.M`) and epic IDs from any `EPIC-NNN-*.md` under `docs/epics/` — planned features.
3. **DesignSource frames** when `--designs=<zip>` is passed — designed but not-yet-coded screens get `buildStatus: planned` with `designSources` populated.
4. **User-provided list** when `--add="<feature description>"` is passed, or when the user offers planned screens during the interactive grouping phase (e.g., "also add: Faults assignment modal").

`screen-map-init` merges, dedupes, presents to the user in chat for grouping decisions.

**Why:** Real product backlogs contain both built-and-shipped screens (sitemap), in-design screens (designs ZIP), and roadmap-planned screens (user request). The screen-map should cover all three states from day one, not just shipped code.

## 3. Interactive grouping is chat-driven

**Decision:** No stdin prompts, no GUI, no forms. The init skill prints a markdown report into the Claude session ("Found 47 routes + 23 mobile screens + 8 design frames + 5 user-requested. Suggested 38 logical concepts; below is the proposed grouping…"). The user replies in chat with corrections ("merge X+Y → building-detail; split Z into Z + Z-modal; skip W"). The skill iterates until the user says "go" / "OK".

**Why:** Matches how Claude sessions naturally work. No bespoke UI to build. Copy-pasting between terminal and a separate prompt is friction.

## 4. Visual Review SPA stays no-build

**Decision:** Preact via `esm.sh` ESM imports, no Vite/Webpack, no build step in the package.

**Fallback:** If browser ESM imports fail in real use (CSP, offline, version drift on `esm.sh`), Vite is the planned fallback — but only after we hit a real failure. Don't pre-optimize.

## 5. `screen-edit <id>` outputs agent-friendly markdown summary

**Decision:** Skill loads context (parent / child / related screen-maps + sitemap entries + OpenAPI snippets for endpoints + diagram refs + optional Playwright screenshot) and emits a markdown summary directly into the Claude session. No HTML report file, no separate artifact.

**Why:** The agent is the consumer. A markdown summary in chat lets the agent immediately reason about the screen and start editing.

## 6. `DesignFrame.imageUrl` resolves via server endpoint

**Decision:** Visual Review server exposes `GET /api/designs/:adapter/:frame-id` that streams the image from the DesignSource (ZipAdapter extracts on-demand from the ZIP, in-memory caches the bytes for the session lifetime).

**Why:** Avoids data-URL bloat in JSON responses. CSP-friendly. Memory-friendly when there are many frames. Clean adapter boundary — adapters return a frame ID, not bytes.

## 7. Polish-first ordering

**Decision:** First three tasks of Phase 2 plan address Phase-1 review-flagged items before introducing new features:

- **T1**: schema robustness — `IsoDateSchema` accepts both `string` and `Date` (gray-matter auto-coerces unquoted ISO dates); body normalization tolerates `\r?\n` (Windows authoring).
- **T2**: `discover.ts` ENOENT narrowing + redundant `.gitkeep` removal + test `mkdtemp` shim removal + `validate.ts` warning lane comment OR first warning rule.
- **T3**: `context.ts` Unicode-normalize-then-strip slugify (Slovak/Czech anchor support) + new `context.test.ts` direct unit coverage + refresh stale `scripts/install-hooks.sh` summary list.

**Why:** These are quality issues that real users (= us, in Phase 3 bootstrap runs) would hit immediately. Folding them in here keeps Phase 2 from accumulating debt and makes the foundation suitable for the higher-traffic skills coming on top of it.

## 8. Out of Scope (Phase 3)

Explicitly deferred to Phase 3 plan:

- `screen-map-update` (drift detection between code and screen-maps).
- `screen-render` (mermaid generators).
- `screen-query` (read-only frontmatter queries).
- Root + per-subproject CLAUDE.md addenda for agent self-management protocol.
- First bootstrap runs of `/screens init` against PPT and Reality.
- Periodic-loop / cron automation around `screen-query`.

## 9. Open Risks Carried into Phase 2

| # | Risk | Mitigation |
|---|------|------------|
| 1 | ESM-only client may break in some browsers (CSP, offline, registry rate-limit) | Vite fallback documented; revisit if real failures occur. |
| 2 | Large ZIP design files could exhaust memory if extracted naively | Extract per-frame on first request; cache in-memory; document a max-frame-size warning in ZipAdapter README. |
| 3 | `screen-map-init` interactive flow may produce ambiguous user replies | Skill should re-prompt when the reply doesn't parse, not silently apply a default. |
| 4 | Worktree-vs-shared-hooks `ROOT_DIR` quirk continues to force `--no-verify` from inside worktrees | Pre-existing; Phase 2 doesn't fix it but adds a follow-up note in the plan's Out-of-Scope section. |
| 5 | `screen-edit` Playwright invocation latency (~3-5s) when local app isn't running | Skill defaults to "skip Playwright if `localhost:5173` doesn't respond within 1s"; honors the `--preview=staging` flag (uses `rlt-deploy`-published staging URL). |
