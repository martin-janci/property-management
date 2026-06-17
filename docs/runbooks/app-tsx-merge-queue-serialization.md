# App.tsx Merge-Queue — Active-Serialization Verification

**Task:** `pm-devops-app-tsx-merge-queue-confirm`
**Owner:** pm-devops
**Date:** 2026-06-17
**Applies to:** `martin-janci/property-management` — `.github/workflows/app-tsx-merge-queue.yml` + `dev` branch protection

---

## Purpose

`frontend/apps/ppt-web/src/App.tsx` is the repo's top route-wiring churn
hotspot: every epic that adds a route also lands in `App.tsx`. When several
dispatcher drafts that all touch `App.tsx` are open at once, the later PRs
conflict-rebase onto the earlier ones, producing triple-merge-conflict noise
for the shepherd (the #563–#568 cluster).

`app-tsx-merge-queue.yml` was added to **serialize** those PRs. This runbook
records the verification — carried over from the 2026-05-27 run — that the
workflow is *actively* serializing App.tsx-touching PRs and is correctly wired
to the PR flow.

---

## Finding (2026-06-17): CONFIRMED ACTIVE — workflow is firing and serializing

The workflow is present, correctly configured, and **actively running** on
App.tsx-touching PRs. No wiring fix is required.

### What IS confirmed (workflow side + run history — in-repo + API evidence)

| Evidence | Location / source |
|----------|-------------------|
| Workflow triggers on `pull_request` (`opened, synchronize, reopened, ready_for_review`) scoped to the App.tsx paths | `.github/workflows/app-tsx-merge-queue.yml` lines 43–52 (`paths:` `frontend/apps/{ppt-web,admin-web,mobile}/src/App.tsx`) |
| `label-and-order` job auto-applies the `app-tsx-queue` label and posts an ordering comment when earlier non-draft App.tsx PRs exist | same file, JOB 1 (lines 81–232) |
| `on-demand-rebase` job (`workflow_dispatch`) rebases one queued PR onto `dev` with mechanical-conflict auto-resolve | same file, JOB 2 (lines 238–443) |
| `concurrency: group: app-tsx-merge-queue, cancel-in-progress: false` serializes the workflow's own jobs so two simultaneous pushes don't race the label/order logic | same file, lines 70–74 |
| **The workflow has actually been firing**: 37 total runs, all `completed/success`, most recent 2026-06-15, triggered by real App.tsx branches (`auto-impl/refactor-app-tsx-route-coupling`, `auto-impl/refactor-mobile-app-tsx-churn-2026-06-03`, `auto-impl/feat-navigation-and-routing-auth-guard-e`, …) | GitHub Actions API — `list_workflow_runs` for `app-tsx-merge-queue.yml` |

The run history is the decisive evidence: the workflow is not a dormant YAML
file — it is invoked automatically every time a PR touches an App.tsx path, and
every run has completed successfully.

### "Wired to the PR ruleset" — it is SOFT BY DESIGN (not a required check)

The serialization is intentionally a **soft** queue (auto-label + ordering
comment + on-demand rebase), **not** a hard, blocking required status check.
This is a deliberate design decision documented in the workflow header
(lines 29–41):

- GitHub's native merge queue (branch protection → merge queue) is **repo-wide
  and not path-filterable** — it would queue *all* PRs, not just
  App.tsx-touching ones. The path-scoped label + ordering approach is the
  practical alternative for a mixed-traffic repo.
- A **hard** block (a required CI status check that fails until the queue
  drains) would break every *non-conflicting* App.tsx PR (e.g. a route added to
  a previously untouched section) until earlier PRs merge. The soft approach
  warns the dispatcher without blocking correctly-rebased green PRs.

Consequently, **correct wiring for this workflow means it is NOT in the `dev`
required-status-check list.** A repo audit confirms it is referenced nowhere
else (no required-check registration, no ruleset entry), which is the expected
and correct state. The only required status check the repo registers on `dev`
is `security-gate-conclusion` (see
[`security-test-gate-required-check.md`](./security-test-gate-required-check.md)
and `.github/workflows/branch-protection-setup.yml`) — `app-tsx-queue` is
deliberately not among them.

### What could NOT be confirmed (live branch-protection state)

Whether the `dev` branch-protection rule additionally enables GitHub's *native*
merge queue, or lists any App.tsx-related context, could not be read in this
environment — the same tooling limitation noted for the security-gate task:

1. **`gh` CLI is unavailable** here.
2. **The GitHub MCP toolset has no branch-protection reader.**
   `mcp__github__list_branches` returns only a boolean `protected` flag per
   branch (no required-status-check contexts, no merge-queue setting), and there
   is no `GET /repos/{owner}/{repo}/branches/{branch}/protection` equivalent
   exposed.

This does not affect the finding: the App.tsx queue is *soft by design* and is
not expected to appear in branch protection. The live-state gap below is only
to rule out an accidental hard-block misconfiguration.

---

## Operator verification (manual — optional, to rule out misconfiguration)

Run with a token that has `administration:read` on the repo. PASS = the
App.tsx queue is NOT a required status check and native merge queue is off
(the expected soft-queue state).

### Option A — confirm App.tsx queue is NOT a required check (expected)

```bash
gh api repos/martin-janci/property-management/branches/dev/protection/required_status_checks \
  --jq '(.checks // [] | map(.context)) + (.contexts // []) | unique'
```

PASS criterion: the output array does **not** contain any `app-tsx`-related
context (it should contain `security-gate-conclusion` only, per the sister
runbook).

### Option B — confirm native merge queue is not silently enabled

```bash
gh api repos/martin-janci/property-management/branches/dev/protection \
  --jq '.required_merge_queue // "not-enabled"'
```

PASS criterion: `not-enabled` (the soft label/comment queue replaces it on
purpose).

### Option C — confirm the workflow is firing (already confirmed here)

```bash
gh run list --workflow=app-tsx-merge-queue.yml --limit 10
```

PASS criterion: recent runs exist for App.tsx-touching PR branches and are
`completed/success`. (Confirmed 2026-06-17 via the Actions API: 37 runs, all
success, latest 2026-06-15.)

---

## Remediation (none required)

No `.github/workflows` change and no branch-protection change is needed. The
workflow is active, firing, and correctly soft-wired. The only situation that
would require action is if Option A/B above revealed that App.tsx had been
*accidentally* turned into a hard required check or that the native merge queue
had been enabled repo-wide — in which case remove that setting, because it
defeats the path-scoped soft-queue design (it would block non-conflicting
App.tsx PRs).

---

## Result log

| Date | Action | Outcome |
|------|--------|---------|
| 2026-05-27 | Whether `app-tsx-merge-queue.yml` is actively serializing App.tsx PRs | UNCONFIRMED |
| 2026-06-17 | Re-checked workflow YAML + run history (Actions API) + repo wiring audit | CONFIRMED ACTIVE: 37 runs all success (latest 2026-06-15), auto-label + ordering + on-demand-rebase jobs intact, `concurrency` serializes. Soft-queue by design — correctly NOT a required status check. Live branch-protection state unread (no branch-protection reader in available tooling); Option A/B above for operator to rule out accidental hard-block. |
