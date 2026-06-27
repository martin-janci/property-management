# code-review-ppt-web-core-payment-matching-no-err

**Vector:** bug
**Score:** 2
**Source:** code-review-2026-06-27 (Phase 1.5 ppt-web-core)
**Confidence:** high

## Hypothesis
`PaymentMatchingPage.tsx` exposes manager-facing confirm/reject mutations whose `useMutation` configs define `onSuccess` (with query invalidation) but no `onError`. When the backend returns 4xx/5xx or the network fails, `isPending` toggles back to `false` and the button re-enables, but no user-facing error indication is rendered — the manager believes the match was applied when it was not. Wire `onError` (toast + persistent inline error from the mutation's `error` field) so the failure path is observable.

## Evidence
- `frontend/apps/ppt-web/src/features/accounting/pages/PaymentMatchingPage.tsx:61-73` — `confirmMutation` and `rejectMutation` both define `onSuccess` with `queryClient.invalidateQueries(...)` but no `onError`.
- `frontend/apps/ppt-web/src/features/accounting/pages/PaymentMatchingPage.tsx:329,337` — confirm/reject buttons disable on `isPending` only; on resolve no `isError` check renders.
- Rotating code review (Phase 1.5, segment `ppt-web-core`, 2026-06-27) — frontend expert finding.

## Files
- `frontend/apps/ppt-web/src/features/accounting/pages/PaymentMatchingPage.tsx`

## Dependencies
<!-- none -->

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Mode: cloud-ok** (no browser or device needed — unit-level React Testing Library is sufficient to pin the new behavior)

## Repro steps
1. Open `PaymentMatchingPage` in a manager session with at least one `pending` payment match candidate.
2. Stub the network so `POST /v1/accounting/payment-matches/:id/confirm` returns 500 (or fail by stopping the api-server).
3. Click **Confirm**.
4. Expected: a toast or inline message says the confirm failed; the button re-enables and the row stays `pending`.
5. Actual: the button re-enables but no failure indication is rendered; the row's state is indistinguishable from "user hasn't clicked yet". Manager re-clicks; mutation fires again; nothing visible changes.

## Suggested approach
1. Add a project-default `onError` config to each `useMutation` in `PaymentMatchingPage.tsx:61-73` — call the existing toast helper (look for a `useToast`/`toast.error` hook used elsewhere in the accounting feature) and store the error so it can be rendered inline.
2. Surface the mutation's `error` field next to the confirm/reject buttons (lines 329/337) — small text in red with the parsed server message, falling back to `'Confirmation failed — please retry'`.
3. Mirror the same pattern for the reject mutation — keep the two error placements consistent.
4. If the codebase has a shared `useMutationWithToast` wrapper, use it instead of inlining a fresh `onError` (check `frontend/apps/ppt-web/src/api/` and `frontend/apps/ppt-web/src/lib/`).
5. Add a vitest test that fakes the mutation to reject and asserts the toast/error UI appears.

## Alternatives considered
- **Global mutation error boundary** — rejected because the page-local pattern in the rest of `frontend/apps/ppt-web/src/features/accounting/` already opts into per-mutation `onError`; introducing a boundary would diverge from the file's conventions.
- **Only invalidate queries on error (so server state is re-pulled)** — rejected because the silent re-pull doesn't tell the manager *anything failed*; the bug is invisibility, not staleness.

## Root-cause trace
1. Symptom: confirm/reject 4xx/5xx fails silently; manager unaware match did not apply.
2. ← `PaymentMatchingPage.tsx:61-73` — `useMutation({ onSuccess })` defines no `onError`.
3. ← TanStack Query default: on no-handler error, the mutation transitions to `error` state but the component renders nothing for it. The mutation's `error`/`isError` are exposed but neither button site reads them.
4. Origin: page authored without an error UX contract — the matching feature shipped focused on the happy path. No specific commit introduced the bug; it's a pattern omission since the page's first land.

## Test plan
- [ ] `frontend/apps/ppt-web/src/features/accounting/pages/PaymentMatchingPage.test.tsx` — fail-on-main: mock the confirm mutation to reject with a 500 response, click confirm, assert `screen.getByRole('alert')` (or the toast/inline element) renders the error string.
- [ ] Regression: assert the success-path test still passes (existing `onSuccess` query invalidation), so the patch doesn't drop the happy-path assertion.
- [ ] `pnpm --filter @ppt/ppt-web test PaymentMatchingPage`

## Out of scope
- Reworking the confirm/reject API contract or error-shape parsing.
- Reorganising other accounting pages — this plan is scoped to `PaymentMatchingPage.tsx`.
- Refactoring `useMutation` into a shared wrapper unless the codebase already has one in use.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-payment-matching-no-err.md`
- Mark the matching `backlog.json` row as `status: "done"`
