/**
 * Financial API client (Epic 11/52).
 *
 * Client functions for financial management endpoints.
 */

import { getOrg, getToken } from '../auth';
import { client } from '../generated/client.gen';
import type {
  AccountsReceivableReport,
  AccountTransaction,
  AllocatePaymentRequest,
  ARReportParams,
  ARReportTotals,
  AutoMatchResult,
  BalanceSheetParams,
  BalanceSheetReport,
  CashFlowReport,
  CreateFeeSchedule,
  CreateFinancialAccount,
  CreateInvoice,
  CreateTransaction,
  DateRangeReportParams,
  ExportReportParams,
  FeeSchedule,
  FinancialAccount,
  FinancialAccountResponse,
  IncomeStatement,
  Invoice,
  InvoiceItem,
  InvoiceResponse,
  LateFeeConfig,
  ListAccountsParams,
  ListFeeSchedulesParams,
  ListInvoicesParams,
  ListInvoicesResponse,
  ListPaymentsParams,
  ListTransactionsParams,
  Payment,
  PaymentAllocation,
  PaymentListResponse,
  PaymentResponse,
  RecordPayment,
  ReminderSchedule,
  ReportType,
  UnitFee,
} from './types';

const API_BASE = '/api/v1/financial';

async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  // These are raw `fetch` calls (not the generated @hey-api client), so they
  // don't get the centralized auth interceptor (#1616). Without this, every
  // financial request went out with no Authorization / X-Tenant-ID and 401'd
  // against the api-server. Draw auth from the same token / active-org providers
  // and prefix the same configured base URL the generated client uses.
  const token = getToken();
  const org = getOrg();
  const baseUrl = client.getConfig().baseUrl ?? '';

  const response = await fetch(`${baseUrl}${url}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(org ? { 'X-Tenant-ID': org } : {}),
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

// ---------------------------------------------------------------------------
// Decimal coercion (#975).
//
// The api-server serialises `rust_decimal` money fields as JSON STRINGS
// (serde-str), but these client types expose them as `number` for ergonomic
// arithmetic and formatting. Normalise the known decimal fields from the wire
// here — at the single client boundary — so callers get real numbers and don't,
// e.g., string-concatenate balances. Genuine integer fields (counts, limits,
// page sizes) are left untouched.
// ---------------------------------------------------------------------------
const toNum = (v: number | string | null | undefined): number =>
  typeof v === 'string' ? Number(v) : (v ?? 0);
const toNumOpt = (v: number | string | null | undefined): number | undefined =>
  v === null || v === undefined ? undefined : toNum(v);

function coerceAccount(a: FinancialAccount): FinancialAccount {
  return { ...a, balance: toNum(a.balance), opening_balance: toNum(a.opening_balance) };
}
function coerceTransaction(t: AccountTransaction): AccountTransaction {
  return { ...t, amount: toNum(t.amount), balance_after: toNum(t.balance_after) };
}
function coerceFeeSchedule(f: FeeSchedule): FeeSchedule {
  return { ...f, amount: toNum(f.amount) };
}
function coerceUnitFee(u: UnitFee): UnitFee {
  return { ...u, override_amount: toNumOpt(u.override_amount) };
}
function coerceInvoice(i: Invoice): Invoice {
  return {
    ...i,
    subtotal: toNum(i.subtotal),
    tax_amount: toNum(i.tax_amount),
    total: toNum(i.total),
    amount_paid: toNum(i.amount_paid),
    balance_due: toNum(i.balance_due),
  };
}
function coerceInvoiceItem(it: InvoiceItem): InvoiceItem {
  return {
    ...it,
    quantity: toNum(it.quantity),
    unit_price: toNum(it.unit_price),
    amount: toNum(it.amount),
    tax_rate: toNumOpt(it.tax_rate),
    tax_amount: toNumOpt(it.tax_amount),
  };
}
function coercePayment(p: Payment): Payment {
  return { ...p, amount: toNum(p.amount) };
}
function coerceAllocation(a: PaymentAllocation): PaymentAllocation {
  return { ...a, amount: toNum(a.amount) };
}
function coerceLateFee(l: LateFeeConfig): LateFeeConfig {
  return { ...l, fee_amount: toNumOpt(l.fee_amount), max_fee_amount: toNumOpt(l.max_fee_amount) };
}
/** Coerce the five aging-bucket money fields shared by entries and totals. */
function coerceArBuckets<T extends ARReportTotals>(b: T): T {
  return {
    ...b,
    current: toNum(b.current),
    days_30: toNum(b.days_30),
    days_60: toNum(b.days_60),
    days_90_plus: toNum(b.days_90_plus),
    total: toNum(b.total),
  };
}
function coerceAccountResponse(r: FinancialAccountResponse): FinancialAccountResponse {
  return {
    account: coerceAccount(r.account),
    recent_transactions: r.recent_transactions.map(coerceTransaction),
  };
}
function coerceInvoiceResponse(r: InvoiceResponse): InvoiceResponse {
  return {
    invoice: coerceInvoice(r.invoice),
    items: r.items.map(coerceInvoiceItem),
    payments: r.payments.map(coerceAllocation),
  };
}
function coercePaymentResponse(r: PaymentResponse): PaymentResponse {
  return {
    payment: coercePayment(r.payment),
    allocations: r.allocations.map(coerceAllocation),
  };
}

// ============================================================================
// ACCOUNTS
// ============================================================================

export async function createAccount(
  organizationId: string,
  data: CreateFinancialAccount
): Promise<FinancialAccount> {
  return fetchApi<FinancialAccount>(`${API_BASE}/accounts`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, ...data }),
  }).then(coerceAccount);
}

export async function listAccounts(params: ListAccountsParams): Promise<FinancialAccount[]> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) {
    searchParams.set('building_id', params.building_id);
  }
  return fetchApi<FinancialAccount[]>(`${API_BASE}/accounts?${searchParams}`).then((a) =>
    a.map(coerceAccount)
  );
}

export async function getAccount(id: string): Promise<FinancialAccountResponse> {
  return fetchApi<FinancialAccountResponse>(`${API_BASE}/accounts/${id}`).then(
    coerceAccountResponse
  );
}

export async function listTransactions(
  accountId: string,
  params?: ListTransactionsParams
): Promise<AccountTransaction[]> {
  const searchParams = new URLSearchParams();
  if (params?.from) searchParams.set('from', params.from);
  if (params?.to) searchParams.set('to', params.to);
  if (params?.limit) searchParams.set('limit', String(params.limit));
  if (params?.offset) searchParams.set('offset', String(params.offset));
  return fetchApi<AccountTransaction[]>(
    `${API_BASE}/accounts/${accountId}/transactions?${searchParams}`
  ).then((t) => t.map(coerceTransaction));
}

export async function createTransaction(
  accountId: string,
  data: Omit<CreateTransaction, 'account_id'>
): Promise<AccountTransaction> {
  return fetchApi<AccountTransaction>(`${API_BASE}/accounts/${accountId}/transactions`, {
    method: 'POST',
    body: JSON.stringify({ account_id: accountId, ...data }),
  }).then(coerceTransaction);
}

export async function getUnitLedger(unitId: string): Promise<FinancialAccountResponse> {
  return fetchApi<FinancialAccountResponse>(`${API_BASE}/units/${unitId}/ledger`).then(
    coerceAccountResponse
  );
}

// ============================================================================
// FEE SCHEDULES
// ============================================================================

export async function createFeeSchedule(
  organizationId: string,
  userId: string,
  data: CreateFeeSchedule
): Promise<FeeSchedule> {
  return fetchApi<FeeSchedule>(`${API_BASE}/fee-schedules`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, user_id: userId, ...data }),
  }).then(coerceFeeSchedule);
}

export async function listFeeSchedules(params: ListFeeSchedulesParams): Promise<FeeSchedule[]> {
  const searchParams = new URLSearchParams();
  searchParams.set('building_id', params.building_id);
  if (params.active_only !== undefined) {
    searchParams.set('active_only', String(params.active_only));
  }
  return fetchApi<FeeSchedule[]>(`${API_BASE}/fee-schedules?${searchParams}`).then((f) =>
    f.map(coerceFeeSchedule)
  );
}

export async function getFeeSchedule(id: string): Promise<FeeSchedule> {
  return fetchApi<FeeSchedule>(`${API_BASE}/fee-schedules/${id}`).then(coerceFeeSchedule);
}

export async function getUnitFees(unitId: string, asOf?: string): Promise<UnitFee[]> {
  const searchParams = new URLSearchParams();
  if (asOf) searchParams.set('as_of', asOf);
  return fetchApi<UnitFee[]>(`${API_BASE}/units/${unitId}/fees?${searchParams}`).then((u) =>
    u.map(coerceUnitFee)
  );
}

export async function assignUnitFee(
  unitId: string,
  feeScheduleId: string,
  data: {
    override_amount?: number;
    effective_from: string;
    effective_to?: string;
  }
): Promise<UnitFee> {
  return fetchApi<UnitFee>(`${API_BASE}/units/${unitId}/fees`, {
    method: 'POST',
    body: JSON.stringify({ fee_schedule_id: feeScheduleId, ...data }),
  }).then(coerceUnitFee);
}

// ============================================================================
// INVOICES
// ============================================================================

export async function createInvoice(
  organizationId: string,
  userId: string,
  data: CreateInvoice
): Promise<InvoiceResponse> {
  return fetchApi<InvoiceResponse>(`${API_BASE}/invoices`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, user_id: userId, ...data }),
  }).then(coerceInvoiceResponse);
}

export async function listInvoices(params: ListInvoicesParams): Promise<ListInvoicesResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.status) searchParams.set('status', params.status);
  if (params.unit_id) searchParams.set('unit_id', params.unit_id);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));
  return fetchApi<ListInvoicesResponse>(`${API_BASE}/invoices?${searchParams}`).then((r) => ({
    ...r,
    invoices: r.invoices.map(coerceInvoice),
  }));
}

export async function getInvoice(id: string): Promise<InvoiceResponse> {
  return fetchApi<InvoiceResponse>(`${API_BASE}/invoices/${id}`).then(coerceInvoiceResponse);
}

export async function sendInvoice(id: string): Promise<Invoice> {
  return fetchApi<Invoice>(`${API_BASE}/invoices/${id}/send`, { method: 'POST' }).then(
    coerceInvoice
  );
}

/**
 * Fetch an invoice rendered as a PDF (#975.6). Returns the raw `Blob` so the
 * caller can trigger a browser download — `fetchApi` can't be used because it
 * parses JSON. Auth + base URL come from the same providers as every other call.
 */
export async function downloadInvoicePdf(invoiceId: string): Promise<Blob> {
  const token = getToken();
  const org = getOrg();
  const baseUrl = client.getConfig().baseUrl ?? '';
  const headers: Record<string, string> = {};
  if (token) headers.Authorization = `Bearer ${token}`;
  if (org) headers['X-Tenant-ID'] = org;

  const response = await fetch(`${baseUrl}${API_BASE}/invoices/${invoiceId}/pdf`, { headers });
  if (!response.ok) {
    throw new Error(`Failed to download invoice PDF (HTTP ${response.status})`);
  }
  return response.blob();
}

export async function listUnitInvoices(
  unitId: string,
  params?: Omit<ListInvoicesParams, 'organization_id' | 'unit_id'>
): Promise<ListInvoicesResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.limit) searchParams.set('limit', String(params.limit));
  if (params?.offset) searchParams.set('offset', String(params.offset));
  return fetchApi<ListInvoicesResponse>(
    `${API_BASE}/units/${unitId}/invoices?${searchParams}`
  ).then((r) => ({ ...r, invoices: r.invoices.map(coerceInvoice) }));
}

// ============================================================================
// PAYMENTS
// ============================================================================

export async function recordPayment(
  organizationId: string,
  userId: string,
  data: RecordPayment
): Promise<PaymentResponse> {
  return fetchApi<PaymentResponse>(`${API_BASE}/payments`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, user_id: userId, ...data }),
  }).then(coercePaymentResponse);
}

export async function getPayment(id: string): Promise<PaymentResponse> {
  return fetchApi<PaymentResponse>(`${API_BASE}/payments/${id}`).then(coercePaymentResponse);
}

/** List all payments for an organization (paginated, optional status filter). */
export async function listPayments(params: ListPaymentsParams): Promise<PaymentListResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.status) searchParams.set('status', params.status);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));
  return fetchApi<PaymentListResponse>(`${API_BASE}/payments?${searchParams}`).then((r) => ({
    ...r,
    payments: r.payments.map(coercePayment),
  }));
}

/** List payments that still have an unallocated balance (reconciliation queue). */
export async function listUnallocatedPayments(organizationId: string): Promise<Payment[]> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', organizationId);
  return fetchApi<Payment[]>(`${API_BASE}/payments/unallocated?${searchParams}`).then((p) =>
    p.map(coercePayment)
  );
}

/** Allocate an existing payment to an invoice (manual match). */
export async function allocatePayment(
  paymentId: string,
  data: AllocatePaymentRequest
): Promise<PaymentAllocation> {
  return fetchApi<PaymentAllocation>(`${API_BASE}/payments/${paymentId}/allocate`, {
    method: 'POST',
    body: JSON.stringify(data),
  }).then(coerceAllocation);
}

/** Bulk auto-match unallocated payments against outstanding invoices. */
export async function autoMatchPayments(organizationId: string): Promise<AutoMatchResult> {
  return fetchApi(`${API_BASE}/payments/auto-match`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId }),
  });
}

export async function listUnitPayments(
  unitId: string,
  params?: ListTransactionsParams
): Promise<Payment[]> {
  const searchParams = new URLSearchParams();
  if (params?.from) searchParams.set('from', params.from);
  if (params?.to) searchParams.set('to', params.to);
  if (params?.limit) searchParams.set('limit', String(params.limit));
  if (params?.offset) searchParams.set('offset', String(params.offset));
  return fetchApi<Payment[]>(`${API_BASE}/units/${unitId}/payments?${searchParams}`).then((p) =>
    p.map(coercePayment)
  );
}

// ============================================================================
// REMINDERS & LATE FEES
// ============================================================================

export async function getReminderSchedules(organizationId: string): Promise<ReminderSchedule[]> {
  return fetchApi(`${API_BASE}/reminder-schedules?organization_id=${organizationId}`);
}

export async function getLateFeeConfig(organizationId: string): Promise<LateFeeConfig> {
  return fetchApi<LateFeeConfig>(
    `${API_BASE}/late-fee-config?organization_id=${organizationId}`
  ).then(coerceLateFee);
}

export async function getOverdueInvoices(organizationId: string): Promise<Invoice[]> {
  return fetchApi<Invoice[]>(`${API_BASE}/overdue-invoices?organization_id=${organizationId}`).then(
    (i) => i.map(coerceInvoice)
  );
}

// ============================================================================
// REPORTS
// ============================================================================

export async function getARAgingReport(params: ARReportParams): Promise<AccountsReceivableReport> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  return fetchApi<AccountsReceivableReport>(`${API_BASE}/reports/ar-aging?${searchParams}`).then(
    (r) => ({
      ...r,
      entries: r.entries.map(coerceArBuckets),
      totals: coerceArBuckets(r.totals),
    })
  );
}

// ============================================================================
// FINANCIAL STATEMENT REPORTS (Story 11.7)
// ============================================================================

function coerceIncomeStatementLine(l: IncomeStatement['revenue'][0]) {
  return { ...l, amount: toNum(l.amount) };
}
function coerceBalanceSheetLine(l: BalanceSheetReport['accounts'][0]) {
  return { ...l, balance: toNum(l.balance) };
}
function coerceCashFlowLine(l: CashFlowReport['inflows'][0]) {
  return { ...l, amount: toNum(l.amount) };
}

export async function getIncomeStatement(params: DateRangeReportParams): Promise<IncomeStatement> {
  const q = new URLSearchParams({
    organization_id: params.organization_id,
    from: params.from,
    to: params.to,
  });
  return fetchApi<IncomeStatement>(`${API_BASE}/reports/income-statement?${q}`).then((r) => ({
    ...r,
    revenue: r.revenue.map(coerceIncomeStatementLine),
    expenses: r.expenses.map(coerceIncomeStatementLine),
    total_revenue: toNum(r.total_revenue),
    total_expenses: toNum(r.total_expenses),
    net_income: toNum(r.net_income),
  }));
}

export async function getBalanceSheet(params: BalanceSheetParams): Promise<BalanceSheetReport> {
  const q = new URLSearchParams({ organization_id: params.organization_id });
  if (params.as_of) q.set('as_of', params.as_of);
  return fetchApi<BalanceSheetReport>(`${API_BASE}/reports/balance-sheet?${q}`).then((r) => ({
    ...r,
    accounts: r.accounts.map(coerceBalanceSheetLine),
    total_assets: toNum(r.total_assets),
    total_liabilities: toNum(r.total_liabilities),
    net_equity: toNum(r.net_equity),
  }));
}

export async function getCashFlowReport(params: DateRangeReportParams): Promise<CashFlowReport> {
  const q = new URLSearchParams({
    organization_id: params.organization_id,
    from: params.from,
    to: params.to,
  });
  return fetchApi<CashFlowReport>(`${API_BASE}/reports/cash-flow?${q}`).then((r) => ({
    ...r,
    inflows: r.inflows.map(coerceCashFlowLine),
    outflows: r.outflows.map(coerceCashFlowLine),
    total_inflows: toNum(r.total_inflows),
    total_outflows: toNum(r.total_outflows),
    net_cash_flow: toNum(r.net_cash_flow),
  }));
}

async function fetchBlob(url: string): Promise<Blob> {
  const token = getToken();
  const org = getOrg();
  const baseUrl = client.getConfig().baseUrl ?? '';
  const response = await fetch(`${baseUrl}${url}`, {
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(org ? { 'X-Tenant-ID': org } : {}),
    },
  });
  if (!response.ok) throw new Error(`Export failed: HTTP ${response.status}`);
  return response.blob();
}

export async function exportReport(report: ReportType, params: ExportReportParams): Promise<Blob> {
  const q = new URLSearchParams({
    organization_id: params.organization_id,
    format: params.format,
  });
  if (params.from) q.set('from', params.from);
  if (params.to) q.set('to', params.to);
  if (params.as_of) q.set('as_of', params.as_of);
  return fetchBlob(`${API_BASE}/reports/${report}/export?${q}`);
}
