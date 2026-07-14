# test-gap-mobile-nfc-untested

**Vector:** test-gap
**Score:** 3
**Source:** commit a166bcab (dispatcher Tier-1d mobile-rn dev-review — 2026-07-14) + signal `code-review-mobile-rn-nfc-no-tests`
**Confidence:** medium

## Hypothesis
The two NFC classes that gate physical building access on the mobile RN app — `NFCCredentialManager` (encrypted secure-store persistence, legacy-key migration, `emergencyRevokeAll`, refresh/expiry) and `NFCAccessController` (`validateAccess`, `handleTap`, midnight-crossing time-restriction windows) — have **zero** test coverage repo-wide. A repository-wide grep for `NFCCredentialManager` / `NFCAccessController` inside `*.test.ts*` returns no matches. This is a security-critical decision path (access-grant/deny + encrypted credential handling) shipping without a single regression test. Adding a focused Jest suite that pins the access-grant matrix and the credential-store round-trip closes the exposure with a small, contained diff.

## Evidence
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:34` — class managing building-access credentials in `expo-secure-store` (encrypted at rest); `loadStoredCredentials` (~L262) does legacy-key migration and discards corrupt blobs; `emergencyRevokeAll` (~L223) is a bulk revoke path
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:249` — `validateAccess()` implements grant/deny (status, expiry, access-point authorization, time-restriction windows crossing midnight L300–336); `handleTap()` transmit/log flow (~L163)
- `frontend/apps/mobile/src/nfc/index.ts:1` — both classes exported publicly with no `*.test.*` or `__tests__` coverage in-segment or repo-wide
- `frontend/apps/mobile/jest.config.js` — `preset: 'jest-expo'`, `testMatch: ['**/*.test.{ts,tsx}']` — sibling suites live in `src/hooks/*.test.ts` etc.; this plan follows the same colocated-`*.test.ts` convention
- Sibling reference for expo-secure-store mocking: `frontend/apps/mobile/src/hooks/useApi.test.ts` (recent, from PR #2305)

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

Reasoning: pure Jest unit-test authoring against the RN app's existing `jest-expo` harness. No emulator, no live device, no browser. Sibling suites (e.g. `useApi.test.ts` landed in PR #2305, `LanguageSwitcher.test.tsx`) already run in CI without ADB — same pattern applies. C5 (ADB) is not needed because we assert against the pure TypeScript decision logic in `NFCAccessController.validateAccess` and the `expo-secure-store` I/O in `NFCCredentialManager` via a jest module mock, not against a physical NFC transceiver.

## Repro steps
1. Run `pnpm -F @ppt/mobile test -- --testPathPattern nfc` in `frontend/`.
2. Expected: no test files match, exit code 1 (jest "no tests matched") — proving the coverage gap.
3. After the plan lands: same command runs the new suite; exit code 0; coverage report shows `NFCCredentialManager` and `NFCAccessController` moved from 0% to a meaningful (≥ 60%) statement/branch coverage.

## Suggested approach
1. Add `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` covering the `validateAccess` decision matrix — deny cases (revoked credential, expired credential, unauthorized access-point, outside time-restriction window) and grant cases (fresh credential, authorized point, inside window). Include the midnight-crossing window at `NFCAccessController.ts:300–336` as its own describe-block (the trickiest branch).
2. Add `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` covering: round-trip persist/load (mocked `expo-secure-store` via `jest.mock('expo-secure-store', …)` — see `useApi.test.ts` for the mock shape); corrupt-blob discard on legacy-key migration; `emergencyRevokeAll` empties the persisted set.
3. Do NOT touch product code (avoid scope creep). If a class's testability is blocked by unexported helpers, prefer a narrow named export (matching PR #2308's pattern for `DocumentUploadScreen`) over refactoring internals.
4. Do NOT touch `handleTap()` — that path requires an NFC transceiver mock and is out of scope; document that gap in *Out of scope* below.
5. Verify: `pnpm -F @ppt/mobile test -- --testPathPattern nfc` → all green in the sandbox; `pnpm -F @ppt/mobile typecheck` clean.

## Alternatives considered
- **Property-based tests via `fast-check`** — rejected because the access-grant decision surface is a small finite matrix (≤ 12 canonical states) that reads more clearly as explicit examples; property tests would bury the security intent behind generator plumbing without materially widening branch coverage.
- **Integration test against a stub NFC transceiver** — rejected because the mobile RN app doesn't ship an NFC test harness and building one would inflate this test-gap plan into an infrastructure change; the pure-logic tests above land the security assertion without pulling in transceiver mocking.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. The gap is direct: the classes shipped (Epic 3 physical-access work) without a test suite; no prior commit "introduced" the gap because tests were never authored.

## Test plan
- [ ] `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` — the grant/deny matrix + midnight window
- [ ] `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — persist/load round-trip + corrupt-blob discard + `emergencyRevokeAll`
- [ ] Command: `cd frontend && pnpm -F @ppt/mobile test -- --testPathPattern nfc`

## Out of scope
- `handleTap()` transceiver path in `NFCAccessController.ts:163` — needs an NFC transceiver mock, defer to a follow-up plan
- The `NFCTagReader` native module (not present in the current tree) — unrelated
- Any refactor of `NFCCredentialManager`'s legacy-key migration logic — behavior-preserving tests only

## After-merge
- Move this file to `plans/_archive/test-gap-mobile-nfc-untested.md`
- Mark the matching `backlog.json` row (`code-review-mobile-rn-nfc-no-tests`) as `status: "done"`
