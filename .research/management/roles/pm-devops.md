# pm-devops — DevOps / Infrastructure Read

_Run: 2026-05-26 (PM rotation, rotating role)_

## Summary

This window landed substantial infra/CI cleanup (#552) and three new always-on backend workers/integrations — push-fanout (#515), Booking.com channel sync (#534/#544), and Airbnb backend (#538) — none of which yet ship with observability, secret-handling gates, or a rollback path. The deploy surface grew (third-party OTA credentials + background workers) faster than the operational tooling around it.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Add CI secret-scanning + env-scoped secrets handling for new OTA channel credentials (Booking.com/Airbnb keys touched in `integrations/install.rs`) | pm-security | CI fails on cred strings; no creds in repo or structured logs |
| high | Add metrics/logging + dead-letter + backoff monitoring for the push-fanout worker (`push_fanout.rs`) | pm-backend | Fanout success/fail rate observable; alert on error threshold before mobile push GA |
| medium | Define rollback/runbook + feature-flag gating for channel-sync workers (`booking_channel.rs`) | none | A bad OTA push can be disabled without a redeploy |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| OTA channel credentials leak into logs/repo (`install.rs` is a churn hotspot) | medium | high | CI secret-scan gate + env-scoped secret store |
| Push-fanout worker fails silently (no observability) | medium | medium | Per-batch metrics + dead-letter + alerting |
| Channel-sync workers have no kill-switch short of redeploy | low | high | Feature-flag per channel + rollback runbook |

## Open questions

- Are Booking.com/Airbnb credentials stored via the existing secret store, or inline in env/config?
- Does the push-fanout worker have retry/dead-letter semantics, or is a failed batch dropped?
- Is there a per-channel feature flag to disable an OTA sync in prod without a deploy?

## Decisions needed

- Adopt a CI secret-scanning gate for integration credentials — owner: pm-devops/pm-security.
