# pm-security — 2026-09-04

_Run mode: rotating (pm_cursor idx 5 → 6). Focus: fresh scan of reality-web self-serve credential flows + standing dep-security items._

## Summary

Sprint remains at 47/49 done. This run's security lens found **three dead client-side password flows** in reality-web — `/account/password`, `/auth/forgot-password`, and `/auth/reset-password` all render UI, wire form submit, and call auth-api client stubs (`changePassword`, `requestPasswordReset`, `confirmPasswordReset`) that unconditionally throw `AuthApiError(501, 'NOT_IMPLEMENTED')`. Traced source→UI call path is fact-grade. Confidence: high. Blast radius: any real portal user attempting to reset or change their password sees a hard "not available" mid-flow; no server-side endpoint to hit. Standing standing dep-security item **gh-issue-2797 (RUSTSEC-2026-0258 h2 empty-DATA-frame DoS)** unchanged — still blocks backend PRs on transitive `h2` dependency.

## next_actions

- Wire the 3 reality-web password client stubs (`requestPasswordReset` / `confirmPasswordReset` / `changePassword`) to reality-server endpoints (`POST /api/v1/users/password-reset`, `POST /api/v1/users/password-reset/confirm`, `PUT /api/v1/users/me/password`), OR hide the `/account/password` + `/auth/forgot-password` + `/auth/reset-password` entries behind a feature flag until the server-side lands. Owner: pm-security. Priority: high. Dependency: none. DoD: at least one of the 3 UI pages is either backed by a real API call OR is unreachable from the account nav.
- Resolve **gh-issue-2797 (RUSTSEC-2026-0258 h2 DoS)** — blocks every backend PR when cargo-deny is enforced. Owner: pm-security. Priority: high. Dependency: none. DoD: transitive `h2` bumped (or `cargo-deny.toml` ignore w/ named risk-owner).

## risks

- reality-web password-flow stubs throw 501 on 3 core user-facing auth surfaces — user attempts to reset/change password will always fail (probability high; impact medium — auth flow is degraded but not exploitable). Mitigation: wire or gate the UI (see next_action).

## open_questions

- Is reality-server intended to own the password-reset endpoints, or should the portal defer entirely to api-server SSO? (Choice determines whether we wire client stubs to reality-server or hide the UI.)

## decisions_needed

- **Password-reset ownership: reality-server vs. api-server SSO delegation** — owner: pm-tech-lead.
