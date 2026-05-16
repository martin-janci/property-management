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

## Picking a plan to ship

When you have an implementation slot, open the **manual implementation
agent** by starting a Claude Code session with `--append-system-prompt-file
.research/implementer-prompt.md` (or paste that file's contents into the
first user message), then prompt:

```
Implement .research/plans/<slug>.md.
```

The implementer prompt declares **seven capabilities** (C1–C7) covering
systematic debugging, seed data, dev-stack startup, browser automation
(Chrome MCP / Preview / playwright), ADB control, verification-before-
completion, and code-review reception. Each plan ticks the capabilities it
needs under *Required capabilities* — the implementation agent sets up
exactly those, no more.

It opens a PR, verifies, and on merge moves the plan to `_archive/` and
marks the backlog row `done`. The next daily research run sees the
shipment in the brief.

## Hand-edits

- Drop a vector by editing its row in `backlog.md` to `status: dropped` and
  leaving a one-line reason. Next routine run respects this.
- Pause the routine entirely: rename `routine-prompt.md` to
  `routine-prompt.md.disabled` — the routine no-ops if its prompt isn't
  present (the routine instructions in claude.ai stay in place but expect to
  read this file at startup).
