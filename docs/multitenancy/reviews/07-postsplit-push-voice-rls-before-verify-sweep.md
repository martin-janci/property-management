# Review #07 — Post-split `push/` + `voice_webhooks/` RLS-before-verify sweep

Date: 2026-11-28
Reviewer: pm-scrum-master (automated implementer sweep)
Task: `pm-scrum-master-sweep-postsplit-modules-rls-before-verify-2026-11-21-retry1`

## Scope & motivation

Two large api-server modules were recently split into surface-scoped child
modules:

- `services/push_fanout.rs` (~3.0k LOC) → `services/push/{mod,apns,fcm,combined,config,worker}.rs`
- `routes/voice_webhooks.rs` (~3.1k LOC) → `routes/voice_webhooks/{mod,handlers,oauth,verify,signature,parsers}.rs`

Both splits advertise themselves as behaviour-preserving. This sweep checks the
**one** property most likely to be quietly dropped by a mechanical file split:
the *RLS-before-verify ordering* — i.e. no handler may run a tenant-scoped DB
query or mutation before (a) the request is authenticated / authorized and
(b) the tenant RLS context is established on the connection that performs the
access.

## Result: PASS — no ordering bug survived the split

Every DB-touching call site in both modules establishes authorization and the
correct connection context before the tenant-scoped access. No fix was required;
no behavioural change is made by this review (documentation only).

## Call-site ledger

### `routes/voice_webhooks/handlers.rs` — Alexa & Google receivers

Both platform webhooks are **intentionally not** gated by an auth/RLS extractor:
a legitimate voice delivery carries no Property-Management bearer token, so an
extractor would 401 every real request before the provider signature could be
checked (the original bug class, fixed pre-split in #2658/#2668 and carried over
verbatim). Ordering per handler:

1. Verify the provider's own request signature **first**
   (`verify_alexa_signature` / `verify_google_request`) — fail-closed on error.
2. Parse the body; if account-linking is incomplete, return the link card with
   **no** DB access.
3. Resolve the linked device via `lookup_voice_device` on an explicitly
   **context-cleared public** connection (`acquire_public`, handlers.rs:125).
   `voice_assistant_devices` is deliberately not RLS-bound (migration 00226), so
   this lookup is authorization *input*, not a tenant-scoped read.
4. `run_voice_command` (handlers.rs:56) acquires the RLS context with the
   **device owner's** org/user (`acquire_with_rls(device.organization_id,
   device.user_id, …)`, handlers.rs:63-64) **before** `process_command` reads any
   tenant-scoped table (faults, announcements, meters), and releases the guard on
   **every** path (the release-on-every-path fix from #3760, preserved).

Verdict: tenant RLS established before tenant-scoped access on both receivers. ✓

### `routes/voice_webhooks/oauth.rs` — token exchange / refresh

- `oauth_token_exchange` (oauth.rs:157): gated by the `TenantAuthUser` extractor
  (JWT + tenant membership validated before the handler body). `org_id`/`user_id`
  come from the verified token; the only DB access is
  `upsert_active_voice_device`, performed **after** `acquire_with_rls(org_id,
  user_id, …)` (oauth.rs:318-319). ✓
- `oauth_token_refresh` (oauth.rs:425): gated by the `AuthUser` extractor, then
  per-user rate-limited. The device lookup (`find_voice_device`, oauth.rs:464)
  runs on a **public** connection — again the not-RLS-bound device table — purely
  to load the resource for the **ownership authorization check**
  (`auth.user_id != device.user_id && !is_platform_admin()`, oauth.rs:487). The
  mutation (`update_voice_device_tokens`, oauth.rs:617) happens only **after**
  both the ownership check **and** `acquire_with_rls(device.organization_id,
  device.user_id, …)` (oauth.rs:603). This is the canonical
  load-resource → authorize → mutate order, not a before-verify leak. ✓

### `routes/voice_webhooks/{verify,signature,parsers}.rs`

No database access. `verify.rs` is a developer-facing signature-introspection
endpoint (fails closed for Alexa — forgery-vector fix carried over). `signature.rs`
performs HTTP certificate-chain fetches (not DB). `parsers.rs` is pure
request/response mapping. Nothing to order. ✓

### `services/push/*` — background fanout worker

Not request handlers and therefore outside the RLS-before-verify request model:
`PushFanoutWorker` drains a Redis queue and delivers via `CombinedPushAdapter`
on the **service-role** pool. The DB touches are device-token reads scoped by an
explicit `WHERE user_id = $1` (`get_tokens_for_user`, combined.rs:173) and a
targeted stale-token delete scoped by `WHERE user_id = $1 AND token = $2`
(`delete_stale_token_by_string`, mod.rs:204) that cannot evict another user's
tokens. There is no tenant RLS context nor caller-authorization concept to order
here — the service-role fanout pattern is intended. ✓

## Evidence trail

Relevant prior fixes, all merged before the split and preserved by it:

- #3760 — release RLS connection on every voice-webhook path.
- #3815 — dedupe device-lookup + processor build in handlers.rs (behaviour-preserving).
- #2658 / #2668 — webhook signature verification hardening (broken-auth / Alexa forgery).
- #890 / #765 / #2769 / #2794 — simulated-token fail-closed, token-encryption
  fail-closed, refresh/exchange throttling, atomic active-device upsert.

## Conclusion

No RLS-before-verify ordering regression survived either module split. The
deliverable for this sweep is this audit note; no code change and no regression
test were warranted (per the task's "already correct → audit note" branch).
