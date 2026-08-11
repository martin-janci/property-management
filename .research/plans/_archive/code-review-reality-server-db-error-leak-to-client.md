# code-review-reality-server-db-error-leak-to-client

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review reality-server 2026-08-01 (backlog id `code-review-reality-server-db-error-leak-to-client`)
**Confidence:** high

## Hypothesis
reality-server is internet-facing. `backend/servers/reality-server/src/util/errors.rs::db_error` exists specifically to prevent internal-detail leaks (its doc-comment says so verbatim: "Raw sqlx::Error / anyhow::Error strings can leak column names, constraint names, sometimes pool / TLS state — none of which a portal client should ever see"), but ~23 `map_err` sites across `routes/{listings,realtors,layout,health}.rs` still return `format!("Failed to X: {}", e)` bodies straight to unauthenticated clients. Every failed public search, realtor lookup, layout revalidate, or health probe currently leaks the raw `sqlx::Error` `Display` — column and constraint names, sometimes pool/TLS state. The smallest safe change is a mechanical rewrite: replace each offending `map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("...", e)))` with `.map_err(|e| util::errors::db_error("<op>", e))` (or the equivalent `?` form), covering all four route files at once, and add a `grep`-based CI check so the pattern cannot regress.

## Evidence
- `backend/servers/reality-server/src/util/errors.rs:1-32` — `db_error(ctx, e)` writes the full error to `tracing::error!` and returns a generic `(500, "Internal server error")`. Doc-comment explicitly names the class of leak the helper prevents.
- `backend/servers/reality-server/src/routes/listings.rs:286-291`, `:298-303`, `:462-472` (approx) — public `search_listings` / `count_listings` / `suggestions` all use `format!("Failed to search listings: {}", e)` / `format!("Failed to count listings: {}", e)` shape; these are the unauthenticated public search endpoints, so the leak is reachable pre-auth.
- `backend/servers/reality-server/src/routes/realtors.rs:72-77` (my_profile), plus additional sites around lines 110, 154, 193, 240, 273, 313 identified in the original review — every DB `map_err` in the file follows the same `format!` pattern.
- `backend/servers/reality-server/src/routes/layout.rs:61,67,87,103` — layout revalidate paths use the same shape; `backend/servers/reality-server/src/routes/health.rs:151` leaks on the health probe error path.
- Contrast: `routes::listings::get_listing` already has an inline `db_error`-shaped helper (per the util::errors.rs doc-comment), i.e. one call site in the same file already knows to hide the raw error — the fix is to hoist all sites to the shared helper.

## Files
- `backend/servers/reality-server/src/routes/listings.rs`
- `backend/servers/reality-server/src/routes/realtors.rs`
- `backend/servers/reality-server/src/routes/layout.rs`
- `backend/servers/reality-server/src/util/errors.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (need to enumerate all leak sites, then verify each rewrite compiles + preserves the tracing signal)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Point reality-server at a Postgres instance where the `reality_listings` table is missing a column expected by `search_listings` (or simulate by breaking the SQL — e.g. dropping the column temporarily on a dev DB).
2. Call `GET /api/v1/listings?limit=1` (or any public search endpoint that hits `portal_repo.search_listings`).
3. Expected: `500 Internal server error` (generic body), full error captured in the server's `tracing::error!` output only.
4. Actual (today): `500` with body `Failed to search listings: error returned from database: column "…" does not exist` — sqlx's raw `Display` surface leaked directly to the portal client, exposing schema shape to any anonymous internet caller.

## Suggested approach
1. Run `rg -n 'format!\("[^"]*: \{\}", e' backend/servers/reality-server/src/routes/` to enumerate every offending site. Save the output as the PR's "sites-touched" checklist.
2. For each site, replace:
   ```rust
   .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to X: {}", e)))
   ```
   with:
   ```rust
   .map_err(|e| util::errors::db_error("X", e))
   ```
   `util::errors::db_error` already routes to `tracing::error!(context = ctx, error = %e, …)` and returns `(500, "Internal server error")`, so ops observability is preserved.
3. Ensure every route file that gains a call has `use crate::util::errors::db_error;` (or `use crate::util` + fully-qualified call) — pick whichever the file's existing import style prefers.
4. Author the regression test described in *Test plan* (public endpoint returning generic body on DB failure).
5. Add a grep-based lint step in CI so the pattern cannot regress: e.g. a `.github/workflows/backend.yml` step that fails if `rg 'format!\("[^"]*: \{\}", *e' backend/servers/reality-server/src/routes/` returns matches. Ship the CI change in the same PR as the code change so the guard lands together.
6. Local verify: `cargo test -p reality-server`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`.
7. Confirm `curl -i http://127.0.0.1:8081/api/v1/listings?limit=1` returns a generic body on a broken DB.

## Alternatives considered
- **Custom `IntoResponse` wrapper type for reality-server** — rejected because it would require touching every handler signature (broader diff) and the leak class is solved just as well by the existing `db_error` helper. A wrapper is the right long-term refactor but is not the minimum-diff security fix.
- **Silence at the tracing layer only** — rejected because the leak is in the HTTP response body, not the log. Redacting logs would remove the ops signal without fixing the client-side exposure.

## Root-cause trace
1. Symptom: unauthenticated `GET /api/v1/listings` returns a 500 whose body contains a sqlx column-not-found error mentioning DB column names — schema leaked to internet clients.
2. ← `backend/servers/reality-server/src/routes/listings.rs:286-291` — `format!("Failed to search listings: {}", e)` interpolates `e: sqlx::Error` directly into the response body.
3. ← the pattern predates `util::errors::db_error`; the helper was introduced (see `errors.rs:11` note about the inline helper in `get_listing`) but the mechanical rewrite of the ~23 pre-existing sites was never done.
4. Origin: the initial reality-server public routes landed with copy-pasted `format!("Failed to X: {}", e)` boilerplate; `util::errors::db_error` was added later but only one call site was migrated at the time.

## Test plan
- [ ] `backend/servers/reality-server/tests/public_endpoints_error_body_tests.rs` — new test `listings_search_returns_generic_body_on_db_error`: start reality-server against an intentionally-broken DB (or mock the repo to return `sqlx::Error::RowNotFound` / `sqlx::Error::Protocol("column \"secret\" does not exist".into())`), call `GET /api/v1/listings?limit=1`, assert body == `"Internal server error"` (or empty JSON), assert no substring `"Failed to"` or `"column"` in the response body, and assert the tracing::error! log line was emitted with the raw error under a filtered subscriber.
- [ ] Parallel regression tests for one representative site per file: `realtors_get_my_profile_returns_generic_body_on_db_error`, `layout_revalidate_returns_generic_body_on_db_error`.
- [ ] Grep-based CI regression: add a `just` recipe / workflow step `just check-no-raw-db-leaks` that runs `! rg -q 'format!\("[^"]*: \{\}", *e' backend/servers/reality-server/src/routes/` (exits non-zero on match).
- [ ] Command: `cargo test -p reality-server --tests` covers the new tests.

## Out of scope
- Rewriting `db_error` to accept structured context / spans — the helper's current shape is enough for this fix.
- api-server's error handling — that server is behind auth and uses a different error stack; treat as a separate audit.
- Non-DB `map_err` sites (e.g. HTTP client, S3) — same category of leak conceptually but different helper needed; separate signal.
- Localisation of the client-facing error string — kept as the current English `"Internal server error"` for this PR.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-db-error-leak-to-client.md`
- Mark the matching `backlog.json` row as `status: "done"`
