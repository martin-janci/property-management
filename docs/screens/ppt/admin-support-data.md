---
id: ppt/admin-support-data
name: Support Data (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: platform/support-data
    component: SupportDataPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/admin-platform-health
    rel: sibling
  - id: ppt/admin-oauth-clients
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-17
epics:
  - Epic-10B
designSources: []
owner: pm-security
---

## Functionality Checklist

- [w] User counts section: total / active / pending / suspended stat cards
- [w] Active sessions stat card (count of non-revoked, non-expired refresh tokens)
- [w] Fault summary: total faults stat card + fault-by-status table (count + % of total)
- [w] Manual Refresh button (re-runs `useSupportData`)
- [w] Error banner if `get_support_data` fails
- [w] Access gated by `audit_read` capability (mirrors backend `AuditRead` + SuperAdmin)

## States

- **Loading**: "Loading…" text under each section heading while the query is in flight
- **Error**: Red alert banner with i18n key `admin.supportData.loadError`
- **Empty faults**: "No faults recorded." message in the fault table

## Notes

### Broader context

Part of Epic 10B Story 10B.5 — the super-admin support-tooling overview. Reads
only aggregate, non-PII counts; the per-user PII endpoints
(`support/users/{id}/...`) and the session-revoke action are documented in
`docs/data/support-data-retention-privacy.md`. Backend route lives at
`GET /api/v1/platform-admin/support-data`
(`backend/servers/api-server/src/routes/platform_admin/audit.rs`), gated by
`RequireCapability(AuditRead)` plus `extract_super_admin_token`. The repository
`get_support_data` runs five counting queries inside a single `REPEATABLE READ`
transaction for a consistent snapshot (#628). Every read emits an append-only
`support_data_viewed` event into `support_tooling_events` (migration 00163,
RLS-hardened in 00165) so support-tooling access is itself audited.

### Specific (recent)

- 2026-06-25 — agent: dispatcher followup (PR #1829) — cleared the `endpoints`
  frontmatter: `get_support_data` is not yet registered in `@ppt/sitemap`, so
  screen-map strict validation rejected it. The route stays documented under
  Notes > Broader context. Follow-up: add `get_support_data`
  (`GET /api/v1/platform-admin/support-data`) to
  `frontend/packages/sitemap/src/data/api-server/index.ts` (alongside the other
  Epic-10B platform-admin endpoints), then restore the endpoint ref here.
- 2026-06-24 — agent: 10b-5-support-data-access — fixed a latent runtime bug:
  the support-data session queries referenced a non-existent `is_revoked` (and
  `updated_at`) column on `refresh_tokens`; corrected to `revoked_at IS NULL` /
  `SET revoked_at = NOW()` to match the schema (migration 00002). Added a
  `#[sqlx::test]` regression and reconciled sprint-status 10B.5 to done.

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-10B-5 → Epic-10B (strip story suffix); /screens validate clean.

<!-- newest entries on top -->
- 2026-06-25 — agent: dispatcher followup — fixed PR #1829 red CI gates:
  ran rustfmt on `support_data_session_columns_tests.rs` (clears check / lint /
  fmt-clippy, which all gate on `cargo fmt --check`) and cleared the
  unverifiable `get_support_data` endpoint ref (clears screen-map strict
  validation). Endpoint still documented in Notes.
- 2026-06-24 — agent: 10b-5-support-data-access — created screen-map (was an
  orphan epic); verified backend wiring (transactional reads, AuditRead +
  super-admin gate, append-only audit trail), fixed `refresh_tokens` column
  mismatch in `PlatformAdminRepository`, documented retention/privacy posture
  delta (FK now ON DELETE RESTRICT), and flipped sprint-status to done.
