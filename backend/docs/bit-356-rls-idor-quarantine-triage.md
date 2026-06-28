# BIT-356 — Triage of the 60 quarantined RLS / IDOR / authz test binaries

Parent: **BIT-354** (repair the BIT-351 quarantine). This child owns the **60 security
binaries** (RLS / tenant-isolation, IDOR / cross-org, authz) `#[ignore]`'d by the
BIT-351 quarantine (`6207e4262`, #1948).

> **Secure-by-default rule applied:** a binary is only **DELETE** when its load-bearing
> schema (a whole table/feature) appears in **zero** migrations. A missing *column* on an
> existing table, a comment-only reference to a dropped table, or any decode/seed/assertion
> drift is **FIX**, never DELETE.

## Headline result

**DELETE-class = ∅ (zero).** Every one of the 60 binaries references schema that **exists**
in `crates/db/migrations/*.sql` on `origin/dev`. There is **no speculative / never-migrated
table** behind any quarantine marker, so **this PR deletes nothing** — deleting any of these
binaries would lose real cross-tenant / IDOR / authz coverage.

Method (pure static analysis, host-independent): for each binary, extract every referenced
table / column / enum (`INSERT INTO` / `FROM` / `JOIN` / `::enum`), then confirm presence
against the full migration corpus (544 tables, 185 enums created across 200 migrations).
Cross-checked by six independent triage passes. The only created-then-dropped-and-never-
recreated table is `portal_users` (dropped 00148; realtors unified into `users` with
`principal_kind='public'`), and **no binary uses it as load-bearing schema** — the single
reference (`inquiry_mark_read_idor_tests.rs`) is in a code comment; the seed correctly
inserts into `users`. `reserve_funds` was dropped **and recreated** (00097, FORCE'd 00179)
— it exists.

## Real repo regression flagged (not a test bug)

**`document_share_type` enum decoded as `String` in `get_shares_rls`.**
`crates/db/src/repositories/document/shares.rs:120` (`get_shares_rls`) does `SELECT s.*`
and then `share_type: r.get("share_type")` into `DocumentShare.share_type: String`
(`models/document.rs:292`). The `document_shares.share_type` column is the Postgres enum
`document_share_type`, whose OID does **not** decode into Rust `String` — `Row::get`
panics at runtime. Every **other** query in the same file casts `share_type::text AS
share_type` (lines 44/73/97); this one path forgot. It is **reachable**: the
`GET /documents/{id}/shares` listing endpoint
(`servers/api-server/src/routes/documents/shares.rs:95`) calls `get_shares_rls`, so
listing a document's shares fails at decode time. **Fix:** replace `SELECT s.*` with an
explicit column list using `s.share_type::text AS share_type` (mirroring the sibling
queries). Tracked as a dedicated child of BIT-354 (security).

## FIX-class clustering — fan-out map for BIT-354

All 60 are FIX. The repair work groups into five clusters; each cluster shares one root
cause, so one harness/helper fix typically re-greens the whole cluster. Re-enable a binary
by removing its `#[ignore = "BIT-351 quarantine ..."]` line **after** it passes the gate.

### Cluster A — `rls-guc-mismatch` / FORCE-RLS behavioral (db crate, ~30 binaries)

These create a `NOSUPERUSER NOBYPASSRLS` role + `SET ROLE` to exercise `FORCE ROW LEVEL
SECURITY` (the `#[sqlx::test]` pool connects as superuser, which bypasses RLS). They depend
on the fully-migrated policy set + `set_request_context()` / `get_current_org_id()` GUC
plumbing (canonical `app.current_org_id`, per migration 00179). Root cause of failure is the
blind-CI "schema never applied" artifact + shared role/grant boilerplate, not schema drift.
**Highest-leverage repair:** extract the role-create + grant + `SET ROLE` + cleanup boilerplate
into one shared test helper, fix it once, sweep the cluster.

`acc_mvp_rls_tests`, `accounting_provider_rls_repo_tests`, `accounting_rls_repo_tests`,
`ai_chat_rls_repo_tests`, `api_ecosystem_developer_force_rls_tests`,
`api_ecosystem_rls_repo_tests`, `budget_rls_repo_tests`,
`building_certification_rls_repo_tests`, `document_shares_rls_cross_tenant_tests`,
`document_template_cross_tenant_tests`, `documents_rls_cross_tenant_tests`,
`emergency_rls_repo_tests`, `enhanced_tenant_screening_rls_repo_tests`,
`epic11_org_guc_rls_tests`, `esg_force_rls_cross_tenant_tests`, `form_rls_repo_tests`,
`insurance_rls_repo_tests`, `legal_rls_repo_tests`, `llm_document_rls_repo_tests`,
`messaging_rls_cross_tenant_tests`, `notification_rls_guc_tests`,
`predictive_maintenance_rls_repo_tests`, `public_plans_rls_tests`,
`reserve_funds_rls_repo_tests`, `sensor_rls_repo_tests`, `sentiment_rls_repo_tests`,
`subscription_rls_repo_tests`, `vendor_rls_repo_tests`, `work_order_rls_repo_tests`,
`workflow_rls_repo_tests`.

> Note: `epic11_org_guc_rls_tests` and `notification_rls_guc_tests` are catalog/GUC tests
> asserting the canonical `app.current_org_id` GUC + FORCE post-00179 — directly relevant to
> the Epic 8A GUC-mismatch work (#755). `accounting_rls_repo_tests` also guards the
> `update_invoice_rls` `.fetch_one`→`RowNotFound`→500 vs `Ok(None)`→404 repo contract (PAP-321).

### Cluster B — `seed/FK-gap` (db crate, ~7 binaries)

Tables exist; failures are missing NOT-NULL / FK columns in the seed chain or a self-created
helper table. `esignature_workflow_repo_tests` self-`CREATE IF NOT EXISTS`'s
`esignature_workflows` (unmounted-integration repo path) — **not** a delete; its repo method
exists in `repositories/integration.rs`.

`dispute_cross_tenant_tests`, `equipment_rls_repo_tests`, `esignature_workflow_repo_tests`,
`integration_rls_repo_tests`, `inquiry_mark_read_idor_tests`,
`registry_parking_spot_cross_tenant_tests`, `rental_cross_tenant_upsert_tests`.

> `dispute_cross_tenant_tests` and `document_template_cross_tenant_tests` use bare
> `#[sqlx::test]` (no `migrator = "db::MIGRATOR"`) — confirm the default migration source
> matches `db::MIGRATOR` during repair.

### Cluster C — `authz-harness / status-drift` + membership split-brain (api-server, 14 binaries)

HTTP-level cross-org IDOR / authz tests. **Single dominant root cause:** the harness
`seed_membership` / `create_authenticated_user_with_org`
(`servers/api-server/tests/common/mod.rs`) insert into **`organization_members`**, but
`RequestPrincipal` / RLS tenant resolution reads active grants from **`user_memberships`**
(the documented "hot path for RequestPrincipal"). So every *same-org → 200* leg fails
(handler sees no membership → 400/404), and because `TestApp` mounts **no `ResolvedTenant`**,
authed-non-member legs return **400/401 instead of the asserted 403**. **Highest-leverage
repair:** make the seed helper also write `user_memberships` (or switch the read), then
re-pin the strict-403 assertions to the 400/401 the no-`ResolvedTenant` harness actually
returns. (See memory: *authz-test-status-codes-testapp*, *requestprincipal-happy-path-harness*.)

`infra_ops_admin_authz_backfill_bit331_tests`, `org_property_authz_backfill_bit331_tests`,
`owner_analytics_cross_org_idor_tests`, `platform_admin_authz_tests`,
`predictive_maintenance_photos_idor_tests`, `push_fanout_stale_token_isolation_tests`,
`rental_connection_token_leak_tests`, `rental_guest_id_document_tests`,
`reserve_funds_cross_org_idor_tests`, `service_history_cross_org_idor_tests`,
`vendor_cross_org_idor_tests`, `violations_cross_org_idor_tests`,
`work_order_cross_org_idor_tests`, `workflow_cross_tenant_idor_tests`.

### Cluster D — `signature/crypto` (api-server, 1 binary)

`portal_webhook_signature_tests` — HMAC-SHA256 fail-closed contract for
`POST /webhooks/portals/reality-portal/views` (401 unsigned, 200 valid). Fragility is
env-var mutation under concurrent test binaries + exact request-body byte matching; real
crypto/route guard, keep + stabilize.

### Cluster E — `auth-token/session` (reality-server, 6 binaries)

Public-portal authz / IDOR tests using the reality-server harness
(`make_app_state` / `mint_token` / `send`, `OptionalRequestPrincipal`, 401-vs-non-401
boundary). All seed `users` (`principal_kind='public'`); FKs were retargeted
`portal_users → users(id)` before the 00148 drop, so seeds are schema-consistent. Failures
are runtime status / auth-token-shape drift, not schema.

`favorites_authz_tests`, `imports_idor_tests`, `inquiries_authz_tests`, `inquiry_idor_tests`,
`portal_listings_idor_tests`, `saved_searches_authz_tests`.

## Recommended fan-out (for CTO)

| Child | Cluster | Binaries | One-shot lever |
|-------|---------|----------|----------------|
| BIT-354-C1 | A — RLS FORCE behavioral | ~30 (db) | shared NOSUPERUSER-role test helper |
| BIT-354-C2 | C — api-server authz split-brain | 14 | fix `seed_membership` → `user_memberships` + re-pin 403→400/401 |
| BIT-354-C3 | B — db seed/FK gaps | 7 | per-binary seed-column repair |
| BIT-354-C4 | E — reality-server authz | 6 | reality harness token/seed repair |
| BIT-354-C5 | D — webhook signature | 1 | stabilize env/body-byte handling |
| BIT-354-R1 | repo regression | — | `get_shares_rls` enum→text cast (security) |

Each FIX binary needs the ~50 min cold `cargo test --workspace` gate cycle to confirm green
before its `#[ignore]` is removed; batch carefully (a faster iteration host is tracked in
BIT-353).
