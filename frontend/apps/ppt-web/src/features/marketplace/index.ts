/**
 * Marketplace feature barrel export.
 * Epic 68: Service Provider Marketplace
 *
 * INTENTIONALLY UNWIRED — do not treat the absence of a `routes/groups/`
 * mount as an orphaned-routes/dead-code defect. This feature is a deliberately
 * parked "built-but-unwired" surface per board decision PAP-55: the UI is fully
 * built with a live api-server backend but is kept OUT of the router/nav until a
 * scoped wiring phase. The `marketplace` slug is listed in
 * `src/features/unwired-features.ts` (source of truth) and enforced by the
 * regression guard in `src/test/unwired-features.test.ts`, which fails CI if the
 * feature is wired without first removing it from that registry. Wiring or
 * deleting this feature here is NOT permitted; see unwired-features.ts for the
 * deliberate-wiring procedure.
 *
 * Note: `/community/marketplace` (features/community/pages/MarketplacePage.tsx)
 * is a DIFFERENT, wired feature (resident classifieds) — not this Epic 68
 * service-provider marketplace.
 */

export * from './components';
export * from './pages';
