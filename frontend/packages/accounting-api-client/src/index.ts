/**
 * Accounting API Client
 *
 * Generated from OpenAPI specification, scoped to the Accounting tag.
 * Run `pnpm generate-accounting-api` to regenerate after API changes.
 */

// Re-export client infrastructure
export { client } from './generated/client.gen';
export type { CreateClientConfig } from './generated/client.gen';
export { createClient, createConfig } from './generated/client';
export type { ClientOptions, Options } from './generated/types.gen';

// Accounting SDK functions
export {
  contactsApiList,
  invoicesApiCreate,
  invoicesApiDelete,
  invoicesApiGet,
  invoicesApiList,
  invoicesApiListItems,
  invoicesApiUpdate,
} from './generated/sdk.gen';

// Accounting types
export type {
  AccountingContact,
  AccountingCreateInvoiceItem,
  AccountingCreateInvoiceRequest,
  AccountingInvoice,
  AccountingInvoiceItem,
  AccountingInvoiceStatus,
  AccountingUpdateInvoiceRequest,
  AccountingVatRate,
  ContactsApiListData,
  ContactsApiListError,
  ContactsApiListErrors,
  ContactsApiListResponse,
  ContactsApiListResponses,
  InvoicesApiCreateData,
  InvoicesApiCreateError,
  InvoicesApiCreateErrors,
  InvoicesApiCreateResponse,
  InvoicesApiCreateResponses,
  InvoicesApiDeleteData,
  InvoicesApiDeleteError,
  InvoicesApiDeleteErrors,
  InvoicesApiDeleteResponse,
  InvoicesApiDeleteResponses,
  InvoicesApiGetData,
  InvoicesApiGetError,
  InvoicesApiGetErrors,
  InvoicesApiGetResponse,
  InvoicesApiGetResponses,
  InvoicesApiListData,
  InvoicesApiListError,
  InvoicesApiListErrors,
  InvoicesApiListItemsData,
  InvoicesApiListItemsError,
  InvoicesApiListItemsErrors,
  InvoicesApiListItemsResponse,
  InvoicesApiListItemsResponses,
  InvoicesApiListResponse,
  InvoicesApiListResponses,
  InvoicesApiUpdateData,
  InvoicesApiUpdateError,
  InvoicesApiUpdateErrors,
  InvoicesApiUpdateResponse,
  InvoicesApiUpdateResponses,
} from './generated/types.gen';

// Shared types needed by accounting consumers
export type {
  SharedErrorResponse,
  SharedMoney,
  SharedPaginationMeta,
  SharedUuid,
} from './generated/types.gen';
