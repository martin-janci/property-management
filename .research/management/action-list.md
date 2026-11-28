# Action list — delivery backlog

<sub>Generated: 2026-11-28T04:15:00Z</sub>

- **Open:** 50
- **In-progress:** 21
- **Done:** 0
- **Failed:** 0

| ID | Priority | Owner | Action | Dep | Status | Deadline |
|----|----------|-------|--------|-----|--------|----------|
| `pm-scrum-master-land-the-2026-12-01-i18n-warn-error-lint-flip-burn-down-merg-2026-11-22` | high | `pm-scrum-master` | Land the 2026-12-01 i18n warn→error lint-flip burn-down: merge or close the 10+ open `code-review-*-i18n-retry1` items + drafts #3676/#3677 within 9 days | — | in-progress | — |
| `pm-data-hardening-burndown-series-2026-11-28` | high | `pm-devops` | Add a machine-readable Hardening burn-down series (date, workstream, rows_total, rows_deleted, rows_converted_to_gated_lane, due_bumps) generated from .github/quarantine/registry.toml and appended daily | — | open | — |
| `pm-data-notification-group-retention-2026-11-28` | high | `pm-backend` | Define grouped-notification retention/reset semantics for nil-id event-type groups (#3863 aftermath); record pre/post-#3863 cutoff for notification-group metrics | — | open | — |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | `pm-devops` | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | — | open | — |
| `pm-scrum-master-cloud-runner-kmp-swagger-ui-egress-unblock-2026-11-16` | high | `pm-scrum-master` | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | — | open | — |
| `pm-scrum-master-map-hardening-prs-to-counters-2026-11-28` | high | `pm-tech-lead` | Map merged hardening PRs to modules-under-watch and quarantine-burndown counters and update sprint-status.yaml | — | open | — |
| `pm-scrum-master-merge-dispatcher-drafts-3891-3893-2026-11-28` | high | `pm-security` | Review and merge dispatcher drafts #3891, #3892 and #3893 (listings.rs db-error leak first) | — | open | — |
| `pm-scrum-master-provision-cloud-runner-2026-11-28` | high | `pm-devops` | Provision or confirm the cloud-runner needed to unblock the mobile-native backlog (0/4) | — | open | — |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-2652-2966-2026-11-24` | high | `pm-devops` | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable. | — | open | — |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-in-the-cloud-runner-2652-2-2026-11-20` | high | `pm-scrum-master` | Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) | — | open | — |
| `pm-scrum-master-unblock-kmp-swagger-ui-egress-2652-2966-2026-11-21` | high | `pm-scrum-master` | Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — claimable still ≈9/72 | — | open | — |
| `pm-scrum-master-unblock-mobile-native-kmp-swagger-ui-egress-in-the-cloud-run-2026-11-22` | high | `pm-scrum-master` | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable | — | open | — |
| `code-review-ppt-web-core-logout-purge-disputes-leases-iot-retry1` | medium | `pm-security` | ppt web core logout purge disputes leases iot [retry 1/2 of failed code-review-ppt-web-core-logout-purge-disputes-leases-iot] | — | in-progress | — |
| `gh-issue-3834` | medium | `pm-tech-lead` | Follow-up: guest-report submission dates hardcoded to en-GB despite full i18n (PR #3816) (Closes #3834) | — | in-progress | — |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | `pm-backend` | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | — | open | — |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | `pm-backend` | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | — | open | — |
| `pm-backend-address-mobile-native-stubs-2026-11-21` | medium | `pm-backend` | Address mobile-native stubs: CreateListingScreen onSubmit NotImplementedError + InquiriesResponse page_size/limit contract mismatch + Ktor HttpTimeout default. Package as one KMP PR per issue once #2652 egress unblocks | — | open | — |
| `pm-data-audit-research-management-churn-2026-11-28` | medium | `pm-scrum-master` | Audit .research/management/*.json churn: diff two consecutive dispatcher runs for non-deterministic fields (timestamps, random ids, ordering); make writes idempotent/sorted | — | open | — |
| `pm-qa-confirm-non-kmp-medium-confidence-2026-11-09` | medium | `pm-qa` | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | — | open | — |
| `pm-scrum-master-quarantine-burndown-weekly-target-2026-11-28` | medium | `pm-qa` | Set a weekly quarantine burn-down target (~17/week) and publish a slice plan to hit 201 by 2027-02-19 | — | open | 2027-02-19 |
| `pm-scrum-master-triage-stale-prs-2026-11-28` | medium | `pm-devops` | Triage stale dependabot PRs (#3443, #3444, #3446-#3449, #3634) and human PRs #2555, #2559: merge, rebase or close | — | open | — |
| `pm-security-submit-overwrite-semantics-2026-11-25` | medium | `pm-security` | Fix submit overwrite semantics on person_month_submissions: ON CONFLICT DO UPDATE lets a co-resident overwrite another resident's pending proposal and rewrites submitted_by. Decide: reject or audit trail; cover with test. | pm-product | open | — |
| `code-review-ai-ocr-rls-bypass-pattern` | low | `pm-tech-lead` | ai ocr rls bypass pattern | — | in-progress | — |
| `code-review-api-core-authz-require-perm-fail-open` | low | `pm-backend` | api core authz require perm fail open | — | in-progress | — |
| `code-review-api-core-digest-sweep-lock-release-leak-retry1` | low | `pm-tech-lead` | release_digest_sweep returns a failed-unlock connection to the pool (advisory-lock leak) [retry 1/2 of failed code-review-api-core-digest-sweep-lock-release-leak] | — | in-progress | — |
| `code-review-api-core-digest-worker-no-replica-claim` | low | `pm-backend` | api core digest worker no replica claim | — | in-progress | — |
| `code-review-api-core-idempotency-tx-across-handler` | low | `pm-backend` | [NEEDS-RESCOPE: confirmed real bug; true fix needs a db-migration slice (pending-claim ledger) + #[sqlx::test] w/ Postgres — out of rust-backend scope] api core idempotency tx across handler | — | in-progress | — |
| `code-review-api-handlers-install-empty-cred-default` | low | `pm-backend` | api handlers install empty cred default | — | in-progress | — |
| `code-review-api-handlers-summarize-discards-persist-err` | low | `pm-backend` | api handlers summarize discards persist err | — | in-progress | — |
| `code-review-api-handlers-tenant-config-rls-clear-swallowed` | low | `pm-backend` | api handlers tenant config rls clear swallowed | — | in-progress | — |
| `code-review-mobile-rn-fault-category-i18n` | low | `pm-backend` | mobile rn fault category i18n | — | in-progress | — |
| `code-review-mobile-rn-list-search-placeholders-i18n` | low | `pm-backend` | mobile rn list search placeholders i18n | — | in-progress | — |
| `code-review-mobile-rn-untested-nfc-credentials` | low | `pm-tech-lead` | mobile rn untested nfc credentials | — | in-progress | — |
| `code-review-ppt-web-core-websocket-prod-console` | low | `pm-backend` | ppt web core websocket prod console | — | in-progress | — |
| `code-review-ppt-web-core-ws-stale-token-on-refresh` | low | `pm-backend` | ppt web core ws stale token on refresh | — | in-progress | — |
| `code-review-ppt-web-ui-bookfacility-silent-availability` | low | `pm-backend` | ppt web ui bookfacility silent availability | — | in-progress | — |
| `code-review-ppt-web-ui-financial-feature-no-i18n` | low | `pm-backend` | ppt web ui financial feature no i18n | — | in-progress | — |
| `code-review-ppt-web-ui-templatelibrary-silent-mutation-fail` | low | `pm-backend` | ppt web ui templatelibrary silent mutation fail | — | in-progress | — |
| `code-review-reality-server-members-noauth` | low | `pm-security` | reality server members noauth | — | in-progress | — |
| `code-review-reality-web-layoutsections-dup-key` | low | `pm-backend` | reality web layoutsections dup key | — | in-progress | — |
| `code-review-api-core-rng-expect-panic` | low | `pm-backend` | api core rng expect panic | — | open | — |
| `code-review-api-handlers-install-webhook-example-url` | low | `pm-backend` | api handlers install webhook example url | — | open | — |
| `code-review-listings-rs-raw-db-error-leak` | low | `pm-security` | listings rs raw db error leak | — | open | — |
| `code-review-listings-rs-unstructured-error-body` | low | `pm-backend` | listings rs unstructured error body | — | open | — |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | `pm-backend` | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | — | open | — |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | `pm-backend` | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | — | open | — |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | low | `pm-backend` | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | — | open | — |
| `code-review-mobile-rn-session-user-unnormalized` | low | `pm-backend` | mobile rn session user unnormalized | — | open | — |
| `code-review-ppt-web-core-disputes-uidispute-nonnull-crash-retry2` | low | `pm-backend` | ppt web core disputes uidispute nonnull crash [retry 2/2 of failed code-review-ppt-web-core-disputes-uidispute-nonnull-crash] | — | open | — |
| `code-review-ppt-web-core-errorhandler-i18n-leak` | low | `pm-backend` | ppt web core errorhandler i18n leak | — | open | — |
| `code-review-ppt-web-core-route-auth-guard-gap` | low | `pm-security` | ppt web core route auth guard gap | — | open | — |
| `code-review-ppt-web-ui-aml-threshold-errors-swallowed` | low | `pm-backend` | ppt web ui aml threshold errors swallowed | — | open | — |
| `code-review-ppt-web-ui-ballotquestion-yesno-i18n-retry2` | low | `pm-tech-lead` | ppt web ui ballotquestion yesno i18n [retry 2/2 of failed code-review-ppt-web-ui-ballotquestion-yesno-i18n] | — | open | — |
| `code-review-ppt-web-ui-community-list-routes-empty-retry2` | low | `pm-backend` | ppt web ui community list routes empty [retry 2/2 of failed code-review-ppt-web-ui-community-list-routes-empty] | — | open | — |
| `code-review-ppt-web-ui-filedisputedialog-i18n-retry2` | low | `pm-tech-lead` | ppt web ui filedisputedialog i18n [retry 2/2 of failed code-review-ppt-web-ui-filedisputedialog-i18n] | — | open | — |
| `code-review-ppt-web-ui-formrenderer-required-bypass` | low | `pm-backend` | ppt web ui formrenderer required bypass | — | open | — |
| `code-review-ppt-web-ui-groupdetail-delete-noop-retry2` | low | `pm-backend` | ppt web ui groupdetail delete noop [retry 2/2 of failed code-review-ppt-web-ui-groupdetail-delete-noop] | — | open | — |
| `code-review-ppt-web-ui-moderation-full-reload-nav` | low | `pm-tech-lead` | ppt web ui moderation full reload nav | — | open | — |
| `code-review-ppt-web-ui-portfolio-perf-i18n` | low | `pm-backend` | ppt web ui portfolio perf i18n | — | open | — |
| `code-review-ppt-web-ui-reading-form-ocr-silent-fail` | low | `pm-backend` | ppt web ui reading form ocr silent fail | — | open | — |
| `code-review-ppt-web-ui-rfqform-i18n-retry2` | low | `pm-tech-lead` | [already-resolved-on-dev, no change needed] ppt web ui rfqform i18n [retry 2/2 of failed code-review-ppt-web-ui-rfqform-i18n] | — | open | — |
| `code-review-reality-server-pm-api-client-build-expect` | low | `pm-backend` | reality server pm api client build expect | — | open | — |
| `code-review-reality-server-readiness-internal-detail-public` | low | `pm-backend` | reality server readiness internal detail public | — | open | — |
| `code-review-reality-web-account-routes-i18n-retry1` | low | `pm-backend` | reality web account routes i18n [retry 1/2 of failed code-review-reality-web-account-routes-i18n] | — | open | — |
| `code-review-reality-web-comparison-unchecked-cast` | low | `pm-backend` | reality web comparison unchecked cast | — | open | — |
| `code-review-reality-web-inquiry-type-raw-enum-retry2` | low | `pm-backend` | [stale-resolved @ 2026-11-10T09:10:40Z: finding already fixed on dev (inquiry-type.ts helper + all locales present, landed c5d7ca5); implementer confirmed no diff needed, no PR] reality web inquiry type raw enum [retry 2/2 of failed code-review-reality-web-inquiry-type-raw-enum] | — | open | — |
| `code-review-reality-web-listing-edit-slovak-i18n-retry1` | low | `pm-backend` | reality web listing edit slovak i18n [retry 1/2 of failed code-review-reality-web-listing-edit-slovak-i18n] | — | open | — |
| `code-review-reality-web-realtor-account-pages-no-i18n-retry1` | low | `pm-backend` | reality web realtor account pages no i18n [retry 1/2 of failed code-review-reality-web-realtor-account-pages-no-i18n] | — | open | — |
| `code-review-reality-web-realtor-listing-i18n` | low | `pm-backend` | reality web realtor listing i18n | — | open | — |
| `pm-scrum-master-review-draft-prs-3582-3585-3586-2026-11-17-retry1` | low | `pm-scrum-master` | Review draft PRs #3582 (T33 quarantine tests), #3585 and #3586 (ppt-web locale churn plans), then mark ready or close [retry 1/2 of failed pm-scrum-master-review-draft-prs-3582-3585-3586-2026-11-17] | — | open | — |
| `stalled-review-pr-3380-retry1` | low | `pm-devops` | Stalled review: PR #3380 (reality-web analytics i18n) — 8d, no reviewDecision [retry 1/2 of failed stalled-review-pr-3380] | — | open | — |
