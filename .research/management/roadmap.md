# PPT Roadmap — upkeep 2026-11-09

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + refilled `action-list.json` (open 9 → 47 after buffer-low ingestion of tier-1d signals 2026-11-04..08) + sprint-status.yaml._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (cursor re-check — negative; no OAuth-AS surface activity in window).
- Delta vs 2026-11-06 run: **7 PRs merged** — all polish on already-shipped surfaces (reality-web share-cmp, ppt-web registry triad, auth password-change revocation, screen-map re-land).
- **Ops-critical blocker: `#3357`** — dispatcher pipeline stalled; tier-1d signals 2026-11-04..08 (19+ backend/web items) never reached action-list.json; 3 approved PRs orphaned from dev. **Patched this run** by manual ingestion; the durable fix is queued as a self-test (`pm-qa-dispatcher-3357-watchdog-test-2026-11-09`) + the Scrum Master sweep (`pm-scrum-master-tier1d-ingestion-sweep-2026-11-09`).
- **Recurring defect classes surfaced**:
  - **Raw-DB-error-leak** (6 open findings across listings.rs, rentals.rs, signatures.rs, tenant_lifecycle.rs, reality-server /readiness + SSO cb) — no contract test; shape re-grows.
  - **Xorg authz** (2 fresh — `create_delegation`, `add_resident`) — same class as 2026-06 inquiry mark_as_read IDOR (#497).
  - **i18n-leak** (portfolio-performance, forms, registry-rules-form) — no biome/vitest guard.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).

## Epic status (short)

- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0 → 1).** `#3353` password-change session revocation touches session layer, not the OAuth AS/token-mgmt code path; `last_checked = 2026-11-09` stamped on 10a-1/2/3.
- **Epic 10B — Platform Administration:** 7/7 done. `#3187` (cross_org_assign_violation_is_not_found) remains red on dev backend CI — technical blocker, not a coverage flip.
- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 9 (TOTP), 79, 80, 81, 82, 83, 84, 85:** all done.

## Ranked plan

### Phase: mvp — high

- **[high]** Fix dispatcher tier-1d signal ingestion (`gh-issue-3357`) — owner: `pm-tech-lead` — why: #3357 is a P0 ops incident; buffer dried to 6/72 while signals piled up. 3 approved PRs orphaned from dev.
- **[high]** `code-review-api-handlers-delegation-xorg-delegate` — owner: `pm-security` — why: cross-org privilege grant (voting/financial scope) via unvalidated delegate_user_id. Tier-1 security.
- **[high]** `code-review-api-handlers-unit-residents-add-xorg-user` — owner: `pm-security` — why: manager can attach foreign-org user as resident of own unit; cross-org data exposure.
- **[high]** `code-review-api-core-accounting-nonatomic-statement-ingest` — owner: `pm-backend` — why: non-atomic bank-statement ingest can persist orphan parent + partial lines on a mid-loop failure — financial-ledger data-integrity.
- **[high]** `code-review-api-core-accounting-matcher-ignores-currency` — owner: `pm-backend` — why: payment matcher compares amounts across currencies; can auto-suggest a CZK invoice for a EUR line.
- **[high]** `code-review-reality-server-readiness-raw-db-error-leak` — owner: `pm-backend` — why: unauthenticated /readiness can expose DB host/user/SQLSTATE on DB failure.
- **[high]** `code-review-api-handlers-refresh-rotation-nonatomic` — owner: `pm-security` — why: revoke before insert without tx; crash window leaves user without a valid refresh token.
- **[high]** `code-review-listings-rs-raw-db-error-leak` — owner: `pm-backend` — why: 36 call sites interpolate raw sqlx error into client body.
- **[high]** `code-review-ppt-web-ui-formrenderer-required-bypass` — owner: `pm-frontend` — why: required-field validator accepts empty string/array → required bypass.
- **[high]** `pm-qa-tier1d-security-no-regression-tests-2026-11-09` — owner: `pm-qa` — why: pre-attach cross-org fixtures to the delegation + unit-residents rows (prevent 2026-06 PR #497 "fix-without-test" pattern).
- **[high]** `pm-qa-dispatcher-3357-watchdog-test-2026-11-09` — owner: `pm-qa` — why: self-test T26 — every open tier-1d signal file must have a backlog or action-list row; closes the #3357 root cause.
- **[high]** `pm-scrum-master-tier1d-ingestion-sweep-2026-11-09` — owner: `pm-scrum-master` — why: manual sweep today covers 2026-11-04..08; the recurring sweep needs to live inside the routine.
- **[high]** `gh-issue-3187` — owner: `pm-security` — why: backend CI on dev red; release-blocker if cut.

### Phase: mvp — medium

- `code-review-api-core-workflow-exec-unbounded-spawn` — `pm-backend` — unbounded tokio::spawn per workflow on handle_event.
- `code-review-api-handlers-signatures-raw-db-err-leak` — `pm-backend` — same leak class as listings.rs; sweep.
- `code-review-api-handlers-rentals-raw-db-error-leak` — `pm-backend` — same class.
- `code-review-listings-rs-unstructured-error-body` — `pm-backend` — unify to shared ErrorResponse envelope.
- `code-review-reality-server-sso-cb-db-err-leak` — `pm-security` — SSO callback leaks raw sqlx error.
- `code-review-ppt-web-ui-portfolio-perf-i18n` — `pm-frontend` — feature folder untranslated.
- `code-review-ppt-web-ui-forms-feature-i18n` — `pm-frontend` — feature folder untranslated.
- `code-review-ppt-web-ui-registry-page-silent-mutation-errors` — `pm-frontend` — handleReview paths still raw.
- `code-review-ppt-web-ui-aml-threshold-errors-swallowed` — `pm-frontend` — threshold-breach errors silently swallowed.
- `code-review-ppt-web-core-privacy-route-unguarded` — `pm-frontend` — anonymous visitor to /settings/privacy goes 401 instead of login redirect.
- `code-review-api-core-accounting-date-proximity-todo` — `pm-backend` — matcher missing date-proximity scoring.
- `pm-qa-ppt-web-i18n-golden-test-2026-11-09` — `pm-qa` — biome/vitest rule against untranslated literals in features/*.
- `pm-qa-raw-sqlx-error-leak-contract-test-2026-11-09` — `pm-qa` — contract test on error-body shape.

### Phase: mvp — low

- `code-review-ppt-web-ui-registry-page-hard-navigation` — `pm-frontend` — window.location.href → useNavigate().
- `code-review-api-core-accounting-decimal-literal-parse` — `pm-backend` — Decimal::from_str → compile-time dec!/const.
- `code-review-api-handlers-tenant-lifecycle-path-leak` — `pm-backend` — platform-admin-gated path leak.
- `code-review-reality-server-readiness-internal-detail-public` — `pm-backend` — scrub commit/build/version from public /readiness.
- `code-review-ppt-web-core-errorhandler-i18n-leak` — `pm-frontend` — validation errors leak English via detail.message.
- `code-review-ppt-web-ui-registry-rules-form-i18n` — `pm-frontend` — RegistryRulesForm untranslated.
- `code-review-ppt-web-ui-reading-form-ocr-silent-fail` — `pm-frontend` — OCR path unwraps on reject.
- `code-review-ppt-web-ui-moderation-full-reload-nav` — `pm-frontend` — same hard-nav class as registry-page.
- `code-review-reality-server-sso-cb-upstream-err-leak` — `pm-security` — upstream provider error detail leaks.
- `code-review-reality-web-pricealerts-raw-err-log` — `pm-frontend` — raw console.error on auth'd mutations.
- `code-review-reality-web-realtor-listing-i18n` — `pm-frontend` — realtor create-listing page untranslated.
- `code-review-api-core-rng-expect-panic` — `pm-backend` — AuthService OS-rng .expect downgrade.
- `code-review-reality-server-pm-api-client-build-expect` — `pm-backend` — reqwest builder .expect at boot; bubble Result.

Buffer: 47/36 open · 0 candidates ranked but unqueued (all fresh tier-1d signals ingested; mobile-native-kmp excluded per #2652 cloud-build block).
