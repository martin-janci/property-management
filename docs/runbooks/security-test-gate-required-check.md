# Security-Test Gate — Required-Status-Check Verification

**Task:** `pm-devops-security-test-gate-required-check`
**Owner:** pm-devops
**Date:** 2026-06-17
**Applies to:** `dev` branch protection on `martin-janci/property-management`

---

## Purpose

The `security-test-gate.yml` workflow blocks a PR labelled `security` from
merging unless it ships with at least one new/modified test file (the policy
established after PR #497 merged an IDOR fix without a regression test). A
failing CI check only *blocks* a merge when it is registered as a **required
status check** in the branch-protection rule for `dev`. This runbook records
the verification of that registration and the remediation path if it is
missing.

The required check name is **`security-gate-conclusion`** — the
always-running `conclusion` job in `security-test-gate.yml`, deliberately
chosen over the conditionally-skipped `security-test-gate` job so the
branch-protection rule has a stable check name to enforce (GitHub treats a
*skipped* check as non-blocking).

---

## Finding (2026-06-17): UNCONFIRMED — could not be read with available tooling

**Status: the live `dev` branch-protection required-status-check list could
NOT be read in this environment.** This was the open question carried over
from the 2026-05-27 run, and it remains open because the verification path is
not available here, not because the wiring is absent.

### What IS confirmed (workflow side — evidence in-repo)

The workflow side is fully and correctly wired:

| Evidence | Location |
|----------|----------|
| Stable always-running check job named `security-gate-conclusion` | `.github/workflows/security-test-gate.yml` (`conclusion` job, `if: always() && github.event_name == 'pull_request'`) |
| One-time/idempotent registrar that adds `security-gate-conclusion` to `dev` required checks | `.github/workflows/branch-protection-setup.yml` (`apply-branch-protection` job) |
| Round-trip payload builder that never weakens existing protection (#771/#923) | `.github/workflows/scripts/branch-protection-payload.py` |
| Self-auditing read-back step that fails the run if the check is not registered after apply | `branch-protection-setup.yml` → "Self-check — verify security-gate-conclusion is required" step |
| Test-file detector + self-test fixture | `.github/workflows/scripts/detect-test-file.sh` |

So a *successful* `workflow_dispatch` run of `branch-protection-setup.yml`
(non-dry-run, with `GH_ADMIN_TOKEN` set) is itself proof of registration —
its self-check step reads `required_status_checks` back and fails if
`security-gate-conclusion` is absent.

### What could NOT be confirmed (live state)

Whether the registrar has actually been *run* against the live `dev` rule —
i.e. whether `security-gate-conclusion` is *currently* in
`required_status_checks` — could not be read here because:

1. **`gh` CLI is unavailable** in this environment.
2. **The GitHub MCP toolset exposed here has no branch-protection reader.**
   `mcp__github__list_branches` returns only a boolean `protected` flag per
   branch (no required-status-check contexts), and there is no
   `mcp__github__get_branch_protection` /
   `GET /repos/{owner}/{repo}/branches/{branch}/protection` equivalent
   available. The `protected` boolean alone does not tell us *which* checks
   are required.

This is a tooling limitation of the verification environment, not a defect in
the workflow wiring.

---

## Operator verification (manual — DO THIS to close the finding)

Run one of the following with a token that has `administration:read` (or
`administration:write`) on the repo. Any one is sufficient.

### Option A — read the required checks directly (fastest)

```bash
gh api repos/martin-janci/property-management/branches/dev/protection/required_status_checks \
  --jq '(.checks // [] | map(.context)) + (.contexts // []) | unique'
```

PASS criterion: the output array contains `"security-gate-conclusion"`.

### Option B — dry-run the registrar (read-only, self-auditing)

```bash
gh workflow run branch-protection-setup.yml --ref dev -f dry_run=true
# then inspect the run's "Self-check" step output:
gh run list --workflow=branch-protection-setup.yml --limit 1
gh run view <run-id> --log | grep -A2 "Required status checks on dev"
```

In dry-run the apply step does not patch, but the self-check step still reads
back and reports whether `security-gate-conclusion` is present.

### Option C — GitHub UI

Settings → Branches → branch protection rule for `dev` → "Require status
checks to pass before merging" → confirm `security-gate-conclusion` is in the
list.

---

## Remediation (if NOT configured)

If Option A returns a list that does **not** contain
`security-gate-conclusion`, the gate is advisory only — a repo admin can merge
a red security PR. Register it:

```bash
# Requires a PAT (or GitHub App token) with administration:write,
# stored as the GH_ADMIN_TOKEN secret:
gh secret set GH_ADMIN_TOKEN --body "<pat-with-administration-write>"

# Apply (idempotent; round-trips existing protection, only adds the check):
gh workflow run branch-protection-setup.yml --ref dev
```

The `branch-protection-setup.yml` workflow:

- fetches current `dev` protection and **preserves** every existing setting
  (strict mode, approvals, code-owner reviews, push restrictions, linear
  history, conversation resolution) — it never ships a hardcoded weakening
  payload (#771/#923);
- appends `security-gate-conclusion` to the required-status-check contexts;
- reads the result back and **fails the run** if the check is not present
  afterwards — so a green run is proof of registration.

A successful non-dry-run is the canonical "configured" evidence; re-run this
runbook's Option A afterwards to capture the confirmed state.

---

## No workflow change required

The workflow-side wiring needed to make the gate enforceable already exists
and is correct (stable `security-gate-conclusion` check name + idempotent,
non-weakening registrar + self-audit). No `.github/workflows` change is needed
for this task. The only outstanding action is the **operator verification /
one-time dispatch** above, which requires an admin token that is not available
to this automated environment.

---

## Result log

| Date | Action | Outcome |
|------|--------|---------|
| 2026-05-27 | Required-check status of `security-test-gate` on `dev` | UNCONFIRMED |
| 2026-06-17 | Re-checked via GitHub MCP (`list_branches` only) + repo audit | Workflow wiring confirmed correct; live branch-protection state still UNCONFIRMED (no branch-protection reader in available tooling, `gh` unavailable). Operator must run Option A/B/C above. |
