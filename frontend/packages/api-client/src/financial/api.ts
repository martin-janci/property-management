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
  ARReportParams,
  CreateFeeSchedule,
  CreateFinancialAccount,
  CreateInvoice,
  CreateTransaction,
  FeeSchedule,
  FinancialAccount,
  FinancialAccountResponse,
  Invoice,
  InvoiceResponse,
  LateFeeConfig,
  ListAccountsParams,
  ListFeeSchedulesParams,
  ListInvoicesParams,
  ListInvoicesResponse,
  ListTransactionsParams,
  Payment,
  PaymentResponse,
  RecordPayment,
  ReminderSchedule,
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

// ============================================================================
// ACCOUNTS
// ============================================================================

export async function createAccount(
  organizationId: string,
  data: CreateFinancialAccount
): Promise<FinancialAccount> {
  return fetchApi(`${API_BASE}/accounts`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, ...data }),
  });
}

export async function listAccounts(params: ListAccountsParams): Promise<FinancialAccount[]> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) {
    searchParams.set('building_id', params.building_id);
  }
  return fetchApi(`${API_BASE}/accounts?${searchParams}`);
}

export async function getAccount(id: string): Promise<FinancialAccountResponse> {
  return fetchApi(`${API_BASE}/accounts/${id}`);
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
  return fetchApi(`${API_BASE}/accounts/${accountId}/transactions?${searchParams}`);
}

export async function createTransaction(
  accountId: string,
  data: Omit<CreateTransaction, 'account_id'>
): Promise<AccountTransaction> {
  return fetchApi(`${API_BASE}/accounts/${accountId}/transactions`, {
    method: 'POST',
    body: JSON.stringify({ account_id: accountId, ...data }),
  });
}

export async function getUnitLedger(unitId: string): Promise<FinancialAccountResponse> {
  return fetchApi(`${API_BASE}/units/${unitId}/ledger`);
}

// ============================================================================
// FEE SCHEDULES
// ============================================================================

export async function createFeeSchedule(
  organizationId: string,
  userId: string,
  data: CreateFeeSchedule
): Promise<FeeSchedule> {
  return fetchApi(`${API_BASE}/fee-schedules`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, user_id: userId, ...data }),
  });
}

export async function listFeeSchedules(params: ListFeeSchedulesParams): Promise<FeeSchedule[]> {
  const searchParams = new URLSearchParams();
  searchParams.set('building_id', params.building_id);
  if (params.active_only !== undefined) {
    searchParams.set('active_only', String(params.active_only));
  }
  return fetchApi(`${API_BASE}/fee-schedules?${searchParams}`);
}

export async function getFeeSchedule(id: string): Promise<FeeSchedule> {
  return fetchApi(`${API_BASE}/fee-schedules/${id}`);
}

export async function getUnitFees(unitId: string, asOf?: string): Promise<UnitFee[]> {
  const searchParams = new URLSearchParams();
  if (asOf) searchParams.set('as_of', asOf);
  return fetchApi(`${API_BASE}/units/${unitId}/fees?${searchParams}`);
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
  return fetchApi(`${API_BASE}/units/${unitId}/fees`, {
    method: 'POST',
    body: JSON.stringify({ fee_schedule_id: feeScheduleId, ...data }),
  });
}

// ============================================================================
// INVOICES
// ============================================================================

export async function createInvoice(
  organizationId: string,
  userId: string,
  data: CreateInvoice
): Promise<InvoiceResponse> {
  return fetchApi(`${API_BASE}/invoices`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, user_id: userId, ...data }),
  });
}

export async function listInvoices(params: ListInvoicesParams): Promise<ListInvoicesResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.status) searchParams.set('status', params.status);
  if (params.unit_id) searchParams.set('unit_id', params.unit_id);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));
  return fetchApi(`${API_BASE}/invoices?${searchParams}`);
}

export async function getInvoice(id: string): Promise<InvoiceResponse> {
  return fetchApi(`${API_BASE}/invoices/${id}`);
}

export async function sendInvoice(id: string): Promise<Invoice> {
  return fetchApi(`${API_BASE}/invoices/${id}/send`, { method: 'POST' });
}

export async function listUnitInvoices(
  unitId: string,
  params?: Omit<ListInvoicesParams, 'organization_id' | 'unit_id'>
): Promise<ListInvoicesResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.limit) searchParams.set('limit', String(params.limit));
  if (params?.offset) searchParams.set('offset', String(params.offset));
  return fetchApi(`${API_BASE}/units/${unitId}/invoices?${searchParams}`);
}

// ============================================================================
// PAYMENTS
// ============================================================================

export async function recordPayment(
  organizationId: string,
  userId: string,
  data: RecordPayment
): Promise<PaymentResponse> {
  return fetchApi(`${API_BASE}/payments`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, user_id: userId, ...data }),
  });
}

export async function getPayment(id: string): Promise<PaymentResponse> {
  return fetchApi(`${API_BASE}/payments/${id}`);
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
  return fetchApi(`${API_BASE}/units/${unitId}/payments?${searchParams}`);
}

// ============================================================================
// REMINDERS & LATE FEES
// ============================================================================

export async function getReminderSchedules(organizationId: string): Promise<ReminderSchedule[]> {
  return fetchApi(`${API_BASE}/reminder-schedules?organization_id=${organizationId}`);
}

export async function getLateFeeConfig(organizationId: string): Promise<LateFeeConfig> {
  return fetchApi(`${API_BASE}/late-fee-config?organization_id=${organizationId}`);
}

export async function getOverdueInvoices(organizationId: string): Promise<Invoice[]> {
  return fetchApi(`${API_BASE}/overdue-invoices?organization_id=${organizationId}`);
}

// ============================================================================
// REPORTS
// ============================================================================

export async function getARAgingReport(params: ARReportParams): Promise<AccountsReceivableReport> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  return fetchApi(`${API_BASE}/reports/ar-aging?${searchParams}`);
}
