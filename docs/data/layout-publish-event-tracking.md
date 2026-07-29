# Layout Publish & Webhook — Analytics Event Tracking Definitions

> Scope: the analytics/telemetry events that describe the **layout publish →
> outbound webhook → reality-web ISR revalidation** pipeline.
> Audience: data / analytics engineers, platform-admin tooling owners, and
> anyone wiring dashboards or alerting on layout-change health.
> Grounded in code as of branch
> `auto-impl/data-layout-publish-event-tracking-2026-5ca2af84` (from `dev`).

This is a **definitions document**. It specifies the canonical event names,
properties, and enumerations for the layout-publish flow so that producers
(api-server, reality-web) and consumers (dashboards, alerts) agree on one
schema. Where the required dimension is not yet on the wire today, this doc
marks it **[gap]** and points at the exact code that would carry it — so the
schema is a target contract, not a claim that every field is already emitted.

## 1. Pipeline recap (what we are tracking)

A platform SuperAdmin mutates a layout via the Layout Admin API; on success
api-server fires a signed webhook; reality-web verifies it and revalidates the
affected ISR cache tags. The relevant code:

| Stage | Code |
|-------|------|
| Mutation handlers (`publish` / `rollback` / `kill` / `unkill`) | `backend/servers/api-server/src/routes/layout/admin.rs` |
| Version snapshot + `published_by` / `published_version` | `backend/crates/db/src/repositories/layout.rs` (`publish`, `rollback`) |
| Schema (`layout_configs`, `layout_config_versions`, `layout_tenant_overrides`) | `backend/crates/db/migrations/00221_create_layout_tables.sql` |
| Outbound signed notifier | `backend/servers/api-server/src/routes/layout/webhook.rs` (`notify_layout_change`, `sign_payload`) |
| Inbound receiver + revalidation | `frontend/apps/reality-web/src/app/api/layout-revalidate/route.ts` |
| Host-scoped fetch tags | `frontend/apps/reality-web/src/lib/layout.ts` (`getResolvedLayout`) |

> **Implementation status (issue #2532).** The producers are now wired. All
> three events are emitted, and the two api-server events are persisted to the
> append-only `layout_change_events` sink (migration
> `00225_create_layout_change_events.sql`). The webhook body now carries
> `delivery_id` (gap D) and `published_version` (gap A). Gaps A, C, D and the
> retention window (Section 7) are closed; gap B emits `target_tenant = "*"`
> until per-tenant override-publish lands; `layout_revalidate_received` is
> emitted from reality-web via the `trackEvent` bus (its cross-service
> persistence to the sink remains a follow-up — reality-web SSR has no DB
> handle). See Section 6 for the per-gap status.

Historically the webhook body was only `{"screen": <string>, "event": <string>}`
and **no analytics event was persisted** for this flow (unlike
`support_tooling_events`, see `docs/data/support-data-retention-privacy.md`).
The events below define what is captured, and Section 6 tracks the status of
each dimension.

## 2. Event taxonomy

Two producers, three logical events. Names are `snake_case`, namespaced under
`layout_`.

| Event | Producer | Fires when |
|-------|----------|-----------|
| `layout_change_published` | api-server (`admin.rs` handlers) | A layout mutation succeeds in the DB — one event per successful `publish` / `rollback` / `kill` / `unkill`. |
| `layout_webhook_dispatched` | api-server (`webhook.rs`) | The outbound POST to `LAYOUT_WEBHOOK_URL` completes (delivered / non-2xx / network error), or is skipped because the webhook is unconfigured. |
| `layout_revalidate_received` | reality-web (`route.ts`) | The `/api/layout-revalidate` handler returns — captures the revalidate outcome and the tags it acted on. |

`layout_change_published` is the **primary business event** (who published
what, when). The other two are **operational** events describing delivery and
downstream cache invalidation. Joining them on a shared `delivery_id`
(Section 6, gap D) reconstructs the full publish → revalidate trace.

## 3. Core dimensions (shared)

Every event in this family SHOULD carry these common dimensions:

| Property | Type | Description | Source today |
|----------|------|-------------|--------------|
| `event` | enum string | The event name from Section 2. | producer literal |
| `screen` | string | Layout screen id, e.g. `reality/listing-detail`. | `req.screen` (`admin.rs`); `body.screen` (`route.ts`) |
| `occurred_at` | timestamptz (UTC, ISO-8601) | Producer-side event time. | `now()` at emit |
| `delivery_id` | uuid | Correlation id shared across the three events for one mutation. **[gap D]** | not yet generated |

## 4. The four required dimensions

These are the dimensions the task calls out explicitly. Each is defined once
here and referenced from the per-event schemas in Section 5.

### 4.1 `published_by`

| Attribute | Value |
|-----------|-------|
| Type | `uuid` (nullable) |
| Meaning | The platform SuperAdmin who performed the mutation — the acting identity for the audit trail. |
| Source | `admin_id` returned by `extract_super_admin_token(&headers, &state)` in `admin.rs`, persisted as `layout_config_versions.published_by` and `layout_configs.updated_by` by the repository (`publish` / `rollback` bind `Some(admin_id)`). |
| Nullability | The column is `UUID` **nullable** (migration `00221`) and the repo signature is `published_by: Option<Uuid>`; `kill` / `unkill` do not currently thread an identity into a version row. Treat `null` as "system / unattributed". |
| PII | A user identifier → personal data. Store the UUID only; never the admin's email/name in this event (mirror the `support_tooling_events` posture). |

### 4.2 `layout_version`

| Attribute | Value |
|-----------|-------|
| Type | `integer` (monotonic per `screen`) |
| Meaning | The published version number **after** the mutation. `publish` and `rollback` both do `published_version = published_version + 1` and snapshot a row into `layout_config_versions(screen, version, config, published_by)`. So a rollback to an old config still yields a NEW, higher `layout_version` (immutable, append-only history). |
| Source | `row.published_version` returned by `LayoutRepository::publish` / `rollback`. **[gap A]** — available in the handler but not currently passed to `notify_layout_change`, whose signature is `(screen, event)`. |
| `kill` / `unkill` | Do **not** create a version (they toggle `layout_kill_flags`, bypassing the publish gate — spec §5). For those events `layout_version` is `null`; use `section_type` (Section 5.1) instead. |

### 4.3 `target_tenant`

| Attribute | Value |
|-----------|-------|
| Type | `string` — an organization UUID, or the sentinel `"*"` (global) |
| Meaning | Which tenant scope the change applies to. |
| Semantics | Layout config is **global / platform-admin-owned and not RLS-scoped** — `layout_configs` / `layout_config_versions` have no `organization_id`; a `publish` affects **all** tenants, so `target_tenant = "*"`. Per-tenant customisation lives in the **separate** `layout_tenant_overrides` table (`organization_id`, `ENABLE + FORCE RLS`, migration `00221`), which the current publish/rollback/kill/unkill handlers do **not** touch. |
| reality-web view | On the receiver, tenant scope is expressed as the **host**: `getResolvedLayout` tags the fetch `host:${host}:layout:listing-detail` alongside the global `layout:listing-detail` (`layout.ts`). So on the revalidate event `target_tenant` is naturally the request host (or `"*"` when the global tag is invalidated). |
| Source | api-server side: **[gap B]** — a global publish is `"*"`; a future per-tenant-override mutation would carry its `organization_id`. reality-web side: derivable from the `Host` header / tag, **[gap B']**. |

### 4.4 `revalidate_outcome`

| Attribute | Value |
|-----------|-------|
| Type | enum string |
| Meaning | The result of the downstream ISR cache invalidation at `/api/layout-revalidate`. |
| Enumeration (grounded in `route.ts` return paths) | see table below |

| `revalidate_outcome` | HTTP | Condition in `route.ts` |
|----------------------|:----:|-------------------------|
| `revalidated` | 200 | Signature valid, body + screen valid → `revalidateTag` called for each tag; response `{revalidated: true, tags}`. |
| `disabled` | 503 | `LAYOUT_WEBHOOK_SECRET` unset → `{error:'disabled'}`. |
| `invalid_signature` | 401 | Missing/malformed `X-Webhook-Signature` or HMAC mismatch (length-guarded `timingSafeEqual`). |
| `invalid_body` | 422 | Body not JSON, or not `{screen: string}`. |
| `invalid_screen` | 422 | `screen` has no `/`, or empty second segment (`layoutTagsFor` returns `null`). |

The **outbound** side (`webhook.rs`) has its own delivery outcome that is
distinct from the receiver's revalidate outcome; it belongs on
`layout_webhook_dispatched` (Section 5.2), not here:

| `dispatch_outcome` | Condition in `notify_layout_change` |
|--------------------|-------------------------------------|
| `skipped_unconfigured` | `LAYOUT_WEBHOOK_URL` or `LAYOUT_WEBHOOK_SECRET` unset/empty → `debug!` no-op. |
| `delivered` | reqwest POST returned a 2xx (`resp.status().is_success()`). |
| `non_2xx` | POST returned a non-2xx status (logged `warn!`). |
| `transport_error` | reqwest client build or `send()` errored (logged `warn!`). |

## 5. Per-event schemas

### 5.1 `layout_change_published`

Emitted once per successful mutation, from the four handlers in `admin.rs`
(after the `Ok`-path DB success, alongside the existing
`webhook::notify_layout_change(...)` call site).

| Property | Type | Notes |
|----------|------|-------|
| `event` | `"layout_change_published"` | literal |
| `screen` | string | `req.screen` |
| `change_kind` | enum: `published` \| `rolled_back` \| `killed` \| `unkilled` | matches the `event` literal already passed to `notify_layout_change` |
| `published_by` | uuid? | §4.1 |
| `layout_version` | int? | §4.2 — set for `published`/`rolled_back`; `null` for `killed`/`unkilled` |
| `target_tenant` | string | §4.3 — `"*"` for global config changes |
| `section_type` | string? | present only for `killed`/`unkilled` (`req.section_type`); identifies the killed section |
| `rolled_back_to` | int? | present only for `rolled_back` — the source `version` requested (`req.version`), distinct from the new `layout_version` produced |
| `delivery_id` | uuid | §3 gap D — correlation key |
| `occurred_at` | timestamptz | emit time |

### 5.2 `layout_webhook_dispatched`

Emitted from `webhook.rs` once the outbound attempt resolves (including the
unconfigured no-op path, so "webhook silently off" is observable).

| Property | Type | Notes |
|----------|------|-------|
| `event` | `"layout_webhook_dispatched"` | literal |
| `screen` | string | |
| `change_kind` | enum (as 5.1) | the `event` argument |
| `dispatch_outcome` | enum | §4.4 outbound table |
| `http_status` | int? | present for `delivered` / `non_2xx` |
| `duration_ms` | int? | POST round-trip; optional |
| `delivery_id` | uuid | correlation key |
| `occurred_at` | timestamptz | |

Explicitly **excluded**: the raw `X-Webhook-Signature`, the secret, and the
target URL host credentials. The signature is a keyed digest and MUST NOT be
logged as an analytics property.

### 5.3 `layout_revalidate_received`

Emitted from the reality-web `/api/layout-revalidate` route handler on every
return path.

| Property | Type | Notes |
|----------|------|-------|
| `event` | `"layout_revalidate_received"` | literal |
| `screen` | string? | `body.screen` when parseable; `null` on `invalid_body` |
| `revalidate_outcome` | enum | §4.4 receiver table |
| `http_status` | int | 200 / 401 / 422 / 503 |
| `tags` | string[] | tags passed to `revalidateTag` (only on `revalidated`) |
| `target_tenant` | string | §4.3 — request host, or `"*"` |
| `delivery_id` | uuid? | echoed from the webhook body (§6 gap D) if present |
| `occurred_at` | timestamptz | |

## 6. What is on the wire today vs. gaps

The events above are the **target contract**. Current reality and the minimal
change to close each gap:

| # | Gap | Status (issue #2532) |
|---|-----|----------------------|
| A | `layout_version` not delivered | **Closed.** `notify_layout_change(pool, screen, event, published_version, delivery_id)` carries the version; `build_webhook_body` puts it on the wire. `null` for kill/unkill. |
| B | `target_tenant` not delivered | **Partial.** Global changes emit `target_tenant = "*"`; a real org id follows when per-tenant override-publish lands. |
| B' | reality-web tenant attribution | **Closed.** `route.ts` reads the `Host` header into `target_tenant` (falling back to `"*"`). |
| C | No event is persisted | **Closed (backend).** `layout_change_published` + `layout_webhook_dispatched` persist to the append-only `layout_change_events` table (migration `00225`, `support_tooling_events` pattern). `layout_revalidate_received` emits via `trackEvent` but is **not yet** persisted cross-service — reality-web SSR has no DB handle; an ingest endpoint is a follow-up. |
| D | No `delivery_id` correlation | **Closed.** A uuid is generated per mutation in `admin.rs`, shared with the persisted `layout_change_published`, put in the webhook body, and echoed by the receiver. |

Recording the status here prevents a future implementer from inventing a
divergent field set.

## 7. Privacy & retention posture

Consistent with `docs/data/support-data-retention-privacy.md`:

- **PII surface is minimal.** The only personal datum is `published_by` (an
  admin UUID). No emails, names, IPs, or user-agents belong on these events.
  The webhook signature/secret and the layout `config` JSON payloads are **not**
  event properties.
- **Fire-and-forget.** Analytics emission MUST NOT change the outcome of the
  admin mutation or the revalidation — mirror the existing notifier, which is
  `tokio::spawn`ed and only `warn!`s on failure.
- **Append-only / immutable** if persisted (gap C): follow
  `support_tooling_events` — DB triggers reject `UPDATE`/`DELETE`,
  `published_by` FK `ON DELETE RESTRICT`/`SET NULL` so the trail survives
  account deletion (anonymised).
- **Retention.** Defined up front (issue #2532): the `layout_change_events`
  sink prunes rows older than **730 days (24 months)** via the DB-native
  `cleanup_old_layout_change_events(retention_days)` (migration `00225`),
  matching the `support_tooling_events` window (migration `00222`) and honouring
  GDPR storage limitation (Art. 5(1)(e)). Deletes are gated by the
  `app.retention_prune` GUC so the append-only immutability trigger still blocks
  every other `UPDATE`/`DELETE`. Wiring the prune into the api-server background
  scheduler is a separate follow-up (same posture as support_tooling_events).

## 8. Open questions (verify before relying on this doc)

1. **Sink choice (gap C).** Should layout-publish analytics land in a new
   `layout_change_events` table, be folded into `support_tooling_events`, or go
   only to an external analytics pipe via the webhook? This doc assumes a
   `support_tooling_events`-style immutable table; confirm before building.
2. **`kill`/`unkill` identity.** These handlers currently do not persist a
   `published_by` version row. If attribution matters for kills, that identity
   must be threaded through — otherwise `published_by` is `null` for those.
3. **Per-tenant overrides (gap B).** `layout_tenant_overrides` exists but is not
   mutated by the current publish flow. When per-tenant publish lands,
   `target_tenant` becomes a real org UUID rather than always `"*"`; confirm the
   override-publish handler emits the same event family.
4. **`delivery_id` propagation (gap D).** Adding it changes the webhook body,
   which is HMAC-signed — both `sign_payload` (api-server) and the receiver's
   verification cover the new bytes automatically, but the body-shape assertion
   in `route.ts` (`{screen: string}`) must be widened to tolerate the extra field.
