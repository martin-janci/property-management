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
├── state.json                # rolling cursors + seen_signals + hotspot_history
├── backlog.json              # CANONICAL ranked vectors (machine-friendly)
├── backlog.md                # rendered human view, regenerated each run
├── signals/
│   └── YYYY-MM-DD.json       # debug trail of raw signals derived this run
├── briefs/
│   └── YYYY-MM-DD.md         # daily report (one per run, overwritten on rerun)
└── plans/
    ├── <slug>.md             # self-contained brief for the implementation agent
    └── _archive/             # plans the implementation agent has shipped
```

**Editing the backlog by hand:** edit `backlog.json` only. `backlog.md` is regenerated each run; hand-edits there will be lost.

## Flow

1. **Routine fires** (daily 05:00 local) → reads `state.json`, scans GitHub
   activity since `last_pr_seen` / `last_commit_sha` / `last_issue_seen`.
2. **Writes the day's brief** at `briefs/YYYY-MM-DD.md` — what shipped, what's
   open/stalled, what changed in code hotspots, anomalies (revert PRs,
   re-opened issues, churn spikes).
3. **Updates `backlog.md`** — appends new vectors with a score and a one-line
   rationale; deduplicates against existing rows.
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
| **Model** | Sonnet 4.6 (or Opus 4.7 for higher-quality analyses; Opus costs more) |
| **Connectors** | **None.** The routine uses `gh` CLI via Bash. |
| **Schedule** | Daily 05:00 local |
| **API trigger** | Optional — handy for ad-hoc `text: "deep"` or `text: "reset"` runs |

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

## Picking a plan to ship

When you have an implementation slot, open the **manual implementation
agent** by starting a local Claude Code session with
`--append-system-prompt-file .research/implementer-prompt.md` (or paste that
file's contents into the first user message), then prompt:

```
Implement .research/plans/<slug>.md.
```

The implementer needs the full local toolset (Docker, Chrome, ADB if mobile-touching) — none of which the cloud sandbox has. **Don't try to make the implementer a cloud routine** unless you've stood up a bridge MCP (see *Future: ppt-bridge-mcp* below).

The implementer prompt declares **seven capabilities** (C1–C7) covering
systematic debugging, seed data, dev-stack startup, browser automation
(Chrome MCP / Preview / playwright), ADB control, verification-before-
completion, and code-review reception. Each plan ticks the capabilities it
needs under *Required capabilities* — the implementation agent sets up
exactly those, no more.

It opens a PR, verifies, and on merge moves the plan to `_archive/` and
marks the backlog row `done`. The next daily research run sees the
shipment in the brief.

## Future: ppt-bridge-mcp

If you want the implementation agent to also run in cloud routines (truly autonomous overnight workflow), stand up a **bridge MCP server** on `mefistos` or NAS:

```
property-management.dev (running on mefistos / NAS, via docker compose)
  └── ppt-bridge-mcp  (Streamable HTTP MCP, OAuth via Anthropic proxy)
        ├── ppt_dev_up / down / logs        — ssh → mefistos → stack up pm-local …
        ├── ppt_seed / ppt_db_query         — ssh → psql / just seed
        ├── ppt_run_test <crate>            — ssh → cargo/pnpm test, returns output
        ├── ppt_browser_open <url>          — headless Chrome on the bridge box → DOM + screenshot
        └── ppt_adb_*                       — adb to a USB-attached emulator on the bridge box
```

Architecture mirrors `nas-mcp`: bearer auth, htpasswd registry, OAuth proxy via Anthropic. Build effort ≈ 1–2 days for v1; reuses the schema sanitizer from `nas-mcp`. Track on the backlog as a vector once this repo's daily routine starts producing them.

## Hand-edits

- Drop a vector by editing its row in `backlog.md` to `status: dropped` and
  leaving a one-line reason. Next routine run respects this.
- Pause the routine entirely: rename `routine-prompt.md` to
  `routine-prompt.md.disabled` — the routine no-ops if its prompt isn't
  present (the routine instructions in claude.ai stay in place but expect to
  read this file at startup).
