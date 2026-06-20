# security-accounting-ui-auth-interceptor-bypass

**Vector:** security
**Score:** 2
**Source:** Issue #1522 · PR #1454 (post-merge reviewer finding) · `features/accounting/hooks/useAccountingAuth.ts`
**Confidence:** high

## Hypothesis
PR #1454 swapped ppt-web's default API client from `OpenAPI.BASE` (axios) to the `@hey-api` flat `client` instance in `main.tsx`, but the new client is not wired to the shared axios interceptor in `lib/api.ts` that injects auth and performs silent 401→refresh. To compensate, the accounting feature added a bespoke `useAccountingAuth` hook that hand-assembles `Authorization` + `X-Tenant-ID` headers and threads them through every call in `AccountingInvoiceManagementPage.tsx` via `headers: auth.headers as unknown as { Authorization: string }` casts at 4 call sites. Consequence: managers with expired access tokens get a hard error on accounting screens instead of a transparent refresh, and the type-safety escape hatch hides the contract mismatch. Centralizing auth on the new `@hey-api` client (request interceptor injects `Authorization`/`X-Tenant-ID` from `AuthContext`; response interceptor mirrors the 401-refresh from `lib/api.ts`) eliminates the parallel auth path and unblocks moving the remaining ~200 legacy callers onto the new surface later.

## Evidence
- `frontend/apps/ppt-web/src/main.tsx` — registers `client.setConfig({ baseUrl })` for the new `@hey-api` client with no interceptors.
- `frontend/apps/ppt-web/src/lib/api.ts` — the legacy axios instance has the request/response interceptors (auth injection + 401-refresh) that the new client lacks.
- `frontend/apps/ppt-web/src/features/accounting/hooks/useAccountingAuth.ts` — bespoke hook that hand-assembles `Authorization` + `X-Tenant-ID` from `AuthContext`.
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx` — 4 call sites with `headers: auth.headers as unknown as { Authorization: string }` casts; ad-hoc inline `['accounting', 'invoices']` / `['accounting', 'contacts']` query keys (Issue #1522 finding 2); stub `onViewInvoice={(id) => console.log('View', id)}` shipped to merged code.
- Backend half (N1–N4) is solid: migrations `00183`/`00184` force RLS on all 11 new tables; money columns are `NUMERIC(18,2)` modeled as `rust_decimal::Decimal`; `accounting_rls_repo_tests.rs` (823 lines) covers cross-tenant READ/WRITE isolation. Issue is purely on the ppt-web client side.

## Files
- `frontend/apps/ppt-web/src/main.tsx`
- `frontend/apps/ppt-web/src/lib/api.ts`
- `frontend/apps/ppt-web/src/features/accounting/hooks/useAccountingAuth.ts`
- `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.tsx`
- `frontend/apps/ppt-web/src/lib/queryKeys.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 → cloud-ok.

Mode: cloud-ok

## Repro steps
1. Log into ppt-web as a manager; open the Accounting → Invoices page (`AccountingInvoiceManagementPage.tsx`).
2. Wait for the access token to expire (or shorten its TTL in dev) while the page is open.
3. Trigger a list refresh (refetch the invoices query).
4. Expected: silent 401-refresh from the response interceptor; the list reloads.
5. Actual (on `dev`): hard error from the accounting query — no refresh — because `@hey-api` client has no response interceptor (the refresh logic lives only on the legacy axios instance in `lib/api.ts`).

## Suggested approach
1. Create `frontend/apps/ppt-web/src/lib/apiClient.ts`: register a request interceptor on the `@hey-api` `client` that pulls the access token + active org from `AuthContext` and sets `Authorization`/`X-Tenant-ID`. Register a response interceptor that mirrors the existing 401→refresh path from `lib/api.ts` (single source of truth — refactor the refresh helper into a shared function both interceptors use).
2. In `main.tsx`, call the new `apiClient` setup at the same place that currently calls `client.setConfig({ baseUrl })`. The `AuthContext` must be available at that point — wire it via a lazy provider read or pass the access-token getter as a closure.
3. Delete `features/accounting/hooks/useAccountingAuth.ts` entirely.
4. In `AccountingInvoiceManagementPage.tsx`: remove the `auth = useAccountingAuth()` call, drop the `headers: auth.headers as unknown as { Authorization: string }` argument from all 4 call sites, and remove the `as unknown as` cast.
5. Add `accounting` namespace to `lib/queryKeys.ts` (`accounting.invoices()`, `accounting.contacts()`, etc.) and replace the inline query keys in `AccountingInvoiceManagementPage.tsx` (Issue #1522 finding 2).
6. Replace `onViewInvoice={(id) => console.log('View', id)}` with a real navigation/detail action (route to the invoice detail page), or remove the stub entirely and let the list render without a click handler until the detail page exists (Issue #1522 finding 2).
7. Validate with `pnpm -F @ppt/ppt-web check && pnpm -F @ppt/ppt-web typecheck && pnpm -F @ppt/ppt-web test`.

## Alternatives considered
- **Add a one-shot 401-refresh inside `useAccountingAuth`** — rejected because it leaves the parallel-auth path in place and forces every future feature onto the same pattern; the ~200 legacy callers blocked behind this issue all want the centralized interceptor.
- **Keep the legacy axios client for everything and roll back the `@hey-api` switch in `main.tsx`** — rejected because PR #1454 codified `@hey-api` as the path forward (generated typed clients depend on the flat `client`); rolling back would invalidate the generated-client effort and the migration intent, and the centralized-interceptor fix is the cheaper compatible path.

## Root-cause trace
N/A — security vector, fix is a direct architectural correction (move auth wiring from a per-feature hook to the global client). The "trace backward" pattern doesn't apply: the issue is missing wiring, not a leaked boundary at runtime.

## Test plan
- [ ] Add `frontend/apps/ppt-web/src/lib/apiClient.test.ts`: assert the request interceptor sets `Authorization`/`X-Tenant-ID` from `AuthContext` and the response interceptor invokes the refresh helper on 401 (uses existing test scaffolding for `lib/api.ts` if present).
- [ ] Add `frontend/apps/ppt-web/src/features/accounting/pages/AccountingInvoiceManagementPage.test.tsx` (or extend the existing one): mock a 401 on the first `list_invoices` call and assert silent refetch after refresh (regression — fails on `dev` because no interceptor is wired).
- [ ] Confirm no `as unknown as` casts remain in `features/accounting/**` (grep the source tree).
- [ ] Command: `pnpm -F @ppt/ppt-web check && pnpm -F @ppt/ppt-web typecheck && pnpm -F @ppt/ppt-web test`.

## Out of scope
- The per-invoice rounding correctness item (Issue #1522 finding 3 — backend) is a separate `rust-backend` follow-up; this plan stays on the ppt-web side.
- Migrating the ~200 legacy callers from the axios client to the `@hey-api` client — landing the centralized interceptor unblocks that work but does not perform it.

## After-merge
- Move this file to `plans/_archive/security-accounting-ui-auth-interceptor-bypass.md`
- Mark the matching `backlog.json` row as `status: "done"`
