# code-review-mobile-rn-report-fault-fake-submit

**Vector:** bug
**Score:** 3
**Source:** Phase 1.5 review of mobile-rn segment (2026-06-16)
**Confidence:** high

## Hypothesis
The mobile RN Property Management app's fault-report flow is a dead-end UI: `ReportFaultScreen.handleSubmit()` does not call the backend at all — it `await`s a `setTimeout(1500)` and then shows a success Alert. Users who tap Report and see the green confirmation believe the fault reached operations; nothing was sent. Wiring `handleSubmit` to a real `POST /api/faults` (with the photos already collected in `formData.photos`) closes the loop. Repair is small (one screen, one fetch, one mutation hook); the test asserts the network call.

## Evidence
- `frontend/apps/mobile/src/screens/faults/ReportFaultScreen.tsx:157-161` — `try { // Simulate API call
  await new Promise((resolve) => setTimeout(resolve, 1500));
  Alert.alert(t('common.done'), t('faults.successSubmit'), …)`.
- `frontend/apps/mobile/src/App.tsx:126` — `<Stack.Screen name="ReportFault" component={ReportFaultScreen} />` mounts the screen in the user-facing nav stack (Property Mgmt mobile, three.two.bit.ppt.management).
- Backend has a faults route — `backend/servers/api-server/src/routes/faults.rs` defines the create-fault POST handler.
- No `useMutation` / `fetch` / `apiClient.faults.*` symbol referenced anywhere in `ReportFaultScreen.tsx` (grep verified).

## Files
- `frontend/apps/mobile/src/screens/faults/ReportFaultScreen.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

Reasoning: the mobile RN unit/component test runs in jest-expo and does not need a device or browser. Verification is `pnpm -F @ppt/mobile test ReportFaultScreen` and `pnpm typecheck`. A device-on-screen smoke would be nice, but not required to land the fix correctly.

## Repro steps
1. `cd frontend && pnpm install`
2. `pnpm dev:mobile` (Expo) — open the Property Management mobile app
3. Navigate to Faults → Report Fault (route name `ReportFault`)
4. Fill the form fields, add a photo, tap Submit
5. Expected: a `POST /api/faults` request fires; UI shows success only after a 2xx. Actual: no network request fires; UI shows success after a fixed 1500 ms delay.

## Suggested approach
1. Generate (or reuse) a faults API client method from `@ppt/api-client` — the OpenAPI spec already covers `POST /api/v1/faults` (see `docs/api/typespec/`); add the call site in `frontend/apps/mobile/src/api/faults.ts` if missing.
2. In `ReportFaultScreen.tsx`, replace the `// Simulate API call` block with a `useMutation` (TanStack Query) that wraps the client call. Pass `{ buildingId, title, description, severity, photos }` from local state.
3. On success: keep the existing Alert + navigation reset. On error: `Alert.alert(t('common.error'), t('faults.errorSubmit'))` (add the i18n key in all 6 message bundles — `en`, `sk`, `cs`, `de`, `hu`, `pl`).
4. Disable the Submit button while `mutation.isPending` (the existing `isSubmitting` boolean covers this — wire it to `mutation.isPending`).
5. Photo upload: faults take `photos: string[]` of (presigned-uploaded) URLs. If the API expects raw multipart, follow the existing image-upload pattern used by `frontend/apps/mobile/src/screens/buildings/...` or by `ppt-web`'s `useFaultMutation`; if that pattern doesn't exist for mobile, ship without photo upload and file a separate follow-up issue.
6. Add `ReportFaultScreen.test.tsx`: mock the mutation client, render with `<NavigationContainer><ReportFaultScreen /></NavigationContainer>`, populate fields, press Submit, assert the mock was called with the expected payload.

## Alternatives considered
- **Hide the screen until the API path is wired** — rejected because the screen is already shipped, removing it is a worse UX regression than fixing the no-op (and the route is referenced from App.tsx + drawer menu).
- **Centralise via `useFaultMutation` shared hook reused by ppt-web** — rejected because ppt-web's hook (if it exists) lives in a different package; copying it cross-package adds dep-graph churn. Inline mutation here, refactor later if a second mobile screen needs the same call.

## Root-cause trace
1. Symptom: User reports fault → "Done" Alert → fault never appears in dispatcher / building manager dashboard.
2. ← `ReportFaultScreen.tsx:158-159` — `// Simulate API call` + `setTimeout(1500)` instead of fetch.
3. ← The screen was originally scaffolded as a UI stub (mobile RN bring-up) and the API wiring step was deferred and never completed. No tracking issue was filed.
4. Origin: introduced when the mobile RN PM app first landed `ReportFaultScreen` (likely in the early mobile RN bring-up sprint; the `// Simulate API call` comment is the marker the author left for "wire me up later").

## Test plan
- [ ] `frontend/apps/mobile/src/screens/faults/ReportFaultScreen.test.tsx` — renders Submit button; press → asserts mock client called with `{ title, description, severity, buildingId, photos }`; on success, asserts `Alert.alert(t('common.done'), t('faults.successSubmit'))`.
- [ ] `ReportFaultScreen.test.tsx` regression — on mutation error, asserts error Alert; no navigation reset.
- [ ] `pnpm -F @ppt/mobile test ReportFaultScreen` — exact command to run locally.

## Out of scope
- Photo upload (presigned URL pipeline) — defer to a separate plan if no shared upload helper exists for mobile RN.
- Wider audit of MOCK_* screens (Buildings, Meters, Leases, …) — tracked separately by `code-review-mobile-rn-screens-mock-data`. Don't bundle the whole mobile RN backend wiring into this PR.
- Backend faults route changes — handler is assumed correct; this plan only fixes the client.

## After-merge
- Move this file to `plans/_archive/code-review-mobile-rn-report-fault-fake-submit.md`
- Mark the matching `backlog.json` row as `status: "done"`
