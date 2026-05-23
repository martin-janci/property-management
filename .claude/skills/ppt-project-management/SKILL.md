---
name: ppt-project-management
description: >
  Project-management & delivery analysis for the PPT research routine (Phase 1.6).
  Runs an always-on Scrum Master synthesis plus role-based deep analysis (rotating one
  role/day by default; all 8 on `full`; a specific role on `pm:<role>`). Reads
  sprint-status + repo activity + research backlog, spawns role subagents, and writes
  delivery artifacts under .research/management/. Use from routine Phase 1.6, or
  standalone for a delivery snapshot.
when_to_use: Called by the research routine Phase 1.6. Also useful standalone for a
  project-delivery snapshot (what shipped, what's next, blockers, per-role analysis).
mode: cloud-ok
---

# PPT Project Management — Research Routine Skill

Produces the delivery picture for the PPT project and writes it under
`.research/management/`. Static analysis only (no compile/run). Spawns the
`pm-*` agents in `.claude/agents/` as subagents.

## Inputs (from the routine)
- `MERGED_PRS`, `OPEN_PRS`, `ISSUES`, `CHURN_FILES` — Phase 1 observation data.
- `$TRIGGER_TEXT` — run mode (see Step 1).
- `state.pm_cursor` — rotation state.

## Step 1 — Decide the role set from `$TRIGGER_TEXT`
| `$TRIGGER_TEXT` | Roles |
|---|---|
| empty / anything not below | rotating: `state.pm_cursor.rotation[next_index]` |
| `full` or `pm-full` | all 8 rotation roles |
| `pm:<role>` (e.g. `pm:security` → `pm-security`) | that one role |

The **Scrum Master always runs**, regardless of mode.

## Mode: `scan` (deep coverage scan — LOCAL ONLY, full toolchain)

Triggered when invoked with arg `scan` (or `$TRIGGER_TEXT` in `{"scan","pm-scan"}`). **Never run this mode in the cloud routine** — it spawns many subagents and greps code broadly. It rebuilds `.research/management/coverage.json`.

1. **Enumerate epics + stories.** List `_bmad-output/epics*.md` and `_bmad-output/implementation-artifacts/stories/*.md`. Build the set of `(epic, story-id, title)`. Derive `phase` (`mvp|phase2|phase3|phase4`) from the epics catalog (`_bmad-output/epics.md` frontmatter / phase groupings).
2. **Classify per epic, in parallel.** For EACH epic, spawn one subagent (via the Agent tool) with the **classifier prompt** below, passing it the epic id + its `(story-id, title)` list. Run them concurrently; cap concurrency to a sane number if the epic count is large.
3. **Aggregate.** Collect every subagent's JSON, concatenate the `stories[]`, add `last_checked = <today>` to each, set top-level `generated = <iso now>` and `scan_kind = "deep"`, and write `.research/management/coverage.json`.
4. **Then run default mode** (below) to produce `roadmap.md` from the fresh coverage.
5. Note in output that `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale) is superseded by this map.

### Per-epic classifier subagent prompt (use verbatim, substitute `<EPIC>` and the story list)

> You classify the delivery status of the stories in ONE epic of the PPT (`property-management`) project. **Read-only — never modify files.** For each story below, determine `status` = `done | partial | not-started` with evidence, using these sources:
> - **sprint-status** (authoritative when present): `_bmad-output/implementation-artifacts/sprint-status.yaml` → `development_status.<story-id>`.
> - **code greps**: derive 2–4 keywords from the story title; grep `backend/servers/*/src/routes`, `backend/servers/*/src/handlers`, and `frontend/*/src` for them (filenames + symbols). Presence of a matching route/handler/component is evidence of implementation.
> - **screen-map**: `docs/screens/` frontmatter `buildStatus` for screens whose slug matches the story.
> - **merged PRs**: `gh pr list --state merged -R martin-janci/property-management --search "<keyword>"` or `git log --grep="<keyword>" --oneline`.
>
> **Classification rules:** `done` = sprint-status `done` OR (matching code present AND related screen(s) `shipped` AND a merged PR). `partial` = sprint-status `in-progress`/`review`, OR code present but a platform slice missing / a stub / no test. `not-started` = no code/screen/PR evidence AND absent or `backlog` in sprint-status. `confidence` = `high` when ≥2 signals agree, else `medium`/`low` (say why in `evidence`). Infer `owner_role` from the platform (`pm-backend` for routes/handlers, `pm-frontend` for screens/components, `pm-mobile`→use `pm-frontend`, security-sensitive→note for `pm-security`). `gaps[]` lists what's missing (empty if `done`). Read at most ~8 files; cap `evidence`/`gaps` at 4 entries each.
>
> Return EXACTLY this JSON and nothing after it:
> ```json
> {"epic":"<EPIC>","stories":[{"id":"<id>","title":"<title>","phase":"<mvp|phase2|phase3|phase4>","status":"<done|partial|not-started>","confidence":"<high|medium|low>","platform":["backend|frontend|mobile"],"owner_role":"pm-<role>","evidence":["..."],"gaps":["..."]}]}
> ```

## Default mode: rank `coverage.json` into the plan

After the Scrum Master + role runs, read `.research/management/coverage.json`.
- If `stories` is empty (never scanned), write a one-line `roadmap.md` saying "No coverage map yet — run `/ppt-project-management scan`", and skip ranking (still do the normal digest). Otherwise:

1. **Candidate tasks.** Take every story with `status` in `{partial, not-started}`. Each `gaps[]` entry becomes a task `"<gap> (<story-id> <title>)"`; if `gaps` is empty, the task is `"Implement <story-id> <title>"`.
2. **Score (balanced rubric).** For each candidate:
   - `phase_weight`: mvp=4, phase2=3, phase3=2, phase4=1
   - `+2` if the story is `partial` (finish-what's-started)
   - **risk**: `+2` if `owner_role=="pm-security"` or the gap text mentions security/auth/crash/data-loss; `+1` if `platform` includes `mobile` (most-behind)
   - **dependency**: if the story/gap is foundational infra others depend on (e.g., notification/WebSocket/auth infra), rank it above its dependents — role agents flag these; add `+1` and note the dependency.
   - `role_capacity`: on ties, prefer spreading `owner_role` across the top-N.
   - `priority` = `high` if score≥7, `medium` if 4–6, `low` if <4.
3. **Write `roadmap.md`** (overwrite):
   - `## State of the project` — `done/partial/not-started` story counts (overall + per platform), and the top 3 biggest gaps.
   - `## Ranked plan` — grouped by phase (mvp first); each task: `- [<priority>] <action> — owner: <owner_role> — why: <one-line rationale>`.
4. **Feed the top 10** ranked tasks into `action-list.json` (merge by id; full item schema: `{id, action, owner_role, priority, dependency, status:"open", deadline:null, source:"gap-scan"}`; `id = "gap-<story-id>-<kebab-slug>"`).
5. **Update `project-state.md`** "What's next" to the top 5 from the roadmap (replacing the sprint-only view).

## Step 2 — Run the Scrum Master
Spawn a subagent: "Read `.claude/agents/pm-scrum-master.md` and act as that agent.
Phase-1 data: <MERGED_PRS/OPEN_PRS/ISSUES summary>. Return your JSON shape."
Collect its JSON.

## Step 3 — Run the selected role(s)
For each selected role `pm-<role>`, spawn a subagent: "Read `.claude/agents/pm-<role>.md`
and act as that agent. Active sprint is in `_bmad-output/implementation-artifacts/sprint-status.yaml`.
Return your JSON shape." Collect each JSON. (Cap: never spawn more than 8 role agents +
the Scrum Master in one run.)

## Step 4 — Write artifacts (the skill writes; agents only returned JSON)
- `project-state.md` — regenerate from the Scrum Master output: exec summary, sprint
  progress, shipped-since-last-run, what's next (top actions w/ owner), blockers, and
  "Role focus today: <roles run>". Append a one-line per-role summary for roles that ran.
- `action-list.json` — merge: keep existing items (update `status` to `done` when a merged PR / closed issue resolved them). Add one item per role `next_action`, using this exact item schema:
  `{"id":"<role>-<kebab-slug>", "action":"<next_actions[].action>", "owner_role":"<the role that returned it>", "priority":"<next_actions[].priority>", "dependency":"<next_actions[].dependency>", "status":"open", "deadline":null, "source":"pm-analysis <today>"}`.
  Agents do not return `deadline` — default it to `null` (set a date only if one is explicit in the evidence). Set `.generated` = now.
- `action-list.md` — regenerate the table from `action-list.json`.
- `risks.json` — merge role `risks`, dedupe by slug, using this exact item schema:
  `{"id":"<role>-<kebab-slug>", "risk":"<risks[].risk>", "probability":"<risks[].probability>", "impact":"<risks[].impact>", "mitigation":"<risks[].mitigation>", "owner_role":"<the role that returned it>", "trigger":null, "status":"open"}`.
  Agents do not return `trigger` — default it to `null` (fill it only if an early-warning sign is stated). Set `.generated` = now.
- `decisions.md` — append any `decisions_needed` not already listed.
- `roles/<role>.md` — overwrite with that role's returned JSON rendered as markdown,
  for each role that ran this run.

## Step 5 — Advance the cursor & return the digest payload
- If the run was a **rotating** run, advance `state.pm_cursor.next_index = (next_index+1) % 8`.
- Set `state.pm_cursor.role_last_run[<role>] = <today>` for every role that ran.
- Return a compact `digest` object the routine sends to Telegram in Phase 6:
  ```json
  {"sprint":"<name>","epics_done":<n>,"epics_total":<n>,
   "shipped":["..."],"next":[{"action":"..","owner":".."}],
   "blockers":["..."],"role_focus":["pm-..."],"quiet":false}
  ```
  Build it from the Scrum Master output by mapping its keys: `sprint`/`epics_done`/`epics_total` ← flatten `sprint_progress`; `shipped` ← `shipped_since_last_run`; `next` ← top 3 `next_actions` (`action` + `owner` = `owner_role`); `blockers` ← `blockers[].item`; `role_focus` ← the roles that ran this run.
  Set `"quiet": true` **only when the run is genuinely empty** — ALL of these hold: zero PRs merged since last run, AND zero new backlog vectors added this run, AND zero plans promoted this run, AND zero new actions added to `action-list.json`, AND no blocker added or cleared. **A run is NEVER quiet if it promoted a plan, added a backlog vector, or surfaced a security/bug finding** — "nothing merged" is not "nothing worth reporting". When in doubt, send (`quiet: false`). The digest skip is only meant to suppress truly dead days, not findings-heavy ones. **This `quiet` is only a preliminary hint** — Phase 1.6 runs before Phases 2/3/5, so the routine's Phase 6 re-evaluates `quiet` with the full run (new vectors, promoted plans, code-review findings, auto-fixes) and is authoritative for whether the Telegram digest sends.

## Token budget
Scrum Master + 1 role on a normal run. `full` is opt-in. Each agent reads ≤8 files,
caps output. Do not read all 150 epics — only `in-progress`/`review`/`blocked` ones.
