# Role: pm-data — 2026-10-07

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only. Previous run: 2026-07-23 (76 days ago).

**Summary:** Data docs exist for 5 areas (dispute KPIs, layout-publish events, Reality mobile-native analytics parity, support-data retention/audit events) but none cover the sprint's core epics — 6 Announcements, 7A Documents, 8A Notification Prefs, 10A OAuth. Reality mobile-native has open gaps on `listing.viewed`, `search.performed`, and `inquiry.submitted` for the native platforms. No data-stack PRs merged this window (9 PRs are i18n, screen-map reconciliation, dependabot).

## Next actions

1. **[high · rust-backend]** Write `docs/data/announcements-engagement-events.md`. Define events for announcement published/read/acknowledged, comment created, pin/unpin — trigger, properties (no PII), owner. Add KPIs: read rate, acknowledgment rate, time-to-ack. Source: `POST /{id}/read` and `/acknowledge` + existing `readCount`/`acknowledgedCount` stats.
2. **[high · rust-backend]** Write `docs/data/notification-delivery-kpis.md`. Cover channel opt-out rate, critical-override rate, sent-vs-delivered-vs-failed. Flag issue #484: the FCM stub swallows failures, so sent counts are falsely inflated. Build on migrations `00234`/`00235` (`held_notification_delivery_tracking`).
3. **[medium · none]** Write `docs/data/document-access-audit-events.md`. Cover upload, download, preview, share events (stories 7a-1, 7a-3, 7a-4, 7a-5) + access-denied counts as an IDOR signal. Define retention window; presigned-URL grants should be auditable.
4. **[medium · rust-backend]** Write an OAuth/MFA security-telemetry doc: token issue/revoke/introspect counts, refresh-reuse attempts, MFA failures, rate-limit hits. Add a log-redaction rule for tokens. Cross-link to issues #480, #481, #487 and `docs/data/support-data-retention-privacy.md`.
5. **[medium · mobile-native / reality-web]** Close the Reality mobile-native analytics gaps (`listing.viewed` on Android+iOS; `search.performed` and `inquiry.submitted` on web+Android+iOS). Turn `docs/data/reality-mobile-native-analytics-parity.md` into planner-sized tasks.
6. **[low · pm-scrum]** Reconcile `docs/EPIC_STORY_STATUS.md` against `sprint-status.yaml` — header still says 2026-05-25; sprint_name/goal list epics 6/7A/8A/10A but the file now carries epics 9/79/80/82/84/85 too; also fix the duplicate `80-2` key.

## Risks

- **Engagement/delivery event specs missing (Epics 6/7A/8A)** — probability high, impact medium. Adoption + notification KPIs cannot be measured, and tracking added late will not cover earlier usage. Mitigation: spec first; backend emits later; backfill from read/acknowledge timestamps.
- **FCM stub inflates sent counts (#484 open)** — probability high, impact high. 8a-3 marked done despite open gates #480 and #484. Mitigation: separate `delivered` metric; close/defer the two issues or revert 8a-3 and mark the delivery KPI provisional.
- **WS token in query param logged (#480)** — probability medium, impact high. Privacy leak into access logs. Mitigation: header or subprotocol auth + log scrub + redaction rule.
- **Layout-publish target_tenant always "*"** — probability medium, impact low. Blocks per-tenant dashboards. Mitigation: track as follow-up tied to per-tenant override-publish.
- **Reality mobile funnel metrics incomparable to web** — probability high, impact medium. Android/iOS lack listing.viewed, search.performed, inquiry.submitted. Mitigation: action 5. Until then label dashboards web-only.

## Open questions

- Is there a production analytics sink and event bus beyond the append-only tables and the frontend `trackEvent` helper? No vendor/warehouse definition found.
- Retention & PII policy for announcement read/acknowledge data and document download audit logs — only support-data defines retention.
- Who owns the KPI definitions: product or data? No central KPI registry file; only disputes.
- Any `docs/data/*` docs for Epic 80/84/85 (e-signature, price tracking, RAG) or realtor-analytics/notification-analytics screens? None found — are those screens reading live aggregates or stubs?
- Any data-stack PRs merged outside the 9 listed (e.g. `docs/data/*` or 00233–00235 migrations) that would change this picture?

## Decisions needed

- Choose the analytics sink + event transport (append-only DB tables vs. frontend `trackEvent` bus vs. third-party) so new event specs have a destination. Owner: architect.
- Decide whether to defer or close #480 and #484, and whether 8a-3 stays done. Owner: pm-scrum + rust-backend.
- Set retention windows and a PII policy for engagement/audit events (announcements, documents, notifications). Owner: security/privacy + pm-data.
- Approve adding data-oriented tasks (actions 1–5) to the dispatcher to help with low claimable count (16/72). Owner: planner.
