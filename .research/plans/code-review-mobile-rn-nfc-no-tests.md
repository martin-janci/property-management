# code-review-mobile-rn-nfc-no-tests

**Vector:** test-gap
**Score:** 3
**Source:** Phase 1.5 review of mobile-rn segment (2026-07-14)
**Confidence:** medium

## Hypothesis
The mobile RN Property Management app ships an NFC building-access module — `NFCCredentialManager` (encrypted credential storage, legacy-key migration, emergency revocation) and `NFCAccessController` (access grant/deny logic including expiry, access-point authorization, and midnight-crossing time-restriction windows) — with **zero test coverage repo-wide**. A regression in credential deserialization, revocation, or the access-decision predicate would silently ship. Adding unit tests around the pure decision helpers and the storage/revocation lifecycle closes the highest-risk uncovered path in the mobile app.

## Evidence
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:34` — `export class NFCCredentialManager` with `loadStoredCredentials` (`:262`, corrupt-blob discard + legacy-key migration), `emergencyRevokeAll` (`:223`), and refresh/expiry logic; no test file references this class.
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts:41` — `export class NFCAccessController`; `validateAccess(credential, accessPointId)` at `:249` implements status/expiry/access-point authorization plus midnight-crossing time-restriction windows (`:300-336`); zero tests exercise it.
- `frontend/apps/mobile/src/nfc/index.ts:1` — both classes exported publicly.
- Repo-wide grep (`grep -rl -i "nfc\|NFCCredential\|NFCAccess" frontend/**/*.{test,spec}.{ts,tsx}`) returns zero matches — confirmed with the routine's Phase 1.5 review on 2026-07-14.
- No sibling test folder for `frontend/apps/mobile/src/nfc/` exists (contrast with `frontend/apps/mobile/src/screens/documents/` which ships 6 sibling test files).

## Files
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts`
- `frontend/apps/mobile/src/nfc/NFCAccessController.ts`
- `frontend/apps/mobile/src/nfc/index.ts`
- `frontend/apps/mobile/src/nfc/types.ts`

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm -F mobile test -- --listTests 2>&1 | grep -i nfc` — expect zero matches (no NFC test file exists on `dev`).
2. `grep -rl -i "NFCCredential\|NFCAccess\|NFCAccessController" frontend/apps/mobile/src` — expect only production sources under `frontend/apps/mobile/src/nfc/` and consumers, no `*.test.*` files.
3. Expected vs actual: expected — sibling `NFCCredentialManager.test.ts` and `NFCAccessController.test.ts` covering the paths named in *Evidence*; actual — the module is entirely uncovered.

## Suggested approach
1. Add `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` covering: constructor + `loadStoredCredentials` corrupt-blob discard (`:262`), legacy-key migration path, `emergencyRevokeAll` clearing all stored credentials (`:223`), refresh/expiry gating. Mock `expo-secure-store` (used elsewhere in the app — mirror the existing pattern).
2. Add `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` covering `validateAccess(credential, accessPointId)` (`:249`) branches: credential status (active vs revoked/expired), access-point authorization (allowed vs missing), and time-restriction windows including the midnight-crossing case (`:300-336`). Table-driven test with fixtures.
3. If `validateAccess` is currently `private`, either export a thin decision helper it delegates to, or exercise it through the public `handleTap` (`:163`) with mocked `credential` fixtures. Prefer the helper-extraction path so the decision matrix is directly testable.
4. Wire the new test files into whatever the mobile Jest config picks up (`frontend/apps/mobile/jest.config.*` — no config change should be needed; the discovery pattern already picks `**/*.test.ts`).
5. Run `cd frontend && pnpm -F mobile test src/nfc` and confirm both new files run and pass.
6. Confirm the tests would fail if `validateAccess`'s midnight-crossing logic were reverted to a naïve `now >= start && now <= end` check (IG3 — negative-mutation check).

## Alternatives considered
- **Full E2E via an actual NFC device (ADB + physical tap)** — rejected because it needs C5, runs on a single device shape, and doesn't scale as a regression net; unit tests around the decision helpers give reproducible coverage of every branch without hardware.
- **Snapshot-test just the `validateAccess` output as JSON** — rejected because snapshots hide the branch matrix (which is exactly the security-relevant surface) behind a single blob; explicit assertion tables per case (status × expiry × access-point × time-window) make regressions self-locating.

## Root-cause trace
N/A — `test-gap` doesn't need backward tracing. This is not fixing a bug that shipped; it's closing the coverage gap on security-critical code so future regressions are caught.

## Test plan
- [ ] `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — new unit test file covering the paths named above
- [ ] `frontend/apps/mobile/src/nfc/NFCAccessController.test.ts` — new unit test file covering the `validateAccess` decision matrix
- [ ] Command to run locally: `cd frontend && pnpm -F mobile test src/nfc` (or `pnpm -F mobile test` for the full mobile suite)
- [ ] Negative-mutation check: revert `validateAccess`'s midnight-crossing branch (`:300-336`) to a naïve range check and confirm the time-window tests fail

## Out of scope
- E2E / device-hardware NFC-tap testing (needs C5; separate follow-up).
- Refactoring the NFC storage schema or credential model.
- Testing consumers of these classes (e.g. NFC-screen React components) — cover the module surface first; consumers are a downstream follow-up.
- Integration with the backend `access_logs` API (currently a fire-and-forget POST from `handleTap`; belongs in a separate integration-test plan if scoped in).

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-nfc-no-tests.md`
- Mark the matching `backlog.json` row as `status: "done"`
