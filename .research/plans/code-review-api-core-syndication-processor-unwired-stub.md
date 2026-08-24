# code-review-api-core-syndication-processor-unwired-stub

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review [api-core] (2026-08-23 tier1d run)
**Confidence:** high

## Hypothesis
`SyndicationService::process_syndication_job()` in `backend/servers/api-server/src/services/syndication.rs` is a simulation stub — the PUBLISH arm mints a fake external id via `format!("{}_{}", payload.portal, uuid::Uuid::new_v4())` and marks the listing `syndication_status = SYNCED` without ever making an HTTP call to the external real-estate portal. Worse, the function has **zero callers in the backend** — jobs are enqueued into `SYNDICATION_QUEUE` from `routes/listings.rs:540,655,661` but the only background worker (`push_fanout.rs:1334 process_pending_jobs`) drains a different queue, so enqueued syndication jobs never dispatch. Net effect: when a manager marks a listing for portal syndication, the DB row is persisted, the UI shows "queued", but the listing is silently never published. Two shipping-blocker gaps are visible; the smallest correct fix hides the customer-exposed enqueue path behind a feature flag and files a follow-up epic so a partial wiring PR doesn't ship a fake success path.

## Evidence
- `backend/servers/api-server/src/services/syndication.rs:15` — `pub const SYNDICATION_QUEUE: &str = "syndication";`
- `backend/servers/api-server/src/services/syndication.rs:289-406` — `process_syndication_job` body. Line 310 comment `// Simulate publishing to portal`; line 316-317 mints a mock external id `format!("{}_{}", payload.portal, uuid::Uuid::new_v4())`; lines 322-330 mark listing `syndication_status = SYNCED` without ever calling a portal client. `grep -rn 'reqwest\|hyper::Client\|Http.*Client' backend/servers/api-server/src/services/syndication.rs` returns zero hits.
- `backend/servers/api-server/src/services/syndication.rs` — `grep -rn 'process_syndication_job' backend --include='*.rs'` returns ONE hit: the definition at line 289. No worker calls this function.
- `backend/servers/api-server/src/services/syndication.rs:131,207,259` — `create_publish_jobs` / `create_status_change_jobs` / `create_update_job` all enqueue into `SYNDICATION_QUEUE`. Called from `backend/servers/api-server/src/routes/listings.rs:1090` (comment: `// queues external-portal syndication jobs — Epic 105`).
- `backend/servers/api-server/src/services/push_fanout.rs:1334` — `process_pending_jobs` is the only worker loop but it drains a different queue (push, not syndication). No syndication worker exists.

## Files
- `backend/servers/api-server/src/services/syndication.rs`
- `backend/servers/api-server/src/routes/listings.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. In a running dev stack, POST to the listings syndication endpoint (see `routes/listings.rs:540`) to enqueue a publish for a test listing on a known-good portal.
2. Observe the response is 202/200 and a row appears in the syndication jobs table.
3. Wait — no external HTTP request goes out (verified by `SELECT COUNT(*) FROM outbound_http_log WHERE ...` or by watching upstream portal). The listing's `syndication_status` in the DB does NOT flip to `SYNCED` (because no worker calls `process_syndication_job`).

Expected after fix: either (a) the enqueue endpoint returns `501 Not Implemented` (or is gated behind a feature flag off in prod) so callers know the feature isn't live, or (b) a real worker + real HTTP client publishes to the portal and the DB reflects reality.

## Suggested approach
1. Add a feature flag `syndication_enabled` (default `false`) in the api-server config (`backend/servers/api-server/src/config.rs` or the crate's canonical config module — grep for an existing flag pattern before inventing a new one; mirror it).
2. In `routes/listings.rs:540,655,661`, wrap the three enqueue call sites behind `if !config.syndication_enabled { return Err(Error::not_implemented("syndication not yet available in this environment")); }` — this stops customer exposure without deleting the plumbing.
3. Add a doc comment on `process_syndication_job` explicitly stating it is an unwired stub and referencing an issue for the wiring work (open a follow-up issue in the same PR body).
4. Add a Rust test in `backend/servers/api-server/src/services/syndication_tests.rs` (or the existing sibling test module) that asserts: when the config flag is off, enqueueing publish/status-change/update endpoints all return `NotImplemented` and no row appears in the syndication jobs table.
5. Run `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-server syndication` locally to confirm.
6. Do NOT wire the actual portal HTTP client in this PR — that requires portal-specific credential handling, per-portal request shape, retry semantics, dead-letter, and is a multi-story epic (Epic 105.x).

## Alternatives considered
- **Wire the real portal client in this PR** — rejected because (a) it needs at least one real portal integration decision (Booking / Airbnb / Sreality / other), (b) requires new secrets handling and per-portal request/response schemas, (c) the shape of the retry + dead-letter pipeline needs its own design pass. Shipping the customer-exposed enqueue path as-is is the more urgent risk.
- **Delete `SyndicationService` and the routes** — rejected because it destroys the scaffold. The enqueue pattern, the queue constant, the job payload shape, and the tests we add here become the substrate for the real wiring PR.

## Root-cause trace
1. Symptom: managers use the "publish to portal" UI, get a success response, but the listing is never published upstream and `syndication_status` never flips.
2. ← `backend/servers/api-server/src/services/syndication.rs:289 process_syndication_job` is a stub that fakes external ids and skips the HTTP call.
3. ← `backend/servers/api-server/src/services/syndication.rs:289` has zero callers — the queue is enqueued from routes but no worker drains it. `push_fanout.rs:1334` drains a different queue.
4. Origin: the scaffolding commit for Epic 105 landed the enqueue path + a stub processor but the worker + real HTTP client were left "for a follow-up". Neither has landed since.

## Test plan
- [ ] New Rust test: `backend/servers/api-server/src/services/syndication_tests.rs::test_enqueue_returns_not_implemented_when_flag_off` — asserts the three enqueue endpoints return `NotImplemented` when `syndication_enabled=false`. Would fail on `dev` today because they return 200 and persist a row.
- [ ] Existing tests: any tests that call `create_publish_jobs` / `create_status_change_jobs` / `create_update_job` need to set `syndication_enabled=true` in the test config, otherwise they'll start failing under the new gate. Grep the workspace and update them.
- [ ] Command: `cargo test -p api-server syndication` (or the crate the tests actually live in — verify locally).

## Out of scope
- Actual portal HTTP client (per-portal request shape, credential handling, response parsing) — separate Epic 105.x work.
- The syndication-queue worker loop — landed together with the real client, not in this hide-the-stub PR.
- Any UI change in ppt-web (the manager UI can keep showing the "publish" affordance; the backend's `NotImplemented` response is what the UI treats as "portal integration not available in this environment").

## After-merge
- Move this file to `plans/_archive/code-review-api-core-syndication-processor-unwired-stub.md`
- Mark the matching `backlog.json` row as `status: "done"`
- Open a follow-up issue: "Epic 105 — wire syndication worker + per-portal HTTP client + retry pipeline" (or attach to existing Epic 105 tracker if one exists).
