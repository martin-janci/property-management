# pm-tech-lead

<sub>Last ran: 2026-11-20</sub>

## Summary

The sprint-status.yaml sprint (Epics 6/7A/8A/10A and others) is entirely 'done', but the actual work is review follow-ups and churn on three oversized files. The risk is structural rather than feature-related: these monoliths and a stalled dispatcher are driving rework.

## Next actions

- **[high]** Split services/push_fanout.rs (3038 LoC) into a push/ module: fcm.rs, apns.rs, combined adapter, worker, config. Keep the PushTransport trait as the seam. — dep: none — DoD: No file exceeds ~800 LoC, behaviour is unchanged, and existing push tests pass unmodified.
- **[high]** Split routes/voice_webhooks.rs (3066 LoC, churned in 5 runs) into signature verification, an idempotent event handler and provider-specific parsers. Move business logic into services/. — dep: none — DoD: The route file is a thin handler layer, and a webhook replay and signature-failure test matrix exists.
- **[high]** Resolve issue #3357 (stalled dispatcher): decide which approved PRs to rebase manually, and define an escalation path for non-mechanical conflicts that replaces the dead-end Tier-1d signal. — dep: dispatcher owner — DoD: The 3 blocked PRs are merged or reassigned, and a documented escalation rule exists.
- **[medium]** Split oauth_integration_tests.rs (3427 LoC) by flow: authorize, token, introspect/revoke, grants. Extract shared fixtures into a common module. — dep: none — DoD: Each suite file is under 800 LoC and CI test time and coverage are unchanged.
- **[medium]** Create a new sprint in sprint-status.yaml. The current sprint goal is all 'done' and no longer reflects the work being merged (review follow-ups, churn characterization). — dep: pm — DoD: The sprint file has a current sprint name, goal and in-progress items.
- **[medium]** Define an NFR baseline for push and voice webhooks: metrics, retry/dead-letter policy, and FCM/APNs token-refresh failure alerting. Check the observability stack for these. — dep: devops — DoD: A documented SLO and dashboard or alert list exists for both pipelines.

## Risks

- **high/high** — Repeated churn in push_fanout.rs and voice_webhooks.rs points to missing module boundaries. Review follow-ups keep re-touching the same files, which raises merge-conflict and regression risk and feeds the dispatcher stall.  _mitigation:_ Decompose the files, and enforce CODEOWNERS or a single-writer rule per hot file in the dispatcher.
- **high/high** — Dispatcher stall (#3357): with 9 PRs dep-blocked, 3 approved PRs conflicting, and 41/50 of the buffer used, the pipeline can deadlock. Throughput then drops while review debt accumulates.  _mitigation:_ Add conflict-aware scheduling, serialize PRs that touch hot files, and let a human resolve non-mechanical conflicts.
- **medium/high** — Webhook and push paths are externally facing and security-sensitive (signature validation, replay, provider credentials such as the APNs JWT and FCM service account). In 3000-line files, security fixes are hard to review.  _mitigation:_ Isolate verification code into a small, fully tested module, and add replay and idempotency tests.
- **high/medium** — Sprint tracking is stale: it is dated 2025-12 with a 2026-10 reconcile, and nothing is in-progress. Planning and coverage reporting cannot detect real drift, and the 'done' claims rest on verify notes rather than on current CI.  _mitigation:_ Reset the sprint, and have the CI gate on coverage.json reconcile against sprint-status.
- **medium/medium** — Several 'done' stories have known gaps: mobile comments and document preview, 9 iOS stub destinations, and the document permission UI. That is hidden scope that could surface as rework.  _mitigation:_ Turn each gap into an explicit backlog story with an owner.

## Open questions

- What is the actual deadlock cause in #3357 (which files conflict, and which PRs)? I did not read the issue body or the dispatcher JSON.
- Which push_fanout.rs and voice_webhooks.rs changes were the 3 churn-characterize PRs, and do they already add the tests needed for a safe refactor?
- Is _bmad-output/architecture.md current relative to the push fanout (FCM/APNs) and voice webhook subsystems? I did not open it, so any ADR for them is unconfirmed.
- What is the idempotency and replay-protection model for voice webhooks, and is signature verification centralized?
- Are there metrics or alerts on the push fanout worker's retry, dead-letter and stale-token deletion paths?

## Decisions needed

- Adopt a file-size ceiling (for example 800 LoC) with a refactor-first rule for hot files before more follow-ups land — owner: tech-lead
- Choose the dispatcher conflict policy (serialize hot-file PRs or auto-rebase with a human escalation path) — owner: dispatcher owner / pm
- Decide whether push and voice webhooks move behind service-layer traits or stay in route and service modules — owner: tech-lead
- Close the stale sprint and define the next sprint goal and scope — owner: pm
