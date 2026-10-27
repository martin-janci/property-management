# code-review-mobile-rn-twofactor-stub-false-enabled

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review segment=mobile-rn (signals/2026-10-27.json)
**Confidence:** medium

## Hypothesis

`frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx` claims the user's account has 2FA enabled based only on a client-side `/^\d{6}$/` regex match — there is no backend enrollment call, no TOTP verification call, no `otpauth://` secret rendered, and no QR code even though the UI instructs the user to "Scan the QR code in your authenticator app". The component docstring at line 4 explicitly admits the backend endpoints are not wired through the mobile auth layer. Shipped as-is, any 6-digit number sends the user to the "Two-factor authentication is enabled." screen while nothing was ever registered server-side; the user is given a false sense of account hardening. The minimal fix is to gate the screen behind a `comingSoon` state (or hide the entry nav target) until the real enroll/verify endpoints are wired — do NOT keep rendering a success state driven by local regex.

## Evidence

- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:40` — `handleVerify()` runs `/^\d{6}$/` on the entered code, then `setStep('enabled')` on match. No fetch, no api call, no mutation.
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:99` — `step === 'enabled'` renders `"Two-factor authentication is enabled."` purely from local state.
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:76` — verify-step help text tells the user to "Scan the QR code in your authenticator app" but no QR / otpauth secret is rendered or fetched.
- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx:4` — docstring: "The backend endpoints for enrollment aren't exposed through our mobile auth layer yet, so this screen wires up local state."
- Signal id `code-review-mobile-rn-twofactor-stub-false-enabled` (confidence=medium, +3, vector=bug) in `.research/signals/2026-10-27.json`.

## Files

- `frontend/apps/mobile/src/screens/auth/TwoFactorScreen.tsx`

## Dependencies

<!-- Decision gate on whether to wire real endpoints vs ship a "coming soon" state belongs to this PR, not an upstream dependency. -->

## Required capabilities

- [x] C1 — Systematic debugging (bug / security-feature correctness)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device (recommended for runtime verification, but static test harness is sufficient for IG3)
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Open the mobile app, navigate to Account → Security → Two-factor authentication; press the enable action, enter any 6-digit number (e.g. `123456`).
2. Expected: the screen either errors with "not yet available" (coming-soon state) or routes through a real backend enrollment flow. Actual: the screen flips to "Two-factor authentication is enabled." while zero network traffic went to the backend; the user's account is not actually hardened.

## Suggested approach

1. Decide the minimal fix: default to the "coming soon / unavailable" gate. Replace the `step === 'enabled'` branch with a `step === 'unavailable'` state that renders `"Two-factor authentication is coming soon — your account is not yet protected by 2FA."` and does NOT advertise it as enabled.
2. In `handleVerify()` (line 40), remove the regex-passes-therefore-enabled logic. Set the state to `'unavailable'` on mount (or hide the enable call-to-action entirely) and keep the verify step unreachable until the backend endpoints exist.
3. Delete the misleading QR-scan help text at line 76; replace with the unavailable copy.
4. Update the i18n keys (sk / cs / de / en) for the new copy; keep the string table in sync with `frontend/apps/mobile/src/locales/*`.
5. If the Settings/Security entry point exists in `frontend/apps/mobile/src/screens/settings/` or similar, hide or disable the "Enable 2FA" row behind the same `unavailable` state so the user never lands on the stub.
6. Add a Jest/React Native Testing Library test that renders `TwoFactorScreen`, pastes `"123456"`, triggers `handleVerify`, and asserts the final rendered copy does NOT contain any form of "enabled" (sk/cs/de/en variants) — i.e. the false-success regression is caught mechanically.
7. Run `pnpm -F mobile test` and `pnpm -F mobile typecheck` locally.

## Alternatives considered

- **Wire the real backend enroll / verify endpoints end-to-end in this PR** — rejected because scope creep: the API contract for mobile 2FA is not in `frontend/packages/api-client/src/auth/` yet, and shipping fake-success for another sprint while the backend lands is strictly worse than shipping an honest "coming soon".
- **Keep the current UI but add a server-side probe that silently POSTs the code and ignores failures** — rejected because it still ends in `step='enabled'` while the server never actually enrolled; the UI remains dishonest about account state.

## Root-cause trace

1. Symptom: user is told "Two-factor authentication is enabled." despite no server-side enrollment having occurred.
2. ← `TwoFactorScreen.tsx:40` — `handleVerify` sets `step='enabled'` purely on regex match.
3. ← `TwoFactorScreen.tsx:99` — the `'enabled'` branch's copy affirms a server state the client has not verified.
4. Origin: initial scaffold of the screen (per the self-admission at `TwoFactorScreen.tsx:4`) — written as placeholder before the backend endpoints were ready, then shipped past the placeholder gate without being locked.

## Test plan

- [ ] New unit test under `frontend/apps/mobile/src/screens/auth/__tests__/TwoFactorScreen.test.tsx` — renders the screen, drives `handleVerify('123456')`, asserts final tree contains the "coming soon / unavailable" copy and does NOT contain any "enabled" variant across the shipped locales.
- [ ] Regression scenario: pre-fix, this test fails (today's code renders "enabled"); post-fix, it passes. Satisfies IG3.
- [ ] Local commands: `pnpm -F mobile test -- TwoFactorScreen` and `pnpm -F mobile typecheck`.

## Out of scope

- Wiring real backend 2FA enroll / verify endpoints and the `otpauth://` secret delivery. Track as a separate feature plan.
- Changes to `frontend/apps/ppt-web/` 2FA flows (web app) — this plan is React Native only.
- Backend / `api-server` changes.

## After-merge

- Move this file to `plans/_archive/code-review-mobile-rn-twofactor-stub-false-enabled.md`
- Mark the matching `backlog.json` row as `status: "done"`
