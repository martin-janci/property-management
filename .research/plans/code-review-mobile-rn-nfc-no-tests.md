# code-review-mobile-rn-nfc-no-tests

**Vector:** test-gap
**Score:** 3
**Source:** hotspot in `frontend/apps/mobile/src/nfc/` (rotating expert review 2026-07-14, mobile-rn segment, Tester expert)
**Confidence:** medium

## Hypothesis
The mobile RN app's NFC surface — `NFCCredentialManager` (encrypted-at-rest credential store, legacy-key migration with corrupt-blob discard, `emergencyRevokeAll`) and `NFCAccessController` (`validateAccess` decision table over status/expiry/access-point authorization/time-restriction windows including midnight crossing, plus the `handleTap` transmit-and-log flow) — governs whether a resident's phone unlocks a physical door. Both classes are exported publicly from `src/nfc/index.ts`, yet a repo-wide grep for tests importing either symbol returns zero hits. A silent regression in the credential-migration or decision-table paths would ship uncaught. The smallest correct fix is unit tests for the pure branches of each class, mocking `expo-secure-store` and any NFC transport, so the decision surface is pinned before any refactor.

## Evidence
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:34` — manages building-access credentials in expo-secure-store (encrypted-at-rest), with a legacy-key migration path that discards corrupt blobs on load (`loadStoredCredentials` ~L262) and an `emergencyRevokeAll` API (~L223). No test file references `NFCCredentialManager`.
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:249` — `validateAccess()` implements status/expiry/access-point-authorization + time-restriction (midnight crossings ~L300–336); `handleTap()` covers transmit + log (~L163). No test file references `NFCAccessController`.
- `frontend/apps/mobile/src/nfc/index.ts:1` — both classes are exported publicly; repo-wide grep confirms neither has a `*.test.*` or `__tests__` co-located.
- `.research/signals/2026-07-14-mobile-rn.json` — surfaced by dev-review Tester expert (segment=mobile-rn, score_delta=3).

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

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. From `dev`, run `grep -R "NFCCredentialManager\|NFCAccessController" frontend/apps/mobile --include='*.test.*' --include='__tests__/*'`.
2. Expected: at least one test file references each class. Actual: zero hits — the entire NFC surface (credential store + tap-time access decision) is uncovered.

## Suggested approach
1. Add `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` mocking `expo-secure-store` to cover: save + reload happy path, corrupt-blob discard on load (write invalid JSON to the mock, assert the loader returns empty state and logs), and `emergencyRevokeAll` (assert every credential is cleared from the mock store).
2. Add `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` covering the `validateAccess` decision table: revoked/expired credential → deny; unauthorized access point for the credential → deny; time-restriction window matches (including a midnight-crossing window like 22:00→06:00) → grant; time-restriction outside window → deny.
3. Extend the `NFCAccessController` test with a `handleTap` case that stubs the transmit adapter (`sendReadTx` / equivalent) and asserts a successful tap emits a log entry, and a failing transmit produces a failure-path log without throwing.
4. If a needed seam is missing (e.g. the transmit function is a bare module import that can't be mocked without extra glue), extract it behind a constructor-injected dependency in the class under test — no behavior change, minimum surface.
5. Run `pnpm -F @ppt/mobile test -- nfc` locally; expect all new cases green.
6. Update `frontend/apps/mobile/src/nfc/index.ts` only if step 4 required a small refactor; otherwise leave production code untouched.
7. Land as `test(mobile): cover NFC credential store + access controller decision paths (Closes …)`.

## Alternatives considered
- **End-to-end test with a real NFC transponder** — rejected because there is no headless NFC harness in CI and no cross-platform way to inject tap events; the decision surface is a pure function of state + wall clock and is fully reachable by unit test.
- **Refactor `NFCAccessController` into pure helpers first, then test** — rejected because the class already exposes `validateAccess` as a pure method; the test-only landing is smaller and lower-risk than a same-PR refactor.

## Root-cause trace
N/A — test-gap doesn't need backward tracing.

## Test plan
- [ ] `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — save+load, corrupt-blob discard, emergencyRevokeAll.
- [ ] `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` — validateAccess decision-table (status, expiry, access-point, time-window incl. midnight crossing) + handleTap success/failure logging.
- [ ] Command: `pnpm -F @ppt/mobile test -- nfc`

## Out of scope
- Any behavioral change to `NFCCredentialManager` or `NFCAccessController` beyond the smallest DI seam required for a mockable transport (step 4). No new features, no revoke-flow changes, no encryption-scheme changes.
- Integration tests that require Android/iOS runtime NFC (`expo-nfc-*` native modules).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-nfc-no-tests.md`
- Mark the matching `backlog.json` row as `status: "done"`
