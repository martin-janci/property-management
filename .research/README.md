# .research/

Daily research routine output for `property-management`. A Claude Code routine
runs once a day, walks recent activity (merged PRs, open + closed PRs, GitHub
issues, commit-log hotspots), and writes structured artifacts that a separate
**human-triggered implementation agent** picks up.

## Layout

```
.research/
├── README.md                 # this file
├── routine-prompt.md         # the routine's Instructions field
├── implementer-prompt.md     # manual implementation agent's system prompt
├── cloud-setup.sh            # bash body for the claude.ai routine env setup script
├── state.json                # rolling cursors + seen_signals + hotspot_history
├── backlog.json              # CANONICAL ranked vectors (machine-friendly)
├── backlog.md                # rendered human view, regenerated each run
├── IMPROVEMENT_IDEAS.md      # deferred follow-ups (added by the harden pass)
├── signals/
│   └── YYYY-MM-DD.json       # debug trail of raw signals derived this run
├── briefs/
│   └── YYYY-MM-DD.md         # daily report (one per run, overwritten on rerun)
├── management/               # Phase 1.6 delivery artifacts (maintained by ppt-project-management skill)
│   ├── project-state.md      # delivery dashboard (regenerated each run)
│   ├── action-list.json      # canonical actions (machine-friendly)
│   ├── action-list.md        # rendered action table (regenerated from action-list.json)
│   ├── risks.json            # risk register (machine-friendly)
│   ├── decisions.md          # decision log (append-only)
│   ├── stakeholders.md       # static role map
│   ├── coverage.json         # 134-story delivery coverage map (done/partial/not-started + evidence + gaps)
│   ├── roadmap.md            # ranked gap plan generated from coverage.json (balanced rubric)
│   └── roles/                # per-role analysis snapshots (<role>.md overwritten each run)
└── plans/
    ├── <slug>.md             # self-contained brief for the implementation agent
    └── _archive/             # plans the implementation agent has shipped
```

- `management/` — Phase 1.6 delivery artifacts: `project-state.md` (dashboard), `action-list.json`/`.md`, `risks.json`, `decisions.md`, `stakeholders.md`, `roles/<role>.md`. Maintained by the `ppt-project-management` skill.
  - `coverage.json` — 134-story delivery coverage map (done/partial/not-started + evidence + gaps), built by `/ppt-project-management scan` (local deep scan) and maintained cheaply by the daily routine.
  - `roadmap.md` — ranked gap plan generated from `coverage.json` (balanced rubric, owner + rationale per task).
- `goal-check.sh` — deterministic goal/convergence checks (GC1 referential integrity, GC2 coverage-progress monotonicity, GC3 buffer bounds). Enforcing as of PR 2 (`GOAL_CHECK_ENFORCE=1` in Phase 6 + CI): GC2 regression aborts the run; GC1/GC3 stay record-only. See the `ppt-goal-gate` skill.

In-repo skills the implementer agent uses live at **`.claude/skills/`** (one
level up, at the repo root). `.claude/skills/` is auto-discovered by any
Claude Code session opened in this repo — local CLI or cloud routine
(the routine clones the repo, so the skills come with it). See
`.claude/skills/README.md` for the index and `.claude/skills/verify-all.sh`
for the environment smoke check.

**Editing the backlog by hand:** edit `backlog.json` only. `backlog.md` is regenerated each run; hand-edits there will be lost.

## Routine vs dispatcher state

Two separate loops write into `.research/` and the distinction matters when
debugging "why hasn't X been touched":

| Loop | Cadence | Branch | State file | Read-only? |
|---|---|---|---|---|
| **Daily routine** | once / day (cloud cron) | commits on `dev` | `.research/state.json` | yes — only reads GitHub + repo |
| **Autonomous dispatcher** | every ~2h | commits on `planning` | `.research/management/assignments.json` (on `planning` branch) | no — claims/reviews/merges |

The two never share a cursor file. If `state.json` hasn't advanced in days
while `assignments.json` is fresh, the routine is stuck (cloud cron broken,
env vars missing, or the trigger payload disabled) — the dispatcher is fine.
If `assignments.json` is stale while `state.json` ticks, the dispatcher is
stuck (rate-limited, no buffer, sandbox failures) — the routine is fine.

The routine's Phase 1 (or Phase 6) sanity check flags when `state.json` has
not advanced in **3+ days** under "Since last run" → "Routine lag" in the
daily brief, so the operator notices on the next routine commit.

## Flow

1. **Routine fires** (daily 05:00 local) → reads `state.json`, scans GitHub
   activity since `last_pr_seen` / `last_commit_sha` / `last_issue_seen`.
2. **Writes the day's brief** at `briefs/YYYY-MM-DD.md` — what shipped, what's
   open/stalled, what changed in code hotspots, anomalies (revert PRs,
   re-opened issues, churn spikes).
3. **Updates `backlog.json`** (canonical) — appends new vectors with a score,
   sources, evidence, and a one-line rationale; deduplicates by `id` and by
   strong title-similarity overlap. **Regenerates `backlog.md`** from
   `backlog.json` after each update so the rendered view never drifts.
4. **Promotes the top items** into ready-to-execute plans at `plans/<slug>.md`
   — each plan is a self-contained brief the implementation agent can read
   cold (file paths, hypothesis, suggested test plan, no this-conversation
   shorthand).
5. **Commits + pushes** `state.json`, the new brief, `backlog.md` updates, and
   any new plans on `main` (requires "Allow unrestricted branch pushes" on
   this repo for the routine).

## Activating the cloud routine

Setup at https://claude.ai/code/routines → **New routine**.

| Field | Value |
|---|---|
| **Name** | `ppt-research` |
| **Instructions** | full contents of `routine-prompt.md` |
| **Repository** | `martin-janci/property-management`, branch `main`, ✅ *Allow unrestricted branch pushes* |
| **Model** | Sonnet 4.6 (or Opus 4.8 for higher-quality analyses; Opus costs more) |
| **Connectors** | Built-in **GitHub MCP** (`mcp__github__*`). It is the primary GitHub I/O path — `git push`/`git fetch` over the git transport are HTTP-403'd by the sandbox proxy (finding `git-push-blocked-by-proxy`), and `gh` CLI through the same proxy is unreliable, so both are fallback-only. The dispatcher's *GitHub I/O contract* (top of `dispatcher-prompt.md`) spells out the per-operation mapping. |
| **Schedule** | Daily 05:00 local |
| **API trigger** | Optional — handy for ad-hoc `text: "deep"` or `text: "reset"` runs |

**Sibling routine — `ppt-research-dispatcher`.** Same setup as above with
these overrides:

| Field | Value |
|---|---|
| **Name** | `ppt-research-dispatcher` |
| **Instructions** | full contents of `dispatcher-prompt.md` |
| **Model** | **Opus 4.8** — the dispatcher's per-PR review (Phase 5) and self-review (Phase 8) both spawn Opus subagents internally; running the dispatcher itself on Opus avoids a Sonnet→Opus context handoff between phases and keeps reasoning consistent across the whole loop. |
| **Schedule** | Every ~2h |
| **Branch target** | commits to `dev` (not `main` — see *Routine vs dispatcher state* below) |

### Environment

Use a dedicated environment (don't share with other routines).

- **Network access:** Trusted is enough — GitHub is on the default allowlist. No extra *Allowed domains* needed.
- **Setup script:** paste the body of `.research/cloud-setup.sh`. Runs once per env build, cached ~7 days. Installs `gh` (not in the default sandbox image), `jq`, `yq`; configures git identity; sanity-checks `gh auth`.
- **Variables:**
  - `GH_TOKEN` (secret) — fine-grained PAT with read PRs/issues/contents on `martin-janci/property-management` plus **write contents** (for committing under `.research/`).
  - `GIT_AUTHOR_NAME` — optional, defaults to `ppt-research-routine`
  - `GIT_AUTHOR_EMAIL` — optional, defaults to `ppt-research-routine@martin-janci.dev`

### What the cloud routine cannot do

The cloud sandbox **cannot** reach your LAN, **cannot** SSH out, **cannot** see physical devices, **cannot** use stdio MCPs. That's fine — this routine is purely read-only over GitHub + writes to `.research/`. Implementation runs elsewhere (see below).

## Picking a plan to ship — two execution modes

**How to pick the next plan:** look at `backlog.md` (top of the table is
highest score), find a row with `status: ready` whose `plan` field points
to a file under `plans/`, then load that file. Or just `ls plans/*.md`
(excluding `_archive/`) for everything ready to go. The implementer agent
ships one plan per session — pick by priority, capability you can satisfy,
and your appetite for the area.

### Local mode (Mac, full toolset)

When a plan needs **C4** (browser) or **C5** (ADB), only this mode works.

```bash
claude --append-system-prompt-file .research/implementer-prompt.md
# then: "Implement .research/plans/<slug>.md."
```

You get local Docker, Chrome MCP, ADB, your dotfiles `stack` CLI, everything.

### Cloud mode (claude.ai routine or remote session, via ppt-bridge)

When a plan doesn't need a browser or device, the **`ppt-bridge` MCP** at
`https://p.rlt.sk/mcp` gives the implementer SSH-exec access to `mefistos`
(dev) and `hetzner` (prod, gated) — no local Docker required.

Setup:
- Open claude.ai → Settings → Connectors → Add custom → URL `https://p.rlt.sk/mcp` → OAuth (Google).
- Use the same `--append-system-prompt-file .research/implementer-prompt.md` (or paste it as the first user message) on the routine session.

The bridge exposes 10 tools — `list_hosts`, `ppt_dev_up/down/logs`, `ppt_seed`,
`ppt_db_query` (with DML guard + `db:mutate` capability), `ppt_run_test`,
`ppt_docker_compose` (`ps`/`logs`/`up`/`down`/`restart`), `ppt_browser_open`
(v2 skeleton), `set_primary_host`. Destructive ops on `hetzner` (marked
`is_prod=true`) require `confirm: true` per call; a kill-switch env var
disables all destructive ops globally. Audit at `https://p.rlt.sk/audit`.

See `implementer-prompt.md` § *ppt-bridge MCP — cloud-side toolset* for the
full capability matrix.

### Capabilities

The implementer prompt declares **seven capabilities** (C1–C7) covering
systematic debugging, seed data, dev-stack startup, browser automation
(Chrome MCP / Preview / playwright), ADB control, verification-before-
completion, and code-review reception. Each plan ticks the capabilities it
needs under *Required capabilities* — the implementation agent sets up
exactly those, no more. C4/C5 imply local mode; everything else can run cloud
or local.

It opens a PR, verifies, **and includes the archive move (`git mv` of the
plan into `_archive/`) plus the backlog `status: "done"` flip as a final
commit in the same PR — before merge, not after** (see implementer-prompt
§ IG8 for the rationale: deferring the flip risks the next routine run
re-promoting the same plan). The next daily research run sees the
shipment in the brief.

## Hand-edits

- **Drop a vector:** edit its row in `backlog.json` (set `status: "dropped"`,
  append an evidence line with the reason), commit. The next routine run
  regenerates `backlog.md` from the updated JSON and respects the drop.
  Don't hand-edit `backlog.md` — it's regenerated each run; your edits
  there will be silently overwritten.
- **Defer a vector:** edit its `updated_at` to today in `backlog.json` —
  resets the 14-day decay clock without other changes.
- **Author a plan by hand:** write `plans/<slug>.md` following the plan
  template in `routine-prompt.md` § *Plan template*. Quality Gate 7 still
  applies — 4 metadata fields + 11 `##` headings + `Mode:` line + at least
  one ticked capability. If your file misses one, the next routine run
  flags it; nothing else blocks you.
- **Pause the routine entirely:** go to claude.ai → Routines → `ppt-research`
  and toggle the schedule off. The routine instructions live in claude.ai,
  not in this file, so a file rename here has no effect.
- **One-off skip:** add a `paused: true` field to `.research/state.json` and
  commit it. The routine reads state at startup; the first instruction in
  *Phase 1 — Observe* is to honour this flag (write a quiet-day brief, bump
  `stats.quiet_days`, exit). To resume, set `paused: false` and commit.
