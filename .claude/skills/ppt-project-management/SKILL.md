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
mode: cloud-ok  # applies to the DEFAULT (rank/role) mode; the `scan` sub-mode is LOCAL-ONLY (never run it in the cloud routine)
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
| `scan` or `pm-scan` | **scan mode** (see below) — run the deep coverage scan + default ranking; do NOT rotate a role agent. LOCAL ONLY. |
| empty / anything not below | rotating: `state.pm_cursor.rotation[next_index]` |
| `full` or `pm-full` | all 8 rotation roles |
| `pm:<role>` (e.g. `pm:security` → `pm-security`) | that one role |

The **Scrum Master always runs**, regardless of mode — **except a pure `scan` invocation**, which runs only the deep scan + default ranking and **skips the role-set steps (Step 2 Scrum Master synthesis + Step 3 role rotation)**; scan is a focused coverage+plan operation, not a daily digest run.

## Mode: `scan` (deep coverage scan — LOCAL ONLY, full toolchain)

Triggered when invoked with arg `scan` (or `$TRIGGER_TEXT` in `{"scan","pm-scan"}`). **Never run this mode in the cloud routine** — it spawns many subagents and greps code broadly. It rebuilds `.research/management/coverage.json`.

1. **Enumerate epics + stories.** List `_bmad-output/epics*.md` and `_bmad-output/implementation-artifacts/stories/*.md`. Build the set of `(epic, story-id, title)`. Derive `phase` (`mvp|phase2|phase3|phase4`) from the epics catalog (`_bmad-output/epics.md` frontmatter / phase groupings).
2. **Classify per epic, in parallel.** For EACH epic, spawn one subagent (via the Agent tool) with the **classifier prompt** below, passing it the epic id + its `(story-id, title)` list. Run them concurrently; cap concurrency to a sane number if the epic count is large.
3. **Aggregate.** Collect every subagent's JSON. For each subagent result, **stamp `epic` (from the result's wrapper `epic` field) onto every story in its `stories[]`** — the classifier returns `epic` at the wrapper level, but the `coverage.json` schema requires `epic` on each story. Then concatenate all stories, add `last_checked = <today>` to each, set top-level `generated = <iso now>` and `scan_kind = "deep"`.
4. **Screens cross-check (MANDATORY).** Spawn ONE additional subagent — the **screens-check subagent** (prompt below) — that maps each (epic, story) to its `docs/screens/<product>/<id>.md` entries via the `screens` family of skills (`screen-query`, `screen-map-validate`, `screen-map-update`), and detects orphans in both directions. Merge its output into `coverage.json`:
   - per-story field `screen_refs: [{screen_id, buildStatus, apiStatus, redesignStatus}]` (empty array means **no UI surface for this story** — explicitly noted as a gap)
   - top-level field `screen_gaps: {orphan_epics: [...], orphan_screens: [...], missing_use_cases: [...]}`
   - per-story `gaps[]` is augmented with `"no screen-map (orphan epic)"` if the story belongs to an orphan epic OR has zero `screen_refs` while status is `done`/`partial` (backend without UI surface is a real gap).
5. **Write `.research/management/coverage.json`** with the merged story + screens data.
6. **Then run default mode** (below) to produce `roadmap.md` from the fresh coverage. The ranker treats `screen_gaps.orphan_epics` and stories with `screen_refs == []` as explicit candidate tasks (specialist hint: `react-web`/`nextjs-web`/`react-native` depending on the product).
7. Note in output that `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale) is superseded by this map.

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

### Screens-check subagent prompt (use verbatim)

> You map PPT epics + stories to their `docs/screens/<product>/<id>.md` entries to determine WHERE each story is implemented in UI, and to detect orphans in both directions. **Read-only — never modify files.**
>
> Use the **screens skills** (they exist as separate Skill files; invoke via `/screens query`, `/screens validate`, `/screens update`, or read their source under `.claude/skills/screen-*/SKILL.md` if not loadable). If the skills are not available in this session, fall back to direct `grep` + frontmatter parsing.
>
> 1. **Inventory.** `find docs/screens -name '*.md' -not -path '*/_diagrams/*'` — list every screen-map. Read each frontmatter (lines between the first two `---`): collect `id`, `product`, `epic` / `epics`, `use-cases`, and per-implementation `buildStatus`, `apiStatus`, `redesignStatus`.
> 2. **Map stories → screens.** For each `(epic, story-id, title)` you receive:
>    - First match: any screen whose frontmatter `epic` / `epics` list contains the epic id (`epic-6`, `epic-7a`, etc.).
>    - Second match: any screen whose `use-cases` contains a UC code present in the story's source markdown (`grep -hE '^UC-[0-9]+' _bmad-output/implementation-artifacts/stories/<story-id>.md`).
>    - Third (weak) match: slug similarity between story id and screen id (Jaro-Winkler ≥ 0.85 or shared 2+ tokens).
>    Prefer strong matches; only fall back to slug when nothing else hits.
> 3. **Per match, record:** `{screen_id, buildStatus, apiStatus, redesignStatus}` using the per-implementation values most relevant to the story's platform (e.g. for a mobile story, prefer the mobile implementation; backend-only stories may match by use-case via the parent screen).
> 4. **Orphan detection.**
>    - `orphan_epics`: epic ids present in the story set with ZERO screen matches across all their stories.
>    - `orphan_screens`: screen-map ids whose frontmatter `epic`/`epics`/`use-cases` reference epics/UCs that do NOT exist in the current epics catalog or story set.
>    - `missing_use_cases`: UC codes that appear in story files but in NO screen-map's `use-cases`.
> 5. **Enrichment (optional).** If `/screens update` reports drift, fold its findings in. If `/screens validate` reports schema errors, list them under a `screen_validation_errors` array.
>
> Read at most ~60 screen-map files (sample evenly across products if the catalog is larger). Cap arrays at 30 entries each.
>
> Return EXACTLY this JSON and nothing after it:
> ```json
> {
>   "story_screens": [
>     {"id": "<story-id>", "screen_refs": [{"screen_id": "<path-slug>", "buildStatus": "<...>", "apiStatus": "<...>", "redesignStatus": "<...>"}]}
>   ],
>   "screen_gaps": {
>     "orphan_epics": ["epic-X", "..."],
>     "orphan_screens": ["<path-slug>", "..."],
>     "missing_use_cases": ["UC-XX", "..."],
>     "screen_validation_errors": ["<short message>", "..."]
>   }
> }
> ```

## Default mode: rank `coverage.json` into the plan

After the Scrum Master + role runs, read `.research/management/coverage.json`.
- If `stories` is empty (never scanned), write a one-line `roadmap.md` saying "No coverage map yet — run `/ppt-project-management scan`", and skip ranking (still do the normal digest). Otherwise:

1. **Candidate tasks.** Take every story with `status` in `{partial, not-started}`. Each `gaps[]` entry becomes a task `"<gap> (<story-id> <title>)"`; if `gaps` is empty, the task is `"Implement <story-id> <title>"`. ALSO take every entry in `screen_gaps.orphan_epics` as a task `"Add screen-map(s) for orphan epic <epic-id>"` (owner_role hint: `pm-frontend`), every `screen_gaps.orphan_screens` as `"Reconcile or remove orphan screen-map <screen-id>"`, and every `screen_gaps.missing_use_cases` as `"Link UC <code> to a screen-map"`.
2. **Score (balanced rubric).** For each candidate:
   - `phase_weight`: mvp=4, phase2=3, phase3=2, phase4=1
   - `+2` if the story is `partial` (finish-what's-started)
   - **risk**: `+2` if `owner_role=="pm-security"` or the gap text mentions security/auth/crash/data-loss; `+1` if `platform` includes `mobile` (most-behind)
   - **dependency**: if the story/gap is foundational infra others depend on (e.g., notification/WebSocket/auth infra), rank it above its dependents — role agents flag these; add `+1` and note the dependency.
   - **screen-gap**: `+1` if the story has `screen_refs == []` while status in `{done, partial}` (backend without UI surface); `+1` if the task itself is a screen-gap task from step 1 ("orphan epic", etc.).
   - `role_capacity`: on ties, prefer spreading `owner_role` across the top-N.
   - `priority` = `high` if score≥7, `medium` if 4–6, `low` if <4.
3. **Write `roadmap.md`** (overwrite):
   - `## State of the project` — `done/partial/not-started` story counts (overall + per platform), and the top 3 biggest gaps. Plus a `Screen coverage` line: `<N> stories without screen-map · <M> orphan epics · <K> orphan screens · <L> missing UC links` (from `screen_gaps`).
   - `## Ranked plan` — grouped by phase (mvp first); each task: `- [<priority>] <action> — owner: <owner_role> — why: <one-line rationale>`. Screen-gap tasks appear under a dedicated `### Screen-map drift` subsection within the relevant phase block.
4. **Refill `action-list.json` to a 36-task buffer.** The downstream dispatcher works at 3 concurrent implementers and runs every 2h (12 runs/day), so it consumes ≈36 task-slots per day. Target: keep ≥36 items with `status:"open"` in `action-list.json` after each run. Merge logic:
   - Count current `open` items (exclude `in-progress`/`done`/`failed`).
   - `slots_to_fill = max(0, 36 - open_count)`.
   - Take the top `slots_to_fill` ranked candidates whose `id` is NOT already in `action-list.json`. If fewer candidates exist than slots, take all (queue can underflow — that's fine; means the backlog is actually drained).
   - Item schema: `{id, action, owner_role, priority, dependency, depends_on, status:"open", deadline:null, source:"gap-scan"}` where `id = "gap-<story-id>-<kebab-slug>"`. Populate `depends_on` via the **`depends_on` normalization (T25 / issue #583)** rule in Step 4's `action-list.json` bullet — a non-empty/non-`"none"` `dependency` MUST have a matching non-empty `depends_on` (resolved id(s) or an `UNRESOLVED:` sentinel), else `depends_on: []`.
   - Do NOT drop existing items; only add. Items move out of `open` only when the dispatcher / implementer marks them `done` or `failed`, or when the rotating role agents (Step 4 of "Write artifacts") resolve them.
5. **Update `project-state.md`** "What's next" to the top 5 from the roadmap (replacing the sprint-only view).
6. **Buffer health line.** Add a single line at the end of `roadmap.md`: `Buffer: <open_count>/36 open · <candidates_remaining> candidates ranked but unqueued`. If `<open_count> < 18` (half-empty), prepend `⚠ Buffer below half — consider running scan to refresh coverage` to the line.

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
  `{"id":"<role>-<kebab-slug>", "action":"<next_actions[].action>", "owner_role":"<the role that returned it>", "priority":"<next_actions[].priority>", "dependency":"<next_actions[].dependency>", "depends_on":<see normalization rule>, "status":"open", "deadline":null, "source":"pm-analysis <today>"}`.
  Agents do not return `deadline` — default it to `null` (set a date only if one is explicit in the evidence). Set `.generated` = now.
  - **`depends_on` normalization (T25 / issue #583).** Every emitted row MUST carry a `depends_on` array — `dependency` alone is decorative (the dispatcher's Phase 3 `claimable()` predicate blocks only on `depends_on`, never on the free-text `dependency`). Derive `depends_on` from the role-returned `dependency`:
    - empty / `"none"` / null, **or a bare owner-role token** (e.g. `pm-backend`, `pm-qa`, `pm-tech-lead`, `rust-backend` — these express *who* does the work, already captured by `owner_role`, not a task-sequencing blocker) → `depends_on: []` and rewrite `dependency` to `"none"` (clear the misleading text).
    - one or more concrete action-list `id`s → `depends_on: [<id>, …]`.
    - otherwise unparseable prose → `depends_on: ["UNRESOLVED:<dependency text, truncated to ~60 chars>"]` (poisoned sentinel: keeps the row non-claimable and surfaces it for human follow-up).
    Never leave a non-empty / non-`"none"` `dependency` paired with an empty `depends_on` — that silently makes the row claimable while its real blocker is unresolved (the gap-3 migration trap; `.research/dispatcher-self-test.sh` T25 hard-fails on it).
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
