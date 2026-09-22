# code-review-reality-web-realtor-detail-modal-silent-fail

**Vector:** bug
**Score:** 3
**Source:** ppt-dev-review 2026-09-22 (reality-web segment)
**Confidence:** high

## Hypothesis
`RealtorManagement.tsx` has a `DetailModal` component (functions live at L820-848) whose three sibling handlers — `handleSave` (edit title/bio), `handleRemove` (delete realtor), and `handleStatusChange` (activate/pause/disable) — each `await updateRealtor.mutateAsync(...)` or `removeRealtor.mutateAsync(...)` with no try/catch, and then unconditionally transition the UI (`setIsEditing(false)`, `onClose()`, or leave the modal open with the toggled status rendered). The hooks (`useUpdateRealtor` at `frontend/packages/reality-api-client/src/agency/hooks.ts:307-331`, `useRemoveRealtor` at L333-345) define only `onSuccess` — no `onError`. When the PATCH/DELETE fails (5xx, RLS deny, `ApiError` on non-2xx), the modal closes, edit mode exits, or the status pill appears toggled — the agency admin believes the write landed when it did not. This is the same silent-fail class as the CRM/Feed create wizards fixed in PR #2969 and the SyncSchedule save fixed in PR #2967, and the same file already has the correct template three components up: `InviteModal.handleSubmit` at L568-581 wraps its `inviteRealtor.mutateAsync` in try/catch and renders `inviteError` at L621-625. Smallest change: mirror the InviteModal pattern in each of the three DetailModal handlers.

## Evidence
- `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:828-835` — `handleSave` is `await updateRealtor.mutateAsync({...}); setIsEditing(false);` with no error path.
- `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:837-840` — `handleRemove` is `await removeRealtor.mutateAsync(...); onClose();` — the modal closes on failure, leaving the agency admin certain the realtor was removed.
- `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:842-848` — `handleStatusChange` is `await updateRealtor.mutateAsync({..., data: { status }});` — no post-await UI branch, but React Query's optimistic-invalidate re-renders the status pill from cache before rollback lands; visually indistinguishable from success.
- `frontend/packages/reality-api-client/src/agency/hooks.ts:307-331` (`useUpdateRealtor`) and `333-345` (`useRemoveRealtor`) — both define `onSuccess` only; no `onError`, no toast, no logging.
- Existing correct pattern in the same file: `InviteModal.handleSubmit` at `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx:568-581` (try/catch + `setInviteError(t('inviteError'))`); rendered alert at L621-625.
- Class of bug already landed on other reality-web silent-fail sites: PR #2967 (SyncSchedule), PR #2968 (FeedCard toggle/delete), PR #2969 (import create wizards).

## Files
- `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx`
- `frontend/packages/reality-api-client/src/agency/hooks.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Log in to reality-web as an agency admin who has ≥1 realtor on their agency.
2. Open the realtor list and click the row-details button to open `DetailModal`.
3. Force the PATCH/DELETE to fail — block `PATCH **/realtors/*` and `DELETE **/realtors/*` in DevTools Network, or point at a `reality-server` returning 500 for those routes.
4. In edit mode, change the title, click Save (`handleSave`); OR click Remove and confirm (`handleRemove`); OR flip the status dropdown to Paused/Disabled (`handleStatusChange`).
5. Expected (all three): modal stays open (or reopens for remove), an alert reading "Failed to update realtor" / "Failed to remove realtor" appears, and the status/title/bio are visibly reverted to the pre-click values.
6. Actual (today, all three): `handleSave` exits edit mode with the local `title/bio` state still filled in (bio+title never left the browser), `handleRemove` closes the modal (the realtor still exists), `handleStatusChange` shows the toggled status pill because React Query invalidates and the cache re-emits stale/optimistic data — no error UI, no toast, no console message beyond the raw unhandled promise rejection.

## Suggested approach
1. In `frontend/apps/reality-web/src/components/agency/RealtorManagement.tsx`, add local error state to `DetailModal`: `const [actionError, setActionError] = useState<string | null>(null);` (single shared field is fine; distinct-per-action is fine too — pick one and stay consistent).
2. Rewrite `handleSave` to clear `actionError`, wrap `mutateAsync` in try/catch, only `setIsEditing(false)` inside the try branch after resolve; in catch `setActionError(t('updateRealtorFailed'))` and `console.error` for parity with `InviteModal`.
3. Rewrite `handleRemove` the same way: clear `actionError`, try/catch around `removeRealtor.mutateAsync(...)`, only `onClose()` inside the try; catch sets `setActionError(t('removeRealtorFailed'))`.
4. Rewrite `handleStatusChange` to clear `actionError`, try/catch; on catch set `setActionError(t('statusChangeFailed'))`. Optional: block the status control (`disabled={updateRealtor.isPending}`) so the user can't fire again while the rollback is pending.
5. Render `{actionError && <div className="detail-modal-error" role="alert" aria-live="assertive">{actionError}</div>}` inside the modal's form area — reuse the `.invite-error` CSS class already used at L621-625 (or add a sibling class if the visual placement needs to differ).
6. Add i18n keys `agency.updateRealtorFailed`, `agency.removeRealtorFailed`, `agency.statusChangeFailed` to each locale JSON under `frontend/apps/reality-web/messages/{en,sk,cs,de}.json`, following the tone of the existing `agency.inviteError`.
7. Add / extend Vitest coverage in `frontend/apps/reality-web/src/components/agency/RealtorManagement.test.tsx` — one test per handler that stubs the corresponding hook's `mutateAsync` to reject and asserts the alert node is rendered and the intended UI transition (`setIsEditing(false)` / `onClose` / status-pill flip) did NOT happen.
8. Run `pnpm --filter @ppt/reality-web test` and `pnpm --filter @ppt/reality-web lint` (Biome + typecheck + vitest via `frontend.yml`).

## Alternatives considered
- **Add `onError` inside `useUpdateRealtor` / `useRemoveRealtor` and surface via a global toast** — rejected because reality-web currently surfaces errors as inline `role="alert"` regions per-component (see `InviteModal.inviteError`, PR #2967 SyncSchedule fix). A global toast infra would widen the diff and diverge from convention; the DetailModal audience is the exact user who initiated the action, so inline is a strictly better UX.
- **Consolidate the three handlers into a single generic `handleAction<T>(mutation, onSuccess)` helper inside the component** — rejected because the three handlers each need a distinct post-success side effect (`setIsEditing`, `onClose`, no-op) and a distinct i18n key. The abstraction would save ~10 lines at the cost of legibility and diff review speed — not worth it for a bug fix.

## Root-cause trace
1. Symptom: agency admin edits a realtor's title/bio (or removes/status-changes), the PATCH/DELETE fails, UI acts as if the write landed.
2. ← `RealtorManagement.tsx:828-848` — all three DetailModal handlers await `mutateAsync` without try/catch, then unconditionally close the modal / exit edit mode.
3. ← `frontend/packages/reality-api-client/src/agency/hooks.ts:307-331,333-345` — `useUpdateRealtor` and `useRemoveRealtor` supply `onSuccess` only; no `onError` fallback anywhere.
4. Origin: introduced in the initial `RealtorManagement.tsx` component commit — the sibling `InviteModal` in the same file was authored with the correct pattern (try/catch + `setInviteError`), but `DetailModal` was implemented without it and never revisited. Same anti-pattern as the CRM/Feed silent-fail items already in the routine's backlog.

## Test plan
- [ ] Extend `frontend/apps/reality-web/src/components/agency/RealtorManagement.test.tsx` with three cases, one per handler: mock the corresponding hook's `mutateAsync` to reject, assert the alert node appears (`screen.getByRole('alert')`), assert `setIsEditing(false)` / `onClose` / status-pill flip did NOT happen.
- [ ] The tests must fail on today's `main` — the handlers today rethrow and the UI transitions unconditionally, so the assertions for the alert would time out and the UI-state assertions would show the transition happened. Structure the failing-on-main as `expect(handleSave()).rejects.toThrow()` for direct IG3 evidence.
- [ ] Command: `pnpm --filter @ppt/reality-web test -- RealtorManagement` (CI: `frontend.yml`).

## Out of scope
- The `useUpdateRealtor` / `useRemoveRealtor` hook signatures / return shapes.
- Backend behaviour of `PATCH /api/v1/agencies/{id}/realtors/{id}` and `DELETE /api/v1/agencies/{id}/realtors/{id}`.
- The `InviteModal` code path — already correct.
- The realtor-list-row action buttons (`RealtorCard` etc.) — a separate walkthrough will decide if they share the same anti-pattern; scoping this plan to the DetailModal keeps the diff tight.
- Resend-invitation handler at L288-296 (unhandled-rejection sibling finding — left in backlog as `code-review-reality-web-realtor-resend-unhandled-rejection`, not promoted this run).

## After-merge
- Update `docs/screens/reality/realtor-management.md` (or the equivalent screen-map doc if the slug differs) Agent Log: `2026-09-22 — routine: surfaced DetailModal save/remove/status error paths`.
- Watch for regressions in `dependabot` bumps of `@tanstack/react-query` — this fix depends on `useMutation`'s error contract.
- Follow-up plan for `code-review-reality-web-realtor-resend-unhandled-rejection` (RealtorManagement.tsx:288-296) — same file, similar shape but strictly an unhandled-rejection + `finally`-without-catch class; left at backlog score 2 pending a stackable second signal or a dispatcher fast-path claim.
