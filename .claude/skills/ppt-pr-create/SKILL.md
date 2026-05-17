---
name: ppt-pr-create
description: Open a PR in this project's style — title, body template, IG3 evidence, CI surface, draft handling.
when_to_use: Verification has passed locally and you're ready to push the implementation branch.
mode: both
capabilities: [C6]
tags: [workflow]
---

# PPT PR Create

Concrete recipe for `gh pr create` on this repo. Pulls the body template
from the implementer prompt and points at the CI workflows so you know what
will run.

## When to invoke

`just check && just test && just build` all passed, and the plan's *Test
plan* commands all exited 0. You're about to open the PR.

## What it gives you

- Title format
- Full PR body template (verbatim from implementer prompt)
- IG3 stash/pop transcript placement
- CI workflows that will fire — so you can preview their scope before push

## Inputs

- `SLUG` — plan slug
- Plan vector (`bug`, `revert`, `risky-churn`, `test-gap`, `enhancement`, …)
- Plan area (`backend`, `frontend`, `mobile`, `infra`, `docs`)

## Steps

1. **Title format:**
   ```
   <vector>(<area>): <plan title>
   ```
   Examples: `bug(backend): fix duplicate user import on retry`,
   `test-gap(frontend): cover empty-state in listing search`.
   Draft / WIP variant: prefix `[WIP] ` and use `gh pr create --draft`.
2. **Body** — paste the template below; fill verification + IG3 evidence.
   The literal line `Closes plan: .research/plans/<slug>.md` is required
   for IG8.
3. **Create**:
   ```bash
   gh pr create --base main --head "impl/$SLUG" \
     --title "<vector>(<area>): <title>" \
     --body "$(cat <<'EOF'
   ## Summary
   <1–3 bullets, plain English>

   ## Closes plan
   Closes plan: .research/plans/<slug>.md

   ## Suggested-approach cross-reference
   - Step 1: <addressed in <file> | skipped because <…>>
   - Step 2: …

   ## Verification
   ```
   $ just check
   <paste exit + tail>
   $ just test
   <paste exit + tail>
   $ <plan's test-plan commands>
   <paste outputs>
   ```

   ## IG3 — failing test on main
   ```
   $ git stash && cargo test <test_name>
   <failure output>
   $ git stash pop && cargo test <test_name>
   <pass output>
   ```

   ## Out-of-scope items I noticed
   - <one-liner — goes to next research scan>
   EOF
   )"
   ```

## CI surface (what will fire)

Workflows under `.github/workflows/`:

| Workflow | Triggers when |
|---|---|
| `ci.yml` | All PRs — top-level orchestration |
| `backend.yml` | `backend/**` changes |
| `frontend.yml` | `frontend/**` changes |
| `mobile-native.yml` | `mobile-native/**` changes |
| `api-validation.yml` | `docs/api/typespec/**` changes |
| `docker-build.yml`, `docker-frontend.yml` | image builds |
| `screen-map.yml` | screen-map generation |
| `version-bump.yml` | version bump automation |
| `release.yml` | release automation |
| `approve-pr.yml`, `auto-approve.yml`, `copilot-rereview.yml` | review-bot wiring |

A `.research/`-only commit will still trip `ci.yml` but the area-gated jobs
(`backend.yml`, `frontend.yml`, `mobile-native.yml`) skip — that's why the
research routine commits keep landing fast.

## Draft / WIP handling

If any of these are true → open as draft with `[WIP]` prefix and document
the blocker in the body, per implementer prompt § *Goals*:

- IG3 failing-on-main can't be reproduced
- An IG4 step is skipped without justification
- Any of `just check / just test / just build` failed

## Deterministic verification

```bash
# 1. gh CLI installed and authenticated
gh --version >/dev/null && gh auth status >/dev/null 2>&1 && echo OK
# expected: OK

# 2. authenticated as the right user (read the user's CLAUDE notes — should be hanibalsk)
gh api user --jq .login
# expected: hanibalsk

# 3. base branch main exists on origin
git ls-remote --heads origin main | grep -q refs/heads/main && echo OK
# expected: OK
```

## Smoke check (single command)

```bash
gh auth status >/dev/null 2>&1 && echo ok
```

## After-task verification

```bash
# After PR open: confirm Closes-plan line is present
PR_NUM=$(gh pr list --head "impl/$SLUG" --json number --jq '.[0].number')
gh pr view "$PR_NUM" --json body --jq .body | grep -q "^Closes plan: .research/plans/$SLUG.md$"
```

## Cross-references

- [`.research/implementer-prompt.md`](../../../.research/implementer-prompt.md)
  § *Opening the PR* — canonical body template
- [`ppt-research-flow`](../ppt-research-flow/SKILL.md) — full flow including
  after-merge archive
- [`ppt-tests`](../ppt-tests/SKILL.md) — IG3 stash/pop dance
