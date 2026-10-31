# code-review-mobile-rn-meter-reading-no-online-sync

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review [mobile-rn] 2026-10-31
**Confidence:** high

## Hypothesis

`MeterReadingScreen.handleSubmit` always enqueues the submission via `addToQueue` and never POSTs directly, so when the user is online the reading sits in AsyncStorage until a NetInfo online→offline→online transition triggers `processQueue`. `useOfflineSupport.addToQueue` (lines 676-699) only appends; `processQueue` is called only from the NetInfo transition listener (line 601) and from the external `syncData`/`SyncProgressToast` manual retry — never from `addToQueue` itself. A device that stays online never fires the transition, so the reading never leaves the queue. The success alert fires regardless, so the user is told the submission landed when it did not. The smallest fix mirrors `ReportFaultScreen.handleSubmit` (line 378-406): when `isConnected && isInternetReachable`, call the API directly and only enqueue on error; keep the queue path strictly for the `isOffline` branch.

## Evidence

- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:128-147` — `addToQueue` is called unconditionally; the online branch (line 144-147) alerts `meters.submitSuccess` without any `apiRequest`.
- `frontend/apps/mobile/src/hooks/useOfflineSupport.ts:676-699` — `addToQueue` appends to `AsyncStorage[QUEUE_KEY]` and increments `queuedActionsCount`; it does not touch `processQueue`.
- `frontend/apps/mobile/src/hooks/useOfflineSupport.ts:593-619` — `processQueue()` is triggered only on `NetInfo.addEventListener` transitions where `state.isConnected && state.isInternetReachable`.
- `frontend/apps/mobile/src/screens/faults/ReportFaultScreen.tsx:378-406` — the correct contrast: enqueue only when `isOffline`; otherwise `await apiRequest('/api/v1/faults', ...)` directly and only enqueue on exception.
- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:117` — `syncStatus: isConnected ? 'syncing' : 'pending'` encodes the bug: the row is marked `syncing` without any sync in flight.

## Files

- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:117`
- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:128`
- `frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:144`
- `frontend/apps/mobile/src/screens/faults/ReportFaultScreen.tsx:378`
- `frontend/apps/mobile/src/hooks/useOfflineSupport.ts:676`

## Required capabilities

- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps

1. Mobile app is running against the dev API with a stable Wi-Fi connection; the device has not transitioned offline → online since app launch.
2. User opens Dashboard → Meters → pick water meter → enter a numeric reading → tap Submit.
3. Alert displays `meters.submitSuccess` (`Reading submitted successfully`). The pending-readings list shows the row with `syncStatus: 'syncing'`.
4. Observe network: no `POST /api/v1/meter-readings` request is sent. Confirm via Chrome DevTools (Metro bundler proxy) or `adb logcat | grep meter`.
5. Expected: the API receives the POST and persists the reading; the row either disappears (optimistic sync) or flips to `synced` after the response. Actual: `AsyncStorage[QUEUE_KEY]` holds the row indefinitely until the user disconnects and reconnects.

## Suggested approach

1. In `MeterReadingScreen.tsx` handleSubmit (line 112-170), split the online and offline branches the same way `ReportFaultScreen.tsx:378-406` does:
   - If `isConnected && isInternetReachable !== false`: call `apiRequest<CreateMeterReadingResult>('/api/v1/meter-readings', { method: 'POST', body: { … } })`. On success, invalidate the meter-readings query key and show `meters.submitSuccess`. On caught error, fall through to the queue path with `meters.submitQueuedOnErrorMessage` (mirror `faults.queuedOnErrorMessage`).
   - Else (offline): keep the existing `addToQueue` path and `meters.savedOffline` alert.
2. Ensure the queued record stays canonical for the offline branch — `addToQueue` is still the right call; just gate it behind `isOffline`.
3. In `useOfflineSupport.ts` (line 676), add a short JSDoc on `addToQueue` clarifying "queue only; call `processQueue` or `syncData` to flush — this function intentionally does NOT trigger a flush, to let callers decide". Prevents the same pattern from leaking into other screens.
4. Add `useTenantId()` / `useApi` already in use elsewhere — reuse the primitive instead of hand-crafting the fetch, matching `ReportFaultScreen`.
5. Update `pendingReadings` state only after the API call resolves, not pre-flight, so the row disappears on success rather than lingering as `syncing`.
6. Keep the `isValidReading` guard — it stays at the top of handleSubmit.
7. Preserve the idempotency behaviour: pass a client-generated `Idempotency-Key` header equal to the local id so a retried POST does not duplicate a reading the server already accepted (same shape as `ReportFaultScreen` uses `idempotencyKeyRef`).

## Alternatives considered

- **Call `processQueue()` from `addToQueue` whenever `isConnected`** — rejected because it changes the queue abstraction's semantics for every caller (ReportFault, votes, etc.); the queue is intentionally decoupled from online-send to let each screen make the right call for its UX.
- **Fire-and-forget `syncData()` from handleSubmit without changing branches** — rejected because it still queues the row and shows `syncing` state before the POST; the UX is correct only if the online call runs synchronously and the queue holds only truly-pending items.

## Root-cause trace

1. Symptom: user sees "Reading submitted successfully" alert; billing cycle closes without the reading. (`frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:144-147`.)
2. ← `addToQueue` appends to `AsyncStorage` and returns. (`frontend/apps/mobile/src/hooks/useOfflineSupport.ts:682-694`.)
3. ← `processQueue` is wired only to NetInfo transition at `useOfflineSupport.ts:593-603`, never to `addToQueue`.
4. Origin: `MeterReadingScreen.tsx` was authored (git-blame on line 128) in an early iteration that assumed `addToQueue` would always flush when online; the hook's contract has since made that explicit — the screen never followed suit. Contrast with `ReportFaultScreen.tsx` (post-#2999 fix), which uses `apiRequest` directly when online.

## Test plan

- [ ] `frontend/apps/mobile/src/screens/meters/__tests__/MeterReadingScreen.test.tsx` — new RTL test: render with `useNetInfo` mocked to `{ isConnected: true, isInternetReachable: true }`, submit a valid reading, assert that `apiRequest` was called with `/api/v1/meter-readings`, assert that `addToQueue` was NOT called, assert that `meters.submitSuccess` was shown.
- [ ] Companion test: `useNetInfo` mocked offline, submit, assert `addToQueue` was called and `apiRequest` was NOT, assert `meters.savedOffline` was shown.
- [ ] Companion test: online, `apiRequest` throws → assert `addToQueue` was called with the error-message label and the user is told the reading was queued for retry.
- [ ] Run locally: `pnpm --filter @ppt/mobile test -- MeterReadingScreen`

## Out of scope

- A broader audit of every screen that calls `addToQueue` directly without a direct-POST branch — this plan fixes the one screen with evidence; the audit is tracked as `pm-qa-shared-numeric-input-helper` sibling work.
- Replacing `useOfflineSupport` with TanStack Query mutation/offline cache — architectural change, separate decision.
- Changing the `syncStatus: 'syncing' | 'pending' | 'synced'` state machine; the fix uses the existing shape.

## After-merge

- Move this file to `plans/_archive/code-review-mobile-rn-meter-reading-no-online-sync.md`
- Mark the matching `backlog.json` row as `status: "done"`
