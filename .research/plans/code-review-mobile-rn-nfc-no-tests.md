# code-review-mobile-rn-nfc-no-tests

**Vector:** test-gap
**Score:** 3
**Source:** dispatcher review — .research/signals/2026-07-14-mobile-rn.json
**Confidence:** medium

## Hypothesis
The two production classes in `frontend/apps/mobile/src/nfc/` — `NFCCredentialManager` (encrypted-at-rest credential store) and `NFCAccessController` (grant/deny decision engine) — implement building-access security logic (expiry checks, access-point authorization, time-restriction windows crossing midnight, emergency-revoke, corrupt-blob migration/discard) with **zero test coverage anywhere in the mobile app or the repo**. Repo-wide grep for `NFCCredentialManager` or `NFCAccessController` under `*.test.*`/`__tests__/**` returns no hits. Any regression here silently mis-grants (or mis-denies) physical building access, so the class of failure is high-severity even though the vector is `test-gap`. Fix is a focused Jest/`jest-expo` test file per class that pins the four decision branches called out in the evidence, plus one migration/corrupt-blob test.

## Evidence
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:34` — class manages credentials in `expo-secure-store` (encrypted-at-rest), performs legacy-key migration with corrupt-blob discard (~L262), exposes `emergencyRevokeAll` (~L223), and does refresh/expiry logic; grep across `*.test.ts(x)` returns zero test files referencing it.
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:249` — `validateAccess()` implements all grant/deny logic (status, expiry, access-point authorization, time-restriction windows crossing midnight ~L300-336) plus `handleTap()` transmit/log flow (~L163); no test exercises this security-critical decision path.
- `frontend/apps/mobile/src/nfc/index.ts:1` — both classes are exported publicly with no `*.test.*` or `__tests__/` neighbour in the segment or repo (confirmed by grep at `frontend/apps/mobile --include='*.test.*'` returning 0 hits).
- Verified 2026-07-15: `ls frontend/apps/mobile/src/nfc/` shows the two `.ts` files exist and are 13 KB / 9 KB — non-trivial surface area.

## Files
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts`
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts`
- `frontend/apps/mobile/src/nfc/index.ts`
- `frontend/apps/mobile/src/nfc/types.ts`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm -F @ppt/mobile test -- --listTests | grep -i nfc` → returns empty list — no test file exists.
2. Introduce an off-by-one in `NFCAccessController.validateAccess()` (e.g. treat `expiresAt <= now` as valid instead of `<`), run `pnpm -F @ppt/mobile test`, observe the CI is green — the regression ships undetected.
3. Introduce a null-return in `NFCCredentialManager.loadStoredCredentials()` on the corrupt-blob branch and observe the emergency-revoke test does not exist to catch it.

## Suggested approach
1. Add `frontend/apps/mobile/src/nfc/__tests__/NFCCredentialManager.test.ts`:
   - Mock `expo-secure-store` via the pattern already used in `frontend/apps/mobile/src/security/__tests__/*.test.ts` if any exist; else inline `jest.mock('expo-secure-store', () => ({ getItemAsync: jest.fn(), setItemAsync: jest.fn(), deleteItemAsync: jest.fn() }))`.
   - Cover: fresh store returns `null`; happy-path load returns parsed credential; corrupt blob (`getItemAsync` returns `"not-json{{"`) is discarded and store cleared; legacy-key migration reads old key, writes new key, deletes old (~L262); `emergencyRevokeAll` clears every credential and calls `deleteItemAsync` for each known key (~L223); expiry field respected on load.
2. Add `frontend/apps/mobile/src/nfc/__tests__/NFCAccessController.test.ts`:
   - Cover `validateAccess()` branches at `:249` — active/expired/revoked status; access-point mismatch; and the midnight-crossing time-restriction window (`start=22:00, end=06:00`, requested at `23:00` and `03:00` both allowed; `12:00` denied) at ~L300–336.
   - Cover `handleTap()` (~L163) transmit success path (mock the transmit dependency), transmit failure surfacing, and audit-log invocation on both grant and deny.
3. Cross-check `jest-expo` config already picks up `**/__tests__/**/*.test.ts` — no config change should be needed. If a top-level test roots array exists, confirm `src/nfc/**` is included.
4. Do NOT mock the classes under test. Mock only the boundaries (`expo-secure-store`, the transmit function, `Date.now`/`useFakeTimers` for expiry/window tests).
5. Update `frontend/apps/mobile/src/nfc/index.ts` only if the `types.ts` shape needs a helper re-export for the tests; otherwise leave it untouched.
6. Run `pnpm -F @ppt/mobile test src/nfc` locally; add the new suite to any CI include pattern only if `jest-expo` does not already discover `__tests__/`.
7. Land as a `test-gap` PR titled `test-gap(mobile): cover NFC credential store + access controller` — no product-code changes.

## Alternatives considered
- **Add integration tests that drive real `expo-secure-store` on a device via Detox** — rejected because it needs C5 (ADB device) and is disproportionate to the risk; the branches under `validateAccess` and the corrupt-blob path are pure logic and can be pinned with Jest at 1/100th the cost.
- **Only test `NFCAccessController` and defer `NFCCredentialManager`** — rejected because the corrupt-blob discard path in `loadStoredCredentials` (~L262) is exactly the kind of silent-erase behaviour that will bite in production without a test; both classes are equally load-bearing.

## Root-cause trace
N/A — test-gap doesn't need backward tracing.

## Test plan
- [ ] `frontend/apps/mobile/src/nfc/__tests__/NFCCredentialManager.test.ts` — cases: fresh, happy-load, corrupt-blob-discard, legacy-key-migration, emergency-revoke, expiry.
- [ ] `frontend/apps/mobile/src/nfc/__tests__/NFCAccessController.test.ts` — cases: active/expired/revoked, access-point mismatch, midnight-crossing window, handleTap grant/deny logging.
- [ ] Both suites must FAIL before the plan lands if the corresponding production line is toggled (expiry `<=` vs `<`, corrupt-blob branch returning stale data).
- [ ] Run: `cd frontend && pnpm -F @ppt/mobile test src/nfc` — expect both suites green.
- [ ] CI mobile job (`.github/workflows/*mobile*` if one exists; else the workspace test job) must pick up the new file with no config change.

## Out of scope
- Changing NFC production behaviour (this is `test-gap` — pin current behaviour, do not fix suspected bugs the tests surface; file follow-up issues).
- Detox / on-device integration.
- Backend / API contract changes for the NFC access-log endpoint.
- `frontend/apps/mobile/src/nfc/types.ts` type refactors (type-level fixes only if needed to make tests compile).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-nfc-no-tests.md`
- Mark the matching `backlog.json` row as `status: "done"`
