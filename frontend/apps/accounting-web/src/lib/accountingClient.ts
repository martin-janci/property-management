/**
 * accounting-web SDK base-URL helper (Phase 2 reconciliation shim).
 *
 * FE-Screens' typed hook modules (`src/components/{invoices,contacts}/api.ts`)
 * import `getAccountingApiBase` from `@/lib/accountingClient` (see
 * contract-notes/fe-screens.md #1), while FE-Scaffold exposed the canonical
 * resolver from `@/config/api`. This module re-exports the canonical resolver so
 * both halves agree on a single base URL without either coder editing the
 * other's files.
 */
export { getAccountingApiBase, DEFAULT_ACCOUNTING_API_URL } from '@/config/api';
