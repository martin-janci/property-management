# Action list — PPT delivery

_Generated 2026-08-28T02:35Z. Regenerated from `action-list.json` each Phase 1.6 run._

| Priority | Owner | Action | Dependency | Status |
|----------|-------|--------|------------|--------|
| high | pm-devops | Add a self-hosted Kotlin/Gradle/Android-SDK runner to the dispatcher's runner pool (label kmp-cloud) so mobile-native/KMP plans can be claimed in cloud mode — unblocks the 6 stuck backlog items and prevents future KMP-only buffer starvation | none | open |
| medium | pm-devops | Add a `runner_requires` field to plan frontmatter so the dispatcher can pre-filter unclaimable-in-cloud plans (e.g. `runner_requires: kmp-cloud`) instead of exhausting the queue and going quiet on buffer-low days | none | open |
| medium | pm-devops | Split the mobile-native/KMP backlog into a local-only lane surfaced via `/next-plan --local` so the user can consume it manually from a workstation that has Gradle/JDK/Android SDK — no cloud-runner dependency | pm-scrum-master | open |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | — | open |
| low | pm-devops | Document a mobile-native/KMP deferral policy — if no KMP runner is provisioned within an agreed window (e.g. 4 weeks), all KMP-only plans are formally deferred to an infra sprint and the backlog board is annotated so stakeholders see the paused state | pm-scrum-master | open |
| low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios | — | open |
| low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2] | — | open |
| low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors | — | open |
| low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS | — | open |
| low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests | — | open |
