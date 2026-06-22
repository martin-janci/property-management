/**
 * Unwired feature registry — Stable Beta (PAP-55 / WS-C, decision on PAP-28).
 *
 * These 10 ppt-web feature directories are **fully built** (1.0k–5.7k LOC each)
 * with live, mounted api-server backends, but are intentionally **not wired**
 * into the router or navigation. Board decision (PAP-51 confirmation, accepted):
 *
 *   > Hide the built-but-unwired ppt-web features from nav now, schedule wiring
 *   > later. KEEP THE CODE. Do not expose half-built surfaces to paying
 *   > customers.
 *
 * "Hidden" here means: no `routes/groups/<feature>.tsx` mounts them, no `<Route>`
 * references them, no nav `<Link>` / command-palette entry / `navigate()` call
 * targets them, and they are absent from `@ppt/sitemap`. The code is retained so
 * each can be wired in a deliberate, scoped phase later (route group +
 * `@ppt/api-client` module + UX sign-off per feature).
 *
 * This list is the source of truth for the regression guard in
 * `src/test/unwired-features.test.ts`, which fails CI if any of these slugs is
 * re-introduced into the routing surface without being removed from this list —
 * i.e. it prevents an accidental, ungated re-exposure to customers.
 *
 * To wire one of these features in a future phase: build its route group + nav
 * entry, then DELETE its slug from this array (the guard test will then permit
 * the wiring). Do not silence the guard test.
 *
 * @see /PAP/issues/PAP-55 — Hide unwired ppt-web features from nav (keep code)
 * @see /PAP/issues/PAP-28 — Roadmap decision: keep-and-schedule vs retire
 * @see /PAP/issues/PAP-24 — §8.2 catalogue of built-but-unwired surface
 * @see /PAP/issues/PAP-33 — Retired 7 dead scaffolds (deleted, not guarded):
 *   migration, integrations, multi-currency, api-ecosystem, delegation,
 *   data-residency, packages. Those dirs no longer exist on `dev`, so the guard
 *   no longer tracks them (a deleted feature cannot be re-exposed). This list
 *   was reconciled from 18 → 11 when PAP-55 landed on top of PAP-33, then
 *   11 → 10 when `person-months` was deliberately wired (Story 3.5, #1714):
 *   its route group (`routes/groups/person-months.tsx`) is mounted in
 *   `AppRoutes.tsx`, so it is no longer hidden and is removed from the guard.
 */
export const UNWIRED_FEATURES = [
  'insurance',
  'marketplace',
  'forms',
  'onboarding',
  'critical-notifications',
  'subscription',
  'government-portal',
  'compliance',
  'registry',
  'portfolio-performance',
] as const;

export type UnwiredFeature = (typeof UNWIRED_FEATURES)[number];
