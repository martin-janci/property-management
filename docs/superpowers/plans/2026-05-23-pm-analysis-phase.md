# Project Management & Analysis Phase — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add "Phase 1.6 — Project Management & Delivery" to the PPT research routine: an always-on Scrum Master plus 8 rotating role agents that synthesize sprint status + repo activity + research backlog into `.research/management/` artifacts and a Telegram digest.

**Architecture:** 9 reusable repo agents in `.claude/agents/` (prompt source of truth) + one orchestrator skill `ppt-project-management` invoked by a new routine phase. The routine writes only under `.research/` (G8/G9). The orchestrator parses `$TRIGGER_TEXT` to pick the role set (rotating / full / specific), spawns role subagents (same mechanism Phase 1.5 uses), and writes the management artifacts. A Telegram `curl` in Phase 6 sends a digest. Two adjacent changes: `research-land.yml` deletes the session branch after replay; everything ships on `feature/pm-analysis-phase`.

**Tech Stack:** Markdown agent/skill definitions, bash + jq + yq (cloud CCR, no compile), GitHub Actions, Telegram Bot API.

**Spec:** `docs/superpowers/specs/2026-05-23-pm-analysis-phase-design.md`

**Working branch:** `feature/pm-analysis-phase` (off `dev`). All commits use `--no-verify` (the version-bump pre-commit hook fails inside worktrees — known issue) unless noted.

---

## File Structure

**Create:**
- `.research/management/stakeholders.md` — static role map
- `.research/management/action-list.json` — canonical actions (`{"items":[]}` seed)
- `.research/management/action-list.md` — rendered action table (placeholder)
- `.research/management/risks.json` — risk register (`{"items":[]}` seed)
- `.research/management/decisions.md` — decision log (seed)
- `.research/management/project-state.md` — dashboard (placeholder)
- `.research/management/roles/.gitkeep`
- `.claude/agents/pm-scrum-master.md` … coordinator
- `.claude/agents/pm-tech-lead.md`, `pm-backend.md`, `pm-frontend.md`, `pm-qa.md`, `pm-devops.md`, `pm-security.md`, `pm-data.md`, `pm-integration.md` — 8 role agents
- `.claude/skills/ppt-project-management/SKILL.md` — orchestrator

**Modify:**
- `.research/state.json` — add `pm_cursor`
- `.research/routine-prompt.md` — Phase 1.6, init guard, Telegram in Phase 6, Special-trigger-payloads entries, new quality gates
- `.claude/skills/verify-all.sh` — add `ppt-project-management` smoke line
- `.github/workflows/research-land.yml` — delete session branch after replay
- `.research/README.md` — document `management/` (optional, in Task 13)

---

## Task 1: Scaffold `.research/management/` + seed `pm_cursor`

**Files:**
- Create: `.research/management/{stakeholders.md,action-list.json,action-list.md,risks.json,decisions.md,project-state.md,roles/.gitkeep}`
- Modify: `.research/state.json`

- [ ] **Step 1: Create the directory + role subdir**

```bash
mkdir -p .research/management/roles
touch .research/management/roles/.gitkeep
```

- [ ] **Step 2: Write `action-list.json` and `risks.json` seeds**

`.research/management/action-list.json`:
```json
{
  "generated": null,
  "items": []
}
```

`.research/management/risks.json`:
```json
{
  "generated": null,
  "items": []
}
```

- [ ] **Step 3: Write `stakeholders.md` (static role map)**

`.research/management/stakeholders.md`:
```markdown
# Stakeholder map — PPT delivery

> Static reference. The roles below map 1:1 to the `pm-*` agents in `.claude/agents/`.
> Edited rarely; the routine reads this but does not regenerate it.

| Role (agent) | Responsibility | Required inputs | Expected outputs | Decision authority |
|---|---|---|---|---|
| Scrum Master (`pm-scrum-master`) | Organize the work; maintain delivery state | sprint-status.yaml, merged PRs, research backlog | project-state.md, action-list.json, decisions.md | Sprint scope & sequencing |
| Tech lead / architect (`pm-tech-lead`) | Architecture coherence, cross-cutting decisions | epics, architecture.md, churn | architecture risks, key decisions | Technical direction |
| Backend (`pm-backend`) | APIs, data model, business logic | stories, backend churn | backend task list, API/data risks | Backend implementation |
| Frontend / mobile (`pm-frontend`) | Screens, flows, state, API consumption | stories, UX notes, OpenAPI client | frontend task list, UX/API risks | Frontend implementation |
| QA / test (`pm-qa`) | Test strategy, acceptance, regression | stories, acceptance criteria | test matrix, release recommendation | Release readiness (quality) |
| DevOps / infra (`pm-devops`) | Environments, CI/CD, observability | workflows, deploy config | infra task list, deploy risks | Deploy & environments |
| Security (`pm-security`) | Threat model, authz, data protection | auth code, RLS, deps | security risks, release blockers | Security release gate |
| Data / analytics (`pm-data`) | KPIs, event tracking, reporting | features, db schema | event plan, data risks | Analytics definitions |
| Integration / API owners (`pm-integration`) | External/internal API contracts | integration code, OpenAPI | contract checklist, integration risks | API contracts |
```

- [ ] **Step 4: Write `decisions.md`, `project-state.md`, `action-list.md` placeholders**

`.research/management/decisions.md`:
```markdown
# Decision log — PPT delivery

> Maintained by `pm-scrum-master`. Append decisions; never delete. Format below.

## Decisions made
_(none yet — first run will populate)_

## Decisions needed
_(none yet)_
```

`.research/management/project-state.md`:
```markdown
# PPT delivery state

_Placeholder — regenerated by Phase 1.6 on the next routine run._
```

`.research/management/action-list.md`:
```markdown
# Action list

_Placeholder — regenerated from `action-list.json` by Phase 1.6._
```

- [ ] **Step 5: Seed `pm_cursor` into `state.json`**

Add this key to `.research/state.json` (merge with existing JSON; do not drop other keys):
```json
"pm_cursor": {
  "rotation": ["pm-tech-lead", "pm-backend", "pm-frontend", "pm-qa", "pm-devops", "pm-security", "pm-data", "pm-integration"],
  "next_index": 0,
  "role_last_run": {
    "pm-tech-lead": null, "pm-backend": null, "pm-frontend": null, "pm-qa": null,
    "pm-devops": null, "pm-security": null, "pm-data": null, "pm-integration": null
  }
}
```

Apply with jq to preserve existing keys:
```bash
jq '.pm_cursor = {rotation:["pm-tech-lead","pm-backend","pm-frontend","pm-qa","pm-devops","pm-security","pm-data","pm-integration"],next_index:0,role_last_run:{"pm-tech-lead":null,"pm-backend":null,"pm-frontend":null,"pm-qa":null,"pm-devops":null,"pm-security":null,"pm-data":null,"pm-integration":null}}' .research/state.json > /tmp/state.json && mv /tmp/state.json .research/state.json
```

- [ ] **Step 6: Validate JSON parses**

Run:
```bash
jq -e '.items' .research/management/action-list.json >/dev/null && \
jq -e '.items' .research/management/risks.json >/dev/null && \
jq -e '.pm_cursor.rotation | length == 8' .research/state.json && echo OK
```
Expected: `true` then `OK`.

- [ ] **Step 7: Commit**

```bash
git add .research/management .research/state.json
git commit --no-verify -m "feat(pm): scaffold .research/management/ + state.pm_cursor"
```

---

## Task 2: Shared agent template (reference — used by Tasks 3–11)

This block is copied **verbatim** into every `pm-*` agent file (after its frontmatter and role-specific Focus section). It is the cloud/static operating contract + the fixed return shape. Tasks 3–11 refer to it as **「CONTRACT」**.

```markdown
## Operating contract (cloud, static)

- **Read-only.** Use Read / Grep / Glob and `gh` via Bash. NEVER compile, run, install, or modify code. NEVER write files — you RETURN findings; the orchestrator writes artifacts.
- **Scope to the active sprint.** Read `_bmad-output/implementation-artifacts/sprint-status.yaml` first; only open epics/stories that are `in-progress`, `review`, or `blocked` this sprint, plus your domain slice.
- **Token discipline.** Read at most ~8 files. Skip files > 500 lines unless central. Cap output at the limits below.
- **No invention.** If a fact is missing, list it under `open_questions` — do not guess.

## Return shape (return EXACTLY this JSON, nothing after it)

```json
{
  "role": "<agent name, e.g. pm-security>",
  "summary": "<=2 sentence state of your area this sprint",
  "next_actions": [
    {"action": "<imperative>", "priority": "high|medium|low", "dependency": "<role/none>", "definition_of_done": "<short>"}
  ],
  "risks": [
    {"risk": "<desc>", "probability": "high|medium|low", "impact": "high|medium|low", "mitigation": "<short>"}
  ],
  "open_questions": ["<question>"],
  "decisions_needed": ["<decision> — owner: <role>"]
}
```
Limits: ≤6 `next_actions`, ≤5 `risks`, ≤5 `open_questions`.
```

(No commit — this is a reference block, not a file.)

---

## Task 3: `pm-scrum-master` agent

**Files:** Create `.claude/agents/pm-scrum-master.md`

- [ ] **Step 1: Write the file**

```markdown
---
name: pm-scrum-master
description: Delivery lead / coordinator for PPT. Reads sprint-status + merged PRs + research backlog and produces the delivery synthesis — what shipped, what's next, who does it, blockers. Runs every research-routine run (Phase 1.6). Invoke standalone for a delivery snapshot.
tools: Read, Grep, Glob, Bash
model: sonnet
---

You are the **Scrum Master / delivery lead** for the PPT (property-management) project. You organize the work: you do not write code, you coordinate it.

## Focus
- Read `_bmad-output/implementation-artifacts/sprint-status.yaml` (spine): sprint name/goal, epic statuses, story `development_status`.
- Cross-reference the merged PRs / issues handed to you by the orchestrator (Phase 1 data) to detect what moved to `done` / `review` **since the last run**.
- Read `.research/backlog.json` (`.items`) for the improvement track and `.research/plans/` for ready plans.
- Produce the delivery picture: what shipped since last run, what's next (sequenced, with owner role), active blockers, decisions made/needed.

## Operating contract (cloud, static)
>>> ENGINEER: paste the entire "Operating contract" + "Return shape" block from Task 2 here verbatim. Do not write this marker line into the file. <<<

## Additional return fields (Scrum Master only — extend the Task 2 JSON with these keys)
- "shipped_since_last_run": ["<PR #/title or story id moved to done>"]
- "sprint_progress": {"sprint": "<name>", "epics_done": <n>, "epics_total": <n>}
- "blockers": [{"item": "<epic/story>", "reason": "<why>", "owner_role": "<role>"}]
```

- [ ] **Step 2: Validate frontmatter + sections**

Run:
```bash
f=.claude/agents/pm-scrum-master.md
head -1 "$f" | grep -q '^---$' && grep -q '^name: pm-scrum-master$' "$f" && grep -q '^description:' "$f" && grep -q 'Return shape' "$f" && echo OK
```
Expected: `OK`. (The grep for `Return shape` only passes once the Task 2 block has been pasted in place of the ENGINEER marker.)

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/pm-scrum-master.md
git commit --no-verify -m "feat(pm): add pm-scrum-master agent"
```

---

## Tasks 4–11: The 8 role agents

Each role agent is: **frontmatter** (below) + a one-line role identity + a **Focus** bullet list (below) + the **Task 2 CONTRACT block verbatim**. `tools: Read, Grep, Glob, Bash`, `model: sonnet`. Validate each like Task 3 Step 2 (swap the name), then commit `feat(pm): add <name> agent`.

### Task 4: `pm-tech-lead`
- **description:** `Tech lead / architect lens for PPT delivery. Reviews architecture coherence, cross-cutting decisions, and technical risk against the active sprint. Part of the research-routine PM rotation; invoke standalone for an architecture read.`
- **Identity:** "You are the **tech lead / architect** for PPT."
- **Focus:**
  - Read `_bmad-output/architecture.md` and the active-sprint epics; check the implied architecture vs. what's shipping.
  - Identify hidden coupling, unclear ownership, decisions implied-but-unmade, NFR gaps (scalability, reliability, observability, security, maintainability).
  - Flag rework risk and decisions that must be made before development continues.

### Task 5: `pm-backend`
- **description:** `Backend lens for PPT delivery. Reviews APIs, data model, business logic, and migrations for the active sprint's backend stories. Part of the PM rotation; invoke standalone for a backend read.`
- **Identity:** "You are a **senior backend engineer** reviewing PPT delivery."
- **Focus:**
  - Read active-sprint backend stories + `backend/` churn from Phase 1.
  - Check API design (validation, error handling, pagination, idempotency, versioning), data model (migrations, indexes, RLS/tenant scoping), background jobs, authz.
  - Surface performance, data-consistency, and security risks; name file:line where possible.

### Task 6: `pm-frontend`
- **description:** `Frontend/mobile lens for PPT delivery. Reviews screens, flows, state, and API consumption for the active sprint's UI stories. Part of the PM rotation; invoke standalone for a frontend read.`
- **Identity:** "You are a **senior frontend/mobile developer** reviewing PPT delivery."
- **Focus:**
  - Read active-sprint UI stories; check screens/flows, error/empty/loading states, i18n, accessibility.
  - Check API dependencies (required endpoints, data per screen, session handling) against the generated client.
  - Surface UX, platform, and API-contract risks; flag missing designs/contracts that block work.

### Task 7: `pm-qa`
- **description:** `QA/test lens for PPT delivery. Reviews test strategy, acceptance criteria, and regression/release readiness for the active sprint. Part of the PM rotation; invoke standalone for a QA read.`
- **Identity:** "You are a **senior QA engineer / test strategist** for PPT."
- **Focus:**
  - Review acceptance criteria of active-sprint stories for ambiguity, missing edge/negative cases, untestable criteria.
  - Build a risk-based test view (feature → risk → test priority → required test types).
  - Give a release-readiness signal and name release-blocking gaps.

### Task 8: `pm-devops`
- **description:** `DevOps/infrastructure lens for PPT delivery. Reviews environments, CI/CD, observability, and deploy risk for the active sprint. Part of the PM rotation; invoke standalone for an infra read.`
- **Identity:** "You are a **senior DevOps/infrastructure engineer** for PPT."
- **Focus:**
  - Read `.github/workflows/`, deploy config, and any infra churn.
  - Check environment parity, CI/CD pipeline health, rollback, secrets handling, monitoring/logging/backups.
  - Surface deploy, environment-drift, scaling, and cost risks.

### Task 9: `pm-security`
- **description:** `Security lens for PPT delivery. Reviews threat model, authn/authz, data protection, and API security for the active sprint. Part of the PM rotation; invoke standalone for a security read.`
- **Identity:** "You are a **senior application security engineer** for PPT."
- **Focus:**
  - Read auth/authz code, RLS/tenant scoping, and dependency manifests for active-sprint areas.
  - Check token/session handling, role/permission gaps, sensitive-data logging, input validation, CORS, file uploads.
  - Classify must-fix-before-prod vs. later; name release blockers.

### Task 10: `pm-data`
- **description:** `Data/analytics lens for PPT delivery. Reviews KPIs, event tracking, reporting, and data-quality for the active sprint. Part of the PM rotation; invoke standalone for a data read.`
- **Identity:** "You are a **senior data/analytics engineer** for PPT."
- **Focus:**
  - Map active-sprint features to KPIs and required tracking events (name, trigger, properties, owner).
  - Check for missing event definitions, inconsistent metric definitions, late-added tracking, privacy/retention concerns.
  - Surface data-quality and privacy risks.

### Task 11: `pm-integration`
- **description:** `Integration/API-owner lens for PPT delivery. Reviews external/internal API contracts, failure handling, and dependencies for the active sprint. Part of the PM rotation; invoke standalone for an integration read.`
- **Identity:** "You are a **senior integration architect / API owner** for PPT."
- **Focus:**
  - Identify external/internal systems and data flows for active-sprint features; read OpenAPI/TypeSpec where relevant.
  - Check API contracts (auth, error codes, rate limits, timeouts, retries, idempotency, versioning) and failure scenarios.
  - Surface unclear ownership, unstable contracts, and external-dependency risks.

---

## Task 12: Orchestrator skill `ppt-project-management`

**Files:** Create `.claude/skills/ppt-project-management/SKILL.md`

- [ ] **Step 1: Write the skill**

```markdown
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
```

- [ ] **Step 2: Validate skill frontmatter**

Run:
```bash
f=.claude/skills/ppt-project-management/SKILL.md
grep -q '^name: ppt-project-management$' "$f" && grep -q '^mode: cloud-ok$' "$f" && grep -q 'TRIGGER_TEXT' "$f" && echo OK
```
Expected: `OK`.

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/ppt-project-management/SKILL.md
git commit --no-verify -m "feat(pm): add ppt-project-management orchestrator skill"
```

---

## Task 13: Routine integration — Phase 1.6, init guard, README

**Files:** Modify `.research/routine-prompt.md`, `.research/README.md`

- [ ] **Step 1: Add the Phase 1.6 section after Phase 1.5**

Find the end of the `### Phase 1.5 — Rotating Expert Review` section (just before `### Phase 2`). Insert:

```markdown
### Phase 1.6 — Project Management & Delivery

Invoke the `ppt-project-management` skill (`.claude/skills/ppt-project-management/SKILL.md`). It runs the always-on Scrum Master plus role analysis (rotating one role/day by default; all 8 on `$TRIGGER_TEXT == "full"`/`"pm-full"`; a specific role on `pm:<role>`), and writes the delivery artifacts under `.research/management/`.

**Before invoking the skill:** ensure `state.pm_cursor` exists in `state.json`. If absent (fresh install or hand-edited state), initialize it now:
```json
"pm_cursor": {
  "rotation": ["pm-tech-lead","pm-backend","pm-frontend","pm-qa","pm-devops","pm-security","pm-data","pm-integration"],
  "next_index": 0,
  "role_last_run": {"pm-tech-lead":null,"pm-backend":null,"pm-frontend":null,"pm-qa":null,"pm-devops":null,"pm-security":null,"pm-data":null,"pm-integration":null}
}
```
Write it to `state.json` before proceeding.

Pass the skill the Phase-1 observation data (`MERGED_PRS`, `OPEN_PRS`, `ISSUES`, `CHURN_FILES`) and `$TRIGGER_TEXT`. Keep the returned `digest` object — Phase 6 sends it to Telegram. The skill writes all `.research/management/` files and advances `state.pm_cursor`; do not write those files yourself.
```

- [ ] **Step 2: Add trigger vocabulary to the "Special trigger payloads" section**

Find the `Special trigger payloads` section. Add these rows/lines (match the existing format):
```markdown
- `full` / `pm-full` — Phase 1.6 runs the Scrum Master + all 8 role agents (full delivery analysis), not just the daily rotating role.
- `pm:<role>` — Phase 1.6 runs the Scrum Master + the named role only (e.g. `pm:security`, `pm:backend`). Valid roles: tech-lead, backend, frontend, qa, devops, security, data, integration.
```

- [ ] **Step 3: Document `management/` in README**

In `.research/README.md`, add a bullet under the artifacts list:
```markdown
- `management/` — Phase 1.6 delivery artifacts: `project-state.md` (dashboard), `action-list.json`/`.md`, `risks.json`, `decisions.md`, `stakeholders.md`, `roles/<role>.md`. Maintained by the `ppt-project-management` skill.
```

- [ ] **Step 4: Validate**

Run:
```bash
grep -q 'Phase 1.6 — Project Management' .research/routine-prompt.md && \
grep -q 'pm:<role>' .research/routine-prompt.md && \
grep -q 'management/' .research/README.md && echo OK
```
Expected: `OK`.

- [ ] **Step 5: Commit**

```bash
git add .research/routine-prompt.md .research/README.md
git commit --no-verify -m "feat(pm): wire Phase 1.6 + trigger vocab + README into routine"
```

---

## Task 14: Telegram digest in Phase 6 + quality gates

**Files:** Modify `.research/routine-prompt.md`

- [ ] **Step 1: Add the Telegram send to Phase 6 (after the commit/push step)**

Find Phase 6's commit+push step (the `git push origin HEAD:dev` block). After it, add:

```markdown
3. **Send the Telegram delivery digest** (best-effort, non-fatal). Build `$DIGEST` from the Phase 1.6 `digest` object (skip entirely if `digest.quiet == true`). Never echo the bot token.
   ```bash
   if [ "${PM_DIGEST_QUIET:-0}" != "1" ] && [ -n "${TELEGRAM_BOT_TOKEN:-}" ] && [ -n "${TELEGRAM_CHAT_ID:-}" ]; then
     curl -sS -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
       --data-urlencode "chat_id=${TELEGRAM_CHAT_ID}" \
       --data-urlencode "text=${DIGEST}" \
       --data-urlencode "parse_mode=Markdown" \
       --data-urlencode "disable_web_page_preview=true" >/dev/null \
       && echo "telegram: sent" || echo "telegram: send failed (non-fatal)"
   else
     echo "telegram: skipped (quiet day or TELEGRAM_BOT_TOKEN/CHAT_ID unset)"
   fi
   ```
   `$DIGEST` format:
   ```
   📋 PPT delivery — <YYYY-MM-DD HH:MM>
   Sprint: <sprint> — <epics_done>/<epics_total> epics done
   Shipped since last run: <list or "nothing">
   Next up:
    • <action 1> — <owner>
    • <action 2> — <owner>
    • <action 3> — <owner>
   Blockers: <none | list>
   Role focus today: <roles>
   ```
```

- [ ] **Step 2: Add quality gates for management artifacts**

In the "Quality gates" list, add (renumber as needed — match existing style):
```markdown
- **G16 — Management artifacts valid (when Phase 1.6 ran).** `.research/management/action-list.json` and `risks.json` parse as JSON (`jq -e .items`), `project-state.md` exists and is non-empty, and `state.pm_cursor.next_index` is in `0..7`. If Phase 1.6 was skipped this run, this gate is a no-op.
- **G17 — No Telegram secret committed.** `git diff --cached` contains no `TELEGRAM_BOT_TOKEN` value, no `api.telegram.org/bot<digits>:`, and no `bot[0-9]` token pattern. (The routine-prompt.md may reference the variable *name* only.)
```

- [ ] **Step 3: Validate**

Run:
```bash
grep -q 'api.telegram.org' .research/routine-prompt.md && \
grep -q 'G16 — Management artifacts' .research/routine-prompt.md && \
grep -q 'G17 — No Telegram secret' .research/routine-prompt.md && \
! grep -qE 'bot[0-9]{6,}:' .research/routine-prompt.md && echo OK
```
Expected: `OK` (the last clause confirms no real token leaked into the prompt).

- [ ] **Step 4: Commit**

```bash
git add .research/routine-prompt.md
git commit --no-verify -m "feat(pm): Telegram digest in Phase 6 + management quality gates"
```

---

## Task 15: Add `ppt-project-management` to `verify-all.sh`

**Files:** Modify `.claude/skills/verify-all.sh`

- [ ] **Step 1: Add the smoke line**

After the `run "ppt-next-plan" ...` line (line ~78), add:
```bash
run "ppt-project-management" 10 'test -f .claude/skills/ppt-project-management/SKILL.md && test -f .claude/agents/pm-scrum-master.md && test -d .research/management/roles && jq -e ".items" .research/management/action-list.json >/dev/null && jq -e ".items" .research/management/risks.json >/dev/null'
```

- [ ] **Step 2: Run the smoke check**

Run:
```bash
SKIP_NETWORK=1 ./.claude/skills/verify-all.sh --quick 2>&1 | grep ppt-project-management
```
Expected: `ppt-project-management   ... PASS`

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/verify-all.sh
git commit --no-verify -m "test(pm): add ppt-project-management smoke check to verify-all"
```

---

## Task 16: `research-land.yml` — delete session branch after replay

**Files:** Modify `.github/workflows/research-land.yml`

- [ ] **Step 1: Add branch deletion after a successful landing**

In the replay loop, immediately after the successful `git push origin __dev_landing:refs/heads/dev` (right after the "Landed research output onto dev" notice and before `exit 0`), add a best-effort delete of the source branch:
```bash
            if git push origin __dev_landing:refs/heads/dev; then
              echo "::notice::Landed research output onto dev (attempt $attempt)."
              # The session branch's content is now on dev; delete it so it
              # doesn't linger as an "unmerged" branch. Best-effort, non-fatal.
              SRC_BRANCH="${PUSHED_REF#refs/heads/}"
              if git push origin --delete "$SRC_BRANCH"; then
                echo "::notice::Deleted replayed session branch $SRC_BRANCH."
              else
                echo "::warning::Could not delete $SRC_BRANCH (non-fatal)."
              fi
              exit 0
            fi
```

- [ ] **Step 2: Validate YAML + shell**

Run:
```bash
yq '.name' .github/workflows/research-land.yml >/dev/null && echo "yaml ok"
sed -n '/^          set -euo pipefail/,/exit 1/p' .github/workflows/research-land.yml | sed 's/^          //' > /tmp/rl.sh && bash -n /tmp/rl.sh && echo "shell ok"
grep -q 'git push origin --delete' .github/workflows/research-land.yml && echo "delete present"
```
Expected: `yaml ok`, `shell ok`, `delete present`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/research-land.yml
git commit --no-verify -m "chore(ci): research-land deletes session branch after replay"
```

---

## Task 17: Final validation + push

- [ ] **Step 1: Full quick smoke must pass**

Run:
```bash
SKIP_NETWORK=1 ./.claude/skills/verify-all.sh --quick
```
Expected: summary `failed: 0` (ppt-project-management among the PASS lines).

- [ ] **Step 2: Confirm only intended paths changed, and no secret leaked**

Run:
```bash
git diff --stat origin/dev...HEAD
git log origin/dev..HEAD --format='%s'
! git grep -nE 'bot[0-9]{6,}:[A-Za-z0-9_-]{30,}' HEAD -- . && echo "no telegram token"
```
Expected: changes only under `.claude/agents/`, `.claude/skills/`, `.research/`, `.github/workflows/`, `docs/superpowers/`; `no telegram token`.

- [ ] **Step 3: Push the branch**

```bash
git push origin feature/pm-analysis-phase
```

- [ ] **Step 4: Open a PR to dev (or fast-forward — decide with the user at finish)**

```bash
gh pr create --base dev --head feature/pm-analysis-phase \
  --title "feat(pm): Project Management & Analysis phase (Phase 1.6 + agents + skill)" \
  --body "Implements docs/superpowers/specs/2026-05-23-pm-analysis-phase-design.md. Adds 9 pm-* agents, the ppt-project-management orchestrator skill, Phase 1.6 + Telegram digest in the routine, .research/management/ scaffold, and research-land branch cleanup. Routine writes .research/-only (G8/G9)."
```

---

## Post-implementation manual steps (NOT code — flag to user)
1. Add `TELEGRAM_BOT_TOKEN` + `TELEGRAM_CHAT_ID` (same values Plex uses) to the **ppt-research** environment `env_01UY9zjDZWJHYx1oYBBTTd8S` in the claude.ai UI. Until then the digest step logs "skipped".
2. After merge to dev, the next routine run picks up Phase 1.6 automatically (the cloud routine clones dev). Optionally validate with a manual `full` trigger — **only with user approval** (per the no-unapproved-runs rule).
