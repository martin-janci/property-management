# NFR Baseline — Push Fan-out & Voice Webhooks

> **Status:** baseline (as-built audit + target NFRs). 2026-11-20.
> **Scope:** outbound push delivery (FCM + APNs) and inbound voice-assistant
> webhooks (Alexa / Google Actions) plus their OAuth token lifecycle.
> **Owner:** pm-tech-lead.
> **Companion docs:** [`non-functional-requirements.md`](../non-functional-requirements.md)
> (system-wide NFRs, RED metrics, SLAs), [`integrations/webhook-hardening-audit-2026-07.md`](./webhook-hardening-audit-2026-07.md)
> (signature / replay hardening), [`phase2/infrastructure-tracing.md`](../phase2/infrastructure-tracing.md).

This document defines a non-functional-requirements (NFR) baseline for two
delivery surfaces that today have **no dedicated metrics, no alerting, and
inconsistent retry/dead-letter handling**:

1. **Push fan-out** — `api-server/src/services/push_fanout.rs` (FCM HTTP v1 +
   APNs HTTP/2), driven by `notification_pipeline.rs` and the quiet-hours
   drain worker.
2. **Voice webhooks** — `api-server/src/routes/voice_webhooks.rs` (Alexa,
   Google Actions, and the voice OAuth exchange/refresh endpoints).

It describes the **current state** (what the code emits today, with file
references) against **target NFRs**, and ends with a prioritised gap table.
It is a planning/observability baseline: it does **not** change runtime code
beyond the noted trivial-wiring opportunities.

---

## 1. Observability stack already present

The platform already ships a production observability spine — the gap is
**coverage**, not infrastructure.

| Capability | Where | Notes |
|---|---|---|
| Prometheus registry + `/metrics` endpoint | `api-server/src/observability.rs` (`init_metrics`, `get_metrics_text`); served at `GET /metrics` in `main.rs` | `metrics` + `metrics-exporter-prometheus`. `METRICS_ENABLED` defaults **on**, port `9090` (`METRICS_PORT`). |
| OpenTelemetry OTLP tracing | `observability.rs` (`init_otel_tracer`) | Off by default; `OTEL_ENABLED`, `OTEL_EXPORTER_OTLP_ENDPOINT`. |
| Sentry error reporting + PII redaction | `observability.rs` (`init_sentry`, `report_error`, `redact_pii`) | Off by default; `SENTRY_ENABLED`, `SENTRY_DSN`. Categorised via `ErrorCategory` (incl. `ExternalService`). |
| Generic HTTP/DB/business metrics | `observability.rs` (`describe_*` block) | `http_requests_total`, `http_request_duration_seconds`, `http_requests_errors_total`, `database_connections_*`, `auth_login_*`, `faults_created_total`, `votes_cast_total`, `errors_total`. |
| **Domain pipeline metrics (exemplar)** | `api-server/src/services/notification_digest.rs` | The **only** notification-path worker with custom metrics: `notification_digest_{sweeps,candidates_considered,sent,failed,excluded_by_filter,abandoned}_total`, each `describe_counter!`-documented at worker start and `counter!`-incremented per sweep. **This is the pattern push/voice should follow.** |
| Admin ops summary | `api-server/src/routes/admin/metrics.rs` (`GET /api/v1/admin/metrics/summary`) | Aggregate counts for an operator tile; not a time-series source. |

**Dead-letter infrastructure already in the repo** (two independent patterns):

- **Bounded-retry-then-dead-letter** on the quiet-hours held-push drain —
  `held_notifications.attempts` + `dead_lettered_at` (migration
  `00234_held_notification_delivery_tracking.sql`), driven by
  `services/quiet_hours_drain.rs` (`poll_interval_secs=60`, `max_attempts=10`).
- **Durable dead-letter table** for payment webhooks —
  `payment_webhook_dead_letters` (migrations `00240`, `00244`): raw signed
  event, `attempts`, `resolved_at`/`resolution`, dedup upsert, org-scoped RLS.

Push fan-out and voice webhooks have **neither** of these.

---

## 2. Current state — Push fan-out

Source: `api-server/src/services/push_fanout.rs` (FCM/APNs adapters),
`services/notification_pipeline.rs` (dispatch), `services/quiet_hours_drain.rs`
(held-push release).

### 2.1 Delivery

- FCM HTTP v1 (`FcmHttpAdapter`) and APNs HTTP/2 (`ApnsHttpAdapter`), combined
  by `CombinedPushAdapter`; tokens routed by platform.
- **No retry/backoff within a dispatch.** `deliver()` makes **one** send
  attempt per device token; if every token fails, `send()` returns
  `ProviderOutcome::Failed` and the attempt is over. There is no in-process
  requeue and no durable dead-letter for a transiently-failed live push.
- Quiet-hours-held pushes are the **only** push path with bounded retry +
  dead-letter (via `quiet_hours_drain` / `held_notifications`, §1).

### 2.2 Delivery receipts / metrics

- `PushDeliveryReceipt` is **log-only + in-memory** — the code comment says
  "stored in memory for now; DB table in follow-up" and it is emitted purely
  as a `tracing::info!("[8A-3] Push delivery receipt", success, token_expired)`
  line. **No counter/histogram is emitted.** Success rate, latency, and
  per-provider failure counts are not queryable in Prometheus.

### 2.3 Stale-token handling

- FCM `UNREGISTERED` / legacy `NOT_REGISTERED`, APNs `BadDeviceToken` /
  `Unregistered` mark the token stale → `delete_stale_token_by_string` evicts
  it (targeted `DELETE … WHERE user_id AND token`). This is **log-only**; no
  `push_tokens_evicted_total` metric.

### 2.4 FCM OAuth token refresh (the "token-refresh failure" NFR)

- `ServiceAccountTokenSource` mints & auto-refreshes the FCM HTTP v1 OAuth2
  access token, proactively `FCM_TOKEN_REFRESH_SKEW_SECS = 300` before Google's
  ~3600 s expiry (`FCM_SERVICE_ACCOUNT_JSON` / `GOOGLE_APPLICATION_CREDENTIALS`).
- **On refresh failure it logs and silently skips Android push for that
  attempt** — `tracing::error!("[8A-3] FCM: failed to refresh OAuth2 access
  token; Android push will be skipped this attempt")`. **No metric, no alert.**
  A credential rotation or clock-skew outage degrades all Android push with no
  signal other than scraping logs.
- Legacy `FCM_OAUTH_TOKEN` is read once at startup and never refreshed
  (deprecated; breaks after ~1 h).

### 2.5 APNs auth

- `mint_apns_jwt` mints an ES256 provider JWT per send from `APNS_P8_KEY`. A
  missing/invalid P8 key returns `None` and the send is skipped — **log-only,
  no metric.**

---

## 3. Current state — Voice webhooks

Source: `api-server/src/routes/voice_webhooks.rs`.

### 3.1 Inbound requests

- Routes: `/alexa`, `/alexa/health`, `/google`, `/oauth/exchange`,
  `/oauth/refresh`, `/verify`.
- Signature verification is in place (Alexa signature; Google RS256 ID-token) —
  see the webhook-hardening audit. **No per-webhook metrics**
  (`voice_webhook_requests_total{platform,outcome}`) and no latency histogram;
  failures surface only as generic `http_requests_errors_total` + a
  `tracing::warn!`.

### 3.2 Voice OAuth token exchange / refresh

- `oauth_token_exchange` and `oauth_token_refresh` make live upstream calls to
  Amazon/Google and are throttled per PM user by an in-process sliding-window
  limiter: **10 attempts / 900 s** (`VOICE_{REFRESH,EXCHANGE}_MAX_ATTEMPTS`,
  `VOICE_{REFRESH,EXCHANGE}_WINDOW`), `429` on breach.
- **On upstream exchange/refresh failure** the handler returns an **RFC 6749
  §5.2**-shaped OAuth error body (`{ "error": <code>, "error_description":
  <static message> }`) built by `voice_oauth_error_response`
  (`services/voice_oauth.rs`), and logs the upstream detail server-side only
  (never echoed to the client — "oauth client error leak" regression). The code
  and status are derived from the cause by `classify_voice_oauth_error` +
  `oauth_error_status`:
  | Upstream cause | `error` | HTTP status |
  |----------------|---------|-------------|
  | Provider rejected the grant (bad/expired/revoked auth code or refresh token) — `ExchangeFailed` / `RefreshFailed` | `invalid_grant` | 400 |
  | Unsupported platform — `InvalidPlatform` | `invalid_request` | 400 |
  | Transport failure, unparseable provider response, missing config — `Http` / `InvalidResponse` / `MissingConfig` | `server_error` | 500 |

  The `error_description` carries only the static non-revealing client message
  (`OAUTH_EXCHANGE_FAILED_MESSAGE` / `OAUTH_REFRESH_FAILED_MESSAGE`). This
  replaces the earlier PPT `code`/`message` envelope (`OAUTH_EXCHANGE_FAILED` /
  `TOKEN_REFRESH_FAILED`), and fixes the prior refresh-failure status (was a
  blanket `500`; a rejected grant is now the spec-correct `400`). The contract
  is pinned by the `voice_oauth_error_response_is_rfc6749_shaped_*`,
  `oauth_error_status_follows_rfc6749`, and `classify_voice_oauth_error_*` unit
  tests in `services/voice_oauth.rs`. **No metric, no alert** — an
  upstream-provider outage or revoked refresh token is still invisible to
  monitoring (gap G3).
- Tokens are encrypted at rest (`encrypt_voice_token_pair`) with an indexed
  `access_token_hash`; not an observability concern but noted for completeness.

### 3.3 Retry / dead-letter

- Voice webhooks are **synchronous request/response**; there is **no** durable
  dead-letter for an inbound event whose downstream processing fails, unlike
  `payment_webhook_dead_letters`. A failed voice OAuth refresh is simply a
  `5xx`/`429` to the caller with no in-house record for reconciliation.

---

## 4. Target NFRs

### 4.1 Metrics catalogue (target)

All counters follow the `notification_digest_*` exemplar: `describe_counter!`
with help text at worker/router init, `counter!`/`histogram!` on the hot path.
Label cardinality kept low (bounded enums only).

**Push fan-out**

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `push_delivery_attempts_total` | counter | `provider`(fcm\|apns) | Denominator for success rate. |
| `push_delivery_success_total` | counter | `provider` | SLI numerator. |
| `push_delivery_failed_total` | counter | `provider`, `reason`(rejected\|transport\|not_configured) | Failure breakdown. |
| `push_delivery_duration_seconds` | histogram | `provider` | Per-send latency. |
| `push_tokens_evicted_total` | counter | `provider`, `reason`(unregistered\|bad_token) | Stale-token churn. |
| `push_fcm_token_refresh_total` | counter | `outcome`(ok\|failed) | **Token-refresh health (alert source).** |
| `push_apns_jwt_mint_total` | counter | `outcome`(ok\|failed) | APNs auth health. |

**Voice webhooks**

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `voice_webhook_requests_total` | counter | `platform`(alexa\|google), `outcome`(ok\|sig_fail\|error) | Inbound volume + signature health. |
| `voice_webhook_duration_seconds` | histogram | `platform` | Handler latency. |
| `voice_oauth_total` | counter | `op`(exchange\|refresh), `outcome`(ok\|failed\|rate_limited) | **Token-refresh health (alert source).** |

**Reuse-first (trivial wiring):** `errors_total` / `report_error` with
`ErrorCategory::ExternalService` already exists in `observability.rs` — the FCM
refresh-failure and voice-refresh-failure branches can call `report_error(..)`
today with no new infrastructure, which is the minimal first step before the
dedicated counters land.

### 4.2 SLO targets

| SLI | Target | Alert threshold |
|---|---|---|
| Push delivery success rate (per provider) | ≥ 99% rolling 1 h | < 95% over 15 min |
| Push send P95 latency | < 2 s | > 5 s over 15 min |
| FCM OAuth refresh failures | 0 sustained | **≥ 1 failure, or any `ok=0` over 2 consecutive refresh cycles (~10 min)** → page |
| APNs JWT mint failures | 0 sustained | **≥ 1 over 5 min** → page |
| Voice webhook signature-failure rate | < 1% | > 5% over 10 min (possible attack/misconfig) |
| Voice OAuth refresh failure rate | < 1% | > 10% over 10 min, or absolute ≥ 5/min → warn |
| Held-push dead-letter rate (`quiet_hours_drain`) | ~0 | any `dead_lettered` > 0 over 1 h → warn |

### 4.3 Retry / dead-letter policy (target)

Adopt the two patterns already in the repo rather than inventing a third:

- **Live push (FCM/APNs transient failure).** Today: single attempt, drop.
  Target: bounded in-handler retry (≤ 2 immediate retries with jittered
  backoff) for transport/5xx errors only (never for `UNREGISTERED`/stale —
  those evict). On exhaustion, record to a durable store modelled on
  `held_notifications` (reuse the `attempts` + `dead_lettered_at` columns via
  the drain) so a provider outage does not silently lose notifications.
- **Voice OAuth refresh failure.** Target: on repeated upstream failure for a
  linked device, persist a dead-letter row (pattern:
  `payment_webhook_dead_letters` — raw context, `attempts`, `resolved_at`) so
  operators can see which linkages need re-consent, instead of only a `5xx`.
- **Idempotency / retry-collapse.** Mirror the
  `ON CONFLICT (provider, provider_session_id)` dedup upsert used by payment
  dead-letters so provider redeliveries bump `attempts` rather than flooding.

### 4.4 Token-refresh failure alerting (target)

The headline NFR. Required signals, in priority order:

1. **FCM OAuth refresh failure** — emit `push_fcm_token_refresh_total{outcome="failed"}`
   (and/or `report_error(ExternalService)` as the trivial first step). Alert
   **page** on any sustained failure: all Android push is down within ~5 min of
   the cached token expiring.
2. **APNs JWT mint failure** — `push_apns_jwt_mint_total{outcome="failed"}`;
   page: all iOS push is down.
3. **Voice OAuth refresh failure** — `voice_oauth_total{op="refresh",outcome="failed"}`;
   warn, escalate to page if failure rate indicates an upstream-provider
   outage rather than per-user revocation.

Alerts are expressed as Prometheus rules against the metrics in §4.1; the repo
has no `alertmanager`/`*.rules.yaml` checked in yet (confirmed by search), so
landing a `monitoring/` rules file is itself a tracked follow-up.

---

## 5. Gap summary & roadmap

| # | Gap | Surface | Current | Target | Priority |
|---|---|---|---|---|---|
| G1 | FCM OAuth refresh failure is log-only | Push | `tracing::error!` only | `push_fcm_token_refresh_total` + page alert (trivial: `report_error` now) | **High** |
| G2 | APNs JWT mint failure is log-only | Push | `None`, skip, log | `push_apns_jwt_mint_total` + alert | **High** |
| G3 | Voice OAuth refresh failure is log-only | Voice | non-leak msg + server log | `voice_oauth_total{op=refresh}` + alert | **High** |
| G4 | No push delivery success/latency metrics | Push | in-memory log-only receipt | `push_delivery_*` counters + histogram | Medium |
| G5 | No per-webhook metrics | Voice | generic `http_*` only | `voice_webhook_requests_total` + histogram | Medium |
| G6 | Live push has no retry/dead-letter | Push | single attempt, drop | bounded retry + durable dead-letter (reuse `held_notifications` pattern) | Medium |
| G7 | Voice has no dead-letter for failed refresh | Voice | `5xx`/`429`, no record | dead-letter table (reuse `payment_webhook_dead_letters` pattern) | Low |
| G8 | No alert rules checked into the repo | Both | none | `monitoring/*.rules.yaml` for §4.2 thresholds | Medium |
| G9 | `push_tokens_evicted_total` not emitted | Push | log-only eviction | counter for stale-token churn | Low |

**Suggested sequencing:** G1–G3 first (highest-leverage, lowest-cost — the
failure branches already exist and `report_error(ExternalService)` is wired),
then G4/G5 observability counters following the `notification_digest_*`
exemplar, then G6–G8 retry/dead-letter durability, then G9.

---

## Appendix — key source references

| Concern | File | Symbol |
|---|---|---|
| Prometheus init / `/metrics` | `api-server/src/observability.rs` | `init_metrics`, `get_metrics_text` |
| Exemplar domain metrics | `api-server/src/services/notification_digest.rs` | `METRIC_*`, `describe_sweep_metrics` |
| Push delivery core | `api-server/src/services/push_fanout.rs` | `FcmHttpAdapter::deliver`, `ApnsHttpAdapter::deliver`, `PushDeliveryReceipt` |
| FCM token refresh | `api-server/src/services/push_fanout.rs` | `ServiceAccountTokenSource::refresh`, `FCM_TOKEN_REFRESH_SKEW_SECS` |
| APNs auth | `api-server/src/services/push_fanout.rs` | `mint_apns_jwt` |
| Quiet-hours drain (retry/dead-letter) | `api-server/src/services/quiet_hours_drain.rs` | `max_attempts`, `RowOutcome::DeadLettered` |
| Held-push dead-letter schema | `crates/db/migrations/00234_held_notification_delivery_tracking.sql` | `attempts`, `dead_lettered_at` |
| Payment dead-letter schema | `crates/db/migrations/00240_create_payment_webhook_dead_letters.sql` | `payment_webhook_dead_letters` |
| Voice webhooks + OAuth | `api-server/src/routes/voice_webhooks.rs` | `oauth_token_refresh`, `voice_refresh_attempt_allowed`, `OAUTH_REFRESH_FAILED_MESSAGE` |
