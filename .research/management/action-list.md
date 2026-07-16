# PPT Action List

_Generated: 2026-07-16T02:20:00Z · 20 items · 20 open_

| Priority | Status | Owner | ID | Action | Dependency | Source |
|---|---|---|---|---|---|---|
| high | open | pm-tech-lead | pm-scrum-master-portal-webhooks-stabilization | Stabilize backend/servers/api-server/src/routes/portal_webhooks.rs — top churn hotspot this run (3 edits: P... | none | pm-analysis 2026-07-16 |
| high | open | rust-backend | pm-security-agency-invite-membership-check | Add check_agency_membership(&mut conn, agency_id, principal.user_id) call to POST /api/v1/agencies/{id}/inv... | none | pm-analysis 2026-07-16 |
| high | open | rust-backend | pm-security-agency-invite-role-enum-validation | Validate CreateAgencyInvitation.role server-side against an allow-list enum before insert in backend/crates... | none | pm-analysis 2026-07-16 |
| medium | open | pm-frontend | code-review-mobile-native-kmp-mylistings-analytics-stub-retry1 | [BLOCKED-DROP: backend endpoints missing — superseded by gap-realtor-listings-analytics-endpoints] gap: MyL... | none | dispatcher-retry-remint 2026-07-16T02:06:06Z (retry_of=code-review-mobile-native-kmp-mylistings-analytics-stub reason=failed-no-pr cooldown_ok newest_failure=2026-07-08T12:21:22Z) |
| medium | open | pm-backend | gap-10a-2-protectedroute-role-fallback-fix-for-multi-retry1 | [RECONCILED-DONE: implementer verified #482 deriveActiveRole+ProtectedRoute fix already on dev, 919 ppt-web... | none | dispatcher-retry-remint 2026-07-16T02:06:06Z (retry_of=gap-10a-2-protectedroute-role-fallback-fix-for-multi reason=failed-no-pr cooldown_ok newest_failure=2026-07-08T08:18:14Z) |
| medium | open | pm-tech-lead | gh-issue-2357 | Follow-up: role gate on GET /ai/chat/escalated shipped without a deny-path regression test (PR #2356) (Clos... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2357) |
| medium | open | pm-tech-lead | gh-issue-2358 | Follow-up: inquiry retry-on-unmatched shipped ahead of its dedup net — retries insert duplicate leads + unb... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2358) |
| medium | open | pm-tech-lead | gh-issue-2359 | Follow-up: /agencies/me resolves left/inactive memberships + no backend test (PR #2355) (Closes #2359) | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2359) |
| medium | open | pm-tech-lead | gh-issue-2360 | Follow-up: per-portal view webhook still inflates syndication stats on replay/retry — increment not gated o... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2360) |
| medium | open | pm-tech-lead | gh-issue-2361 | Follow-up: login cache eviction is incomplete vs. logout — prior org&#39;s tenant-scoped data survives into... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2361) |
| medium | open | pm-tech-lead | gh-issue-2362 | Follow-up: consolidate reality-web listing-detail shape validation into one parseListingDetail() normalizer... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2362) |
| medium | open | pm-tech-lead | gh-issue-2363 | Follow-up: signer adopts an e-signature without ever seeing the document; signing token left in the URL/his... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2363) |
| medium | open | pm-tech-lead | gh-issue-2364 | Follow-up: guard against the Decimal-as-string decode trap recurring on sibling reality-server surfaces (PR... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2364) |
| medium | open | pm-tech-lead | gh-issue-2365 | Follow-up: AC-5 price-alert toggle shipped with no executing test + non-persisting OFF state (PR #2348) (Cl... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2365) |
| medium | open | pm-tech-lead | gh-issue-2366 | Follow-up: direct-to-S3 upload drops building_id — building-scoped documents lose their association (PR #23... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2366) |
| medium | open | pm-tech-lead | gh-issue-2367 | Follow-up: validate every synthesized candidate id against IdSchema in scanCandidates (PR #2344) (Closes #2... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2367) |
| medium | open | pm-tech-lead | gh-issue-2368 | Follow-up: mobile upload allow-list duplicates backend constants with no drift guard; photo picker bypasses... | none | dispatcher-issue-ingest 2026-07-16T02:05:48Z (#2368) |
| medium | open | pm-scrum-master | pm-scrum-master-coverage-rescan-epic-84 | Re-run scoped coverage.json scan against dev HEAD to confirm 84-1-s3-presigned-urls and 84-2-esignature-ema... | none | pm-analysis 2026-07-16 |
| medium | open | rust-backend | pm-security-agency-member-extractor | Introduce a shared Axum extractor (e.g. AgencyMember) that enforces agency membership at the type level for... | pm-security-agency-invite-membership-check | pm-analysis 2026-07-16 |
| medium | open | rust-backend | pm-security-reality-list-members-authz-audit | Audit list_members handler at backend/servers/reality-server/src/routes/agencies.rs (~345) — currently has ... | none | pm-analysis 2026-07-16 |
