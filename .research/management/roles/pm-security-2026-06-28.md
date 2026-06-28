# pm-security — 2026-06-28

_Rotating role — 32-day gap since 2026-05-27. Static read; no compile/run. Last routine run 2026-06-16._

---

## Summary

The last 12 days (since 2026-06-16) landed three meaningful security fixes — Airbnb reservation manager-gate (#1741), duplicate JWT-trusting IoT WebSocket removal (#1737), and PAP-321 accounting hardening (#1809) — alongside high-volume feature additions in messaging, tenant-migration, guest-ID upload, and regional-compliance. Five test-hardening-batch issues from thb-2026-05-25 remain open (#480, #481, #482, #483, #487), two of which are release blockers (JWT token in access logs, refresh-token revocation bypass). Three active security plans (LLM-doc IDOR, realtor inquiry-read IDOR, inquiry IDOR regression test) are all still relevant and unmerged.

---

## 1. Security regressions / fixes in the last 12 days

**Fixes landed (positive signals):**

- **Airbnb reservations manager-gate** (`fix #1741`, PR #1667, `routes/integrations/oauth.rs`): `list_airbnb_reservations` was exposed to any org member; now requires `verify_manager_role_in_org`. Regression test added (`reservations_manager_gate_rejects_resident`). Closes the sibling gap from #1635/#1639.
- **IoT duplicate JWT-trusting WS channel removed** (`fix #1737`, PR #1668): `ws_sensor.rs` (245 lines) deleted; the JWT-only channel that bypassed DB auth checks is gone. Frontend converged on the DB-checked handler. Reduced attack surface for token replay on WebSocket.
- **Accounting PAP-321 hardening** (`fix #1809`): Financial calculation hardening follow-ups. Low blast radius for auth.

**New surface added — requires scrutiny:**

- **Guest ID-document upload + OCR (Story 18.2, #1750, commit 96682ccb0):** Adds `POST /api/v1/...` multipart endpoints that accept government ID scans and persist them with an OCR seam. File: `backend/migrations/00192_rental_guest_id_documents.sql`. Concern: needs MIME/size allow-list validation confirmed; PII classification of ID scans must be verified; S3 key prefix and ACL must be org-scoped. No security-focused test visible in commit stat.
- **23 tenant-migration endpoints (BIT-260, commit 66ed8776d, `routes/migration.rs`):** Landed 2026-06-27, 24 hours ago. Uses `require_platform_admin` guard (verified in code). Risk: bulk-export endpoints (`migration_repo` export lifecycle) could yield org-wide PII if the platform-admin guard has any bypass or if cross-org tenant_id extraction is wrong. New, untested surface.
- **Message attachments S3 presigned upload flow (BIT-184, commit d459a4d15):** Presigned URL generation for file attachments. Must confirm: URL lifetime, allowed MIME types, org scoping of S3 path prefix.
- **N-party group conversations (BIT-183, commit 80427ecdb, `routes/messaging.rs`):** Expanded participant list now exposed. Commit `2d19c9882` (2026-06-17) exposes full participant list for group threads — confirm cross-tenant participant enumeration is blocked.
- **Regional-compliance endpoints (Epic 72, commit 684d7f68c, `routes/regional_compliance/`):** 19 new endpoints handling AML/DSA-adjacent compliance data. No auth concerns visible in commit message; the `aml_dsa/` split already uses `require_compliance_role` correctly (confirmed in code), but the new regional-compliance cluster is unverified.

---

## 2. Top concerns from open test-hardening-batch issues

GitHub API is unavailable in this environment; issue details sourced from `sprint-status.yaml` (thb-2026-05-25).

- **#480 (severity: high, OPEN):** WebSocket auth token passed in query parameter is being logged; JWT token appears in access logs. Additionally, WS sessions are not re-validated after JWT expiry — a long-lived WS connection remains authenticated after the token expires. Gates story 8a-3. Direct secret-leak risk; release blocker.
- **#481 (severity: high, OPEN):** OAuth refresh-token revocation bypassed — `revoked_at IS NULL` predicate was removed from the query; revoked tokens remain reusable. Breaks RFC 9700. Gates stories 10a-1, 10a-3. Release blocker for OAuth story promotion.
- **#482 (severity: medium, OPEN):** `ProtectedRoute` role fallback uses `tenants[0]` for multi-tenant users — wrong tenant context selected; no unit tests. Gates story 10a-2.
- **#483 (severity: medium, OPEN):** Voice device IDOR fix (prior sprint) shipped with zero tests; `list-commands` leaks object existence via empty array vs 403. No story gate.
- **#487 (severity: medium, OPEN):** MFA rate-limit/brute-force coverage missing in e2e tests. Gates story 10a-1.
- **#486 (CLOSED 2026-05-26):** `getToken()` bypass closed — centralized into `authenticatedFetchJson`. No further action.

Referenced issues #1758–#1791 could not be fetched (GitHub API unavailable). Cannot assess their content.

---

## 3. Active security plans — status assessment

### `.research/plans/security-llm-doc-idor.md`
**Status: Still relevant, unmerged.**
`routes/ai.rs` handlers `publish_description` (line 2620), `list_listing_descriptions` (line 2599), `get_photo_enhancement` (line 2847) still discard `_principal` and call tenant-blind repo queries (confirmed in plan; no commit in the last 12-day window closes this). `publish_description` is a cross-tenant state-mutating write — the most critical of the three. The `ai.rs` file was not in the recent commit log for routes. This IDOR remains open. Severity: release-blocker for any Epic 64 LLM-document story promotion.

### `.research/plans/security-realtors-mark-inquiry-read-idor.md`
**Status: Still relevant, unmerged.**
`reality-server/src/routes/realtors.rs:250` `mark_inquiry_read` still binds no principal and calls unscoped `reality_portal_repo.mark_inquiry_read(id)`. No commit in the 12-day log touches `reality-server` routes. The unscoped write IDOR at `POST /api/v1/realtors/inquiries/{id}/read` remains live. Fix is a 3-line change (bind principal, call the already-existing scoped sibling method `mark_inquiry_read_for_realtor`).

### `.research/plans/test-gap-inquiry-idor-regression.md`
**Status: Still relevant, test file still missing.**
PR #497's `mark_inquiry_read_for_realtor` ownership fix still has no regression tests. The plan proposes `backend/servers/reality-server/tests/inquiry_idor_tests.rs`; no such file appears in recent commits. Without a test, a future refactor of the two-step ownership-EXISTS-then-UPDATE can silently reintroduce the IDOR.

All three plans remain unarchived and active. None have been superseded by recent merges.

---

## 4. Security hotspot analysis — top-3 churn files

### `backend/crates/db/src/repositories/document.rs` → split to `document/` (commit d23360875, 2026-06-23)
**Assessment: Structural split only; behavior-preserving. Low immediate risk.**
The commit message is explicit: "pure, behavior-preserving move: no SQL, signatures, or logic changed." Post-split, `shares.rs` exposes two concerns worth flagging:
- `revoke_share_rls` at line 160–175: the UPDATE has no ownership predicate (`WHERE id = $1` only, no `shared_by` or org check). If the handler calling this does not enforce ownership before calling the repo method, any authenticated user who knows a share UUID can revoke it. This is an application-layer responsibility; requires handler-level verification.
- `find_share_by_token_rls` at line 85–109: no org filter on the token lookup; token secrecy is the only isolation mechanism. Confirm token entropy is sufficient (`generate_share_token()` in `internal.rs` — not reviewed in this pass).

### `backend/servers/api-server/src/routes/aml_dsa.rs` → split to `aml_dsa/` (commit 41115f67b)
**Assessment: Compliance-sensitive surface; split well-guarded. Medium concern on new regional-compliance.**
Post-split `aml.rs`, `dsa.rs`, `edd.rs`, `moderation.rs` all gate correctly with `require_compliance_role` or `require_platform_compliance_role` (verified in code). The shared helpers in `shared.rs` are well-tested with unit assertions. The split itself is low risk. The adjacent new Epic-72 regional-compliance endpoints (`routes/regional_compliance/`) are unconfirmed; need a separate auth audit pass since they handle AML-adjacent data and landed in the same 12-day window.

### `backend/servers/api-server/src/routes/forms.rs` → split to `forms/` (commit 13fb2c28b)
**Assessment: RLS-scoped throughout; well-structured. Low immediate risk.**
`submissions.rs` uses `RlsConnection` uniformly — `org_id` and `user_id` resolved from the RLS context, not path parameters. No tenant-blind queries visible. `require_signatures` validation present. IP address extraction via `X-Forwarded-For` is first-value-of-CSV (line 48–52): correct for most proxy setups, but susceptible to IP spoofing if the first hop is attacker-controlled. Low risk in managed infrastructure; flag if deployed behind a configurable proxy.

---

## next_actions

1. **[high] Fix refresh-token revocation bypass (issue #481):** Restore `revoked_at IS NULL` predicate in the OAuth refresh-token query. Add RFC 9700 revocation regression test. DoD: revoked token returns 401; test in CI. Blocks story 10a-1 and 10a-3. dependency: pm-backend.

2. **[high] Fix JWT token in WebSocket query-param access logs (issue #480):** Move WS auth token from query param to header or sub-protocol; suppress token value from request logs. Add post-expiry session invalidation. DoD: no JWT in access log lines; expired token closes WS connection. Blocks story 8a-3. dependency: pm-backend.

3. **[high] Fix LLM-doc IDOR in ai.rs (security-llm-doc-idor plan):** Bind and use principal in `publish_description`, `list_listing_descriptions`, `get_photo_enhancement`; add org predicate to the three repo queries. DoD: cross-tenant write returns 404, regression test passes. dependency: pm-backend.

4. **[high] Fix realtor mark-inquiry-read IDOR (security-realtors-mark-inquiry-read-idor plan):** Wire `principal` into `realtors.rs:250`, call `mark_inquiry_read_for_realtor`; delete unscoped `mark_inquiry_read` method. DoD: non-owning realtor gets 404, regression test added. dependency: pm-backend.

5. **[medium] Audit guest-ID document upload (Story 18.2) for PII/security posture:** Confirm MIME/size allow-list, S3 key org-scoping, PII classification of government ID scans, and access-control on retrieval endpoints. DoD: upload handler passes security checklist; PII classification documented. dependency: pm-backend, pm-data.

6. **[medium] Add inquiry IDOR regression tests (test-gap-inquiry-idor-regression plan):** Create `backend/servers/reality-server/tests/inquiry_idor_tests.rs` with the three ownership scenarios. DoD: test file in CI, fails if `realtor_id` scoping removed. dependency: pm-qa.

---

## risks

- **Refresh-token revocation bypass (#481) — high probability, high impact:** Revoked OAuth refresh tokens are reusable. Breaks RFC 9700 token revocation. Release blocker for any OAuth story. Mitigation: restore `revoked_at IS NULL` predicate immediately.
- **JWT in WebSocket access logs (#480) — high probability, high impact:** JWT tokens written to access/request logs are extractable by any log-reader. Long-lived WS sessions bypass token expiry. Mitigation: move token to header, suppress from logs, add expiry check in WS upgrade path.
- **LLM-doc cross-tenant write IDOR (security-llm-doc-idor) — high probability, high impact:** `publish_description` allows any authenticated user to publish another tenant's LLM-generated listing description. Mutation-before-ownership-check pattern. Mitigation: land the fix from the plan before promoting Epic 64 stories.
- **Tenant-migration bulk-export surface (BIT-260, 24 hours old) — medium probability, high impact:** 23 new endpoints landed 2026-06-27 with `require_platform_admin` guard. Guard code verified in routes, but cross-org data isolation in `migration_repo` export queries is unreviewed. A single missing org predicate in a bulk-export path yields full-org PII dump. Mitigation: targeted auth/scope audit of migration export handlers before any prod deployment.
- **Guest-ID PII classification gap (Story 18.2) — medium probability, medium impact:** Government ID scan uploads lack visible PII classification, retention policy, and confirmed org-scoped S3 key structure. If S3 keys are predictable or keys are globally readable, scanned IDs are accessible cross-tenant. Mitigation: confirm S3 key entropy and bucket policy; add data-classification doc.

---

## open_questions

- Issues #1758, #1766, #1770, #1772, #1782–#1786, #1791 could not be fetched (GitHub API token invalid in this environment). Are any of these auth/IDOR/PII issues that should be escalated as release blockers?
- Does `revoke_share_rls` in `document/shares.rs` (no ownership predicate) get called from a handler that enforces ownership before invocation? If not, it is an IDOR on share revocation.
- What is the token length/entropy of `generate_share_token()` in `document/internal.rs`? Confirm it is at least 128-bit random.
- Have the 19 regional-compliance endpoints (Epic 72, `routes/regional_compliance/`) been through a role-gate audit comparable to the `aml_dsa/` compliance audit?
- Is the `ProtectedRoute` multi-tenant role fallback (#482) exercised anywhere in the admin-web OAuth flows currently in review? If yes, wrong-tenant context is a real auth regression on existing users.

---

## decisions_needed

- **Release gate decision:** Should issues #480 (JWT in logs) and #481 (refresh-token revocation bypass) block the next `dev`→`main` cut? Recommend yes. Owner: pm-scrum-master + pm-tech-lead.
- **PII classification for government ID scans (Story 18.2):** Who owns the data-retention and access-control policy for scanned government IDs? Must be decided before the feature ships to prod. Owner: pm-data.
