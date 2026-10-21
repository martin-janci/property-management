# code-review-api-handlers-migration-upload-drops-file

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-10-19 api-handlers (new finding)
**Confidence:** high

## Hypothesis
`upload_import_file` (POST `/api/v1/migration/import/upload`, platform-admin bulk-import endpoint for Epic 66) silently discards every uploaded file: it reads up to 100 MB into a `Vec<u8>` named `_file_data` (the leading underscore is Rust's "intentionally unused" marker), fabricates a `file_path = format!("imports/{org}/{uuid}")` with no call to any storage writer, then persists an import_job row pointing at the non-existent path and returns `200 OK` with "File uploaded successfully. Validation will begin shortly." The downstream `validate_import` / `approve_import` flow then operates on a key that never existed. Smallest change: thread the multipart bytes through `state.storage_service.upload_bytes(...)` and use the returned storage key as `file_path` before creating the import_job; fail-closed (400/503) when `storage_service` is not configured.

## Evidence
- `backend/servers/api-server/src/routes/migration.rs:693` declares `let mut _file_data: Vec<u8> = Vec::new();` — the underscore prefix suppresses Rust's dead-store warning.
- `backend/servers/api-server/src/routes/migration.rs:715-720` reads the multipart body into memory with `field.bytes().await?` and assigns `_file_data = data.to_vec();`. The buffer is never referenced again (`grep -n _file_data backend/servers/api-server/src/routes/migration.rs` returns only these two lines).
- `backend/servers/api-server/src/routes/migration.rs:757` fabricates `let file_path = format!("imports/{}/{}", org_id, Uuid::new_v4());` with no S3 / MinIO / filesystem writer call.
- `backend/servers/api-server/src/routes/migration.rs:764-779` passes that fabricated `file_path` into `migration_repo.create_import_job(...)` with status `Pending`; line 796 returns `200` with job_id and the success message.
- Reachability: router wired at `backend/servers/api-server/src/lib.rs:320` under `/api/v1/migration`; module declared in `backend/servers/api-server/src/routes/mod.rs:101` as `pub mod migration;`; route mounted at `backend/servers/api-server/src/routes/migration.rs:59`.

## Files
- `backend/servers/api-server/src/routes/migration.rs:678`
- `backend/servers/api-server/src/routes/migration.rs:757`
- `backend/servers/api-server/src/lib.rs:320`

## Dependencies
<!-- No stem collisions with current plans/ or active assignments (verified 2026-10-19). -->

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Start the api-server (or run via `ppt-bridge` MCP) with a tenant that has a platform-admin user and `storage_service` configured against MinIO/S3.
2. `curl -X POST -H "$AUTHZ" -F 'file=@buildings.csv' -F 'template_id=<uuid>' https://<host>/api/v1/migration/import/upload` with a small (e.g. 1 KB) CSV, where `$AUTHZ` holds the admin JWT auth header.
3. Expected: the uploaded CSV lands in the storage bucket under the returned `file_path` key, and `GET /api/v1/migration/import/jobs/{job_id}` can read it back.
4. Actual: response body contains a plausible `job_id` + "File uploaded successfully", but the storage bucket is empty for that key — `aws s3 ls s3://<bucket>/imports/<org>/` shows no object, and the import_job row's `file_path` resolves to nothing. `validate_import` on the job id fails downstream (or claims zero records).

## Suggested approach
1. In `upload_import_file` (`backend/servers/api-server/src/routes/migration.rs:678-798`), after the loop finishes consuming the multipart field and `file_data` is populated, verify `state.storage_service.is_some()` and return `503 SERVICE_UNAVAILABLE` otherwise (fail-closed; do not create the DB row).
2. Replace the fabricated-path lines (`:757`) with a real upload: `let storage_key = format!("imports/{}/{}", org_id, Uuid::new_v4()); state.storage_service.as_ref().unwrap().upload_bytes(&storage_key, &file_data, content_type).await?;` — mirror the pattern used by the documents module (grep for `storage_service.upload_bytes` in `backend/servers/api-server/src/routes/documents/` for the exact call shape and error mapping).
3. Rename `_file_data` → `file_data` to drop the dead-store marker (and so a future reader can't silently reintroduce the stub by removing a `_` prefix).
4. Pass `storage_key` into `migration_repo.create_import_job(...)` as `file_path` instead of the fabricated string.
5. Keep the existing MAX_IMPORT_SIZE check at `:734` unchanged.
6. Add a unit/integration test under `backend/servers/api-server/tests/` that uploads a known fixture and asserts (a) the storage service received a matching key and (b) the response status is 200 only when the upload succeeded.
7. Regenerate OpenAPI if the response error shape changes (it should not; keep the Response types).

## Alternatives considered
- **Hide the endpoint behind a feature flag until the storage path is wired** — rejected because the endpoint is already announced in the admin UI and in Epic 66's story 66.3; hiding it now would break the admin flow and leave the data-loss class latent for the next admin who re-enables the flag.
- **Persist the bytes to a local tmpfile and defer upload to a worker** — rejected because the api-server does not have a reliable local-disk invariant (containerised, no persistent volume at the import path) and because the existing storage_service already provides the same semantics with less code; a tmpfile hand-off adds a second failure mode without buying anything.

## Root-cause trace
1. Symptom: `aws s3 ls s3://<bucket>/imports/<org>/` is empty after a successful-looking upload response; `validate_import` reports 0 records.
2. ← Immediate cause at `backend/servers/api-server/src/routes/migration.rs:757`: the handler constructs a fake `file_path` and never calls any storage writer.
3. ← Upstream cause at `backend/servers/api-server/src/routes/migration.rs:693, :720`: the multipart bytes are collected into `_file_data`, whose leading-underscore name silences the dead-store warning that would otherwise have made this stub visible to `cargo clippy`.
4. Origin: `upload_import_file` landed as a stub (likely under the "mock setup" convention also visible at `migration.rs:1268` on the export flow) during Epic 66's initial scaffolding; it was never wired to the storage service in a follow-up because no test asserted end-to-end object persistence.

## Test plan
- [ ] New integration test `backend/servers/api-server/tests/migration_import_upload_persists_file.rs` (or similar) that stands up a mock storage service, posts a known CSV blob to `/api/v1/migration/import/upload`, asserts the mock received exactly those bytes at the returned `file_path`, and asserts a subsequent `GET /api/v1/migration/import/jobs/{job_id}` returns the same storage key.
- [ ] Unit test on the handler asserting a `503 SERVICE_UNAVAILABLE` when `state.storage_service.is_none()` and that NO import_job row is created in that case.
- [ ] Run: `cd backend && cargo test -p api-server migration_import_upload` (new tests) and `cargo test -p api-server --test migration` (regression on the rest of the module).

## Out of scope
- Rewriting the `validate_import` / `approve_import` flow — only the upload persistence is in scope.
- Adding resumable / chunked uploads or background offloading — the current single-request contract is kept.
- Changing the OpenAPI response shape — only the error branch for missing storage_service is added, as an existing 5xx error.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-migration-upload-drops-file.md`
- Mark the matching `backlog.json` row as `status: "done"`
