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

`just verify` is green — you have the `VERIFY-PLAN base=<sha> files=<n>`
block and the `VERIFY OK <hash>` line to paste into the PR body — and the
plan's *Test plan* commands all exited 0. You're about to open the PR.
(No `just build`: local release builds are banned — see
[`_verify-rules.md`](../_verify-rules.md).)

## What it gives you

- Title format
- Full PR body template (verbatim from implementer prompt)
- IG3 two-commit TDD transcript placement (test commit → fix commit; no stash)
- CI workflows that will fire — so you can preview their scope before push

## Inputs

- `SLUG` — plan slug
- Plan vector — one of the canonical eight: `bug`, `refactor`, `perf`,
  `test-gap`, `dx`, `security`, `dep-update`, `triage` (`triage` is
  never promoted to a plan; see `routine-prompt.md` Phase 3 gates).
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
3. **Pick the base branch.** Default base is `dev`. The only other allowed
   base is `main` (release flow), and only when you explicitly intend a
   release PR. **Refuse any other base unless `--allow-stacked` is set.**

   ```bash
   BASE="${BASE:-dev}"
   ALLOW_STACKED="${ALLOW_STACKED:-0}"
   case "$BASE" in
     dev|main) ;;
     *)
       if [ "$ALLOW_STACKED" != "1" ]; then
         echo "refusing non-dev/main base '$BASE' — pass ALLOW_STACKED=1 to override (stacked PRs aren't tracked by the dispatcher's assignments.json and can't be merged bottom-up by ppt-pr-merge's mechanical-conflict resolver)" >&2
         exit 2
       fi
       echo "stacked-PR base accepted via ALLOW_STACKED=1 — make sure you understand the implications (see 'Stacked PRs' below)" >&2
       ;;
   esac
   ```

3.5. **Redundancy check — does an open PR already cover this work?**

   Run BEFORE pushing. **Invariant** (enforced jointly with the
   dispatcher's Phase 3 guards and the routine's promotion gate): at most
   one non-terminal unit of work per slug-stem. This step is the last
   defense — it catches the case where two implementers were already
   spawned in parallel before either had pushed a PR, so the dispatcher's
   open-PR scan couldn't see the collision yet.

   ```bash
   # 1. Derive the slug stem (drop -impl/-fix/-v2/-retry/-followup/-wip
   #    suffix optionally followed by digits — matches the dispatcher rule).
   STEM=$(echo "$SLUG" | sed -E 's/-(impl|fix|v2|retry|followup|wip)[0-9]*$//')

   # 2. Look for an open PR with the same stem in title OR branch.
   gh pr list --state open --limit 100 \
     --json number,title,headRefName,isDraft \
     --search "in:title $STEM" > /tmp/dup-title.json
   gh pr list --state open --limit 100 \
     --json number,title,headRefName,isDraft \
     --search "head:auto-impl/$STEM" > /tmp/dup-head.json
   gh pr list --state open --limit 100 \
     --json number,title,headRefName,isDraft \
     --search "head:impl/$STEM" >> /tmp/dup-head.json

   # 3. Hit predicate: any matched row whose headRefName, after stripping the
   #    auto-impl/ or impl/ prefix, has the same stem as $STEM.
   HITS=$(jq -r --arg stem "$STEM" '
     . | map(select(
       (.headRefName | sub("^(auto-impl|impl)/"; "") | sub("-(impl|fix|v2|retry|followup|wip)[0-9]*$"; ""))
       == $stem
     )) | .[] | "#\(.number) \(.headRefName)"
   ' /tmp/dup-title.json /tmp/dup-head.json | sort -u)

   if [ -n "$HITS" ]; then
     echo "REDUNDANT: another open PR already covers stem=$STEM:"
     echo "$HITS"
     # Post a comment on the FIRST conflicting PR so the operator + dispatcher see it.
     FIRST_PR=$(echo "$HITS" | head -1 | sed -E 's/^#([0-9]+).*/\1/')
     gh pr comment "$FIRST_PR" --body "Dispatcher dedup: implementer for \`$SLUG\` aborted PR-create — this PR already covers the same slug stem. If the two scopes actually differ, rename one slug stem to break the collision."
     echo "Skipping gh pr create. Mark the assignment as needs-human-judgement and exit." >&2
     exit 3
   fi
   ```

   If `HITS` is empty → fall through to step 4. If non-empty → exit 3 and let
   the implementer wrapper (dispatcher Phase 4 / ppt-implement) surface this
   in the assignment row as `pr=none note=redundant-with:#<n>`. The
   dispatcher's Phase 2 will route this to `status=failed` on the next run
   (which is correct — the work is not lost, the *other* PR is shipping it).

   **File-overlap secondary check** (optional, runs only if step 2 was
   empty). For each open `auto-impl/*` PR, list its changed files via
   `gh pr view <n> --json files -q '.files[].path'`, intersect with this
   branch's changed files (`git diff --name-only $BASE...HEAD`). If
   `|intersection| >= 2` with at least one non-test file → same abort as
   above with `reason=file-overlap`.

4. **Create**:
   ```bash
   gh pr create --base "$BASE" --head "impl/$SLUG" \
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
   $ just verify
   <paste the VERIFY-PLAN base=<sha> files=<n> block verbatim>
   <paste the VERIFY OK <hash> line>
   $ <plan's test-plan commands>
   <paste outputs>
   ```

   ## IG3 — failing test on main (two-commit TDD evidence)
   ```
   $ git log --oneline impl/$SLUG ^main
   <fix-sha>   fix: <issue>
   <test-sha>  test: add regression for <issue>
   # Full checkout at the test commit — detached HEAD, no fix yet:
   $ git checkout <test-sha>
   $ cargo test <test_name>
   <failure output proving the bug exists>
   # Back to the impl branch — both commits applied:
   $ git checkout impl/$SLUG
   $ cargo test <test_name>
   <pass output proving the fix works>
   ```
   Don't `git checkout <test-sha> -- <test-path>` (partial) — that leaves the
   fix files on disk, so the "pre-fix" run silently passes. Use a full
   `git checkout <test-sha>` so the working tree matches the snapshot.

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

## Stacked PRs (non-default; opt-in)

By default this skill refuses any `--base` other than `dev` or `main`. Stacked
PRs (PR-on-PR, where the base is another in-flight feature branch) are
intentionally not supported in the normal flow because:

- The dispatcher's `.research/management/assignments.json` only tracks rows
  whose PR targets `dev`. A stacked PR is invisible to claim/review/merge
  accounting and can be silently lost.
- `ppt-pr-merge` resolves mechanical conflicts against `base` (default `dev`).
  When base is a moving feature branch, the resolver's known patterns
  (`sqlx`, `Cargo.lock`, generated openapi/api-client, lockfiles) don't apply
  the same way and the merger can't unstick bottom-up automatically.
- The nightly auto-rebase workflow only rebases onto `dev`; stacked PRs would
  be force-pushed against a stale base.

If you really need a stacked PR — e.g. splitting one large refactor into a
review-friendly chain — pass `ALLOW_STACKED=1` to opt in. **You then own**
the merge order manually: merge the bottom of the stack first, retarget the
next PR to `dev`, repeat. Do not invoke `ppt-pr-merge` on a stacked PR
without first retargeting its base to `dev`.

```bash
BASE=feature/foo-bar ALLOW_STACKED=1 ./ppt-pr-create …   # opt-in form
```

## Draft / WIP handling

If any of these are true → open as draft with `[WIP]` prefix and document
the blocker in the body, per implementer prompt § *Goals*:

- IG3 failing-on-main can't be reproduced
- An IG4 step is skipped without justification
- `just verify` failed (paste the `VERIFY FAIL: <cmd>` line as the blocker)

## Deterministic verification

```bash
# 1. gh CLI installed and authenticated
gh --version >/dev/null && gh auth status >/dev/null 2>&1 && echo OK
# expected: OK

# 2. authenticated as a user / token that can open PRs on this repo.
#    Personal default is `hanibalsk`; collaborators, fork contributors, or
#    automation tokens are equally valid. Admin permission is NOT required —
#    GitHub lets anyone with read access push to a fork + open a PR. We
#    just want to confirm `gh auth status` is wired up.
gh auth status >/dev/null 2>&1 && gh api user --jq .login
# expected: any non-empty login, AND the next command must exit 0
gh repo view martin-janci/property-management --json id --jq .id >/dev/null
# expected: exit 0 (repo is reachable from this token)

# 3. base branch dev exists on origin (default PR base)
git ls-remote --heads origin dev | grep -q refs/heads/dev && echo OK
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
- [`ppt-tests`](../ppt-tests/SKILL.md) — IG3 two-commit TDD pattern
