# Screen-Map System — Design Spec

**Date:** 2026-05-07
**Author:** Martin Janci (brainstorming with Claude)
**Status:** Approved for implementation planning

## 1. Goal

Build a project-management layer for the Property Management monorepo that:

- Maintains a complete screen-by-screen description of both products (PPT — `ppt-web` + `mobile`; Reality — `reality-web` + `mobile-native`).
- Tracks per-platform implementation status, redesign status, and API status independently.
- Lists endpoints, related screens, shared components, mermaid diagrams, and free-form notes per screen.
- Lets an agent self-manage the data (drift detection, validation, periodic updates).
- Provides a Visual Review tool with a per-screen working page (Next button, OK / Note checkboxes) usable by the human reviewer.
- Bootstraps from existing project artefacts: code (routes, screens), `@ppt/sitemap`, `docs/use-cases.md`, epics/AC, and optional design assets (ZIP / Claude Design API).

## 2. Architectural Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Hybrid storage: existing `@ppt/sitemap` package as the canonical route/endpoint register, `docs/screens/<product>/<id>.md` as the rich human-and-agent store. | Sitemap is strictly typed code data; screen-map is a living document. Markdown with YAML frontmatter is grep-friendly, git-diff-friendly, agent-editable. |
| 2 | One screen-map file per *logical concept*, scoped to a *product*. `docs/screens/ppt/building-detail.md` covers `ppt-web` + `mobile` instances of the same UX. | Mirrors real-world product boundaries. PPT app = web + RN; Reality = Next.js + KMP. Cross-product reuse is rare. |
| 3 | Visual Review delivered via a mini local HTTP server launched by a skill, with a static SPA client and direct markdown writes. | No paste-back step; immediate feedback; can iframe the live app side-by-side with the design frame. |
| 4 | Initializer is hybrid: bulk-scan candidates → interactive grouping → bulk-write. Designs are pluggable via a `DesignSource` interface. | Combines speed of auto-detection with accuracy of human-validated groupings. ZIP adapter ships first; Claude Design adapter is a stub. |
| 5 | Granularity = logical UX unit (pragmatic). A modal / sheet earns its own file iff it has its own functionality checklist, endpoints, OR redesign milestone. Otherwise it's a `## States` section in the parent. | Avoids file explosion while keeping major flows individually trackable. |
| 6 | Three-axis status per platform: `buildStatus`, `redesignStatus`, `apiStatus`. | Build and redesign run as independent tracks; API can lag implementation. Collapsing them loses information needed for planning. |
| 7 | Full skill inventory (7 skills + 1 slash command). | Heavy first investment, but covers full lifecycle: bootstrap, drift, review, validate, focused edit, render, query. |

## 3. Layout

```
property-management/
├── docs/
│   └── screens/                          # NEW — human-and-agent canonical store
│       ├── README.md                     # how this works
│       ├── _template.md                  # template for new screens
│       ├── _diagrams/                    # mermaid files referenced by screens
│       ├── ppt/                          # PPT product (ppt-web + mobile)
│       │   ├── building-detail.md
│       │   ├── faults-list.md
│       │   └── ...
│       └── reality/                      # Reality product (reality-web + mobile-native)
│           ├── property-detail.md
│           └── ...
├── frontend/
│   └── packages/
│       ├── sitemap/                      # EXISTING — canonical route/endpoint source
│       └── screen-map/                   # NEW — TS API + tooling
│           ├── src/
│           │   ├── types.ts              # ScreenMap, FeatureItem, Implementation, AgentLogEntry
│           │   ├── parse.ts              # markdown → ScreenMap
│           │   ├── write.ts              # ScreenMap → markdown (preserves user edits in body)
│           │   ├── validate.ts           # consistency vs sitemap + OpenAPI + filesystem
│           │   ├── scan.ts               # auto-detect routes/screens from code
│           │   ├── design-source/
│           │   │   ├── index.ts          # DesignSource interface
│           │   │   ├── zip-adapter.ts    # ZIP with frames (first-class)
│           │   │   └── claude-design.ts  # stub, throws NotImplementedError
│           │   └── review-server/
│           │       ├── server.ts         # bun.serve() / hono
│           │       ├── api.ts            # endpoints: load, save, finish
│           │       └── client/
│           │           ├── index.html
│           │           ├── app.tsx       # preact SPA, ESM-only (no build step)
│           │           ├── styles.css
│           │           └── components/
│           ├── tests/
│           └── package.json
└── .claude/
    ├── skills/                           # NEW — agent skills
    │   ├── screen-map-init/
    │   ├── screen-map-update/
    │   ├── screen-map-review/
    │   ├── screen-map-validate/
    │   ├── screen-edit/
    │   ├── screen-render/
    │   └── screen-query/
    └── commands/
        └── screens.md                    # slash command dispatcher
```

CLAUDE.md addendum: a short reference section at root `CLAUDE.md`, with detailed per-subproject rules at `frontend/CLAUDE.md`, `frontend/apps/ppt-web/CLAUDE.md`, `mobile-native/CLAUDE.md`, etc. Claude Code merges these automatically based on the current working directory.

## 4. Screen-Map File Format

### 4.1 Path

`docs/screens/<product>/<id>.md` where `<product> ∈ {ppt, reality}` and `<id>` is a kebab-case slug.

### 4.2 Sample

```markdown
---
# === IDENTITY ===
id: ppt/building-detail
name: Building Detail
product: ppt                        # ppt | reality
sitemapRefs:                        # IDs from @ppt/sitemap
  ppt-web: ppt-building-detail
  mobile: mobile-building-detail-screen

# === STATUS (3 axes per platform) ===
implementations:
  ppt-web:
    route: /buildings/:id
    component: BuildingDetailPage
    buildStatus: shipped            # planned | in-progress | shipped | n/a
    redesignStatus: applied         # not-started | in-progress | applied | n/a
    apiStatus: complete             # stub | partial | complete | n/a
  mobile:
    screen: BuildingDetailScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete

# === RELATIONS ===
endpoints:                          # IDs from @ppt/sitemap
  - building_get
  - building_update
  - units_list
relatedScreens:
  - { id: ppt/buildings-list, rel: parent }
  - { id: ppt/building-edit, rel: action }
  - { id: ppt/unit-detail, rel: child }
sharedComponents:
  - BuildingHeader
  - UnitsTable
  - StatusBadge
diagrams:
  - { ref: docs/sequence-diagrams.md#building-detail-load, kind: sequence }
  - { ref: ./_diagrams/building-detail-flow.mmd, kind: flow }
useCases: [UC-12, UC-13]
epics: [Epic-15]
designSources:
  - { adapter: zip, file: designs/2026-Q2.zip, frame: building-detail-v3 }

# === META ===
owner: pm-frontend
lastReview: 2026-05-04
---

## Functionality Checklist

Platform tags: `[w]` = ppt-web / reality-web, `[m]` = mobile / mobile-native, `[w,m]` = both, `[-]` = neither.

- [x] [w,m] View building info card (name, address, owner)
- [x] [w,m] List units with status badges
- [x] [w] Edit building info → opens `ppt/building-edit` modal
- [ ] [m] Edit building info (mobile parity, planned)
- [ ] [w,m] Export to PDF (planned, Q3 2026)

## States

- **Empty** (no units): show "Add first unit" CTA, links to `ppt/unit-create`
- **Loading**: skeleton building card + 3 unit-row skeletons
- **Error** (fetch fail): retry button + localised error message (sk/cs/en/de)
- **Permission denied**: shown when user lacks tenant/role

## Notes

### Broader context
- Header card pattern is shared with `reality/property-detail` (same visual, different product).
- Units table uses generic `<DataTable>` from `@ppt/ui-kit` — table changes propagate to `faults-list`, `disputes-list`.
- Redesign milestone "Header v3" applies to all PPT mobile, tracked in Epic-15.

### Specific (recent)
- 2026-05-01: Redesign moved unit count badge into header (web done, mobile in-progress).
- 2026-04-22: Added export-to-pdf to roadmap for Q3 2026.

## Agent Log

<!-- newest entries on top -->

- 2026-05-07 — agent: confirmed mobile redesign blocker (Figma frame `building-detail-v3-mobile` missing in ZIP). Filed issue.
- 2026-05-03 — agent: updated `apiStatus: complete` after verifying `building_update` has a concrete handler in api-server.
```

### 4.3 Semantic rules

- **Frontmatter is the authoritative status source.** No status info is encoded in markdown checkboxes.
- **Functionality Checklist** uses `[w]`/`[m]`/`[w,m]`/`[-]` platform prefixes. Skill `screen-query` can count per-platform completion.
- **States** holds inline-states that did not earn their own file (per Decision 5).
- **Notes** has two subsections:
  - **Broader context** — semi-stable invariants, design patterns, cross-screen relationships.
  - **Specific (recent)** — dated short-lived events, decisions, blockers.
- **Agent Log** is an audit trail. Every skill that mutates the file appends one line: `<date> — <actor>: <terse summary>`. Newest entries on top.

## 5. Skill Inventory

All skills dispatch via the `/screens <subcommand>` slash command in `.claude/commands/screens.md`.

### 5.1 `screen-map-init`

**Purpose:** bootstrap a fresh screen-map for one product.

**Args:** `--product=ppt|reality` (required), `--designs=<path-to-zip>` (optional), `--scope=full|epic-N` (optional), `--force` (optional, overrides existence guard).

**Flow:**

1. **Bulk-scan** candidates: `App.tsx` routes, `mobile/app/**`, `@ppt/sitemap` entries, `docs/use-cases.md`, `docs/architecture.md`, epics/AC.
2. **Interactive grouping** — present a dashboard ("47 routes, 23 mobile screens, suggested 38 logical concepts; merge X+Y into `building-detail` because shared route shape Z"). User confirms or corrects via terminal prompt.
3. **Bulk-write** — generate the markdown files. Frontmatter pre-populated with auto-detected statuses; notes left empty; Agent Log seeded with init entry.

**Safety:** if `docs/screens/<product>/` already exists and `--force` is not set, abort with a hint to use `--force` (overwrites empty files only) or `screen-map-update` (preserves user edits).

### 5.2 `screen-map-update`

**Purpose:** detect drift between code/sitemap and screen-maps, propose patches.

**Args:** `--product=ppt|reality` (optional, default both), `--since=<git-ref>` (optional).

**Detected drifts:**

- New route in code is not referenced by any screen-map → prompt to assign or create.
- Screen-map references endpoint missing from sitemap → flag.
- `sharedComponents` entry not exported from `@ppt/ui-kit` or app → flag.
- `useCases`/`epics` referenced in screen-map but absent from project docs → flag.
- New use cases / epics added to project docs since last update → suggest mapping.

**Output:** chat report + proposed patches. User approves selectively, skill applies.

### 5.3 `screen-map-review`

**Purpose:** the human-driven Visual Review with a per-screen working page.

**Args:** `--product=ppt|reality` (optional), `--filter=<frontmatter-query>` (optional, e.g. `redesignStatus:in-progress`).

**Flow:**

1. Skill starts the Visual Review server (Section 6) on a free port (5179+).
2. Browser opens at `http://127.0.0.1:5179/?session=<token>`.
3. User clicks through screens (left = metadata + checklists, right = preview), Save & Next on each.
4. Server writes to markdown directly (Agent Log + Notes > Specific).
5. Skill's slash command waits for `POST /api/session/finish` or Ctrl-C, then graceful-shuts down the server.

**Save semantics — what review writes:**

- Always: an `Agent Log` entry summarising the review (`2026-05-07 — review: 4 OK, 1 note ("missing on mobile" on "Edit info")`).
- If user typed a general note for the screen: an entry in `Notes > Specific`.
- Always (when review session finishes): updates `lastReview` in frontmatter.

**Save semantics — what review does NOT write:**

- It does not change `buildStatus`, `redesignStatus`, or `apiStatus`. Status mutations are reserved for `screen-edit` / `screen-map-update`. Review captures *user feedback*, not *implementation status*.

### 5.4 `screen-map-validate`

**Purpose:** consistency check.

**Args:** `--strict` (optional, non-zero exit on any failure).

**Checks:**

- Frontmatter conforms to schema (Zod / TypeBox in `@ppt/screen-map`).
- All `endpoints` IDs exist in `@ppt/sitemap`.
- All `sitemapRefs` IDs exist in `@ppt/sitemap`.
- All `relatedScreens.id` resolve to existing screen-map files.
- All `sharedComponents` are real exports.
- All `diagrams.ref` paths and anchors are reachable.

**Hooks:**

- Pre-commit hook (via `scripts/install-hooks.sh`) runs `validate` on changed `docs/screens/**` files; commit fails on errors.
- CI workflow runs `validate --strict` on PRs that touch `docs/screens/**` or route files; fail blocks merge.

### 5.5 `screen-edit <id>`

**Purpose:** focused context-load for working on a single screen.

**Args:** `<id>` (required, e.g. `ppt/building-detail`).

**Flow:**

1. Load the screen-map plus parent / child / related screen-maps.
2. Load referenced sitemap entries, OpenAPI snippets for endpoints.
3. Load source files for `BuildingDetailPage.tsx` and the mobile screen.
4. Load referenced diagrams.
5. Optionally invoke Playwright on `/buildings/:id` to take a screenshot if `pnpm dev` is running.
6. Return an agent-friendly summary.

**Use case:** prefix any implementation task on a known screen with `/screens edit <id>` so the agent loads canonical context instead of fishing for it.

### 5.6 `screen-render`

**Purpose:** generate visualisations from screen-map data.

**Args:** `--scope=product|epic|all`, `--out=<path>`.

**Generates:**

- **Site graph** — nodes = screens, edges = `relatedScreens.rel` (parent / child / action / sibling).
- **Endpoint matrix** — screens × endpoints heatmap.
- **Status dashboard** — counts per platform per build/redesign/api status.

**Output:** mermaid blocks saved as `docs/screens/_diagrams/<scope>.mmd`, plus a generated overview markdown.

### 5.7 `screen-query`

**Purpose:** read-only ad-hoc queries against frontmatter.

**Args:** `--filter=<query-string>`, `--format=table|json|md`.

**Examples:**

```
/screens query "redesignStatus:not-started AND product:ppt"
/screens query "implementations.mobile.buildStatus:planned"
/screens query "endpoints:building_update"
```

Output is a list of matching screens with key columns (id, name, statuses, lastReview).

## 6. Visual Review Server

### 6.1 Stack

- Runtime: Node 20+ (matches the rest of `frontend/`); `hono` for routing — `hono` runs identically on Node, Bun, or Deno, so the runtime choice is a follow-up.
- Client: Preact via ESM `esm.sh`, no build step. (Vite as a fallback if browser ESM imports prove too restrictive in practice.)
- Storage: direct reads/writes against `docs/screens/<product>/*.md` via `@ppt/screen-map`.

### 6.2 Endpoints

```
GET  /api/session                      → { product, screens: [...summary], currentIdx, sessionToken }
GET  /api/screens/:id                  → { frontmatter, checklist, states, notes, agentLog, previewUrls }
POST /api/screens/:id/review
       body: { decisions: [{itemKey, ok, note?}], generalNote? }
       → updates markdown; returns { nextScreenId } or { done: true }
POST /api/session/finish               → graceful shutdown signal
GET  /api/designs/:adapter/:frame-id   → image stream (proxied from DesignSource)
```

### 6.3 Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ TOPBAR  ← Prev   [Screen 3 / 12] ppt/building-detail   Next →    │
├────────────────────────────────┬─────────────────────────────────┤
│ LEFT (metadata + checklists)   │ RIGHT (preview)                 │
│ Status: shipped, redesign:     │ ┌─────────────────────────────┐ │
│   in-progress (mobile)         │ │ iframe localhost:5173/...   │ │
│                                │ │                             │ │
│ Functionality (per item):      │ │     [live ppt-web]          │ │
│ ☑ View building info ──[ok]   │ └─────────────────────────────┘ │
│   [add note...]                │                                 │
│ ☑ List units ─────────[ok]    │ Toggle: [Live app] [Design ZIP]│
│ ☐ Edit info → modal           │         [Side-by-side]          │
│   ⚠ Note: "missing on mobile"│                                  │
│                                │                                 │
│ States: empty / loading / err │                                  │
│                                │                                 │
│ General note for screen:       │                                 │
│ [textarea............]         │                                 │
│                                │                                 │
│ [Save & Next ──────────────►]  │                                 │
└────────────────────────────────┴─────────────────────────────────┘
```

### 6.4 Security & lifecycle

- Bind to `127.0.0.1` only.
- Random session token in URL; API rejects requests without matching token.
- No persistent server-side state outside markdown.
- Server graceful-shuts down on `POST /api/session/finish` or terminal SIGINT.

## 7. DesignSource Adapter

```typescript
export interface DesignFrame {
  id: string;
  name: string;
  imageUrl: string;
  width: number;
  height: number;
  metadata?: Record<string, unknown>;
}

export interface DesignSource {
  name: string;
  list(): Promise<DesignFrame[]>;
  get(id: string): Promise<DesignFrame | null>;
}
```

### 7.1 ZipAdapter (first-class, ships in v1)

Config:

```yaml
designSources:
  - { adapter: zip, file: designs/2026-Q2.zip, frame: building-detail-v3 }
```

Convention for ZIP layout:

```
designs/2026-Q2.zip
├── frames/
│   ├── building-detail-v3-web.png
│   ├── building-detail-v3-mobile.png
│   └── ...
└── manifest.json     # { frames: [{id, name, file, width, height, ...}] }
```

`screen-map-init` reads `manifest.json` during interactive grouping and offers a `<frame-id> → <screen-id>` mapping step.

### 7.2 ClaudeDesignAdapter (stub, v1)

`throw NotImplementedError("see docs/claude-design-integration.md")`. Interface scaffolded so swap-in is a single-file change.

## 8. Bootstrap Workflow

```
1. user: /screens init --product=ppt
   ↓
2. screen-map-init:
   - scan: App.tsx routes, mobile/app/**, @ppt/sitemap, docs/use-cases.md, docs/architecture.md, epics
   - report: "Found 47 routes + 23 mobile screens. Suggested 38 logical concepts.
              Below is the grouping proposal — confirm OK or correct."
   ↓
3. user replies (text in chat or interactive prompt):
   - "merge X+Y+Z into building-detail"
   - "split foo into foo + foo-modal"
   - "skip Z"
   ↓
4. screen-map-init bulk-write:
   - creates docs/screens/ppt/<id>.md per final concept
   - frontmatter: statuses auto-detected
   - agent log seeded
   ↓
5. screen-map-validate runs automatically; report any errors.
   ↓
6. user runs: /screens review --product=ppt
   - walks 38 screens, adds redesign notes, missing-feature notes.
   ↓
7. git commit, push, PR.
```

For Reality: same flow with `--product=reality`.

## 9. Agent Self-Management Protocol

Documented in CLAUDE.md addenda (root + per-subproject as needed).

### A) On screen-related code changes

1. Before commit: `screen-edit <id>` to load context.
2. Update `buildStatus` / `apiStatus` in frontmatter if outcomes changed.
3. Add Agent Log entry: `<date> — agent: <terse summary>`.
4. Update `Notes > Specific` if change is relevant to future agents (e.g. "API now requires X-Tenant header").
5. Run `screen-map-validate`.

### B) On new route / mobile screen added

1. `screen-map-update` detects drift.
2. Prompt: "New route `/foo` — create new screen-map or attach to existing?"
3. User decides; skill applies.

### C) On redesign milestone (Figma frame ready)

1. `/screens review --product=ppt --filter=redesignStatus:not-started` walk-through.
2. Each screen: OK or note. Review writes Agent Log + Notes.
3. After implementation: agent in `screen-edit <id>` flips `redesignStatus: in-progress → applied`.

### D) Periodically (manual in v1)

- `/screens query "buildStatus:shipped AND redesignStatus:not-started AND product:ppt"` — find redesign roadmap candidates.
- `/screens render --scope=ppt` — refresh status dashboard mermaid.

Automation (cron / `/loop /screens query ...`) is a follow-up after v1 use shapes the cadence.

## 10. Testing & CI

- **Unit tests** in `frontend/packages/screen-map/tests/` for parse, write, validate, scan, design-source.
- **Integration test** for Visual Review server: spawn server, simulate `POST /api/screens/:id/review`, verify markdown mutation.
- **CI workflow** `.github/workflows/screen-map.yml`: on PRs touching `docs/screens/**` or route files, run `screen-map-validate --strict`. Fail blocks merge.
- **Pre-commit hook** (`scripts/install-hooks.sh`): runs `validate` on the changed subset only.

## 11. Out of Scope (v1)

- Cron / autonomous-loop driven periodic queries (Section 9.D).
- Claude Design API adapter (stub only).
- Cross-product screens (a single screen-map covering both PPT and Reality). Section 2 explicitly product-scopes screens.
- Screen-map → external tracker (Linear / Jira) sync.
- Multi-user concurrent review sessions (server is single-user / single-session).
- OpenAPI completeness check beyond endpoint-id existence (e.g. validating request shapes).

These can each become follow-up iterations once v1 is in active use.

## 12. Open Questions / Risks

| # | Item | Mitigation |
|---|------|------------|
| 1 | Auto-detection of `apiStatus` may be brittle (depends on heuristics: stub handler vs real). | Start with conservative defaults (`stub` if endpoint exists in OpenAPI but handler returns `todo!()` in Rust source); allow manual override. |
| 2 | Interactive grouping in `screen-map-init` is a slow human-in-the-loop step. | First run can take a session; subsequent updates use `screen-map-update` which is incremental. |
| 3 | ZIP design import requires manifest convention; teams without it cannot use the adapter. | Document the manifest.json shape in `docs/screens/README.md`. Ship a small CLI to generate it from a Figma export if needed. |
| 4 | Pre-commit hook may slow commits on large screen-map edits. | Validate only changed files in pre-commit; full validate runs in CI. |
| 5 | Visual Review server iframe-ing live app may break on auth-protected routes. | Server can pre-fill a dev-mode session token / mock auth; documented in review README. |
