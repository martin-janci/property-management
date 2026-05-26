# Gap-Driven PM Planning — Design

**Date:** 2026-05-23
**Status:** Approved (design)
**Topic:** Make the PPT Scrum Master do real, gap-driven project planning instead of sprint-status reporting.
**Builds on:** the merged Phase 1.6 PM phase (`docs/superpowers/specs/2026-05-23-pm-analysis-phase-design.md` — 9 `pm-*` agents + `ppt-project-management` skill).

---

## 1. Problem & goal

The Scrum Master today reads only `_bmad-output/implementation-artifacts/sprint-status.yaml` (one current sprint, 29 stories) + merged PRs + `.research/backlog.json`, so it concludes "0/5 epics done, land PR #435 — nothing to do." Reality:

- **134 BMAD stories** across **17 epics** (MVP 74 / phase2 23 / phase3 13 / phase4 24); `sprint-status.yaml` tracks only ~29.
- **610 use-case lines** (`docs/use-cases.md`).
- A **stale** `gap-analysis-remediation.md` (last updated 2025-12-31 / Epic 86) that listed **120+ gaps** — Backend 63+ (8 fixed), Frontend 19 (6 fixed), **Mobile 40+ (0 fixed)** — long out of date (project now ~Epic 150).
- Screen-map: 81/99 screens shipped, but that's UI only.

**Goal:** build a current, evidence-based **story-coverage map** of all 134 stories, and have the Scrum Master produce a **real, prioritized plan with rationale** from it — runnable on demand as a skill, maintained cheaply by the daily routine.

**Spine decision:** story/epic delivery coverage (the 134 stories) is the backbone. Use-case (610 UC) coverage mapping and a full code-level gap re-analysis are **explicit follow-up layers** (Section 9), built on top of this spine later.

---

## 2. Coverage map — `.research/management/coverage.json` (the spine)

The single source of truth for "what's unfinished." One entry per BMAD story (~134):

```jsonc
{
  "generated": "<iso8601>",
  "scan_kind": "deep | upkeep",        // deep = full local scan; upkeep = incremental routine pass
  "stories": [
    {
      "id": "6-1",
      "epic": "epic-6",
      "phase": "mvp",                  // mvp | phase2 | phase3 | phase4
      "title": "Announcement creation & targeting",
      "status": "done | partial | not-started",
      "confidence": "high | medium | low",
      "platform": ["backend", "frontend", "mobile"],   // platforms the story spans
      "owner_role": "pm-backend",      // primary owning role
      "evidence": ["sprint-status: review", "routes/announcements.rs present", "screen ppt/announcements: shipped"],
      "gaps": ["no mobile screen", "no e2e test"],     // what's missing (empty if done)
      "last_checked": "2026-05-23"
    }
  ]
}
```

### Classification rules (evidence → status)
- **done** — `sprint-status: done` **OR** (implied code present **AND** the story's screen(s) `shipped` **AND** a merged PR references it). All implied platforms covered.
- **partial** — some evidence but incomplete: `sprint-status: in-progress|review`, **or** backend present but a platform slice missing (e.g., no mobile screen), **or** a handler/route stub, **or** code present with no test.
- **not-started** — no code/screen/PR evidence **and** absent from `sprint-status` (or `sprint-status: backlog`).
- **confidence** — `high` when ≥2 corroborating signals agree; `low` when inferred from a single weak signal (record why in `evidence`).

`gaps[]` is the actionable payload — each gap on a `partial`/`not-started` story becomes a candidate task.

---

## 3. Deep scan — runs LOCALLY via the skill (full toolchain)

Triggered by `/ppt-project-management scan`. Not run in the cloud routine (too heavy).

1. Enumerate the 17 epics + 49 story files under `_bmad-output/` (epics list the rest of the 134 inline).
2. **Parallelize by epic:** spawn one subagent per epic (~8 stories each). Each subagent, for its stories, gathers evidence:
   - `sprint-status.yaml` status (authoritative when present),
   - **code greps** for the routes/handlers/components/endpoints the story implies (e.g., a story about announcements → grep `announcements` in `backend/.../routes` + `frontend/.../components`),
   - **screen-map** `buildStatus` for related screens (`docs/screens/`),
   - merged-PR references (`gh pr list`/`git log` grep for the story id/keywords).
   Then classify each story (rules in §2) with evidence + gaps.
3. The skill aggregates all subagent results into `coverage.json` (`scan_kind: "deep"`).
4. Note in the run output that `gap-analysis-remediation.md` (Epic 86) is superseded by this fresh map.

Token discipline: each epic-subagent reads only its ~8 stories + targeted greps (not whole files); caps evidence lines. 17 parallel focused subagents, local only.

---

## 4. Scrum Master ranking — the real plan

The enhanced `pm-scrum-master` (with role-agent input) reads `coverage.json` and ranks the `partial` + `not-started` work using the **balanced rubric**:

| Factor | Effect on rank |
|--------|----------------|
| Phase weight | `mvp` > `phase2` > `phase3` > `phase4` |
| Dependency order | foundational/infra stories before dependents (e.g., notification/WebSocket infra before notification features) |
| Partial-before-not-started | finishing started work ranks above greenfield |
| Risk boost | security gaps, crash/data-loss paths, and the most-behind platform (mobile, 0/40 baseline) get bumped up |
| Role capacity | spread across roles; don't pile every top task on one role |

Each ranked task carries: `action`, `owner_role`, `dependency`, `priority`, and a **one-line rationale** ("why this rank"). Role agents (`pm-security`, `pm-tech-lead`, etc.) contribute domain judgment to the ranking (security flags, dependency/infra ordering).

### Outputs (under `.research/management/`)
- **`roadmap.md`** (NEW) — the ranked plan: top-N gap tasks with owner + rationale, grouped by phase, with a short "state of the project" header (epics done/total, platform coverage, biggest gaps).
- **`action-list.json`** — the top prioritized gap tasks fed in as actions (`source: "gap-scan"`, full item schema from the PM-phase spec, `owner_role`, `dependency`, `status: open`).
- **`project-state.md`** — its "What's next" section now sources the ranked roadmap, not just the current sprint.

---

## 5. Invocation — two modes of `ppt-project-management`

One skill, two modes (per approved design):
- **`/ppt-project-management scan`** — on-demand, **local**: run the deep parallel scan (§3) to rebuild `coverage.json`, then rank (§4). The "real planning now" path.
- **`/ppt-project-management`** (default) — rank/plan from the **current** `coverage.json` (no re-scan). Runs locally or in the cloud routine.

The Scrum Master agent and the 8 role agents are unchanged in identity; the skill gains the `scan` mode + the coverage.json read + the rubric ranking.

---

## 6. Cloud routine upkeep (Phase 1.6, cheap)

The daily routine does **not** re-scan all code. Each run it:
1. Marks stories `done` (or advances `partial`) when a merged PR this run maps to them.
2. Re-checks the evidence for a **rotating slice** of stories (e.g., one epic per run, via `state.pm_cursor` extended with a coverage cursor) so the whole map gets refreshed over ~17 days without a heavy scan.
3. Re-runs the Scrum Master ranking from the maintained `coverage.json` (§4).

So `coverage.json` stays current cheaply; the authoritative full refresh is the on-demand local `scan`.

---

## 7. Relationship to existing artifacts
- `sprint-status.yaml` → trusted **input** (authoritative for its ~29 stories); `coverage.json` supersets it to all 134. Read-only — the routine never writes BMAD artifacts.
- `.research/backlog.json` (improvement vectors) → separate track; the Scrum Master may fold high-priority/security vectors into the ranked plan, but it is not the gap spine.
- `gap-analysis-remediation.md` (stale) → **superseded** by `coverage.json`; not edited by this feature.

---

## 8. Token strategy (summary)
- **Deep scan:** local only, parallel by epic (17 focused subagents), targeted greps not full-file reads. Not run in the cloud.
- **Daily routine:** reads the maintained `coverage.json` (one file), updates merged-PR statuses, re-checks one rotating epic, re-ranks. No mass code reading.
- Full re-baseline whenever the user runs `scan`.

---

## 9. Out of scope now — explicit follow-up layers
1. **Use-case (610 UC) coverage mapping** — map each `docs/use-cases.md` UC to implementing stories + working code, surfacing uncovered/partial use cases. **Confirmed next follow-up** (per user). Layers onto `coverage.json` (add a `use_cases[]` ref per story + a `uc-coverage.json`).
2. **Full code-level gap re-analysis** — refresh the 120+ findings from `gap-analysis-remediation.md` against today's code (stubs, unwired handlers, missing endpoints per platform). A later layer enriching `gaps[]`.

Neither blocks the story-coverage spine; both build on it.

---

## 10. Open questions / deferred to implementation
- Exact `coverage.json` ↔ epic/story id normalization (epic file naming vs sprint-status keys vs story filenames) — resolve during the scan implementation.
- Whether `roadmap.md` top-N is fixed (e.g., 15) or per-phase quota — tune during build.
- Coverage-cursor shape in `state.json` for the rotating upkeep (extend `pm_cursor` vs a new `coverage_cursor`).
