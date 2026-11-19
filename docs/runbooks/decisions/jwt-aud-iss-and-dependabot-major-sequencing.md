# Decision: Sequence JWT iss/aud hardening (#3427) and the three Dependabot majors behind the swagger-ui egress fix

| Status | Accepted |
| --- | --- |
| Date | 2026-11-16 |
| Area | DevOps / Security / Dependencies |
| Owners | pm-scrum-master (sequencing); pm-security / pm-backend (#3427 + #3444); pm-devops (#3443); pm-frontend (#3447) |
| Affected PRs / issues | #3427 (JWT iss/aud), #3444 (jsonwebtoken 10→11), #3443 (utoipa-swagger-ui 9→10), #3447 (vitest 4→5), #2966 / #2944-2946 (swagger-ui egress fix) |
| Affected files (this change) | `docs/runbooks/decisions/jwt-aud-iss-and-dependabot-major-sequencing.md` (this note only) |

## Context

Four security/dependency items are open at once and three of them touch the same
two hot paths (the backend JWT mint/verify chain and the backend OpenAPI/Swagger
surface). Landing them in an arbitrary order risks either a build-breaking
collision or a double disruption of the auth hot path. This note records the
**order** and the **pairing** — it does **not** perform any of the bumps or the
hardening, which are separate, larger, individually-reviewed tasks.

The four items:

- **#3427 — Harden first-party JWT: emit + validate `iss`/`aud`.** Spun out of a
  code-review finding and deliberately labeled `blocked`: the api-core validators
  omit `set_issuer`/`set_audience`, but no first-party token mints `iss`/`aud` and
  there is no config of expected values, so enabling validation in api-core alone
  would reject **every** token and break all auth. The real risk is aud-confusion
  — api-server and reality-server mint JWTs with the **same shared `JWT_SECRET`**,
  so a reality-portal token can be replayed at api-server. A meaningful fix is
  cross-service (distinct identifiers per issuer, emit at both mints, enable
  validation in `auth.rs` + `principal.rs` + `JwtService`) and needs a
  refresh-token grace window to avoid a fleet-wide forced re-login. It needs a
  human design decision before any code lands.
- **#3444 — jsonwebtoken 10→11.** A Dependabot major with real breaking changes
  on the exact APIs the JWT chain uses: `EncodingKey.inner` → `as_bytes`,
  `DecodingKey.as_bytes`/`try_get_hmac_secret` removed (use `try_get_as_bytes`),
  `Validation.insecure_disable_signature_validation` removed, and several enums
  (`Algorithm`, `KeyAlgorithm`, …) now `non_exhaustive`.
- **#3443 — utoipa-swagger-ui 9→10.** A Dependabot major whose 10.0.0 line changes
  exactly the machinery the egress fix depends on: "Add custom embedding for
  Swagger UI assets", a bundled Swagger UI bump to 5.32.6, and a `zip` crate bump
  to 9.0x. The backend pins `utoipa-swagger-ui = { version = "9", features =
  ["axum", "vendored"] }` specifically so the build script never fetches the
  swagger-ui zip from github.com at build time (the **swagger-ui egress fix**,
  #2966 / #2944-2946), which is what keeps the backend buildable in
  egress-restricted cloud runners.
- **#3447 — vitest 4→5.** A frontend dev-dependency major (test runner only),
  pinned as `vitest ^4.1.11` across the `frontend/` workspace. No coupling to the
  backend JWT or Swagger surfaces.

## Decision

Land the work in this order; the first three are a **serial backend chain**, the
fourth is an **independent parallel track**:

| # | Item | When | Why this slot |
| --- | --- | --- | --- |
| 0 | **swagger-ui egress fix** (#2966 / `vendored` feature) | Prerequisite — confirm stable on `dev` first | Everything downstream assumes the backend builds egress-free. Do not disturb the Swagger surface until this is confirmed green in restricted runners. |
| 1 | **utoipa-swagger-ui 9→10** (#3443) | After the egress fix is confirmed | v10 changes asset embedding + the `zip` dep. Re-validate the `vendored`/egress-free build in a restricted runner **as the acceptance gate** for this bump, so a regression surfaces here rather than later in an unrelated PR. |
| 2 | **jsonwebtoken 10→11** (#3444) **paired with** **JWT iss/aud hardening** (#3427) | After #3443 | Both rewrite the same auth hot path (`services/jwt.rs`, `extractors/auth.rs`, `extractors/principal.rs`). Pair them so **one** security review and **one** forced-re-login window cover both, instead of churning the auth chain twice. |
| 3 | **vitest 4→5** (#3447) | Any time — parallel | Frontend-only dev dependency, no overlap with the backend chain. Sequencing it against the others would only add artificial delay. |

### Why #3444 and #3427 are paired (not merely adjacent)

- They edit the **same files**. #3444 is a mechanical API migration of the mint
  (`EncodingKey`) and verify (`DecodingKey`, `Validation`) call sites; #3427 adds
  `iss`/`aud` emission at those same mints and `set_issuer`/`set_audience` at those
  same verifiers. Doing them separately means touching `jwt.rs` / `auth.rs` /
  `principal.rs` twice, with two reviews and two merge-conflict windows.
- They share a **blast radius**. #3427 already forces a refresh-token migration
  window (accept-missing-aud grace) and a fleet-wide re-login risk; folding the
  jsonwebtoken major into the same change means the auth hot path is destabilized
  **once**, behind one coordinated security review and one deploy window.

### Decoupling fallback (if #3427 stays blocked)

#3427 is `blocked` pending a human design decision (canonical `iss`/`aud`
identifiers per the CLAUDE.md namespace table, plus the grace-window plan). If
that decision is not ready when the queue reaches slot 2, **#3444 may proceed
alone** as a pure mechanical API migration — it does not depend on the iss/aud
design. Pairing is the preferred path (one disruption); independent landing of
#3444 is the acceptable fallback so a routine dependency bump is not held hostage
to an open design question. The reverse is **not** allowed: #3427 must not land
without #3444 having been resolved first, because enabling validation on top of a
not-yet-migrated jsonwebtoken API doubles the auth-path churn this ordering exists
to avoid.

### Why vitest is independent

#3447 is a `devDependencies` bump of the test runner in the `frontend/` pnpm
workspace. It cannot affect the backend build, the egress posture, or the JWT
chain. It carries its own normal major-bump risk (config/matcher changes across
`accounting-web`, `admin-web`, `ppt-web`, `reality-web`, and the api-client
packages) and should be validated by `pnpm test` across those packages, but on
its own schedule.

## Consequences

- The backend security/deps work is a single serial lane with a clear gate at
  each step; a Swagger/egress regression is caught at #3443's acceptance rather
  than diffused into a later PR, and the auth hot path is reviewed and redeployed
  once rather than twice.
- #3444 is unblocked from #3427's open design question via the documented
  fallback, so the jsonwebtoken major does not stall indefinitely.
- vitest can land whenever the frontend owner is ready, in parallel, without
  coordination overhead.
- No code, dependency manifest, or lockfile is changed by this note. The
  `backend/Cargo.toml` pins (`utoipa-swagger-ui = "9"`, `jsonwebtoken = "10"`) and
  the `frontend` `vitest ^4.1.11` pins are intentionally left untouched here.

## Follow-up (open items — each a separate task)

1. **pm-devops / pm-backend — #3443.** Bump utoipa-swagger-ui to 10, then prove
   the `vendored`/egress-free build still holds in a restricted runner before
   merge. Update the comment block in `backend/Cargo.toml` (lines ~47-51) if the
   vendored mechanism changed under v10.
2. **pm-security / pm-backend — #3444 + #3427.** First settle the #3427 design
   (canonical `iss`/`aud` constants shared in api-core; per-issuer `aud` —
   `three.two.bit.ppt.management` for api-server, `three.two.bit.ppt.reality` for
   reality-server; refresh-token grace window), then land the jsonwebtoken-11 API
   migration and the iss/aud emit+validate together behind one security review, or
   land #3444 alone per the decoupling fallback if #3427 is still blocked.
3. **pm-frontend — #3447.** Bump vitest to 5 and run `pnpm test` across the
   frontend workspace; land independently.
