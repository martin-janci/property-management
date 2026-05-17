---
name: ppt-research-flow
description: Drive a plan from .research/plans/ through implementation, PR, and post-merge archive.
when_to_use: A user hands the implementer agent a path to a .research/plans/<slug>.md and asks for it to be shipped.
mode: both
capabilities: [C6]
tags: [workflow]
---

# PPT Research Flow

End-to-end choreography for taking one `.research/plans/<slug>.md` and shipping
it as a PR that the next research routine run can mark `done`. This skill is
the *spine* — it doesn't restate goals, it links to them.

## When to invoke

You are given (or are about to read) a plan under `.research/plans/`. Before
writing any code, walk this skill so you know which other skills to load and
what the exit criteria are.

## What it gives you

- A single authoritative checklist for the eight implementer goals (IG1–IG8)
- The branch / commit / PR / archive sequence with real commands
- Pointers to per-stack skills you should pull in based on the plan's
  *Required capabilities*

## Inputs

- `SLUG` — the plan basename (no extension), e.g. `fix-user-import-validation`
- The plan file at `.research/plans/$SLUG.md` (must already exist on `main`)

## Steps

1. **Read the plan twice.**
   ```bash
   cat .research/plans/$SLUG.md
   ```
2. **Match capabilities to skills.** The plan's *Required capabilities*
   ticks one or more of C1–C7 (see [implementer-prompt.md](../../../.research/implementer-prompt.md)
   §*Capabilities*). For each ticked capability:
   - C1 / C6 / C7 → `superpowers:*` skills, no extra setup
   - C2 → `ppt-db-migrations` (seed gap) + `ppt-bridge-mcp` for cloud seed
   - C3 → `ppt-dev-stack` (local) or `ppt-bridge-mcp` (`ppt_dev_up`)
   - C4 → local-only; Chrome MCP / Preview / playwright
   - C5 → local-only; `adb-app-control`
   - Anything touching `backend/` → `ppt-rust-backend`
   - Anything touching `frontend/` → `ppt-frontend`
   - Anything touching `mobile-native/` → `ppt-mobile-native`
   - Anything touching `docs/api/typespec/` → `ppt-typespec`
3. **Branch** from `main`:
   ```bash
   git switch -c "impl/$SLUG" main
   ```
4. **Pre-flight check** (IG2 baseline):
   ```bash
   just check
   ```
5. **Implementation loop** — follow `implementer-prompt.md` § *Implementation
   loop (per Suggested-approach step)*. Write the failing test first when the
   vector requires IG3 (`bug`, `revert`, `risky-churn`, `test-gap`).
6. **Verify** — run `just check && just test && just build`, plus the plan's
   *Test plan* commands verbatim. See `ppt-tests` for stack-specific subsets.
7. **Open the PR** — see `ppt-pr-create` for the body template and IG3
   two-commit (test → fix) evidence procedure.
8. **Archive commit BEFORE merge** (IG8 requirement — must be in the PR's
   diff, not a follow-up PR):
   ```bash
   git mv ".research/plans/$SLUG.md" ".research/plans/_archive/$SLUG.md"
   # edit .research/backlog.json: set the matching item's status="done"
   # and append "shipped in PR #N" to its evidence array
   git commit -m "research: mark $SLUG done (PR #<num>)"
   git push
   ```
   Reviewers should not approve a PR missing this commit. The flip lands
   before merge so the next routine run sees `status: "done"` and doesn't
   re-promote.

## Deterministic verification

```bash
# 1. plan file actually exists
test -f ".research/plans/$SLUG.md" && echo OK
# expected: OK

# 2. on the right branch
[ "$(git branch --show-current)" = "impl/$SLUG" ] && echo OK
# expected: OK

# 3. plan declares Mode: line (cloud-ok vs local-only)
grep -E '^Mode: (local-only|cloud-ok)' ".research/plans/$SLUG.md"
# expected: one match

# 4. archive directory exists for the post-merge move
test -d .research/plans/_archive && echo OK
# expected: OK
```

## Smoke check (single command)

```bash
test -d .research/plans/_archive && test -f .research/implementer-prompt.md && echo ok
```

## After-task verification

```bash
# IG8 — plan moved + backlog row flipped (run after the archive commit)
git log -1 --diff-filter=R --name-only | grep -q "_archive/$SLUG.md"
# backlog.json items have `id` and `plan` (not `slug`). Match the id that the
# routine derived from the plan slug, e.g. "test-gap-fix-user-import":
jq -e --arg p "plans/$SLUG.md" '.items[] | select(.plan == $p) | .status == "done"' .research/backlog.json
# (alternative if you know the id directly:
#   jq -e --arg id "<vector>-<slug>" '.items[] | select(.id == $id) | .status == "done"' .research/backlog.json )
```

## Cross-references

- [`.research/implementer-prompt.md`](../../../.research/implementer-prompt.md) — IG1–IG8
  definitions and the implementation loop (this skill does not duplicate them)
- [`ppt-pr-create`](../ppt-pr-create/SKILL.md) — PR body template
- [`ppt-tests`](../ppt-tests/SKILL.md) — picking the right test command
- [`ppt-bridge-mcp`](../ppt-bridge-mcp/SKILL.md) — cloud execution path
