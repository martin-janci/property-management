# Improvement ideas — research + skills

Sorted by ROI (impact vs effort). Each row is concrete enough to fit on a
PR title line, not a vague wish. The hardening PR fixed critical + important
issues; the rows below are deferred.

| # | Idea | Why | Effort | Risk |
|---|---|---|---|---|
| 1 | Run `verify-all.sh --quick` in repo CI (new `.github/workflows/skills-smoke.yml`) | Catches skill drift (broken paths, missing recipes) before it hurts the implementer agent. The harness already exists; wiring is ~20 lines. | low | low |
| 2 | `just new-plan <slug>` recipe that scaffolds a plan from the template | Hand-authoring a plan today means copy-pasting from `routine-prompt.md` § Plan template. A recipe eliminates the metadata/heading drift that Quality Gate 7 has to police. | low | low |
| 3 | Normalize `ppt-deploy/SKILL.md` to the same authoring style as the 10 research-scaffold skills | Add `## When to invoke / What it gives you / Inputs / Steps / Deterministic verification / Smoke check / After-task verification / Cross-references` sections. Frontmatter already added in this PR. | low | low |
| 4 | `.research/IDEAS_TRIAGE.md` for `vector: triage` rows | Phase 3 excludes `triage` from plan promotion. Today triage rows just pile up in `backlog.md`. Surface them in a separate weekly digest so they actually get human attention. | low | low |
| 5 | Centralize C1–C7 capability definitions in one file (`.research/capabilities.md`) and reference from both prompts + skills | Today, C1–C7 semantics live in `implementer-prompt.md`. `routine-prompt.md`'s plan template enumerates them with parenthetical reminders that can drift. Single source of truth. | medium | low |
| 6 | Pre-commit hook for the research routine (when run locally) — runs G1–G12 goal checks before allowing commit | Today the routine is supposed to run goal checks in Phase 4 in-prompt. A real `pre-commit` chained command would catch drift even on hand-edits. | medium | medium |
| 7 | `/audit`-style dashboard at `.research/SUMMARY.md` (regenerated each run) — "what has the routine done this week" | Glance: # of briefs, # plans promoted, # plans archived, top-5 vectors by score, hotspot churn leaders. Brief is daily; dashboard is weekly aggregate. | medium | low |
| 8 | Lockfile or epoch token in `state.json` to detect concurrent-run collisions | Today we document the assumption ("no lockfile, single run"). A 16-byte epoch on routine start, asserted on commit, would actively detect overlap and abort cleanly. | medium | low |
| 9 | Validate `backlog.md` is generated, not hand-edited — store the SHA256 of the rendered output in `backlog.json` and fail Gate 4 if hand-edited | Today Gate 4 says "regenerating produces byte-identical file" — but the rendering function isn't formal. Embedding a checksum makes drift instantly visible. | medium | low |
| 10 | Hot-reload `.claude/skills/` when files change during a Claude session | Faster iteration on skill content — today, editing `SKILL.md` requires a new session to pick up the change. | medium | low |
| 11 | Plain-text version of the plan template extracted to `.research/plan-template.md` (instead of embedded in `routine-prompt.md`) | Hand-authors and tooling (idea #2) need the template as a real file, not a fenced-code block in a 24KB prompt. Routine can still consume it via `cat`. | low | low |
| 12 | Skill for "interact with the research routine itself" — `/ppt-research-trigger` slash command that POSTs `text: "deep"` or `text: "reset"` to the routine's API trigger | Today the user reads the README, finds the URL, and crafts curl by hand. A slash command makes the special trigger payloads discoverable. | low | low |
| 13 | `cloud-setup.sh` should `set -x` only when `DEBUG=1` and assert `GH_TOKEN` is non-empty AND the token has the expected scopes (decode header to confirm `repo:contents`) | Today the script confirms `gh auth status` works, which is good. Decoding the token scopes catches "I gave it read-only" silently before the routine's first push fails. | low | low |
| 14 | Make `ppt-bridge-mcp` smoke check skippable when offline (`SKIP_NETWORK=1`) | Today the smoke unconditionally curls `https://p.rlt.sk/healthz`. Useful when iterating on skills offline. | low | low |
| 15 | `ppt-deploy` smoke check should additionally verify `pmctl --help` runs (when binary is installed) | Today we just check files exist. A real toolchain check would catch `pmctl` regressions. Gate behind `command -v pmctl` to keep it optional. | low | low |
| 16 | Replace G10 (`backlog.md` matches regenerated) with a formal `render_backlog` function spec — document the exact sort key, column widths, trailing newline policy | Without a spec, "byte-identical" is fragile. A 10-line pseudocode section in `routine-prompt.md` removes the ambiguity. | medium | low |
| 17 | Add `Mode: cloud-ok` / `Mode: local-only` derivation as a real check (`grep -E '^Mode: (local-only|cloud-ok)'`) inside Quality Gate 7 in `routine-prompt.md` | Today G7 says "must declare Mode" but the check verb is informal. Make it a runnable line like the other gates. | low | low |
| 18 | Add G13: assert `plans/_archive/` only grows or stays equal (archived plans never undo) | Defensive against accidental rollback during merge conflicts. Cheap one-liner: `[ "$(ls plans/_archive | wc -l)" -ge "$(git show HEAD~1 --name-only -- plans/_archive | wc -l)" ]`. | low | low |
| 19 | Document the implicit "next plan picker" as a `/next-plan` slash command — reads `backlog.md`, sorts by score, prints the top `status: ready` row with its plan path | Today the README documents the manual flow. A skill or slash command would make it a one-key operation. | low | low |
| 20 | Backlog freshness widget: show "last update" timestamp at top of `backlog.md` so a stale view is obvious | Today nothing visually distinguishes a freshly-rendered `backlog.md` from a stale one if `state.json` somehow lost cursor. Trivial top-of-file timestamp. | low | low |

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
