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
    @header("Authorization") token: string,
    ...PaginationQuery
  ): Contact[] | UnauthorizedError | ForbiddenError;
}

@route("/api/v1/accounting/invoices")
@tag("Accounting")
interface InvoicesApi {
  @doc("List invoices for current tenant")
  @get
  list(
    @header("Authorization") token: string,
    ...PaginationQuery
  ): Invoice[] | UnauthorizedError | ForbiddenError;

  @doc("Create new invoice")
  @post
  create(
    @header("Authorization") token: string,
    @body body: CreateInvoiceRequest
  ): {
    @statusCode statusCode: 201;
    @body body: Invoice;
  } | UnauthorizedError | ForbiddenError | BadRequestError;

  @doc("Get invoice by ID")
  @get
  @route("/{id}")
  get(
    @header("Authorization") token: string,
    @path id: uuid
  ): Invoice | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Update invoice")
  @patch
  @route("/{id}")
  update(
    @header("Authorization") token: string,
    @path id: uuid,
    @body body: UpdateInvoiceRequest
  ): Invoice | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;

  @doc("Delete invoice")
  @delete
  @route("/{id}")
  delete(
    @header("Authorization") token: string,
    @path id: uuid
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("List invoice items")
  @get
  @route("/{id}/items")
  listItems(
    @header("Authorization") token: string,
    @path id: uuid
  ): InvoiceItem[] | UnauthorizedError | ForbiddenError | NotFoundError;
}
