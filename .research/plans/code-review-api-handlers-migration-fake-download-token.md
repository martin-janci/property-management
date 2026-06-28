# code-review-api-handlers-migration-fake-download-token

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 code review of `api-handlers` segment (2026-06-28); commit 66ed8776d (PR #1859)
**Confidence:** high

## Hypothesis
`download_template` in `routes/migration.rs` (lines 529-534) returns a `TemplateDownloadResponse` whose `download_url` is `format!("/api/v1/migration/templates/{}/file?format={:?}&token={}", template_id, query.format, Uuid::new_v4())`. The `Uuid::new_v4()` token is **never persisted, signed, or recorded anywhere** — and no `/file` route currently exists in the router (`router()` lines 43-75). The contract advertised to callers is "you got a capability URL — go fetch it". When a later PR wires the `/file` endpoint, the natural mistake is to ignore the unsigned, unverified token (because today's code generates it stateless) — at which point any caller who knows the `template_id` can pull the file, and platform-admin migration exports become an unauthenticated bulk-export surface. Fix: remove the fake token now, return a tracking handle instead (`download_id`) backed by a server-side issued capability table; or, return the file body directly from this same handler if the export is small.

## Evidence
- `backend/servers/api-server/src/routes/migration.rs:529-534` — `let download_url = format!("/api/v1/migration/templates/{}/file?format={:?}&token={}", template_id, query.format, Uuid::new_v4());`. Returned verbatim in `TemplateDownloadResponse { download_url, ... }`.
- `backend/servers/api-server/src/routes/migration.rs:42-75` — `router()` defines `/templates/{template_id}/download` (this handler) but **no** `/templates/{template_id}/file` route; nothing in the file consumes the token.
- Phase 1.5 confirmed via grep that no other module in the workspace registers `/templates/{template_id}/file` or persists the token in any table.
- The endpoint is gated by `require_platform_admin` at handler entry, but the **returned URL bears no auth** — if a downstream `/file` handler skips its own `require_platform_admin` (because the platform-admin already gave out the URL), the file becomes reachable by anyone who can read the response.
- Same anti-pattern is duplicated in the migration export download URL — see `download_export` in the same file (worth a follow-up gap-scan).

## Files
- `backend/servers/api-server/src/routes/migration.rs:529`
- `backend/servers/api-server/tests/migration_db_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security flag — reviewer should pick a side: stream-here vs persisted-capability table)

**Execution mode:** `Mode: cloud-ok` (backend-only, no UI / device).

## Repro steps
1. As a platform admin, call `POST /api/v1/migration/templates/{template_id}/download?format=xlsx`. Note the returned `download_url`.
2. Observe the `token=<uuid>` segment — copy any random UUID into place. The server has no record of either.
3. Today: GET on the URL fails (404 — no `/file` route mounted), so the leak is latent.
4. Hypothetical: when `/file` lands, if its handler trusts the URL contract ("the caller proves it has the token"), any caller — including a non-admin who has read this response — can fetch the file. The test below asserts the **current** contract pins to a real auth check, blocking the latent leak.

## Suggested approach
1. **Remove the fake token from the response now.** Either: (a) inline-stream the template body from this handler (the `/file` route never lands, and there's no follow-up endpoint to misuse); (b) replace `Uuid::new_v4()` with a row in a new `template_download_grants` table (`download_id`, `template_id`, `granted_to user_id`, `expires_at`, `org_id`) and return `download_url = "/api/v1/migration/templates/file/<download_id>"`.
2. **Add `template_download_grants` migration** if going with (b): `download_id uuid PRIMARY KEY`, `template_id uuid REFERENCES`, `granted_to_user_id uuid`, `granted_org_id uuid`, `expires_at timestamptz NOT NULL DEFAULT now() + INTERVAL '15 minutes'`, `consumed_at timestamptz NULL`.
3. **Implement `GET /templates/file/{download_id}`** — look up the grant, check unexpired + unconsumed + `granted_org_id == rls.tenant_id()` (also gate behind `require_platform_admin` for defense-in-depth), stream the file, mark `consumed_at = now()`.
4. **Test the rejection path** — see Test plan: a `download_id` that belongs to a different org must 404, an expired one 410, a consumed one 410.
5. **Look at `download_export` in the same file** — confirm whether it has the same fake-token shape. If yes, write a follow-up backlog row (don't expand this plan's scope).
6. **Update OpenAPI** — `TemplateDownloadResponse.download_url` description now says "single-use signed grant URL, expires in 15 minutes" instead of the implied "stateless URL".

## Alternatives considered
- **Inline-stream from this handler** — rejected because Excel/JSON templates can be 10+ MB and a long-polled download blocks the request connection (axum's per-request RAM budget matters); but a viable fallback if the grants table is rejected.
- **Sign the existing URL with HMAC + expiry timestamp instead of a DB grant** — rejected because we already track sessions per-tenant in the DB; another HMAC secret is one more rotation surface for negligible code savings.

## Root-cause trace
1. Symptom: handler advertises a capability URL it cannot enforce; the token has no server-side state.
2. ← `migration.rs:529` — `Uuid::new_v4()` interpolated into the URL string, never persisted.
3. ← The handler was implemented in PR #1859 as part of the Epic 66 23-endpoint sweep; the migration-job + template-table backings were added in `00192_rental_guest_id_documents.sql` (and migration-specific siblings) but the grant table was not.
4. Origin: PR #1859 (`fix(api-server): implement 23 tenant-migration endpoints — templates, import jobs, exports (BIT-260)`, 2026-06-27), commit `66ed8776d88810c88a0bd3b37fa050ded5832618`.

## Test plan
- [ ] `backend/servers/api-server/tests/migration_download_token_tests.rs::download_url_grant_is_persisted_and_org_scoped` — call `POST /templates/<id>/download`, capture the returned `download_url`, assert a `template_download_grants` row exists with `granted_org_id == calling_org`.
- [ ] `…::cross_org_caller_cannot_consume_grant` — org-A grants, org-B GETs the `download_url` → expect 404 (or 403 if you prefer; pick one and stick).
- [ ] `…::expired_grant_returns_410` — set `expires_at = now() - 1 minute`, GET → 410 Gone.
- [ ] `…::consumed_grant_returns_410_on_second_call` — single-use.
- [ ] Local: `cargo test -p api-server migration_download_token_tests`.

## Out of scope
- Reworking `list_templates` pagination (separate Phase 1.5 finding `code-review-api-handlers-migration-template-pagination`, score 2 — still in backlog).
- Refactoring `migration.rs` into sub-modules — it's at 1755 lines and a candidate for the next `repeated-churn` split, but that's a separate vector.
- Audit of every `Uuid::new_v4()` in the route layer — log only matches inside `download_url` / `presigned_url` strings; that's the contained shape.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-migration-fake-download-token.md`
- Mark the matching `backlog.json` row as `status: "done"`
