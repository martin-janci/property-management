# pm-devops — DevOps / Infrastructure

_Ran: 2026-05-27 (PM rotation, idx 4)_

## Summary

Backend + frontend CI workflows are healthy, but the **mobile (EAS) release pipeline is not yet wired into `.github/workflows/`** — the `eas-build-android.yml` / `eas-build-ios.yml` files referenced by the open queue (gap-85-2 CI-fix items) live only in unmerged draft PRs, so mobile has no merged release path today. The `security-test-gate.yml` workflow exists but its enforcement-vs-advisory status (required status check on `dev` branch protection) is unconfirmed after the PR #497 no-test incident.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Land the two blocked EAS mobile CI fixes together (gap-85-2-android-ci-fix + gap-85-2-ios-ci-fix): downgrade non-existent action pins (checkout/setup-node/pnpm-action-setup @v6 → @v4) in eas-build-android.yml + eas-build-ios.yml, add eas-cli devDependency and the missing Android npm scripts | pm-frontend | both workflows present in `.github/workflows/`, green on a no-op push; mobile release pipeline no longer red |
| medium | Confirm `security-test-gate.yml` actually fails PRs labelled `security` that ship without a test file (the policy QA flagged after PR #497); add a required-status-check on the gate in `dev` branch protection so it is enforced, not advisory | pm-qa | gate appears as a required check on `dev`; a security-labelled PR with no test file is blocked |
| medium | App.tsx is the persistent top frontend churn hotspot (route wiring lands there every sprint); enable a merge queue / auto-rebase ordering for App.tsx-touching PRs so the concurrent dispatcher draft cluster (#563–#568) does not triple-conflict on the router file | none | auto-rebase-stale-drafts.yml or a merge queue serializes App.tsx-touching PRs |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Mobile release pipeline has no merged path: EAS Android/iOS build workflows exist only in draft PRs (#566 cluster) with broken `@v6` action pins; no mobile artifact can be cut until they land and are corrected. | high | medium | Land gap-85-2-android-ci-fix + gap-85-2-ios-ci-fix together; verify action pins resolve and eas-cli is on PATH in CI. |
| `security-test-gate.yml` may be advisory only (not a required check): PR #497 shipped a security IDOR fix with zero tests despite the gate existing, suggesting it does not block merges. | medium | high | Make the gate a required status check on `dev`; re-run against a synthetic test-less security PR to confirm it blocks. |
| App.tsx router-file churn + 6 concurrent dispatcher drafts (#563–#568) risk repeated triple-conflict rebases, stalling the merge queue. | medium | medium | Serialize App.tsx-touching PRs via merge queue / auto-rebase ordering. |

## Open questions

- Is `security-test-gate.yml` currently configured as a required status check on `dev` branch protection, or only run advisorily? (Needs a `gh api repos/.../branches/dev/protection` check — out of static scope this run.)
- Are EAS build secrets (EXPO_TOKEN, EXPO_PROJECT_ID, Android keystore, iOS provisioning) provisioned in repo/org secrets, or will the workflows fail at the credentials step even after the pin fix?

## Decisions needed

- Adopt a merge queue for App.tsx-touching PRs — owner: pm-devops (with pm-frontend sign-off).
