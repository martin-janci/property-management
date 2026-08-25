# code-review-mobile-rn-nfc-revoke-silent-fail

**Vector:** security
**Score:** 3
**Source:** mobile-rn segment review 2026-08-25 (Phase 1.5)
**Confidence:** medium

## Hypothesis

`emergencyRevokeAll` in the mobile app's NFC credential manager currently awaits a local SecureStore write before calling the server's revoke endpoint. If the local write throws (e.g. a chunk `setItemAsync` fails on iOS during app-store data pressure, or SecureStore is disabled), the server call is never fired AND the previous encrypted credential blob remains readable on next launch — a stolen phone keeps working credentials while the backend still lists them active. The file's own doc explicitly warns against exactly this class of "credentials usable offline after a revoke attempt". Fix by making the server revoke the first action (or the always-executed action via try/finally), so at minimum the server-side state moves to "revoked" regardless of local IO failure.

## Evidence

- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:298-307` — `emergencyRevokeAll()` awaits `this.storeCredentials()` on line 302 before the `apiRequest('/api/v1/access/credentials/revoke-all', 'POST')` call on line 306. A throw from `storeCredentials` skips the API call entirely.
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:419-440` — `storeCredentials()` writes chunk-by-chunk with `SecureStore.setItemAsync` (line 431), then writes the manifest (line 434). A partial-chunk failure leaves the OLD manifest at `CREDENTIALS_KEY` still pointing at OLD chunks, so `loadStoredCredentials()` at line 342 reassembles a still-usable credential set on next app launch.
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:289-296` — the method's own docblock states "A failed (or unreachable) server revoke must never leave credentials usable offline — that would defeat the entire point of the 'lost phone' panic action". The current code violates the mirror-image invariant (failed local write => no server revoke fired), which produces the same net outcome.
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — grep found no test that mocks `SecureStore.setItemAsync` to reject during `emergencyRevokeAll`, so this regression path has never been exercised.

## Files

- `frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:298`
- `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts`

## Dependencies

<!-- no dependencies -->

## Required capabilities

- [x] C1 — Systematic debugging (security fix; must trace the failure path)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device (unit-test-only fix; no on-device run required)
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
- Neither C4 nor C5 is ticked → `cloud-ok`

Mode: cloud-ok

## Repro steps

1. In `NFCCredentialManager.test.ts`, add a new test that stubs `SecureStore.setItemAsync` to reject (e.g. `mockRejectedValueOnce(new Error('SecureStore unavailable'))`) and stubs the `apiRequest` fetch call to a spy.
2. Instantiate `NFCCredentialManager`, load a fake credential set, then call `emergencyRevokeAll()`.
3. Expected: the server-revoke fetch is called at least once (the spy records a POST to `/api/v1/access/credentials/revoke-all`).
4. Actual (today): the fetch spy is never called — the local write rejection short-circuits the method before the server call is scheduled.

## Suggested approach

1. Refactor `emergencyRevokeAll()` (`frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:298-307`) so the server revoke is scheduled first (or in `finally`). One shape: call `apiRequest('/api/v1/access/credentials/revoke-all', 'POST')` first, capture the promise, then run the local wipe. If the server call succeeds, credentials are already dead server-side regardless of local IO. If the server call fails, still attempt the local wipe so offline use is blocked, then re-throw so the caller can prompt a retry.
2. If reversing the order changes user-visible timing (offline block latency), keep the local wipe first but wrap in `try { … } finally { await apiRequest(...) }` and rethrow after both settle.
3. Update the docblock (lines 289-296) to describe the new invariant explicitly: "server revoke fires unconditionally, even when the local wipe throws".
4. Add the regression test from *Repro steps* to `NFCCredentialManager.test.ts` — mock `SecureStore.setItemAsync` to reject once and assert the server-revoke fetch spy is called.
5. Add a companion test for the reverse failure (server-revoke rejects, local wipe succeeds) to lock in the existing invariant the doc already claims.
6. Run `pnpm --filter @ppt/mobile test -- NFCCredentialManager` in `frontend/`; confirm both new tests fail before the fix and pass after.

## Alternatives considered

- **Retry the local wipe in a loop before the server call** — rejected because it doesn't help when SecureStore is permanently unavailable (e.g. iOS Keychain restricted): the user still ends up with credentials on disk AND no server-side revoke. The server call must happen regardless.
- **Fire-and-forget the server call before the local wipe** — rejected because a silent server-revoke failure is not visible to the caller; the current signature returns a rejected promise on failure so the UI can prompt "retry when online", and dropping that surface loses the user's ability to know their revoke didn't land.

## Root-cause trace

1. Symptom: after `emergencyRevokeAll()` throws because `storeCredentials` (`NFCCredentialManager.ts:419`) failed to write, the backend has no record of the revoke and the phone reboots with the old credentials still readable.
2. ← Immediate cause at `NFCCredentialManager.ts:302`: `await this.storeCredentials()` before the server call means any thrown error prevents `apiRequest(...)` at line 306 from ever being reached.
3. ← Upstream cause at `NFCCredentialManager.ts:430-434`: chunked write writes chunks first then the manifest; if a chunk `setItemAsync` at line 431 rejects, the manifest at line 434 is never rewritten so the previous manifest+chunks remain intact and readable on next load.
4. Origin: introduced when the chunked-storage path was added (see the `MAX_CREDENTIAL_CHUNKS` scaffolding in `NFCCredentialManager.ts:409-439`); the original single-slot format (line 402) had a smaller failure window because there was only one write to fail. The `emergencyRevokeAll` sequencing was not revisited when chunked writes made partial-failure much more likely.

## Test plan

- [ ] `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — new test: `emergencyRevokeAll_calls_server_even_when_SecureStore_write_fails` mocks `setItemAsync` to reject and asserts the server-revoke fetch spy fires.
- [ ] `frontend/apps/mobile/src/nfc/NFCCredentialManager.test.ts` — new test: `emergencyRevokeAll_wipes_local_even_when_server_revoke_fails` asserts the local `credentials` array is cleared and stored as `[]` after the server call rejects.
- [ ] Command to run locally: `cd frontend && pnpm --filter @ppt/mobile test -- NFCCredentialManager`

## Out of scope

- Reworking the chunk write to be atomic (a separate, larger refactor)
- Adding a background retry queue for failed server revokes (product decision — the current contract is "surface the failure to the caller so it prompts a retry")
- Changes to `NFCAccessController.ts` test coverage (tracked separately as `code-review-mobile-rn-nfc-access-no-tests`)

## After-merge

- Move this file to `plans/_archive/code-review-mobile-rn-nfc-revoke-silent-fail.md`
- Mark the matching `backlog.json` row as `status: "done"`
