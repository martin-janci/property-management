# pm-integration — 2026-05-29

_Rotating role this run (cursor idx 7). Static analysis only._

## Summary

The integration surface carries three open correctness/reliability defects — Airbnb at-least-once dedup (duplicate `SYNC_EXTERNAL` jobs), the Redis push-fanout queue never drained (silent drop), and the marketplace install/OAuth UI still stubbed — against a sprint background of active OAuth-provider work (epic-10a), e-signature signerParties (PR #719), and a Dependabot **sqlx 0.8→0.9** major bump (PR #666) that touches every repository in the workspace. OAuth security gates (issue #481 revoked-token bypass, #487 MFA rate-limit) still block 10a-1/10a-3 from done, and the e-signature webhook handler has no idempotency guard on status transitions.

## Next actions

| Priority | Action | Owner | Dependency |
|---|---|---|---|
| high | Add idempotent enqueue for Airbnb `SYNC_EXTERNAL` (event_id dedup table + already-queued check before `CreateBackgroundJob`) — closes backlog `bug-webhook-airbnb-dup-sync-jobs` (webhook.rs:1000–1042) | pm-backend | rust-backend |
| high | Fix OAuth refresh-token revocation bypass (#481): restore the `revoked_at IS NULL` predicate; gates 10a-1/10a-3 | pm-backend | rust-backend |
| high | Implement Redis BLPOP drain in `PushFanoutWorker` (push_fanout.rs:621) — closes backlog `dx-push-fanout-blpop-drain` | pm-backend | rust-backend |
| high | Assess + schedule sqlx 0.9 migration (PR #666): audit query!/migrate breakage, plan workspace upgrade before merge | pm-backend | rust-backend |
| medium | Wire `IntegrationMarketplacePage` install flow + OAuth URL nav (TODO stubs at IntegrationMarketplacePage.tsx:234,238) — closes `dx-integration-marketplace-stubs` | pm-frontend | react-web |
| medium | Add terminal-state idempotency guard to the e-signature webhook (skip update when current status is completed/voided/declined) | pm-backend | rust-backend |

## Risks

- **sqlx 0.8→0.9 major bump (PR #666)** — affects every workspace query; merging without a coordinated upgrade may break compile-time query! checks / migrate API and block all backend CI. _Prob medium · Impact high._ Mitigation: freeze from auto-merge; `cargo check --workspace` on a branch first.
- **OAuth refresh-token revocation bypass (#481)** — revoked tokens remain usable; hard RFC-9700 violation on the in-progress OAuth provider. _Prob high · Impact high._ Mitigation: block 10a-1/10a-3 promotion (already gated); fix before any external exposure.
- **Redis push-fanout silent no-op** — jobs enqueued to `push_fanout_queue` are never drained; Epic 8A stories marked done with the gap open. _Prob high · Impact medium._ Mitigation: implement BLPOP drain + queue-depth alert before production reliance.
- **Airbnb webhook duplicate jobs** — at-least-once bursts create redundant sync load + possible reservation-state races. _Prob high · Impact medium._ Mitigation: event_id dedup table (ON CONFLICT DO NOTHING).
- **e-signature webhook no idempotency** — provider re-delivery can overwrite a terminal state. _Prob medium · Impact medium._ Mitigation: terminal-state guard before the workflow update; coordinate with PR #719.

## Open questions

- Does the Airbnb integration emit `event_id` on all event types (code guards on `Option<event_id>`), or only on reservation events? The dedup strategy depends on it being reliable.
- OAuth Airbnb-callback state verification (oauth.rs:79) uses a stateless `split(':')` with no CSRF-nonce expiry — is a server-side state store planned for epic-10a?
- sqlx 0.9 (PR #666): has a breaking-change audit run? 0.9 changed `query!` expansion + the `migrate` API surface.
- e-signature signerParties (PR #719): are DocuSign/Adobe/HelloSign webhook contracts for the new manager/landlord roles validated against provider sandboxes, or modelled from docs only?
- Booking.com push validation regression tests are missing (backlog `test-gap-booking-push-validation-untested`) — who owns the install.rs `BATCH_TOO_LARGE`/`INVALID_AVAILABLE_COUNT` test?

## Decisions needed

- Accept or defer the sqlx 0.9 upgrade (PR #666) this sprint — owner: pm-backend / tech-lead.
- Airbnb dedup: DB-migration event_id table vs worker-level idempotent upsert as the canonical fix — owner: pm-backend.
- Must the Redis push-fanout drain ship before Epic 8A is considered production-shippable? (sprint marks 8A done, but the fanout path is a logging no-op) — owner: pm-product / pm-backend.
