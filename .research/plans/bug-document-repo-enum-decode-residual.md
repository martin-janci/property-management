# bug-document-repo-enum-decode-residual

**Vector:** bug
**Score:** 3
**Source:** PR #1565 (commit bb617665) + hotspot `backend/crates/db/src/repositories/document.rs`
**Confidence:** medium

## Hypothesis
PR #1565 fixed the `category::text` / `access_scope::text` enum-decode bug in
`find_by_id_with_details_rls` after `test_upload_metadata_roundtrips_through_get_handler`
went red on dev, but the same `SELECT d.*` projection survives in
`get_current_version_rls` at `document.rs:1976`. Both queries hydrate the same
`Document` struct, where `category` / `access_scope` are typed `String` but the
columns are the PG enums `document_category` / `document_access_scope`. Any call
to `get_current_version_rls` for a document whose `category != 'general'` (or any
non-default `access_scope`) will hit the same #1008 `ColumnDecode` failure the
PR was supposed to close. Mirror the fix from `find_by_id_with_details_rls` —
explicit per-column projection with `::text AS …` aliases — and add a regression
test that parallels `test_upload_metadata_roundtrips_through_get_handler` for
the version-history read path.

## Evidence
- `backend/crates/db/src/repositories/document.rs:1976` — `SELECT d.* FROM documents d` still in tree on `dev` (verified 2026-06-18 against `origin/dev` HEAD `82da44a`).
- PR #1565 (commit bb61766513174deb655752031f2549851a660a03) body: "[`d.*`] returns `category` (document_category) and `access_scope` (document_access_scope) as raw PG enums; reading them into the `String` struct fields via Row::get fails ColumnDecode (#1008)."
- `backend/crates/db/src/repositories/document.rs:608-609` (the fix landed in #1565): `d.category::text AS category_text, d.access_scope::text AS access_scope_text` — the canonical projection to mirror.
- `backend/crates/db/src/models/document.rs` (search for `pub struct Document`) — `category: String` and `access_scope: String` fields confirm the decode mismatch.
- Document hotspot: 8 changes in the 2026-06-16/18 window (#1551, #1565) — file is hot and the projection drift is the recurring symptom.

## Files
- `backend/crates/db/src/repositories/document.rs:1976`
- `backend/crates/db/src/repositories/document.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception

**Mode: cloud-ok** — pure backend Rust + sqlx; no UI flow, no device. Runs through `ppt-bridge` MCP for `cargo test -p db`.

## Repro steps
1. From `origin/dev` (HEAD `82da44a`), open a SQL shell on the `pm` DB and insert a document with `category = 'invoice'` (non-default enum value) and a current version:
   - `INSERT INTO documents (id, organization_id, building_id, title, file_key, file_name, mime_type, size_bytes, category, access_scope, uploaded_by, created_by, is_current_version) VALUES (…, …, …, 'Test', 'k', 'f.pdf', 'application/pdf', 1, 'invoice', 'org', <uid>, <uid>, true);`
2. Call `DocumentRepository::get_current_version_rls(executor, <doc_id>)` from a test that uses an `RlsConnection`.
3. Expected: returns `Some(Document { category: "invoice", access_scope: "org", … })`.
   Actual: returns `Err(SqlxError::ColumnDecode { … })` — the row decode fails on the enum-as-String boundary, the same shape `test_upload_metadata_roundtrips_through_get_handler` hit on `find_by_id_with_details_rls` prior to PR #1565.

## Suggested approach
1. Read `backend/crates/db/src/repositories/document.rs:1961-1984` (the `get_current_version_rls` body) to confirm the `SELECT d.*` is the only enum-exposing projection in that function.
2. Replace the `SELECT d.*` with an explicit column list mirroring `find_by_id_with_details_rls` lines 596-625 — every non-enum column read as-is, plus `d.category::text AS category_text, d.access_scope::text AS access_scope_text` aliases. Verify the `Document` struct's `#[sqlx(rename = "category_text")]` / `access_scope_text` attributes match — if PR #1565 added them via `#[sqlx(rename_all)]` or explicit renames, the new projection must use the same aliases.
3. Grep `backend/crates/db/src/repositories/document.rs` for any other `SELECT d\.\* FROM documents` or `RETURNING \*` patterns on the `documents` table that hydrate `Document` directly; the seven `RETURNING *` patterns at lines 71/242/974/1629/1828/2033/2766 need the same audit. Either replace with explicit projection or confirm they're hydrating a struct whose fields are all non-enum (e.g. a typed `DocumentId` return).
4. Add a regression test under `backend/crates/db/tests/document_*_tests.rs` (mirror the file pattern of `document_upload_tests`): `get_current_version_returns_for_invoice_category` — set up a doc with `category = 'invoice'`, call `get_current_version_rls`, assert the result decodes into `Document` with `category == "invoice"`.
5. Add a parallel `RETURNING *` regression test for whichever of the seven write-path queries was found to leak enum decoding in step 3 — at least one (the one hydrating `Document`, not a typed return).
6. Run `cargo test -p db -- document` (or the bridge equivalent) against the dev DB; confirm both the new test and `test_upload_metadata_roundtrips_through_get_handler` still pass.
7. Open a `fix(db)` PR titled `fix(db): cast enum projection on get_current_version_rls + audit remaining document.rs * patterns (post-#1565)`. Link this plan in the body. Include both new regression tests in the diff (IG3).

## Alternatives considered
- **Introduce a typed `sqlx::Type` wrapper** for `category` / `access_scope` (newtype around the enum) so `Document` decodes natively without `::text` aliases — rejected because it ripples into every consumer (handlers, serde, OpenAPI codegen) and the team has consistently chosen the `::text AS …` projection pattern (PRs #1486/#1492/#1537/#1565). Stay with the established convention.
- **Replace the `SELECT d.*` join with a `find_by_id_with_details_rls` call followed by a separate version walk** — rejected because the join is the whole point of the query (resolve the root document then pick the current version in a single SQL round-trip). Splitting would regress latency on a hot path.

## Root-cause trace
1. Symptom: A future caller of `get_current_version_rls` for a non-default-category document gets `Err(SqlxError::ColumnDecode { index, source })` with `source = "invalid input value for enum String: …"` (the #1008 surface).
2. ← `document.rs:1976` — `SELECT d.* FROM documents d` projects `category` / `access_scope` as raw PG enums, but the `query_as::<_, Document>` decoder reads them via `Row::get::<String, _>` on String-typed struct fields.
3. ← `models/document.rs` (pub struct Document) — `category: String` / `access_scope: String` were chosen so handlers can serialize them without enum-string-conversion glue, which made the projection-side cast the only safe path (the same architectural decision PR #1565 paper-trailed).
4. Origin: latent since the original `documents.category` migration moved from `VARCHAR` to the `document_category` enum (search migrations for `ALTER TYPE document_category` / `ALTER TABLE documents ALTER COLUMN category TYPE`). Recently surfaced by PR #1454/#1469's document-listing work that started exercising non-default categories.

## Test plan
- [ ] New: `backend/crates/db/tests/document_get_current_version_enum_decode_tests.rs::get_current_version_returns_for_invoice_category` — fails on `origin/dev` HEAD `82da44a` before the fix (IG3), passes after.
- [ ] Regression scenario: the existing `test_upload_metadata_roundtrips_through_get_handler` still passes (PR #1565's guarantee must not regress).
- [ ] Run: `cargo test -p db -- document` (via `ppt-bridge` MCP `cargo_test_remote` or `stack up pm-local` locally).

## Out of scope
- Refactoring `Document` to a typed enum representation (deferred — see Alternatives 1).
- Reworking the seven `RETURNING *` write paths beyond the one identified in step 3 — those get audited but only the leaking ones are fixed in this PR (others stay if they don't hydrate `Document`).
- Touching `models/document.rs` field types or any downstream serialization.

## After-merge
- Move this file to `plans/_archive/bug-document-repo-enum-decode-residual.md`
- Mark `bug-document-repo-enum-decode-residual` in `backlog.json` as `status: "done"` with `resolution: "PR #<N> — …"`.
