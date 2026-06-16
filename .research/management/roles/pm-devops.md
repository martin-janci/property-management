# pm-devops — DevOps / Infrastructure

_Ran: 2026-06-16 (PM rotation, idx 4 → 5; previous run 2026-05-27, 20 days stale)_

## Summary

`dev` backend is currently RED — PR #1426 merged despite breaking compile (issue #1437), blocking ALL backend CI gates until #1435/#1436 lands. This is the second dev-red incident in 14 days (cf. #1332 unblocked 2026-06-14 via #1379) and indicates a missing `dev`-push smoke gate: `backend.yml` runs on PR but not on push, so a merge that conflicts with main can break compile silently. On the mobile front, `eas-build-android.yml` and `eas-build-ios.yml` are now both present in `.github/workflows/` (cleared since 2026-05-27 from draft PRs) — green-status verification still pending. `app-tsx-merge-queue.yml` is present (carry-over action delivered). Pre-push fmt/clippy gate (#1431) merged but is local-hook-only and would not have caught #1426.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | URGENT: Land #1435 or #1436 to restore `dev` backend compile (issue #1437) — until this lands every backend PR's CI is red regardless of its own quality | pm-backend | `dev` HEAD `cargo check --workspace --tests` passes; backend.yml green on a fresh dev push |
| high | Add `cargo check --workspace --tests` smoke gate on `dev` push (not just PR) — backend.yml is currently PR-only. Would have caught #1426 → #1437 before propagation | pm-backend | backend.yml's `on:` includes `push: branches: [dev]`; smoke job runs first and is fail-fast |
| medium | Confirm `eas-build-android.yml` + `eas-build-ios.yml` green on a workflow_dispatch run — both workflow files now exist (cleared from 2026-05-27 backlog) but action pins / eas-cli install / EXPO_TOKEN secret presence not verified post-merge | pm-frontend | both workflows pass a no-op build job; release secrets confirmed in repo/org settings |
| medium | Decide pre-push fmt/clippy gate (#1431) scope: local hook only, mirror as CI status check, or both. Local-only does not catch contributors with hooks disabled (and would NOT have caught #1426) | pm-tech-lead | DEC entry in decisions.md; if "both" — CI job added gated on dev-push |
| medium | Confirm `security-test-gate.yml` is a REQUIRED status check on `dev` branch protection (carry-over from 2026-05-27) — `gh api repos/.../branches/dev/protection` | pm-qa | gate appears in required_status_checks.contexts; security-labelled PR with no test file is blocked |
| low | Confirm `app-tsx-merge-queue.yml` (carry-over delivered) is actively serializing App.tsx-touching PRs — verify wired to the PR ruleset | none | workflow has triggered at least once on a real App.tsx PR; merge-queue path confirmed |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| `dev` backend RED via PR #1426 / issue #1437 — ALL backend CI gates broken until fix lands; second dev-red incident in 14 days | high | high | Land #1435/#1436 immediately; add `dev`-push smoke gate; audit PR #1426 merge log for missing required check |
| Mobile EAS pipeline green-status unverified — workflows exist but pins/secrets/cli not confirmed post-merge | medium | medium | Trigger no-op workflow_dispatch; audit EXPO_TOKEN / EXPO_PROJECT_ID / Android keystore / iOS provisioning secrets |
| `security-test-gate.yml` still possibly advisory-only on `dev` (unconfirmed since 2026-05-27) — security-labelled PRs may merge without tests | medium | high | Verify branch-protection required-check listing; promote to required if advisory |
| Pre-push fmt/clippy gate (#1431) is local hook only — cannot stop a #1426-class compile break from a hook-disabled contributor | medium | medium | Mirror as fast CI status check (`cargo fmt --check && cargo clippy -q && cargo check`) on `dev`-push |

## Open questions

- Was `backend.yml` configured as a REQUIRED check on `dev` at the time PR #1426 merged, and did it actually pass? (Audit via `gh pr view 1426 --json statusCheckRollup` and branch-protection state.)
- Are EAS build secrets (EXPO_TOKEN, EXPO_PROJECT_ID, Android keystore, iOS provisioning) provisioned in repo/org secrets, or will the workflows fail at the credentials step even with pins fixed?
- Is `security-test-gate.yml` currently a required status check on `dev` branch protection, or only advisory? (Requires `gh api` — out of static scope.)
- What fraction of contributors have the local pre-push fmt/clippy hook (#1431) installed? Is install enforced by `setup.sh` or opt-in?

## Decisions needed

- **Scope of pre-push fmt/clippy gate (#1431):** local hook only / CI status check / both — owner: pm-tech-lead.
- **`dev`-push smoke gate enforcement model:** fail-fast (block the push) vs warn-only (notify but allow) — owner: pm-tech-lead + pm-devops.
- **CI bisect protocol for `dev` red:** who owns + escalates when `dev` breaks (PR #1426 → #1437 took >1 day to surface) — owner: pm-scrum-master.
