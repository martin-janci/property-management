# pm-security — 2026-07-06

_Rotating role this run (pm_cursor idx 5 → 6). Static read; no compile/run. Last ran 2026-05-27 (40 days stale — the longest gap in the rotation)._ 

## Summary

PR #2120 (merged this window) fixed a real bug in `outages.rs` where mutation handlers gated on `TenantExtractor::role.is_manager()` — a role sourced from the JWT `role` claim, which the production login flow (`JwtService::generate_access_token`) **never populates**, so `AuthUser::role` defaults to `None`→`Guest`. The fix correctly moved the gate to `RlsConnection::role()` (DB-validated via `organization_members`).

**However, the identical anti-pattern is still live at 34 call sites across 12 route files**, all in this sprint's active epics:

- `announcements/crud.rs` (4), `announcements/lifecycle.rs` (4), `announcements/engagement.rs` (3), `announcements/comments.rs`, `announcements/ai_draft.rs`, `announcements/stats.rs` — **Epic 6, in-progress**
- `documents/core.rs` (7), `documents/folders.rs`, `documents/shares.rs`, `documents/versions.rs` — **Epic 7A, in-progress**
- `granular_notifications.rs` — **Epic 8A**
- `templates.rs`

Because it fails **closed** to `Guest`, this isn't a privilege-escalation hole — it's a likely **functional/security-adjacent regression** where real managers get silently 403'd on core sprint deliverables (create/update announcements, document folder/share ops) in production, while integration tests pass because they mint raw JWTs carrying a `role` claim directly (masking the gap, exactly as the #2120 commit message notes for outages).

Draft PR #1797 (`fix/backend-authz-ocr-and-rental-pii`, 13 days open) — content diff shows exactly the fix needed (AuthUser required on OCR endpoints, manager-gate on rental booking/guest PII reads). Current working-tree code already contains equivalent protections, but shallow-clone ancestry could not confirm whether #1797 itself is what landed — flagged as open question.

## next_actions

- **[high]** Sweep announcements (`crud.rs`, `lifecycle.rs`, `engagement.rs`, `comments.rs`, `ai_draft.rs`, `stats.rs`) and documents (`core.rs`, `folders.rs`, `shares.rs`, `versions.rs`), plus `granular_notifications.rs` and `templates.rs` — replace `tenant.role.is_manager()` (JWT-derived) with `rls.role().is_manager()` or `ValidatedTenantExtractor`, matching the #2120 pattern. DoD: no mutating handler in these 12 files derives its manager gate from `TenantExtractor`/`AuthUser.role`; regression test drives a real login flow (`create_authenticated_user_with_org`), not a raw-token mint. dependency: pm-backend.
- **[high]** Confirm merge/close status of PR #1797 (OCR auth + rental PII gate) — fix content is present in the tree but shallow ancestry is inconclusive; if superseded, close #1797 to avoid duplicate/conflicting rework. dependency: pm-scrum-master.
- **[medium]** Add a CI grep/clippy check that flags `TenantExtractor` + `.role.is_manager()`/`.role ==` authorization checks on mutating handlers, to prevent this bug class from being reintroduced (second confirmed instance after the `ai.rs` equipment IDOR cluster and `report_schedule.rs`). DoD: lint runs in `backend.yml` and fails on any new `TenantExtractor`-role-based mutation gate outside an explicit allowlist. dependency: pm-devops.
- **[medium]** Verify dependabot #2018 (aes-gcm bump) against its usage in `api-server` and `integrations` crates before merge — confirm no breaking nonce/API changes and credential/token encryption round-trips still pass. dependency: pm-backend.
- **[medium]** Re-scope and land the OAuth 10a-1/10a-2/10a-3 security test suite (revoked-token, refresh-token-family-reuse replay, PKCE S256) before any of those stories are promoted — items #481/#487 remain open per the standing test-hardening gate. dependency: pm-qa.
- **[medium]** Locate and review PR #1812 (`reality_portal.rs` repository split, RLS boundaries) directly on GitHub — local git/gh access could not resolve the branch or diff this session. dependency: none.

## risks

- **34-site `TenantExtractor`-role anti-pattern (high/high):** silently 403's real managers on Epic 6/7A/8A mutations in production while tests pass. Mitigation: apply #2120 pattern across 12 files + real-login-flow regression test.
- **PR #1797 merge status unconfirmed (medium/high):** if still open while equivalent code exists in `dev`, a rebase/merge could silently reintroduce the anonymous-OCR-upload or rental-guest-PII-enumeration gap. Mitigation: confirm PR state via GitHub before further OCR/rentals work.
- **OAuth 10a-1/10a-2/10a-3 untested security contract (medium/high):** no introspection/refresh-rotation/PKCE test coverage; #481/#487 still open. Mitigation: land the OAuth security test suite gating story promotion.
- **aes-gcm bump crypto-surface change (low/high):** `#2018` touches crypto used for stored credentials/tokens; an unreviewed bump could change nonce handling or break decryption of already-encrypted data. Mitigation: manual crypto-surface review + full test pass before merge.
- **PR #725 ai-maintenance IDOR fix stalled (medium/high):** last seen at `verdict=changes` (session-IDOR, sentiment-IDOR, missing test) ~1 month ago; if still unmerged the vector stays live with no visibility this run. Mitigation: confirm current PR status; close the three change requests immediately.

## open_questions

- Is PR #1797 merged, still draft, or superseded? Fix content is present in the tree but ancestry couldn't be confirmed locally.
- What is the current state of PR #1812 (`reality_portal.rs` repository split)?
- Is PR #725 (ai-maintenance IDOR, B1/B2/B3 change requests) still open a month later?
- Does `security-test-gate.yml` actually block merges lacking a test file, or is it still advisory-only?
- Are the 34 `TenantExtractor`-role call sites actually causing production 403s for real managers today, or is there a compensating middleware/role-claim path not visible in the routes layer?

## decisions_needed

- Whether to treat the 34-site `TenantExtractor`-role anti-pattern in announcements/documents/notifications/templates as a release blocker for Epic 6/7A given it mirrors the just-fixed #2120 production bug — owner: pm-security / pm-backend.
- Whether to close or rebase draft PR #1797 given its fix content appears already present in the working tree — owner: pm-scrum-master.
- Whether to gate 10a-1/10a-2/10a-3 promotion on the still-open OAuth security test suite (#481/#487) this sprint — owner: pm-qa / pm-tech-lead.
