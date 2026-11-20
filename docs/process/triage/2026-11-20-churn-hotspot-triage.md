# Churn-Hotspot Triage — 2026-11-20

Owner role: `pm-scrum-master` · Priority: medium

This note triages the research dispatcher's current churn hotspots and decides,
for each, whether the right next action is a **refactor** (decompose a structurally
unwieldy file) or a **characterization test** (pin current behavior before it
changes again). It also records the follow-ups carried forward from the prior
cycle. It is a planning artifact only — no runtime code changes accompany it, and
backlog state under `.research/**` is intentionally left untouched.

## How to read the classification

- **Refactor** — the file keeps churning because it is a god-file: changes are
  mechanically awkward, not behaviorally risky. The payoff is splitting it into
  focused modules. Behavior is already covered by existing tests, so the refactor
  is a structural move guarded by the current suite.
- **Characterization test** — the file keeps churning in behaviorally risky
  branches that are *not* pinned by a test at the right layer (DB-backed or
  transport-level). Lock the observable behavior first with golden/recorded-
  response tests, then any later refactor is safe.

A file can need both; the heading gives the **primary** next action and the
rationale names the secondary guard where one applies.

## Hotspots

| File | runs_seen | Size | Tests present | Primary action |
|------|:---------:|-----:|---------------|----------------|
| `backend/servers/api-server/tests/suites/oauth_integration_tests.rs` | (high) | 3427 LoC | 61 `#[sqlx::test]` | **Refactor** |
| `backend/servers/api-server/src/routes/voice_webhooks.rs` | 5 | 3066 LoC | 53 inline `#[test]`, 2 `#[sqlx::test]` | **Refactor** (+ char. guard) |
| `backend/servers/api-server/src/services/push_fanout.rs` | 4 | 3038 LoC | 56 inline `#[test]`, 0 transport-level | **Characterization test** |

### 1. `oauth_integration_tests.rs` — Refactor

**Classification: Refactor (split the monolithic suite).** This file *is* the
characterization layer for the Epic 10A OAuth provider — 61 end-to-end
`#[sqlx::test]` cases spanning PKCE S256, consent/revoke, refresh-rotation reuse
detection, the authorization audit trail, and provider-security negatives. You
cannot "add characterization tests" to a test file; the churn here is editing
friction from a 3.4k-line monolith that every OAuth change has to append to and
re-read. Split it into focused submodules by flow (e.g. `oauth/pkce_flow.rs`,
`oauth/consent_revoke.rs`, `oauth/refresh_rotation.rs`, `oauth/provider_security.rs`)
behind a thin `tests/suites/oauth/mod.rs`, sharing the existing `TestApp` /
`MIGRATOR` fixtures. No behavior change; the split is its own guard because the
cases run unchanged.

### 2. `voice_webhooks.rs` — Refactor (with a characterization guard)

**Classification: Refactor (decompose the god-route).** A 3k-line route module
that bundles the Alexa Skills Kit handler, the Google Actions handler, request
signature verification, timestamp/replay checks, rate limiting, and OAuth-token
user auth. Behavior is already fairly well covered by 53 inline unit tests (the
signature-verification and timestamp helpers in particular), so the dominant
cost of its churn is structural, not a coverage gap — extract provider handlers
and the signature-verification layer into sibling modules
(`voice/alexa.rs`, `voice/google.rs`, `voice/signature.rs`).

**Secondary guard (do first):** only **2** of its tests are DB-backed
(`#[sqlx::test]`). The OAuth-authenticated command path — token lookup → RLS
connection → `VoiceCommandProcessor` dispatch — is security-sensitive and is the
part a refactor is most likely to silently break. Add 2–3 DB-backed
characterization tests pinning that path (valid token → 200 + expected command,
revoked/expired token → 401, bad signature → 403) **before** moving code. Small,
targeted, not a full re-cover.

### 3. `push_fanout.rs` — Characterization test

**Classification: Characterization test (pin the transport error-code branches).**
This worker owns the FCM HTTP v1 and APNs HTTP/2 delivery adapters plus the
`CombinedPushAdapter` router and the Redis-polling `PushFanoutWorker`. It carries
56 inline unit tests, but **zero** exercise the live-shaped transport responses:
the highest-value, highest-risk logic — mapping provider error codes
(`NOT_REGISTERED` / `INVALID_REGISTRATION` on FCM, `BadDeviceToken` /
`Unregistered` on APNs) to **stale-token deletion from the DB** — is only
reachable through an HTTP round-trip that the current tests stub past. That is
exactly the kind of branch that churns and regresses unnoticed. Add
characterization tests against **recorded** FCM/APNs responses (the adapters
already support a base-URL override for testing) that assert: success → no
deletion, each documented stale-token error → the token row is deleted, and
transient 5xx → retained + retried. Pin this first; only then split the three
adapters + worker into submodules as a follow-up refactor.

## Carried-forward follow-ups

These items ride forward from the prior cycle. They are tracked here for
continuity; none are actioned by this note.

- **`#3650` — retry-after draft (ppt-web api client).** Issue
  `code-review-ppt-web-core-retry-ignores-retry-after`: "honor `Retry-After` in
  the ppt-web api client retry loop." Currently **closed** upstream — confirm the
  landed fix actually reads the `Retry-After` header (seconds *and* HTTP-date
  forms) and caps the backoff, or reopen/supersede if the draft was closed
  without the header being honored. Owner: pm-frontend.
- **reality-web preview i18n items.** Residual hardcoded/untranslated strings in
  the reality-web listing/preview surfaces. Related trail:
  `#3175` (closed) — remaining hardcoded `toLocaleString('sk-SK')` price displays
  (PR #3166); and the untranslated-copy sweep tracked by the
  `code-review-reality-web-realtor-mgmt-untranslated` plan. Re-verify the
  listing/sell-wizard **preview** step routes all user-visible strings and number
  formats through `next-intl` across `sk, cs, de, en`. Owner: pm-frontend.

## Suggested sequencing

1. `push_fanout.rs` characterization tests (highest behavioral risk, no DB/
   transport coverage today).
2. `voice_webhooks.rs` DB-backed guard tests, then the god-route split.
3. `oauth_integration_tests.rs` suite split (pure structural, lowest risk).
4. Confirm/close the two carried-forward follow-ups with pm-frontend.
