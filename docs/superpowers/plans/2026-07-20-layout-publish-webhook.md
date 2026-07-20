# Layout Publish Webhook Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On any layout mutation that changes resolved output (publish, rollback, kill, unkill), api-server fires a signed webhook; reality-web verifies it and `revalidateTag`s the affected layout tags — so ISR pages refresh within seconds of a publish instead of waiting out `revalidate: 60` (spec §7 delivery row; deferred from the defensive-rendering plan).

**Architecture:** (1) api-server gains a tiny outbound notifier (`routes/layout/webhook.rs`): env-configured (`LAYOUT_WEBHOOK_URL` + `LAYOUT_WEBHOOK_SECRET`; unset → debug-log no-op), HMAC-SHA256-signed body per the repo's existing `X-Webhook-Signature` convention, fired fire-and-forget via `tokio::spawn` + reqwest (5 s timeout) from the publish/rollback/kill/unkill handlers AFTER their DB success — never affecting the response. (2) reality-web gains `POST /api/layout-revalidate` (route handler): constant-time HMAC verification against its own `LAYOUT_WEBHOOK_SECRET` (unset → 503 disabled), maps the screen to tags, calls `revalidateTag`. (3) reality-web's layout fetch always includes the GLOBAL tag (`layout:listing-detail`) alongside the host-scoped one, so one webhook invalidates all hosts.

**Tech Stack:** Rust (hmac/sha2 workspace deps, reqwest, tokio), Next.js route handler + Node `crypto.timingSafeEqual`, Vitest.

## Global Constraints

- **Branch:** `feature/layout-publish-webhook` from `dev` (after #2430 merges; if pending, branch from `feature/layout-preview-bridge` and rebase later — note which).
- **Signature scheme:** MATCH the repo's existing inbound convention — read `require_portal_webhook_verification` in `backend/servers/api-server/src/routes/portal_webhooks.rs` + its helper in `state.rs` for the exact header (`X-Webhook-Signature`) and encoding (hex vs `sha256=` prefix) — the outbound signer and the reality-web verifier must both use exactly that format (ADAPT: report the exact format found).
- **Fire-and-forget:** the notifier NEVER changes handler outcomes. `tokio::spawn`ed; errors logged `tracing::warn!`, success `tracing::debug!`. Payload: `{"screen": "<screen>", "event": "publish"|"rollback"|"kill"|"unkill"}`.
- **Env names:** api-server `LAYOUT_WEBHOOK_URL`, `LAYOUT_WEBHOOK_SECRET`; reality-web `LAYOUT_WEBHOOK_SECRET` (same secret value operationally). All unset-safe: outbound no-ops; inbound returns 503 `{error:'disabled'}`.
- **Tag mapping** (reality-web, single source of truth fn): screen `reality/listing-detail` → `['layout:listing-detail']`; generic rule `layout:<part after '/'>`. The layout fetch in `src/lib/layout.ts` changes its tags to ALWAYS include the global tag: host present → `[host:…:layout:listing-detail, layout:listing-detail]`, else `[layout:listing-detail]` (update its existing tests accordingly).
- Only reality screens matter to ISR, but the notifier fires for ALL screens (cheap; the receiver revalidates harmless unused tags for ppt screens — keep it simple).
- Gates: backend `cargo fmt --all && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings` (fmt EXPLICITLY — a missing fmt broke CI on the previous branch) + RLS script; frontend `pnpm -F @ppt/reality-web test` + `pnpm check && pnpm typecheck`. Known pre-existing failures untouched.
- Commit scopes: `feat(api-server)`, `feat(reality-web)`, `docs(...)`. ADAPT rule as before.

## File Structure

```
backend/servers/api-server/src/routes/layout/webhook.rs   # sign_payload + notify_layout_change
backend/servers/api-server/src/routes/layout/mod.rs       # pub mod webhook;
backend/servers/api-server/src/routes/layout/admin.rs     # notify calls in 4 handlers
frontend/apps/reality-web/src/app/api/layout-revalidate/route.ts
frontend/apps/reality-web/src/app/api/layout-revalidate/route.test.ts
frontend/apps/reality-web/src/lib/layout.ts               # global tag always included
frontend/apps/reality-web/src/lib/layout.test.ts          # tag assertions updated
docs/repo-map.md
```

---

### Task 1: api-server outbound notifier

**Files:**
- Create: `backend/servers/api-server/src/routes/layout/webhook.rs`
- Modify: `backend/servers/api-server/src/routes/layout/mod.rs` (`pub mod webhook;`)
- Modify: `backend/servers/api-server/src/routes/layout/admin.rs` (after each successful DB mutation in `publish`, `rollback`, `kill`, `unkill`: `webhook::notify_layout_change(&req.screen, "<event>");`)
- Modify (if needed): `backend/servers/api-server/Cargo.toml` — `hmac`/`sha2` from workspace deps if not already present (check; `sha2` is in `common`'s deps, api-server may need its own entries).

**Interfaces:**
- Produces: `pub fn sign_payload(secret: &str, body: &[u8]) -> String` (pure — exact format per the repo convention, ADAPT) and `pub fn notify_layout_change(screen: &str, event: &'static str)` — reads env each call (no state plumbing); when both vars set, spawns a task: reqwest POST to the URL, headers `Content-Type: application/json` + `X-Webhook-Signature: <sign_payload(...)>` (header name per convention), body `{"screen":…,"event":…}`, 5 s timeout, log warn on non-2xx/error. When either var unset → `tracing::debug!` and return.
- Test: `#[cfg(test)]` unit tests for `sign_payload` (known-vector: fixed secret + body → assert the exact expected string, computed once and hardcoded; plus different-secret ≠ same output). No network tests.

- [ ] **Step 1:** Read the existing verification helper to pin the signature format; write the failing sign_payload tests; implement; wire the four call sites (AFTER `Ok`-path DB success, before building the response value is fine — but only on success paths).
- [ ] **Step 2:** Verify FOREGROUND: `cd backend && cargo fmt --all && cargo check -p api-server && cargo clippy -p api-server --all-targets -- -D warnings && cargo test -p api-server webhook` + RLS script.
- [ ] **Step 3:** Commit — `feat(api-server): signed layout-change webhook notifier`

---

### Task 2: reality-web receiver + global tag (TDD)

**Files:**
- Create: `frontend/apps/reality-web/src/app/api/layout-revalidate/route.ts`
- Test: `frontend/apps/reality-web/src/app/api/layout-revalidate/route.test.ts`
- Modify: `frontend/apps/reality-web/src/lib/layout.ts` + `layout.test.ts` (global tag always present)

**Interfaces:**
- Route handler contract: `POST` only. Read raw body text FIRST (signature is over raw bytes). `LAYOUT_WEBHOOK_SECRET` unset → 503 `{error:'disabled'}`. Missing/malformed `X-Webhook-Signature` header or HMAC mismatch (Node `crypto.createHmac('sha256', secret)` + `timingSafeEqual` on equal-length buffers — guard length first) → 401 `{error:'invalid signature'}`. Body must parse as `{screen: string}` → else 422. Tags = `layoutTagsFor(screen)` exported helper (`['layout:' + screen.split('/')[1]]`, plus nothing else; screens without `/` → 422). `revalidateTag(tag)` for each (import from `next/cache`); respond 200 `{revalidated: true, tags}`.
- Signature format MUST mirror Task 1's (coordinate via the plan: whatever Task 1 reports as the convention — the Task 2 implementer reads Task 1's report file for the pinned format).
- Tests (mock `next/cache`'s `revalidateTag` via `vi.mock`; drive the exported `POST` with constructed `Request` objects; set/unset env via `vi.stubEnv`): secret unset → 503; bad signature → 401 and revalidateTag NOT called; valid signature (compute with Node crypto in the test) → 200, revalidateTag called with `layout:listing-detail`; screen without slash → 422; GET not exported.
- `layout.ts` tags change + test update: fetch call's `next.tags` asserted to include the global tag in both host/no-host cases.

- [ ] **Step 1:** TDD; run `pnpm -F @ppt/reality-web test` (route + layout tests green, suite otherwise unchanged).
- [ ] **Step 2:** Commit — `feat(reality-web): signed layout revalidation endpoint and global layout tags`

---

### Task 3: Gates + docs

- `docs/repo-map.md` layout bullet: add `Publish webhook: api-server layout/webhook.rs → reality-web /api/layout-revalidate (LAYOUT_WEBHOOK_URL/SECRET envs).`
- Backend + frontend full gates per Global Constraints (fmt first!).
- Commit — `docs(repo-map): layout publish webhook pointers`

---

## Deliberate scope decisions

- **No retry/queue** — a missed webhook self-heals via the existing `revalidate: 60` safety net (spec: on-demand primary + time-based fallback).
- **No per-host tag fan-out from the webhook** — the global tag covers all hosts; host-scoped tags remain for listing-content invalidation.
- **No admin UI for webhook status** — env-configured, ops-owned.
- **ppt-web/mobile unaffected** — client-side caching already handles freshness there.

## Out of scope (subsequent plans)

Mobile registries/renderers; `layout_editor_*` capability; per-tenant preview.
