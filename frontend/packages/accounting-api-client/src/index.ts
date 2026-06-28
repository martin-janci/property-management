/**
 * Accounting Portal API Client (@ppt/accounting-api-client)
 *
 * Typed client for accounting-server (resource server on :8082, validates
 * api-server-issued JWTs). Consumed by @ppt/accounting-web.
 *
 * The generated hey-api client is produced by `pnpm generate` from the
 * server's OpenAPI (CONTRACT §8) into `./generated`. Until Phase 2 wires the
 * real spec, the hand-typed fallback below provides the full typed surface so
 * accounting-web typechecks. After generation, uncomment the generated
 * re-export; the hook surface (useContacts/useInvoices/…) stays identical.
 */

// Generated client + types (enabled in Phase 2 once the spec is exported):
// export * from './generated';

// Low-level transport + structured error.
export { AccountingApiError, request } from './client';
// Runtime config: base URL + Bearer-token wiring (host app calls
// configureAccountingApi once at startup).
export {
  API_PREFIX,
  apiUrl,
  authHeaders,
  configureAccountingApi,
  DEFAULT_ACCOUNTING_API_URL,
  getApiBase,
  getAuthToken,
  resetAccountingApi,
  type TokenProvider,
} from './config';
// Domain hooks.
export * from './contacts';
export * from './invoices';
// tanstack-query keys.
export { contactKeys, invoiceKeys } from './query-keys';

// Hand-typed fallback domain types (superseded by ./generated in Phase 2).
export type {
  Contact,
  ContactKind,
  ContactRequest,
  CreateInvoice,
  DecimalString,
  DocType,
  Invoice,
  InvoiceItem,
  InvoiceItemInput,
  InvoiceStatus,
  InvoiceWithItems,
  Timestamp,
  UpdateInvoice,
  Uuid,
} from './types';

/** SDK version marker (mirrors reality-api-client's version export). */
export const ACCOUNTING_API_VERSION = '1.0.0';
