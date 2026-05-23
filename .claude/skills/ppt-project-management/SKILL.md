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
- `action-list.json` — merge: keep existing items (update status if a PR closed them),
  add new `next_actions` from all roles with a stable `id` (`<role>-<kebab-slug>`),
  `owner_role`, `priority`, `dependency`, `status:"open"`, `source`. Set `.generated` = now.
- `action-list.md` — regenerate the table from `action-list.json`.
- `risks.json` — merge role `risks` (dedupe by slug), set `.generated` = now.
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
  Set `"quiet": true` when nothing shipped AND no new/changed action AND no blocker change.

## Token budget
Scrum Master + 1 role on a normal run. `full` is opt-in. Each agent reads ≤8 files,
caps output. Do not read all 150 epics — only `in-progress`/`review`/`blocked` ones.
