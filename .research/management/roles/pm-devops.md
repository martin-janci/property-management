# pm-devops — 2026-08-28

**Rotating role slot:** pm-devops (last-run 2026-06-16, 2+ months overdue).
**Trigger context:** buffer-low — `claimable=6/72`, all mobile-native/KMP, unlandable in the cloud runner (no Gradle/JDK/Android SDK). Dispatcher can't work through the queue because KMP plans need a build toolchain the cloud runner lacks.

## Summary

Cloud runner cannot build `mobile-native/` (KMP: Kotlin 2.3.21 + AGP 8.7.3 + KSP 2.1.0 + Android SDK 34 + iOS via macOS runner), so all 6 remaining backlog items are stuck and the dispatcher is silently going quiet. The fix requires either a KMP-capable runner joined to the pool, a formal local-only lane the user consumes manually, or an explicit defer-to-infra-sprint decision. Doing nothing keeps repeating the buffer-low signal every day while hiding whether the backlog is genuinely exhausted or just infra-blocked.

## Next actions

| # | Action | Priority | Dependency | Definition of done |
|---|--------|----------|------------|--------------------|
| 1 | Add a self-hosted Kotlin/Gradle/Android-SDK runner to the dispatcher's runner pool (label `kmp-cloud`) so mobile-native/KMP plans can be claimed in cloud mode. | high | none | Runner registered with label `kmp-cloud`; one KMP plan lands end-to-end through dispatcher without human intervention; runner health surfaced in `stack status`. |
| 2 | Add a `runner_requires` field to plan frontmatter so the dispatcher can pre-filter unclaimable-in-cloud plans (`runner_requires: kmp-cloud`) instead of exhausting the queue and going quiet on buffer-low days. | medium | none | Field parsed by `next-plan` and `dispatcher/claim`; a KMP-only backlog surfaces as `all-remaining-unclaimable` in the digest, not `quiet=true`. |
| 3 | Split the mobile-native/KMP backlog into a local-only lane surfaced via `/next-plan --local` so the user can consume it manually from a workstation with a mobile-native toolchain ready. | medium | pm-scrum-master | `/next-plan --local` returns the highest-score KMP-tagged plan; documented in `docs/git-workflow.md` under a "local-only lane" heading. |
| 4 | Document a mobile-native/KMP deferral policy — if no KMP runner is provisioned within the agreed window (e.g. 4 weeks), all KMP-only plans are formally deferred to an infra sprint and the backlog board is annotated so stakeholders see the paused state. | low | pm-scrum-master | Policy checked in under `docs/`; existing 6 KMP plans get a `status: deferred` annotation if the window lapses. |

## Risks

| # | Risk | Probability | Impact | Mitigation |
|---|------|-------------|--------|------------|
| 1 | Dispatcher goes silent every day the buffer is KMP-only — hides genuine backlog exhaustion signals under a false "quiet" outcome, delaying decisions. | high | medium | Add `runner_requires` filter (action #2) and surface `all-remaining-unclaimable=<n>` in the daily digest so a human decides whether to spawn a runner or defer. |
| 2 | Self-hosted runner adds a maintenance surface (SDK licensing, Android emulator, disk pressure, iOS macOS runner) with no team owning it. | medium | medium | Define ownership + runbook before provisioning; prefer ephemeral runners over always-on; document rotation duty. |

## Decisions needed

1. Self-hosted KMP runner vs local-only lane vs defer-to-infra-sprint — owner: pm-tech-lead + pm-devops.
2. If self-hosted: who owns the runner (SDK updates, disk pressure, cost) — owner: pm-tech-lead.

## Open questions

- Is an iOS/macOS runner in scope, or is a Kotlin+Android-only KMP runner enough to unblock the current 6 plans? (KMP shared module compiles Android-side; iOS-only work would still block.)
- Do any of the 6 backlog items have a Ktor/kotlin-native-only path that could be exercised without an Android SDK, shrinking the runner scope?
