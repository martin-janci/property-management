# security-jwt-iss-aud-cross-service

**Vector:** security
**Score:** 3
**Source:** triage of issue #3427 (spun out of code-review task `code-review-api-core-jwt-no-aud-iss`) · 2026-11-13 · pm-scrum-master
**Confidence:** high

## Triage verdict (issue #3427)

Issue #3427 carries labels `security, follow-up, blocked`. The triage question was: does
`blocked` point to a **real dependency** or does it just **await prioritisation**?

**Verdict: awaits a human design decision — NOT a code/task dependency.** No other in-flight
PR or backlog item blocks this work. The `blocked` label is being used deliberately so the
dispatcher's issue-ingest does not auto-claim a security-sensitive, cross-service change and
ship it autonomously. The one true blocker is a *design choice a human must sign off*:

1. **Canonical identifiers** — what `iss` and `aud` values each service mints (they must be
   distinct per issuer for the binding to be meaningful).
2. **Refresh-token migration window** — binding `aud`/`iss` without a grace period invalidates
   every in-flight refresh token on deploy, forcing a fleet-wide re-login.

Both are deliberate, security-sensitive decisions that the original authors deferred (three
consistent in-code maintenance-contract comments). **Outcome: PROMOTE to this plan (the
finding is valid and actionable), but the first commit of any implementation MUST NOT land
until the canonical-identifier decision is signed off — autonomous auto-claim stays off.**

## Hypothesis
The api-core JWT validators are missing `set_audience` / `set_issuer`, **but** no first-party
token mints `iss`/`aud` and no config defines expected values — so there is nothing to assert
against today. The genuine risk is **audience confusion**: api-server and reality-server mint
JWTs with the **same shared `JWT_SECRET`** and neither stamps an audience, so a reality-portal
token is structurally replayable at api-server. The correct fix is cross-service: mint distinct
`iss`/`aud` at *both* issuers, then enable binding at *all* verifiers, behind a refresh-token
grace window. Enabling `set_issuer`/`set_audience` in api-core alone would make `jsonwebtoken`
reject every existing token (`InvalidAudience`/`InvalidIssuer`) and break all authentication —
which is exactly why this must not be a one-sided api-core change.

## Evidence (verified on `dev` @ 2026-11-13)
- Issuer `backend/servers/api-server/src/services/jwt.rs` — `Claims` has fields
  `sub/exp/iat/jti/token_type/org_id/roles/kind` and **no `iss`/`aud`**;
  `generate_access_token_with_kind` / `generate_refresh_token` emit neither (HS256).
- Verifier `backend/crates/api-core/src/extractors/auth.rs:71-77` — `Validation::default()` +
  `leeway = 30`, with the comment: *"iss/aud are not currently emitted by JwtService — when
  they are added, set_audience / set_issuer here to bind them."* (confirmed present).
- Verifier `backend/crates/api-core/src/extractors/principal.rs:112-114` — *"We do NOT enforce
  any audience/issuer here — those are issuance concerns…"*
- Issuer validator `jwt.rs:192-197` — maintenance-contract comment: future aud/iss hardening
  MUST be applied to `validate_access_token` / `validate_refresh_token` as well.
- `backend/servers/reality-server/src/state.rs:367-395` — mints a `{ sub, kind, iat, exp }`
  token signed with `self.jwt_secret` (the **same** `JWT_SECRET`); no `iss`/`aud`.
- Config sweep: **no** `JWT_ISSUER` / `JWT_AUDIENCE` anywhere for first-party tokens. The only
  `set_audience`/`set_issuer` calls are external-consumer concerns —
  `backend/servers/deploy-server/src/auth/oidc.rs:120-121` (deploy OIDC) and
  `backend/servers/api-server/src/routes/voice_webhooks.rs:1448-1449` (Google issuers). Only
  `BASE_URL` / `API_BASE_URL` exist and could serve as issuer.

## Files
- `backend/servers/api-server/src/services/jwt.rs` — emit `iss`+`aud` on access + refresh; bind in `validate_access_token`/`validate_refresh_token`.
- `backend/crates/api-core/src/extractors/auth.rs` — `set_issuer`/`set_audience` on the shared verifier.
- `backend/crates/api-core/src/extractors/principal.rs` — align with the binding decision.
- `backend/servers/reality-server/src/state.rs` — emit `iss`+`aud` on the reality token mint.
- `backend/crates/api-core/src/…` (new) — shared `iss`/`aud` consts referenced by both issuers.
- Config / `.env.example` — expected `iss`/`aud` (or derive from the namespace table).

## Dependencies
- **Human design sign-off (hard gate):** canonical `iss`/`aud` identifiers + refresh-token
  migration window. This is a prioritisation/design dependency, not a code dependency on
  another task. Until signed off, keep `blocked` and do not auto-claim.

## Required capabilities
- [x] C1 — Systematic debugging (security / cross-service auth change)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser · **local-only**
- [ ] C5 — ADB device · **local-only**
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security-sensitive, controversial — forced re-login)

**Execution mode (auto-derived):** no C4/C5 → cloud-ok for the *code*, but **gated on human
design sign-off before the first commit lands**.

Mode: cloud-ok (blocked on design decision)

## Repro steps (audience-confusion risk, demonstrating why the fix is warranted)
1. Obtain a valid reality-portal token from reality-server (`{ sub, kind, iat, exp }`, signed with the shared `JWT_SECRET`).
2. Present it to an api-server endpoint that authenticates via the api-core `JwtVerifier`.
3. Expected (post-fix): rejected with `InvalidAudience` (reality token has `aud=…reality`, api-server expects `aud=…management`).
4. Actual (on `dev`): the api-core verifier runs `Validation::default()` (sig + `exp` only), so the token is accepted on structural grounds — no audience boundary exists between the two services.

## Suggested approach (promoted from the issue's proposed plan)
1. **Decide canonical identifiers** (HUMAN — the gate). Per the CLAUDE.md namespace table:
   `iss = three.two.bit.ppt`; api-server `aud = three.two.bit.ppt.management`; reality-server
   `aud = three.two.bit.ppt.reality`. Add shared consts in api-core referenced by both issuers
   (avoids drift; satisfies the code-reuse concern).
2. **Emit** `iss` + `aud` in `JwtService` (access + refresh) and in reality-server's token mint.
3. **Enable binding** — `set_issuer` / `set_audience` in api-core `auth.rs`, `principal.rs`, and
   `JwtService::validate_access_token` / `validate_refresh_token`, honoring the three existing
   maintenance-contract comments. Add wrong-aud / wrong-iss reject + correct-accept tests.
4. **Refresh-token migration window** — accept-missing-aud grace (verify with a validation that
   does not require `aud` for a deploy window, or dual-validate) so in-flight refresh tokens do
   not force an immediate fleet-wide re-login; remove the grace in a follow-up after the window.

## Alternatives considered
- **api-core-only `set_issuer`/`set_audience`** — rejected: no token emits the claims, so
  `jsonwebtoken` rejects every token and breaks all auth. This is the trap that made the
  original code-review implementer return `blocked`.
- **Distinct `JWT_SECRET` per service instead of `aud`** — would also stop cross-service replay,
  but is a bigger operational change (key rotation, two secrets in every env) and does not give
  the standard `aud`/`iss` assertions downstream tooling expects. Prefer the `aud`/`iss` route;
  revisit per-service secrets separately if desired.
- **Hard cutover (no grace window)** — rejected: invalidates in-flight refresh tokens on deploy,
  a fleet-wide forced re-login spike.

## Root-cause trace
1. Symptom: a reality-portal JWT is structurally acceptable at api-server (shared secret, no audience boundary).
2. ← Both issuers (`api-server/src/services/jwt.rs`, `reality-server/src/state.rs`) mint tokens with no `iss`/`aud`.
3. ← Every first-party verifier uses `Validation::default()` (sig + `exp` only); the binding was deliberately deferred (three in-code comments) pending a canonical-identifier + migration decision.
4. Origin: first-party JWT scheme shipped without per-issuer audience identifiers; hardening deferred as a cross-service design decision rather than an api-core-local fix.

## Test plan
- [ ] Unit: `JwtService` tokens carry the expected `iss`/`aud`; `validate_*` accept correct and reject wrong `iss`/`aud` (`InvalidIssuer`/`InvalidAudience`).
- [ ] Unit: api-core verifier (`auth.rs`, `principal.rs`) rejects a reality-`aud` token at an api-server-scoped verifier and accepts a management-`aud` token.
- [ ] Migration-window: a legacy token with no `aud` is still accepted while the grace flag is on, rejected once off.
- [ ] Command: `cargo test -p api-core jwt` and `cargo test -p api-server jwt` (place cross-service case under `backend/servers/api-server/tests/`).

## Out of scope
- Rotating to per-service `JWT_SECRET`s (tracked separately if pursued).
- deploy-server OIDC and Google voice-webhook validation (already set `aud`/`iss`; external-consumer concerns, unaffected).

## After-merge
- Move this file to `plans/_archive/security-jwt-iss-aud-cross-service.md`.
- Close issue #3427 (remove `blocked` once the design decision is recorded) and mark the matching `backlog.json` row `status: "done"`.
- Schedule the grace-window removal follow-up after the refresh-token migration window elapses.
