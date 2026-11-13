# pm-devops — 2026-11-13

## Summary
Dispatcher pipeline recovered from stall (10+ zero-merge runs → 58 PRs merged in 4 days), but the research-routine scheduler itself drifted 4 days (last_run_iso=2026-11-09). 9 open Dependabot PRs including 3 cross-stack majors need a human review window.

## Next actions
1. **[high]** Unblock mobile-native/KMP builds in the cloud runner (issue #2652 CLOSED but structural gap persists). DoD: a KMP code-review task can be claimed and merged by the cloud dispatcher. _(Carried from earlier runs — long-lived.)_
2. **[medium]** Investigate the 4-day research-routine scheduler lag. DoD: a Phase-1 hard-fail (or dispatcher-visible alert) fires when `(now − last_run_iso) > 48h`.
3. **[medium]** Human-review the 3 cross-stack major Dependabot bumps: #3447 vitest 4→5, #3444 jsonwebtoken 10→11, #3443 utoipa-swagger-ui 9→10. DoD: all three either merged or blocked with a documented reason.

## Risks
- **Cloud routine scheduler silent drift** (medium/medium). Mitigation: add an explicit lag-floor check.
- **Major-version Dependabot stack thickening** (medium/low). Mitigation: WIP cap + 72h review SLA.
- **Mobile-native-kmp cloud starvation** (carried, high/medium): ~5 permanently unclaimable backlog rows.

## Open questions
- Does the cloud routine scheduler emit any telemetry on failed invocations, or did 4 days of lag genuinely not surface anything?

## Decisions needed
- **Lag-floor policy** — should `(now − last_run_iso) > 48h` hard-fail the next run? Owner: pm-devops.
- **Dependabot major-review SLA** — define a WIP cap + review window for majors. Owner: pm-devops.
