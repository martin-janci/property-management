# Endpoint Audit — Classification Spec (BIT-247)

You are auditing backend HTTP endpoints in the property-management repo. Read-only.
Worktree root: `/home/dev/bit247-audit` (detached at `origin/dev`). Do NOT modify route/test source; only write your assigned group file.

## How to find endpoints
- Each route module builds an axum `Router`. Endpoints are declared with `.route("<path>", method(handler))` (methods: get/post/put/patch/delete). A single `.route` line may chain multiple methods via `.get(h).post(h)` or repeated `.route` with same path.
- The **mount prefix** for each module is in `backend/servers/api-server/src/lib.rs` (and a few in `backend/servers/api-server/src/main.rs`: `tenant_config` → `/tenant-config`, `caddy_ask` → `/internal/caddy-ask`). For reality-server, prefixes are in `backend/servers/reality-server/src/main.rs`.
- **Full path = mount_prefix + route_path** (collapse the `/` between, axum `{id}` style params are fine). Grep lib.rs/main.rs for `routes::<module>::` to get the exact prefix. Some modules expose multiple routers (e.g. `ai::ai_chat_router` at `/api/v1/ai/chat`); resolve each.

## Status taxonomy (pick exactly one per endpoint)
- `done` — real handler (does real work: queries repo/db, returns typed data) AND covered by at least one passing test that exercises its full path.
- `partial` — real handler but NO test hits its path (or only a weak/authz-only test that never exercises the success path).
- `stub` — handler exists but is not real: body contains `todo!()`, `unimplemented!()`, returns `StatusCode::NOT_IMPLEMENTED`/501, returns hardcoded/empty/mock data with a TODO, or the module/file is marked `ROADMAP`/`stub`/"not implemented"/unmounted.
- `missing` — referenced in spec/use-cases but no handler. (Usually rare at file level; note if a documented endpoint is absent.)

## Test coverage check
- Tests live in `backend/servers/<server>/tests/*.rs`. They hit endpoints by URI string, e.g. `.uri("/api/v1/faults/statistics")` or `json_request(Method::POST, "/api/v1/faults", ...)`.
- For each endpoint, grep the tests dir for its path (try the static prefix, e.g. `/api/v1/faults`, then narrow). Record which test file(s) cover it. A path with `{id}` → grep the static portion before the param.
- Be honest: an authz/IDOR test that only asserts 401/403 does NOT prove the success path → that endpoint is `partial` unless a happy-path test also exists. Note this in the Notes column.

## Output: write ONE markdown file at the path I give you
Format:
```
# <Domain Title>

_Server: <api-server|reality-server>. Modules: <list>._

## <module>.rs  (mount: <prefix>)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/faults | list_faults | done | faults_tests.rs | |
...

## Summary
- done: N | partial: N | stub: N | missing: N | total: N
```
One `##` section per module. Count every endpoint. End with the Summary block.

## Required: return THIS compact block as your final message (for aggregation)
```
GROUP: <group-name>
FILE: docs/endpoint-checklist/groups/<file>.md
done=<n> partial=<n> stub=<n> missing=<n> total=<n>
STUBS: <comma-list of stub/missing full paths, or "none">
NOTES: <one line of anything notable — drift, unmounted routers, duplicate mounts>
```
Be precise and deterministic. Do not invent endpoints or tests. If unsure, classify conservatively (prefer `partial` over `done`).
