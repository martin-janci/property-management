import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Accounting;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Native contact (CRM)")
model Contact {
  @doc("Unique identifier")
  id: uuid;

  @doc("Tenant identifier")
  tenantId: uuid;

  @doc("Contact name")
  name: string;

  @doc("ICO (identification number)")
  ico?: string;

  @doc("DIC (tax identification number)")
  dic?: string;

  @doc("Email address")
  email?: email;

  @doc("Phone number")
  phone?: string;

  @doc("Physical address")
  address?: string;

  @doc("IBAN for payments")
  iban?: string;

  ...AuditableEntity;
}

@doc("Status of an issued invoice")
enum InvoiceStatus {
  Draft: "draft",
  Issued: "issued",
  Paid: "paid",
  PartiallyPaid: "partially_paid",
  Overdue: "overdue",
}

@doc("VAT rate types for CZ/SK")
enum VatRate {
  @doc("Zero VAT (non-taxable) - 0%")
  Zero: "zero",
  @doc("Reduced rate (10% in CZ, 10% in SK)")
  Reduced: "reduced",
  @doc("First reduced rate (15% in CZ, currently not used in SK)")
  FirstReduced: "first_reduced",
  @doc("Standard rate (21% in CZ, 20% in SK)")
  Standard: "standard",
}

@doc("Native issued invoice")
model Invoice {
  @doc("Unique identifier")
  id: uuid;

  @doc("Tenant identifier")
  tenantId: uuid;

  @doc("Contact identifier")
  contactId: uuid;

  @doc("Invoice number")
  number: string;

  @doc("Issue date")
  issueDate: date;

  @doc("Due date")
  dueDate: date;

  @doc("Taxable supply date")
  taxableSupplyDate?: date;

  @doc("Currency code (e.g., CZK)")
  currency: string;

  @doc("Total amount including VAT")
  totalAmount: float64;

  @doc("Base amount excluding VAT")
  baseAmount: float64;

  @doc("VAT amount")
  vatAmount: float64;

  @doc("Variable symbol for payment identification")
  variableSymbol?: string;

  @doc("Current status")
  status: InvoiceStatus;

  @doc("Amount already paid")
  paidAmount: float64;

  ...AuditableEntity;
}

@doc("Line item for an invoice")
model InvoiceItem {
  @doc("Unique identifier")
  id: uuid;

  @doc("Invoice identifier")
  invoiceId: uuid;

  @doc("Tenant identifier")
  tenantId: uuid;

  @doc("Description of the item")
  description: string;

  @doc("Quantity")
  qty: float64;

  @doc("Unit price excluding VAT")
  unitPrice: float64;

  @doc("VAT rate percentage")
  vatRate: float64;

  @doc("Total amount including VAT")
  totalAmount: float64;

  @doc("Base amount excluding VAT")
  baseAmount: float64;

  @doc("VAT amount")
  vatAmount: float64;
}

@doc("Request to create a new invoice")
model CreateInvoiceRequest {
  @doc("Contact identifier")
  contactId: uuid;

  @doc("Invoice number")
  number: string;

  @doc("Issue date")
  issueDate: date;

  @doc("Due date")
  dueDate: date;

  @doc("Taxable supply date")
  taxableSupplyDate?: date;

  @doc("Currency code (e.g., CZK)")
  currency: string;

  @doc("Variable symbol")
  variableSymbol?: string;

  @doc("Initial status")
  status?: InvoiceStatus;

  @doc("Invoice line items")
  items: CreateInvoiceItem[];

  @doc("Optional country code (e.g., 'CZ', 'SK') to resolve named VAT rates")
  country?: string;
}

@doc("Request to create a new invoice item")
model CreateInvoiceItem {
  @doc("Description")
  description: string;

  @doc("Quantity")
  qty: float64;

  @doc("Unit price excluding VAT")
  unitPrice: float64;

  @doc("VAT rate percentage")
  vatRate: float64;

  @doc("Optional named VAT rate (overrides vatRate if provided)")
  vatRateType?: VatRate;
}

@doc("Request to update an existing invoice")
model UpdateInvoiceRequest {
  @doc("Contact identifier")
  contactId?: uuid;

  @doc("Invoice number")
  number?: string;

  @doc("Issue date")
  issueDate?: date;

  @doc("Due date")
  dueDate?: date;

  @doc("Taxable supply date")
  taxableSupplyDate?: date;

  @doc("Currency code")
  currency?: string;

  @doc("Variable symbol")
  variableSymbol?: string;

  @doc("Status")
  status?: InvoiceStatus;

  @doc("Paid amount")
  paidAmount?: float64;
}

@route("/api/v1/accounting/contacts")
@tag("Accounting")
interface ContactsApi {
  @doc("List contacts for current tenant")
  @get
  list(
    @header("Authorization") token?: string,
    ...PaginationQuery
  ): Contact[] | UnauthorizedError | ForbiddenError;
}

@route("/api/v1/accounting/invoices")
@tag("Accounting")
interface InvoicesApi {
  @doc("List invoices for current tenant")
  @get
  list(
    @header("Authorization") token?: string,
    ...PaginationQuery
  ): Invoice[] | UnauthorizedError | ForbiddenError;

  @doc("Create new invoice")
  @post
  create(
    @header("Authorization") token?: string,
    @body body: CreateInvoiceRequest
  ): {
    @statusCode statusCode: 201;
    @body body: Invoice;
  } | UnauthorizedError | ForbiddenError | BadRequestError;

  @doc("Get invoice by ID")
  @get
  @route("/{id}")
  get(
    @header("Authorization") token?: string,
    @path id: uuid
  ): Invoice | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Update invoice")
  @patch
  @route("/{id}")
  update(
    @header("Authorization") token?: string,
    @path id: uuid,
    @body body: UpdateInvoiceRequest
  ): Invoice | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;

  @doc("Delete invoice")
  @delete
  @route("/{id}")
  delete(
    @header("Authorization") token?: string,
    @path id: uuid
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("List invoice items")
  @get
  @route("/{id}/items")
  listItems(
    @header("Authorization") token?: string,
    @path id: uuid
  ): InvoiceItem[] | UnauthorizedError | ForbiddenError | NotFoundError;
}

// ============================================================================
// Bank statements & payment matching (booking-import → invoice reconciliation).
//
// NOTE: these models use snake_case field names to match what the api-server
// actually serializes (the handlers return the `db::models::accounting` structs
// directly, which have no `serde(rename_all)`). Decimal amounts are emitted as
// strings (`rust_decimal` `serde-str`), so `amount`/`confidence` are `string`.
// ============================================================================

@doc("Imported bank statement metadata")
model BankStatement {
  @doc("Unique identifier")
  id: uuid;

  @doc("Tenant identifier")
  tenant_id: uuid;

  @doc("Original uploaded filename")
  source_filename: string;

  @doc("When the statement was imported")
  imported_at: dateTime;

  @doc("Statement period label, if parsed")
  period?: string;

  @doc("Account IBAN the statement belongs to")
  account_iban: string;
}

@doc("A single transaction line parsed from a bank statement")
model BankStatementLine {
  @doc("Unique identifier")
  id: uuid;

  @doc("Parent statement identifier")
  statement_id: uuid;

  @doc("Tenant identifier")
  tenant_id: uuid;

  @doc("Booking date of the transaction")
  booking_date: date;

  @doc("Transaction amount (decimal serialized as string)")
  amount: string;

  @doc("Currency code")
  currency: string;

  @doc("Counterparty IBAN, if present")
  counterparty_iban?: string;

  @doc("Variable symbol used for payment identification")
  variable_symbol?: string;

  @doc("Raw reference text from the source row")
  raw_ref?: string;

  @doc("Match state: unmatched | suggested | matched")
  match_state: string;
}

@doc("State of a payment-match decision")
enum PaymentMatchState {
  Suggested: "suggested",
  Confirmed: "confirmed",
  Rejected: "rejected",
}

@doc("A suggested or decided match between a statement line and an invoice")
model PaymentMatch {
  @doc("Unique identifier")
  id: uuid;

  @doc("Tenant identifier")
  tenant_id: uuid;

  @doc("Statement line being matched")
  statement_line_id: uuid;

  @doc("Invoice the line is matched to")
  invoice_id: uuid;

  @doc("Match confidence in 0..1 (decimal serialized as string)")
  confidence: string;

  @doc("User who decided the match, if decided")
  decided_by?: uuid;

  @doc("When the match was decided, if decided")
  decided_at?: dateTime;

  @doc("Current state of the match")
  state: PaymentMatchState;
}

@route("/api/v1/accounting/statements")
@tag("Accounting")
interface StatementsApi {
  @doc("List imported bank statements for the current tenant")
  @get
  list(
    @header("Authorization") token?: string
  ): BankStatement[] | UnauthorizedError | ForbiddenError;

  @doc("List the parsed lines of a bank statement")
  @get
  @route("/{id}/lines")
  listLines(
    @header("Authorization") token?: string,
    @path id: uuid
  ): BankStatementLine[] | UnauthorizedError | ForbiddenError | NotFoundError;
}

@route("/api/v1/accounting/lines")
@tag("Accounting")
interface StatementLinesApi {
  @doc("List payment-match suggestions/decisions for a statement line")
  @get
  @route("/{id}/matches")
  listMatches(
    @header("Authorization") token?: string,
    @path id: uuid
  ): PaymentMatch[] | UnauthorizedError | ForbiddenError | NotFoundError;
}

@route("/api/v1/accounting/matches")
@tag("Accounting")
interface PaymentMatchesApi {
  @doc("Confirm a payment match (link the statement line to the invoice)")
  @post
  @route("/{id}/confirm")
  confirm(
    @header("Authorization") token?: string,
    @path id: uuid
  ): {
    @statusCode statusCode: 200;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Reject a payment match")
  @post
  @route("/{id}/reject")
  reject(
    @header("Authorization") token?: string,
    @path id: uuid
  ): {
    @statusCode statusCode: 200;
  } | UnauthorizedError | ForbiddenError | NotFoundError;
}
