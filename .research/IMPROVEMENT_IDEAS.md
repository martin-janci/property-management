# Improvement ideas — research + skills

Sorted by ROI (impact vs effort). Each row is concrete enough to fit on a
PR title line, not a vague wish. The hardening PR fixed critical + important
issues; the rows below are deferred. Row numbers are stable — addressed
ideas are removed in place, not renumbered, so commits and PR descriptions
can keep referencing them by their original `#` indefinitely.

| # | Idea | Why | Effort | Risk |
|---|---|---|---|---|
| 3 | Normalize `ppt-deploy/SKILL.md` to the same authoring style as the 10 research-scaffold skills | Add `## When to invoke / What it gives you / Inputs / Steps / Deterministic verification / Smoke check / After-task verification / Cross-references` sections. Frontmatter already added in this PR. | low | low |
| 5 | Centralize C1–C7 capability definitions in one file (`.research/capabilities.md`) and reference from both prompts + skills | Today, C1–C7 semantics live in `implementer-prompt.md`. `routine-prompt.md`'s plan template enumerates them with parenthetical reminders that can drift. Single source of truth. | medium | low |
| 6 | Pre-commit hook for the research routine (when run locally) — runs G1–G14 goal checks before allowing commit | Today the routine is supposed to run goal checks in Phase 4 in-prompt. A real `pre-commit` chained command would catch drift even on hand-edits. | medium | medium |
| 7 | `/audit`-style dashboard at `.research/SUMMARY.md` (regenerated each run) — "what has the routine done this week" | Glance: # of briefs, # plans promoted, # plans archived, top-5 vectors by score, hotspot churn leaders. Brief is daily; dashboard is weekly aggregate. | medium | low |
| 8 | Lockfile or epoch token in `state.json` to detect concurrent-run collisions | Today we document the assumption ("no lockfile, single run"). A 16-byte epoch on routine start, asserted on commit, would actively detect overlap and abort cleanly. | medium | low |
| 9 | Validate `backlog.md` is generated, not hand-edited — store the SHA256 of the rendered output in `backlog.json` and fail Gate 4 if hand-edited | Today Gate 4 says "regenerating produces byte-identical file" — but the rendering function isn't formal. Embedding a checksum makes drift instantly visible. | medium | low |
| 10 | Hot-reload `.claude/skills/` when files change during a Claude session | Faster iteration on skill content — today, editing `SKILL.md` requires a new session to pick up the change. | medium | low |
| 16 | Replace G10 (`backlog.md` matches regenerated) with a formal `render_backlog` function spec — document the exact sort key, column widths, trailing newline policy | Without a spec, "byte-identical" is fragile. A 10-line pseudocode section in `routine-prompt.md` removes the ambiguity. | medium | low |

## Notes

- Ideas are picked from the review pass on `feat/research-skills-harden`.
  Items I considered but did *not* enter:
  - "Remove `ppt-deploy` from `.claude/skills/`" — explicit "no deletion
    of skills" hard constraint. It's a real skill with real value; the
    only issue is it predates the research scaffold's authoring style.
  - "Move `verify-all.sh` to `~/.claude/skills/`" — it's intentionally
    in-repo so the cloud routine clones it and CI can run it. Moving
    would break that.
  - "Bump jq to 1.7+ in `cloud-setup.sh`" — apt-installed jq is fine for
    G2 once we switch to `--slurpfile`, which the hardening PR did.
- Addressed-and-removed rows (still referenceable by their original #):
  - **#1** — `verify-all.sh --quick` in repo CI → landed as `.github/workflows/skills-smoke.yml`
  - **#2** — `just new-plan <slug>` recipe → landed in `justfile`
  - **#4** — `.research/IDEAS_TRIAGE.md` for `vector: triage` → landed
  - **#11** — Plan template extracted to `.research/plan-template.md` → landed
  - **#12** — `ppt-research-trigger` skill → landed at `.claude/skills/ppt-research-trigger/`
  - **#13** — `cloud-setup.sh` GH_TOKEN scope assertion + DEBUG-gated `set -x` → landed
  - **#14** — `SKIP_NETWORK=1` gate for `ppt-bridge-mcp` smoke → landed in `verify-all.sh`
  - **#15** — `pmctl --help` exercise in `ppt-deploy` smoke (gated by `command -v`) → landed
  - **#17** — G7 *Mode declared* runnable grep check → landed in `routine-prompt.md`
  - **#18** — G13 *archive only grows* → landed in `routine-prompt.md`
  - **#19** — `ppt-next-plan` skill → landed at `.claude/skills/ppt-next-plan/`
  - **#20** — Backlog freshness widget (top-of-file timestamp) → landed in `backlog.md` + render contract in `routine-prompt.md`
