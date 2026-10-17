# pm-integration — 2026-10-17

_Rotating role this run (cursor idx 7 → wraps to 0). Static analysis only. Last run before today: 2026-07-24 (3 months stale)._

## Summary

Reality-server **already exposes** public `GET /api/v1/realtors/{user_id}/profile` (unauthenticated, keyed by user_id), so the reality-web `/[locale]/realtor/[id]` shared-mock defect is a **frontend wiring + ID-semantics problem**, not a missing endpoint. The sibling api-server `community.rs` mounts `GET /groups/{id}` but **no DELETE or PUT/PATCH**, so the ppt-web community-group delete already wired in PR #3101 will 404/405 at runtime until the backend route lands (issue #3109). Same anti-pattern class — frontend wires a mutation whose backend endpoint doesn't yet exist. Also noted: TOTP encrypt_secret() panics on CSPRNG failure (totp.rs:153) instead of returning TotpError, converting a rare entropy error into a 500/abort on the MFA path.

## Next actions

| Priority | Action | Owner | Dependency |
|---|---|---|---|
| high | Implement `DELETE /api/v1/community/groups/{id}` on api-server (route in community.rs next to get_group, utoipa path, auth + owner/manager + tenant/RLS scoping, 204/403/404 shape, idempotent repeat delete → 404), per issue #3109 | pm-backend | none |
| high | Until DELETE lands, gate or feature-flag the ppt-web group-delete UI (or surface an explicit "not available" error) so users do not hit a silent 404/405 | pm-frontend | none |
| high | Wire reality-web `/[locale]/realtor/[id]` page to existing `GET /api/v1/realtors/{user_id}/profile` via `@ppt/reality-api-client`, replacing MOCK_AGENT; confirm `[id]` ≡ user_id UUID; add 404 state; thread `useTranslations` | pm-frontend | none |
| medium | Decide whether the public realtor profile contract needs extending: confirm `ProfileResponse` is safe for unauth callers (no private fields such as contact email/phone); add a public listings or reviews summary only if the UI needs it. Reviews already exist at `/realtors/{id}/reviews` | pm-security | none |
| medium | Back the realtor contact form with a real call — either existing reality-server inquiries endpoint or a new realtor-targeted inquiry route — with rate limiting and validation (spam/abuse risk on an unauthenticated form) | pm-backend | none |
| medium | Run a cross-service contract-drift audit: diff every frontend-called path (ppt-web + reality-web hooks + mobile) against mounted axum routes + OpenAPI; flag DELETE/PUT/PATCH first; add check to `api-validation.yml` CI | pm-qa | none |

## Risks

- **Repeating anti-pattern: frontend wires mutations whose backend endpoints don't exist (#3101 → #3109).** Shipped UI produces runtime 404/405 against the real API, discovered only post-merge. _Prob high · Impact medium._ Mitigation: require backend route + OpenAPI entry as merge prerequisite for the frontend wiring PR, or require a feature flag; add CI route-drift check.
- **reality-web realtor page uses a shared mock on a public SEO-indexed route** (every URL shows the same agent). User-visible correctness/trust issue, hides ID-semantics mismatch (page `[id]` vs backend `user_id`). _Prob high · Impact medium._ Mitigation: wire to `GET /realtors/{user_id}/profile`; agree on one canonical public identifier.
- **Public unauthenticated profile endpoint may expose private fields**, and the contact form has no backend rate limiting / abuse protection once wired. _Prob medium · Impact high._ Mitigation: field-level PII review of `ProfileResponse`; per-IP rate limit + validation on the inquiry route.
- **Group delete semantics undefined** (cascade vs soft-delete of posts/members/reactions; who may delete) — a quick implementation could orphan data or allow cross-tenant deletion. _Prob medium · Impact high._ Mitigation: specify authorization + cascade + RLS before implementation; negative tests for cross-building and non-owner access.
- **TOTP encrypt_secret panic on CSPRNG failure (totp.rs:153)** — rare entropy failure becomes 500/abort on the auth/MFA path; error code undefined. _Prob low · Impact medium._ Mitigation: return `TotpError` mapped to documented 5xx with no secret leakage; add test seam.

## Open questions

- Is the reality-web `/realtor/[id]` URL segment a user_id UUID, a slug, or a realtor-profile id, and does it match the `{user_id}` path param on reality-server?
- Does `ProfileResponse` (RealtorProfile) contain private fields unsafe on the unauth `GET /realtors/{user_id}/profile`?
- Authorization rules + cascade behavior for deleting a community group (creator only, building manager, org admin)? Do posts/members soft-delete or hard-delete?
- Are other frontend-wired DELETE/PUT/PATCH calls (community posts, events, marketplace items) also missing backend routes? community.rs currently has no delete/update for groups, posts, events, items.
- Does `reality-web/src/lib/realtor-api.ts` already target the existing profile route, or a different path?

## Decisions needed

- Canonical public realtor identifier (user_id vs slug) for reality-web URLs + API — owner: pm-tech-lead
- Group delete semantics (authz, soft vs hard, cascade) — owner: product owner + pm-backend
- Policy: a frontend PR may not wire a mutation until the backend route + OpenAPI entry are merged, or must sit behind a flag — owner: pm-scrum-master
- Whether the contact form posts to the existing inquiries endpoint or a new realtor inquiry endpoint — owner: pm-tech-lead

## Verified facts this run (static analysis)

- `backend/servers/reality-server/src/routes/realtors.rs` mounts `GET /realtors/{user_id}/profile` (no auth extractor) — public endpoint exists.
- `backend/servers/api-server/src/routes/community.rs` mounts `GET /groups/{id}` but **no DELETE / PUT / PATCH** on groups — explains issue #3109.
- `backend/servers/api-server/src/services/totp.rs:153` uses `.expect("OS rng failed")` where sibling fallible calls (lines 149, 162) use `map_err(|e| TotpError::EncryptionError(...))`?.
