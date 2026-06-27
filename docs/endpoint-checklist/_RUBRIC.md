<!-- Internal rubric for the BIT-247 endpoint audit. Defines the status taxonomy and method used to classify every endpoint. -->
# Endpoint Audit Rubric (BIT-247)

Authoritative method for classifying every `api-server` and `reality-server` endpoint.

## Status taxonomy

| Status | Definition |
|--------|------------|
| `done` | Handler is implemented **and** is exercised by a passing test in `backend/servers/<server>/tests/*.rs` (matched by route path or handler fn name). |
| `partial` | Handler is implemented (real DB/business logic) but has **no** dedicated test, or only an indirect/smoke test that does not assert behaviour. |
| `stub` | Handler exists but returns a placeholder: `todo!()`, `unimplemented!()`, `StatusCode::NOT_IMPLEMENTED` (501), hardcoded/empty response, `// TODO`, or the router is **unmounted** (commented out / ROADMAP) in `lib.rs`/`main.rs`. |
| `missing` | Named in the OpenAPI/TypeSpec spec (`docs/api/`) or a use-case but **no handler** exists. |

> `done` REQUIRES a real passing test. Do not mark `done` on the strength of implementation alone — that is `partial`.

## Path derivation

Full path = mount prefix (from `backend/servers/api-server/src/lib.rs` or `src/main.rs`) + the `.route("…")` path inside the router. The prefix is on the `.nest("/api/v1/…", routes::<module>::router())` line — note some span two lines (prefix on the line above `routes::<module>::`). Some modules are **unmounted** (commented out / marked `ROADMAP`) → those endpoints are `stub`. Sub-routers merged inside a module's `mod.rs` inherit the parent prefix.

## Per-group file format

Each group writes `docs/endpoint-checklist/<NN>-<slug>.md` with:

```
# <Group> endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `GET /api/v1/faults` | `faults.rs:list_faults` | done | `faults_tests.rs` | |
...

## Tally
done: N  partial: N  stub: N  missing: N  total: N
```

Keep Notes terse (≤8 words). Group is the domain bucket, not the file.
