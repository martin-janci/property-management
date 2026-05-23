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
  Set `"quiet": true` **only when the run is genuinely empty** — ALL of these hold: zero PRs merged since last run, AND zero new backlog vectors added this run, AND zero plans promoted this run, AND zero new actions added to `action-list.json`, AND no blocker added or cleared. **A run is NEVER quiet if it promoted a plan, added a backlog vector, or surfaced a security/bug finding** — "nothing merged" is not "nothing worth reporting". When in doubt, send (`quiet: false`). The digest skip is only meant to suppress truly dead days, not findings-heavy ones.

## Token budget
Scrum Master + 1 role on a normal run. `full` is opt-in. Each agent reads ≤8 files,
caps output. Do not read all 150 epics — only `in-progress`/`review`/`blocked` ones.
