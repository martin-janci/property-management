/**
 * Unwired feature registry — Stable Beta (PAP-55 / WS-C, decision on PAP-28).
 *
 * These 18 ppt-web feature directories are **fully built** (1.0k–5.7k LOC each)
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
 * @see /PAP/issues/PAP-55 — Hide 18 unwired ppt-web features from nav (keep code)
 * @see /PAP/issues/PAP-28 — Roadmap decision: keep-and-schedule vs retire
 * @see /PAP/issues/PAP-24 — §8.2 catalogue of built-but-unwired surface
 */
export const UNWIRED_FEATURES = [
  'insurance',
  'marketplace',
  'forms',
  'onboarding',
  'critical-notifications',
  'migration',
  'subscription',
  'government-portal',
  'integrations',
  'compliance',
  'registry',
  'multi-currency',
  'portfolio-performance',
  'api-ecosystem',
  'delegation',
  'person-months',
  'data-residency',
  'packages',
] as const;

export type UnwiredFeature = (typeof UNWIRED_FEATURES)[number];
