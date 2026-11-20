/**
 * Marketplace pages barrel export.
 * Epic 68: Service Provider Marketplace
 *
 * These 6 pages are INTENTIONALLY not mounted in `src/routes/**`. The feature is
 * a deliberately parked "built-but-unwired" surface (board decision PAP-55);
 * the `marketplace` slug in `src/features/unwired-features.ts` and the guard in
 * `src/test/unwired-features.test.ts` keep it out of the router/nav until a
 * scoped wiring phase. Do not file these as orphaned/dead-code routes.
 */

export * from './CreateRfqPage';
export * from './MarketplacePage';
export * from './ProviderDetailPage';
export * from './ProviderProfilePage';
export * from './QuoteComparisonPage';
export * from './RfqListPage';
