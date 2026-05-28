# Project Management & Analysis Phase — Design

**Date:** 2026-05-23
**Status:** Approved (design)
**Topic:** Add a project-management / delivery-analysis phase to the daily PPT research routine

---

## 1. Overview & goals

The PPT daily research routine (cloud CCR agent, `.research/routine-prompt.md`) currently:
observes repo activity (Phase 1), runs a rotating code review (Phase 1.5), scores a
backlog, promotes plans, optionally auto-fixes, and writes a brief.

This feature adds **Phase 1.6 — Project Management & Delivery**: a standing
delivery-management layer that turns the routine's observations + the project's own
planning artifacts into a living PM picture and pushes a concise digest to Telegram.

It answers, every run: **what shipped since last time, what's next, who should do it,
what's blocked, and what each technical role should focus on.**

The "project" being managed is **the PPT app development itself** — the roles are the
people building `property-management`. The phase is a *synthesis* layer; it does not
replace the research backlog or the BMAD sprint artifacts, it organizes them.

### Success criteria
- Every routine run updates a living `project-state.md` dashboard and a canonical
  `action-list.json`, and (unless fully quiet) sends a Telegram digest.
- The expensive multi-role analysis is **token-bounded**: one role per day by default.
- The role prompts exist as **reusable repo agents** (`@pm-security`, etc.), usable
  standalone outside the routine.
- All routine-written output stays under `.research/` (G8/G9 scope holds).

---

## 2. Cadence model

The **Scrum Master** runs **every** routine run (cheap: reads `sprint-status.yaml` +
Phase-1 PR/issue data + research backlog, then synthesizes). The expensive part is the
8 specialist **role agents**; their cadence is driven by the trigger payload
(`$TRIGGER_TEXT`):

| `$TRIGGER_TEXT` | Roles run that run |
|-----------------|--------------------|
| `""` (normal scheduled run) | Scrum Master + **1 rotating role** (8-day cycle via `state.pm_cursor`) |
| `pm-full` or `full` | Scrum Master + **all 8 roles** |
| `pm:<role>` (e.g. `pm:security`, `pm:backend`) | Scrum Master + **that role only** |

Rotation guarantees full coverage every 8 days at ~1 role/day cost. Unneeded roles are
simply not run — tokens spared, rotation backfills. (Mirrors the proven Phase 1.5
segment rotation.)

### The 8 rotating roles + the always-on coordinator
1. **pm-scrum-master** (always) — delivery lead / coordinator
2. pm-tech-lead — tech lead / architect
3. pm-backend — backend
4. pm-frontend — frontend / mobile
5. pm-qa — QA / test
6. pm-devops — DevOps / infrastructure
7. pm-security — security
8. pm-data — data / analytics
9. pm-integration — integration / API owners

---

## 3. Inputs (source of truth for "the work")

| Input | Path | Used for |
|-------|------|----------|
| Sprint status (spine) | `_bmad-output/implementation-artifacts/sprint-status.yaml` | epic/story status: `backlog / ready-for-dev / in-progress / review / done / blocked` |
| Epics | `_bmad-output/epics*.md`, `_bmad-output/epics/` | epic detail (read on demand, not all daily) |
| Stories | `_bmad-output/implementation-artifacts/stories/` | story detail for the active sprint |
| Repo activity | Phase 1 output (merged PRs, issues, churn) | "what shipped since last run" |
| Research track | `.research/backlog.json`, `.research/plans/` | improvement/bug vectors as additional work items |

Cost control: the Scrum Master reads `sprint-status.yaml` (single ~90-line file) as the
spine and only opens the specific epics/stories that are `in-progress`/`review` this
sprint. Role agents read only their domain slice of the active sprint.

---

## 4. Components

### 4a. Repo agents — `.claude/agents/pm-*.md` (9 files)
Each agent file embeds that role's prompt (adapted from the user's master + role
prompts) with a **cloud/static contract**: read-only, no compile/run, scoped to the
active sprint, capped output, returns a fixed JSON+markdown shape. These are normal
Claude Code agent definitions, so a human can invoke them standalone (`@pm-security`).

Single source of truth: the orchestrator skill spawns a subagent per selected role and
points it at `.claude/agents/pm-<role>.md` ("read this file, act as this role, analyze
the active sprint, return the shape below"). The agent file is therefore both the
standalone agent AND the prompt the routine replays — no duplication. (The cloud routine
already spawns subagents this way in Phase 1.5 / `ppt-dev-review`, so the mechanism is
proven in CCR.)

### 4b. Orchestrator skill — `.claude/skills/ppt-project-management/SKILL.md`
Invoked by Phase 1.6. Responsibilities:
1. Parse `$TRIGGER_TEXT` → decide role set (rotating | full | specific).
2. Run the Scrum Master synthesis (sprint-status + Phase-1 + backlog).
3. Spawn the selected role agent(s) as subagents; collect their analyses.
4. Write/update the `.research/management/` artifacts.
5. Advance `state.pm_cursor`.
6. Produce the Telegram digest payload (the routine sends it in Phase 6).

### 4c. Routine integration — Phase 1.6 in `.research/routine-prompt.md`
A new section after Phase 1.5 that invokes the skill, plus:
- a guard initializing `state.pm_cursor` if absent (mirrors the Phase 1.5 review_cursor guard);
- the Telegram send step (Phase 6 or end of 1.6), gated to skip on fully-quiet runs;
- new quality-gate coverage so `.research/management/` files are validated like other artifacts.

---

## 5. Storage layout — `.research/management/`

All under `.research/` → satisfies G8 (routine commit is `.research/`-only) and G9 (no secrets).

| File | Lifecycle | Contents |
|------|-----------|----------|
| `project-state.md` | regenerated every run | exec summary, sprint progress (epics done/total), shipped-since-last-run, what's-next, top blockers/risks, today's role focus |
| `action-list.json` | updated every run | canonical actions: `{id, action, owner_role, priority, dependency, status, deadline, source}` |
| `action-list.md` | regenerated from json | human-readable action table |
| `decisions.md` | append/update | decision log: made / needed / who decides / by-when |
| `risks.json` | updated | risk register: `{id, risk, probability, impact, mitigation, owner_role, trigger, status}` |
| `stakeholders.md` | static, rare edits | role map: responsibility, required inputs/outputs, decision authority |
| `roles/<role>.md` | overwritten when that role runs | latest per-role deep analysis (responsibilities, next 1–2wk tasks, deps, risks, open questions, DoD) |

`state.json` gains:
```jsonc
"pm_cursor": {
  "rotation": ["pm-tech-lead","pm-backend","pm-frontend","pm-qa","pm-devops","pm-security","pm-data","pm-integration"],
  "next_index": 0,                 // which rotating role runs on the next normal run
  "role_last_run": { "pm-security": "2026-05-23", ... }  // null = never
}
```

---

## 6. Telegram digest

- **When:** sent in **Phase 6, after the routine commit** (so the digest reflects landed state); **skip only when fully quiet** (nothing shipped AND no new/changed action AND no blocker change). Best-effort: a failed send is logged in the brief and is non-fatal.
- **How:** `curl` to `https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendMessage`, `chat_id=$TELEGRAM_CHAT_ID`, reusing the same values the Plex routine uses (env vars, not committed).
- **Secret safety:** token is read from env only; it is NEVER written to any `.research/` file, log line, or commit (G9 already greps for `Authorization: Bearer` etc.; the digest body must not echo the token).
- **Content (concise):**
  ```
  📋 PPT delivery — <YYYY-MM-DD HH:MM>
  Sprint: <sprint_name> — <epics_done>/<epics_total> epics done
  Shipped since last run: <N PRs / stories moved to done>
  Next up:
   • <action 1> — <owner_role>
   • <action 2> — <owner_role>
   • <action 3> — <owner_role>
  Blockers: <none | list>
  Role focus today: <role(s) run this run>
  ```

### Setup dependency (UI, flagged for the user)
Env vars are per-environment. `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` must be added
to the **ppt-research** environment (`env_01UY9zjDZWJHYx1oYBBTTd8S`) with the same values
the Plex environment uses. Until then, the digest step logs "telegram not configured" and
the routine continues (non-fatal).

---

## 7. Token-budget discipline

- Scrum Master: 1 cheap file (`sprint-status.yaml`) + already-collected Phase-1 data + backlog.json. No code reading.
- Role agents: read only the active-sprint epics/stories + their domain slice; cap output (≤ ~6 next-actions, ≤ ~5 risks); no compile/run; skip files > 500 lines unless central.
- Default daily cost ≈ Scrum Master + 1 role. `full` is opt-in (manual trigger).

---

## 8. Scope & gate compliance

- **Routine writes only `.research/management/**` + the existing `.research/` artifacts** → G8 (commit is `.research/`-only) holds.
- The **agents + skill + routine-prompt + this spec are infrastructure**, committed once by the implementing PR — NOT written by the routine at runtime.
- G9 (no secrets): Telegram token from env only, never serialized.
- New quality-gate checks (Phase 6) validate `action-list.json` / `risks.json` parse and `project-state.md` exists when Phase 1.6 ran.

---

## 9. Out of scope (YAGNI)
- No GitHub Issue/PR creation from the PM phase (Phase 5 auto-fix already owns repo-writes; PM is read+report).
- No editing of `sprint-status.yaml` or BMAD artifacts by the routine (read-only).
- No web dashboard — `project-state.md` + Telegram are the surfaces.
- No per-story estimation/velocity math beyond counts in v1.

---

## 10. Open questions / decisions deferred to implementation
- Exact rotation order (above is a reasonable default; tune during build).
- Whether `roles/<role>.md` should keep a short history tail or always overwrite (v1: overwrite; cheap).
- Branch/merge strategy for shipping this feature (feature branch + PR to dev vs direct) — decide at finish time.
