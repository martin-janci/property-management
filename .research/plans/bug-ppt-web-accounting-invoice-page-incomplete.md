# bug-ppt-web-accounting-invoice-page-incomplete

**Vector:** bug
**Score:** 3
**Source:** code-review (ppt-web-core) + issue #1521 + issue #1522
**Confidence:** high

## Hypothesis
`AccountingInvoiceManagementPage` was merged via PR #1454 (PAP-210 / N5 native
accounting MVP) with three connected gaps that ship the page in a half-working
state:

1. The `onViewInvoice` callback is a literal `console.log('View', id)` (line 106) —
   clicking *View* on any row in the production app does nothing observable.
2. Both `useQuery` calls (`invoices` line 23, `contacts` line 34) have no
   `isError` branch — when the backend 401s or 500s, the UI silently renders
   an empty list with no recovery path.
3. The page calls the @hey-api generated client through four
   `auth.headers as unknown as { Authorization: string }` casts (lines 28/39/52/66)
   that fully suppress the typed Authorization-header contract the rest of
   ppt-web is now on, so type drift in the generated client cannot fail the build.

Issues #1521 and #1522 (both untriaged, opened during the PR #1454 review wake)
identify the same code area — #1521 calls the page out for broken auth and no
i18n/tests, #1522 reports the accounting UI bypassing the shared API auth
interceptor after the @hey-api migration. All three signals converge on the
same screen. The smallest correct change is: wire `onViewInvoice` to a real
detail route (or hide the column until the route exists), add `isError` UI on
both queries, and replace the four `as unknown as` casts with the shared auth
header helper the rest of ppt-web uses (looking up the @ppt/api-client typed
shape).

## Evidence
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:106` — `onViewInvoice={(id) => console.log('View', id)}` (verified against `origin/dev` HEAD `82da44a`).
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:23,34` — two `useQuery` calls with `isLoading` destructured but no `isError` / `error` returned to the JSX (verified via grep — JSX block contains no `isError`/`error` reference).
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:28,39,52,66` — four `auth.headers as unknown as { Authorization: string }` casts (one per query/mutation construction) crossing the @ppt/api-client boundary.
- Issue #1521 — "Follow-up: payment-matching UI shipped in wrong app, with broken auth + no i18n/tests (PR #1454)" — opened 2026-06-17, no label.
- Issue #1522 — "Follow-up: accounting UI bypasses shared API auth interceptor after @hey-api migration (PR #1454)" — opened 2026-06-17, no label.
- PR #1454 — `feat: native accounting MVP (N1-N4) + @hey-api migration` (merged 2026-06-17) — the source of the page in its current shape.

## Files
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:106`
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx:28`
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [x] C4 — Browser (Chrome MCP / Preview / playwright) — needed to verify the "View" handler now routes / surfaces a real flow, and to confirm the error UI renders on a forced 401
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode: local-only (reason: C4 — flow needs a real browser to confirm the View click navigates and the error UI shows on a forced 401; cannot be exercised with `cargo test` / `vitest` alone)**

## Repro steps
1. From `origin/dev` HEAD `82da44a`, start ppt-web (`pnpm -F @ppt/ppt-web dev`) against any backend that has at least one invoice row visible to the logged-in user.
2. Sign in as a manager. Navigate to `/accounting/invoices` (the route group PR #1454 added under `frontend/apps/ppt-web/src/routes/groups/accounting.tsx`).
3. Click *View* on any invoice row.
   - Expected: navigate to an invoice-detail screen.
   - Actual: nothing observable; the DevTools console logs `View <uuid>`. No route change.
4. Stop the backend mid-page (so the next `useQuery` refetch 500s).
   - Expected: the page surfaces a localised error state (with retry).
   - Actual: the page renders an empty list as if the user had no invoices.
5. (Optional) Run `pnpm -F @ppt/api-client gen` to regenerate types, then in `AccountingInvoiceManagementPage.tsx` make the `auth.headers` shape drift; build succeeds because the `as unknown as { Authorization: string }` casts erase the type signal.

## Suggested approach
1. Read `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx` end-to-end (118 lines). Identify the four `auth.headers as unknown as { … }` cast sites and the two `useQuery` destructures.
2. Look up the shared auth-header helper used by sibling pages (grep `frontend/apps/ppt-web/src/features` for `useAuthHeaders` or the `@ppt/api-client` `createClient` pattern with an `interceptor`). Replace the four `as unknown as { Authorization: string }` casts with the helper's return type — let the @hey-api generated client see a typed value. If the helper does not exist yet, factor one out under `frontend/apps/ppt-web/src/features/accounting/api/useAccountingAuth.ts` and use it from this page only (issue #1522's "shared interceptor" remediation can land in a follow-up — out of scope for *this* PR).
3. Replace `onViewInvoice={(id) => console.log('View', id)}` (line 106) with either a `useNavigate` push to the existing detail route (search `frontend/apps/ppt-web/src/routes/groups/accounting.tsx` for an invoice-detail route — if it exists, route there) or, if no detail route exists yet, hide the *View* column in `AccountingInvoiceList` and open a tracking issue. **Do not** ship `console.log` as a click handler.
4. Add an `isError`-based fallback to both `useQuery` destructures: render a translated error state (`useTranslation` from `react-i18next`) with a retry button that calls `refetch()`. Mirror the existing error-card pattern used elsewhere in `features/` (grep for `isError &&`).
5. Add a Vitest unit test covering: (a) the stub handler is gone — the rendered *View* button either has an `href`/`to` prop or is not present in the DOM at all; (b) when the `invoices` query is in `isError`, the error-card renders with the i18n key; (c) the four call-sites no longer use `as unknown as` (grep-assert is enough).
6. Run `pnpm -F @ppt/ppt-web typecheck && pnpm -F @ppt/ppt-web test -- AccountingInvoiceManagement`. Visually verify with `pnpm dev:ppt` per *Repro steps* — *View* now navigates (or is hidden), and a forced 500 renders the error card.
7. Open a `fix(ppt-web)` PR. Link issues #1521 + #1522 in the body. Note that the *cross-app* part of #1521 ("shipped in the wrong app") is **out of scope** for this PR — flagged separately.

## Alternatives considered
- **Move the entire accounting page to a follow-up PR and delete the route from PR #1454 retroactively** — rejected because the page is already shipped on `dev` and likely on staging; a delete-then-rebuild PR creates a regression window. Patch in place instead.
- **Replace the stub View handler with a `<Link to={…}>` even though the detail route doesn't exist yet** — rejected because a dead `<Link>` rendering 404 is worse UX than hiding the button. If no detail route exists, hide the column and let a follow-up plan add the route.

## Root-cause trace
1. Symptom: clicking *View* does nothing; backend errors hide instead of surface.
2. ← `AccountingInvoiceManagementPage.tsx:106` — `onViewInvoice` is a `console.log` literal; no `useNavigate`/`<Link>`/router push.
3. ← `AccountingInvoiceManagementPage.tsx:23,34` — `useQuery` destructures `data` / `isLoading` only; the JSX never branches on `isError`, so the renderer treats a 500 the same as an empty success.
4. ← `AccountingInvoiceManagementPage.tsx:28,39,52,66` — `auth.headers as unknown as { Authorization: string }` casts silence the typed @hey-api client; the type system can't warn that the page is sidestepping the project-wide auth-header contract.
5. Origin: PR #1454 — landed the page as an MVP scaffold; the *View* handler and the error UI were placeholder stubs, and the cast pattern was the path-of-least-resistance to compile against the freshly-regenerated @hey-api client. Issues #1521 + #1522 were the post-merge audit catching it.

## Test plan
- [ ] New: `frontend/apps/ppt-web/src/features/accounting/pages/__tests__/AccountingInvoiceManagementPage.test.tsx` — at minimum: (a) the *View* button either has an `href`/`to` prop or is not in the DOM; (b) rendering with a query that's `isError` shows the translated error card; (c) grep-assert that the source file contains zero `as unknown as` occurrences. Fails on `origin/dev` HEAD `82da44a` (IG3), passes after the fix.
- [ ] Regression: the existing PR #1454 tests (if any — grep `pnpm -F @ppt/ppt-web test --listTests | grep -i accounting`) continue to pass.
- [ ] Run: `pnpm -F @ppt/ppt-web typecheck && pnpm -F @ppt/ppt-web test -- AccountingInvoiceManagement`.

## Out of scope
- Migrating the page (or the entire `features/accounting/` tree) to the *other* app, as issue #1521's "wrong app" comment suggests — that's a re-org call for the user and a separate plan.
- Adding the missing i18n keys called out in issue #1521 — they belong to the same area but are a separate translation pass; track in a follow-up.
- Fixing the project-wide "accounting UI bypasses shared API auth interceptor" architectural issue from #1522 — only the local cast cleanup is in scope; the interceptor wiring is its own plan.
- Touching `AccountingInvoiceList` beyond the *View* column visibility decision in Suggested-approach step 3.

## After-merge
- Move this file to `plans/_archive/bug-ppt-web-accounting-invoice-page-incomplete.md`
- Mark `bug-ppt-web-accounting-invoice-page-incomplete` in `backlog.json` as `status: "done"` with `resolution: "PR #<N> — …"`.
- Close GitHub issues #1521 and #1522 if the PR's scope addressed their core complaints; otherwise leave a triage comment narrowing each to its un-addressed remainder.
