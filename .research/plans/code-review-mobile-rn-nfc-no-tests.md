# code-review-mobile-rn-nfc-no-tests

**Vector:** test-gap
**Score:** 3
**Source:** rotating-expert-review 2026-07-14 (mobile-rn segment)
**Confidence:** medium

## Hypothesis
`NFCCredentialManager` and `NFCAccessController` in `frontend/apps/mobile/src/nfc/` implement building-access security logic — encrypted-at-rest credential storage with legacy-key migration + corrupt-blob discard, emergency revoke-all, access-grant decisions across status/expiry/authorization/time-restriction windows crossing midnight, and tap transmit/log — yet a repo-wide grep returns zero test files referencing either class. A regression in `validateAccess()` or `loadStoredCredentials()` would ship undetected and either lock legitimate residents out or grant access after credentials are revoked. Adding a `NFCAccessController.test.ts` + `NFCCredentialManager.test.ts` pair covering the deny/allow decision matrix, credential migration path, and emergency-revoke closes the highest-value gap in the mobile security surface.

## Evidence
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:34` — class manages expo-secure-store credentials with legacy-key migration + corrupt-blob discard (`loadStoredCredentials` ~L262), `emergencyRevokeAll` (~L223), and refresh/expiry.
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:249` — `validateAccess()` implements grant/deny across status, expiry, access-point authorization, and time-restriction windows crossing midnight (~L300-336); `handleTap()` orchestrates transmit + log (~L163).
- `frontend/apps/mobile/src/nfc/index.ts:1` — both classes exported publicly; `find frontend/apps/mobile/src/nfc/ -name "*.test.*"` returns 0 results.
- Rotating-expert-review 2026-07-14 (`.research/signals/2026-07-14-mobile-rn.json`) — Tester expert flagged this as the top mobile-rn finding for the segment.

## Files
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts`
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts`
- `frontend/apps/mobile/src/nfc/index.ts`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. `cd frontend/apps/mobile && pnpm test -- --testPathPattern="nfc/"` — expect Jest to report "No tests found" (matches current state).
2. After the change: same command should list `NFCCredentialManager.test.ts` and `NFCAccessController.test.ts` and pass with coverage of the decision matrix, migration path, and revoke.

## Suggested approach
1. Add `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` covering `validateAccess()` truth table: (a) valid+in-window → grant; (b) expired → deny; (c) unauthorized access-point → deny; (d) time window crossing midnight (23:00–02:00) with a 01:00 tap → grant; (e) window boundary edges — off-by-one at open/close.
2. Add `handleTap()` tests that exercise the transmit + log flow via `NFCAccessController` with mocked `NFCCredentialManager` and log sink — verify success + failure paths are logged distinctly.
3. Add `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` covering: (a) `loadStoredCredentials()` legacy-key migration path — seed old key, assert new key is written and old cleared; (b) corrupt-blob discard — mock a JSON parse failure and assert the store is cleared rather than throwing; (c) `emergencyRevokeAll()` — assert all keys are removed and the in-memory cache is invalidated; (d) refresh-on-near-expiry.
4. Mock `expo-secure-store` per `jest-expo` conventions (already used elsewhere in `frontend/apps/mobile/src/**/*.test.ts`).
5. Run `pnpm test -F @ppt/mobile -- --testPathPattern="nfc/"` locally and in CI (frontend.yml).
6. Update `docs/screens/` if any screen documents reference NFC and lacked test callouts.

## Alternatives considered
- **Add a single integration test that exercises the whole tap-to-log flow via a synthetic Expo runtime** — rejected because the seams needed (native NFC, secure-store) would require extensive mocking and the failure attribution would be worse than focused unit tests on the two classes.
- **Defer to a follow-up epic (mobile hardening 2026-Q3)** — rejected because the classes are already shipped and consumed via `nfc/index.ts`, and the segment cursor won't return to mobile-rn for weeks; a plan today unblocks a small, self-contained implementer task.

## Root-cause trace
N/A — test-gap doesn't need backward tracing.

## Test plan
- [ ] `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` — covers the `validateAccess()` decision matrix + `handleTap()` transmit/log paths.
- [ ] `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — covers legacy-key migration, corrupt-blob discard, `emergencyRevokeAll`, and refresh-on-near-expiry.
- [ ] Run: `pnpm -F @ppt/mobile test -- --testPathPattern="nfc/"` → all new tests pass; run `pnpm -F @ppt/mobile test` → suite still green.

## Out of scope
- Refactoring `NFCAccessController` / `NFCCredentialManager` — behavior must not change; this plan pins current behavior only.
- Adding tests for `expo-secure-store` itself (a third-party dep).
- End-to-end tests requiring a physical NFC reader.
- Native (Kotlin/Swift) NFC bridge changes.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-nfc-no-tests.md`
- Mark the matching `backlog.json` row as `status: "done"`
