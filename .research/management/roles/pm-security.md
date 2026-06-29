# pm-security — 2026-06-29

_Rotating role this run (pm_cursor idx 5 → 6). Static read; no compile/run._

## Summary

This window's security read confirms three legacy findings closed and one **new
cross-org notification fan-out risk** in the document-sharing surface
(`resolve_share_recipients` in `backend/servers/api-server/src/routes/documents/shares.rs`
around lines 730–737 — the BUILDING branch lacks an `organization_id` join, so a
manager who can guess or enumerate a foreign building UUID can fan share
notifications to residents of that foreign organization). Three open OAuth
test-hardening gates (#480, #481, #487) still block Epic 10A story pickup; 33
days have passed since #480 was opened and the WebSocket-JWT-in-query-string
exposure is unmitigated.

## Confirmed resolved (this window)

- **#617** cookie `Path` — `Path=/api/v1/auth` in `build_refresh_cookie`
  (`backend/servers/api-server/src/routes/auth.rs:1012`); regression tests at lines 2472,
  2554, 2590, 2597.
- **#481 OAuth refresh revocation** — `find_refresh_token_by_hash` filters
  `revoked_at IS NULL` (`backend/crates/db/src/repositories/oauth.rs:406–421`);
  `find_refresh_token_by_hash_including_revoked` correctly scoped to family-reuse detection.
- **#1806 DB-backed manager gate** — `verify_manager_role_in_org` reads
  `organization_members` for the path org, not JWT claims
  (`backend/servers/api-server/src/routes/integrations/sync.rs`).
- **P1-04 audit-hash leak** — no `{:?}` debug-format in production tracing/audit paths.

## New risk surfaced this window

**Cross-org notification fan-out via BUILDING share-type** (medium probability, high impact).
`resolve_share_recipients` BUILDING branch at `shares.rs:730–737` does
`WHERE building_id = $1` with no constraint that the building belong to the
caller's organization. Must add `JOIN buildings b ON b.id = $1 WHERE b.organization_id = <caller_org_id>`
before 7a-5 ships. Add regression test in `document_sharing_tests.rs`.

## Next actions (security lens)

1. **HIGH — Add org-scope to `resolve_share_recipients` BUILDING branch** (pm-backend) —
   `shares.rs:730–737`. DoD: BUILDING branch filters by org_id; cross-org building_id
   returns empty recipient set; regression test added.
2. **HIGH — Close issue #485** (pm-frontend) — UUID validation on share-panel target_id +
   backend rejects non-UUID with 400 before 7a-5 promotes.
3. **HIGH — Land OAuth end-to-end security test suite** (pm-qa) — revoked-token rejection
   (RFC 9700), token-family replay burn, PKCE S256 enforcement. Required before 10a-1 / 10a-3
   promote.
4. **HIGH — Fix #480** (pm-backend) — move WebSocket auth token from query param to
   header/cookie; add session re-validation after JWT expiry on WS connections.
5. **HIGH — Confirm or close #614 / #624** (pm-backend) — locate where the original
   reports schedule PUT handler landed after `report_schedule.rs` was removed and verify
   tenant/org scope + capability check are present.
6. **MEDIUM — Fix #482** (pm-frontend) — `ProtectedRoute` must not fall back to
   `tenants[0]` for multi-tenant users; add unit tests.

## Risks

- **Cross-org notification fan-out** (BUILDING share-type, no org-scope join) — medium / high.
- **OAuth security contract untested end-to-end** — three open gates (#481, #487, #482)
  predate any 10a implementation; high / high.
- **JWT in WebSocket query string (#480)** — 33 days unmitigated; tokens hitting access logs;
  high / medium.
- **Document share UUID input not validated (#485)** — medium / medium.
- **7a-2 folder CI red** — gates 7a-3 / 7a-4 / 7a-5 cannot safely start; high / medium.

## Open questions

- Where did the original reports schedule PUT handler land after `report_schedule.rs` was
  removed — is the current handler tenant-scoped + capability-gated?
- Does `create_share` validate that `target_id` (USER / BUILDING share types) belongs to the
  caller's organization, or only that the document itself is RLS-visible?
- Has any interim mitigation been applied to issue #480 (WS JWT log scrubbing, token
  rotation on connect)?
- Are 10a-* OAuth stories still entirely ready-for-dev with no implementation, or has
  exploratory branch work begun that hasn't been reflected in sprint-status.yaml?
- Does the public share access path (`/documents/shared/{token}`) enforce a rate limit or
  brute-force throttle on token enumeration?

## Decisions needed

- Classify the BUILDING fan-out org-scope gap as a release blocker for 7a-5 vs. post-merge
  follow-up — owner: pm-security / pm-backend.
- Must all of #480 / #481 / #487 be closed before any 10a story is started, or only before
  they are promoted to done? — owner: pm-tech-lead.
