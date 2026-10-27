# code-review-mobile-rn-twofactor-stub-false-enabled

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review [mobile-rn] 2026-10-27 (signals/2026-10-27.json)
**Confidence:** high

## Hypothesis
`frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx` ships a UI scaffold that fakes 2FA enrollment: `handleVerify()` only checks a client-side `/^\d{6}$/` regex and then immediately `setStep('enabled')`, with no backend call anywhere in the component. Any six-digit number the user types makes the screen render "Two-factor authentication is enabled." — a false sense of account security — even though no TOTP secret was registered server-side. The fix is to wire the already-generated `AuthApi` 2FA endpoints (`/auth/me/2fa/setup` + `/auth/me/2fa/enable`) and render the real otpauth QR returned by setup, flipping the UI to `enabled` only after a 200 response from `enable`.

## Evidence
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:40` — `handleVerify()` runs only the local regex and does `setStep('enabled')`; no `fetch`, no api-client call anywhere in the component.
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:76` — the verify copy instructs the user to "Scan the QR code in your authenticator app", but no QR / otpauth secret is ever fetched or rendered.
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:4` — component docstring self-admits the stub nature: "The backend endpoints for enrollment aren't exposed through our mobile auth layer yet, so this screen wires up local state".
- `frontend/packages/api-client/src/generated/types.gen.ts` exports `AuthApiSetup2FaData` / `AuthApiEnable2FaData` / `AuthApiDisable2FaData` and `AuthTwoFactorSetupResponse` — the client already has the shape; the mobile screen just does not call it.
- `backend/servers/api-server/src/routes/admin/mfa/enroll.rs` — backend route generates the TOTP secret, encrypts, and stores `user_2fa` with `enabled = false` until the verification call lands; the contract the mobile screen needs is already there.

## Files
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx`
- `frontend/apps/mobile/src/App.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. In the mobile app (dev build), open `Profile → Security → Set up two-factor authentication`.
2. On the verify step, type any six digits (e.g. `123456`) and tap "Verify and enable".
3. **Expected:** the server rejects the code (no setup call was issued) and the UI stays on `verify` with an error.
4. **Actual:** the screen immediately renders "Two-factor authentication is enabled." despite no TOTP seed existing server-side.

## Suggested approach
1. In `TwoFactorScreen.tsx` add a `useEffect` on mount that calls `AuthApi.setup2Fa` and stores `{secret, otpauthUrl}` in local state; keep the `idle` step visible until setup resolves (show a spinner).
2. Replace the hand-written "Scan the QR code" help block with a real QR rendered from `otpauthUrl` (import `react-native-qrcode-svg`, already a transitive dep via Expo, or add it if missing).
3. Rewrite `handleVerify()` as an async handler that calls `AuthApi.enable2Fa({ code })`; map the response:
   - 200 → `setStep('enabled')`
   - 400 `invalid_code` → set `error = t('auth.invalid2faCode')`, stay on `verify`
   - other errors → set `error = t('common.errorRetry')`, stay on `verify`.
4. Guard the API client with the existing auth-token provider (same hook used in `LoginScreen.tsx`); do not fetch raw.
5. On a `Disable` press from `step === 'enabled'`, call `AuthApi.disable2Fa` and reset to `idle`.
6. Add explicit loading state (`isSubmitting`) so the "Verify and enable" button disables while the call is in flight.
7. Delete the "backend endpoints for enrollment aren't exposed..." sentence from the component docstring once the wiring lands — the docstring is a provable lie after this change.

## Alternatives considered
- **Leave the regex-only client stub and only fix copy** — rejected because the component actively lies about security state; the fix is "make it real", not "soften the lie".
- **Hide the screen behind a feature flag and leave the stub** — rejected because `App.tsx:232` already routes real users to it; hiding without wiring just moves the false claim and leaves the backend endpoints uncalled from mobile.

## Root-cause trace
1. Symptom: user sees "Two-factor authentication is enabled." after entering any six-digit number; `user_2fa` row in the DB stays `enabled = false`.
2. ← `TwoFactorScreen.tsx:46` — `setStep('enabled')` is unconditional on regex pass.
3. ← `TwoFactorScreen.tsx:40` — `handleVerify()` has no network call; the component was shipped as a UI scaffold intentionally per its docstring at line 4.
4. Origin: initial commit of the screen (pre-dates this routine's cursor) landed with the "backend endpoints … aren't exposed through our mobile auth layer yet" docstring; the enrollment endpoints have since been generated into `@ppt/api-client` (`AuthApiSetup2FaData` / `AuthApiEnable2FaData`), but the mobile screen was never revisited.

## Test plan
- [ ] Add `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.test.tsx` — mock `AuthApi.setup2Fa` + `AuthApi.enable2Fa`; assert:
  - idle → verify transition fires setup and surfaces the otpauth QR
  - entering a 6-digit code calls `enable2Fa` once with that code
  - 200 response flips to `enabled`; 400 keeps `verify` and renders the error
  - no backend call ⇒ no step transition (regression test for the current stub bug)
- [ ] Regression scenario: integration test in `src/test/integration/auth.integration.test.tsx` adds a case asserting the screen never flips to `enabled` when the enable API returns 400.
- [ ] Local command: `pnpm --filter mobile test TwoFactorScreen` (jest-expo).

## Out of scope
- Backend-side changes (endpoints already exist; this plan is purely wiring).
- Backup-code UI / recovery flow — tracked separately if the backend adds it.
- 2FA UX on ppt-web (different app, different plan if needed).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-twofactor-stub-false-enabled.md`
- Mark the matching `backlog.json` row as `status: "done"`
