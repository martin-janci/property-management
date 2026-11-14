# pm-security — 2026-11-14

## Summary

Static check only (sprint-status.yaml, project-state and the Dependabot/E2E PRs were not read in this pass). #3427 is still open in code: neither access-token validator checks `iss`/`aud`, and minted tokens carry no `iss`/`aud` claims. The ppt-web route-guard work from #3486/#3482 looks widely applied (ProtectedRoute appears ≈383× across 64 files), but the #3505 E2E failure and whether all four #3486 route groups are guarded were not verified in this pass.

## Next actions

1. **[high]** Implement #3427: have `JwtService` emit `iss`/`aud` (per service) and enforce `set_issuer`/`set_audience` in BOTH `api_core::extractors::auth::jwt_verifier` (`auth.rs` ≈L71–76) and `JwtService::validate_access_token`/`validate_refresh_token` (`services/jwt.rs` ≈L198–224), which are separate copies of the validation. Also check `PrincipalClaims` and reality-server verifiers. Roll out with a dual-accept window so tokens already issued without iss/aud do not log users out. DoD: tokens carry iss/aud; all verifier copies reject wrong or missing iss/aud; a cross-service token-replay test (api-server token sent to reality-server and the reverse) fails; the #3427 `blocked` label is cleared or the blocker is documented.
2. **[high]** Triage #3505/#3507 as an auth-guard regression. Confirm the ProtectedRoute gating from #3482/#3486 did not break legitimate flows. Fix the E2E fixtures (login/session seeding) rather than loosening guards. Add a route-test matrix asserting unauthenticated access to community, disputes, messaging, neighbors and `/settings/privacy` redirects to login. DoD: E2E green; `mgmt-routes-auth-guard.route.test.tsx` covers every newly gated path with both unauthenticated (redirect) and authenticated (renders) cases. No guard removed to make tests pass.
3. **[medium]** Before merging the jsonwebtoken 11 Dependabot PR (`backend/Cargo.toml` currently pins 10 with `rust_crypto`), review its changelog for Validation defaults (exp/nbf/aud/algorithm handling) and the crypto-backend features. Do it after or together with #3427, and keep the PR gated on auth tests that cover expired, wrong-alg, wrong-secret and wrong-aud tokens. DoD: changelog reviewed and the bump merged, or deferred with a reason; token tests above pass on 11.

## Risks

- **medium/high** — No `iss`/`aud` binding (#3427). A token minted by one service or environment sharing JWT_SECRET is accepted by any other. The comments in `auth.rs` and `jwt.rs` say iss/aud are not emitted. Both validators must change together or they will diverge. Mitigation: ship #3427 with distinct per-service / per-environment secrets as a fallback control, and use the dual-accept rollout.
- **medium/high** — The community, disputes, messaging, neighbors and `/settings/privacy` routes were client-side unguarded before #3486 and #3482. ProtectedRoute is only UX gating. If backend endpoints for those features lack role/tenant checks or RLS, the exposure is real regardless of the SPA fix. Whether the backend enforces them is unverified. Mitigation: audit the backend handlers and RLS for disputes, messaging, neighbors and privacy settings — auth, tenant scoping, role checks — and confirm they were not relying on SPA guards.
- **low/medium** — Stalled Dependabot majors. `jsonwebtoken` 11 sits on the auth path and a stale pin delays advisories, but we are already on 10, so exposure is modest. Rebasing against churn in the auth-guard tests could silently weaken coverage. `utoipa-swagger-ui` 10 and `vitest` 5 are lower risk, but swagger-ui is exposed at `/swagger-ui`. Mitigation: schedule the `jsonwebtoken` bump after #3427; check that `/swagger-ui` is disabled or gated in production.

## Open questions

- What is the exact blocker on #3427 (design decision, rollout concern, or capacity), and what did the #3480 triage conclude?
- What is the root cause of the #3505 E2E failure: stale fixtures, or a guard blocking a flow that should be public?
- Do the backend endpoints behind the community, disputes, messaging and neighbors routes enforce authentication, tenant scoping and role checks independently of the SPA?
- Do api-server and reality-server share `JWT_SECRET`, and is it shared across environments? This determines the cross-service replay exposure.
- Is `/swagger-ui` enabled in production builds?

## Decisions needed

- Decide the iss/aud values and rollout strategy for #3427 — dual-accept window vs hard cutover that forces re-login — owner: pm-scrum-master / tech-lead.
- Decide whether to split JWT secrets per service or environment as an interim control — owner: tech-lead / DevOps.
