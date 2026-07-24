/**
 * Financial management types (Epic 11/52).
 *
 * Types for financial accounts, transactions, invoices, payments, and reporting.
 */

// ============================================================================
// ENUMS
// ============================================================================

export type FinancialAccountType = 'operating' | 'reserve' | 'utilities' | 'unit_ledger' | 'custom';

export type TransactionType = 'debit' | 'credit';

export type TransactionCategory =
  | 'maintenance_fee'
  | 'utility_charge'
  | 'special_assessment'
  | 'penalty'
  | 'payment_received'
  | 'refund'
  | 'transfer'
  | 'adjustment'
  | 'opening_balance'
  | 'other';

export type FeeFrequency = 'monthly' | 'quarterly' | 'semi_annual' | 'annual' | 'one_time';

export type InvoiceStatus =
  | 'draft'
  | 'sent'
  | 'paid'
  | 'partial'
  | 'overdue'
  | 'cancelled'
  | 'void';

export type PaymentMethod =
  | 'bank_transfer'
  | 'card'
  | 'cash'
  | 'check'
  | 'online'
  | 'direct_debit'
  | 'other';

export type PaymentStatus = 'pending' | 'completed' | 'failed' | 'refunded' | 'cancelled';

// ============================================================================
// FINANCIAL ACCOUNTS
// ============================================================================

export interface FinancialAccount {
  id: string;
  organization_id: string;
  building_id?: string;
  unit_id?: string;
  name: string;
  account_type: FinancialAccountType;
  description?: string;
  currency: string;
  balance: number;
  opening_balance: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateFinancialAccount {
  building_id?: string;
  unit_id?: string;
  name: string;
  account_type: FinancialAccountType;
  description?: string;
  currency?: string;
  opening_balance?: number;
}

export interface AccountTransaction {
  id: string;
  account_id: string;
  amount: number;
  transaction_type: TransactionType;
  category: TransactionCategory;
  description?: string;
  reference_id?: string;
  counterpart_account_id?: string;
  invoice_id?: string;
  payment_id?: string;
  balance_after: number;
  transaction_date: string;
  recorded_by?: string;
  notes?: string;
  created_at: string;
}

export interface CreateTransaction {
  account_id: string;
  amount: number;
  transaction_type: TransactionType;
  category: TransactionCategory;
  description?: string;
  counterpart_account_id?: string;
  invoice_id?: string;
  payment_id?: string;
  transaction_date?: string;
  notes?: string;
}

// ============================================================================
// FEE SCHEDULES
// ============================================================================

export interface FeeSchedule {
  id: string;
  organization_id: string;
  building_id: string;
  name: string;
  description?: string;
  amount: number;
  currency: string;
  frequency: FeeFrequency;
  unit_filter: Record<string, unknown>;
  billing_day?: number;
  is_active: boolean;
  effective_from: string;
  effective_to?: string;
  created_by?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateFeeSchedule {
  building_id: string;
  name: string;
  description?: string;
  amount: number;
  currency?: string;
  frequency?: FeeFrequency;
  unit_filter?: Record<string, unknown>;
  billing_day?: number;
  effective_from?: string;
  effective_to?: string;
}

export interface UnitFee {
  id: string;
  unit_id: string;
  fee_schedule_id: string;
  override_amount?: number;
  effective_from: string;
  effective_to?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// INVOICES
// ============================================================================

export interface Invoice {
  id: string;
  organization_id: string;
  unit_id: string;
  invoice_number: string;
  billing_period_start?: string;
  billing_period_end?: string;
  status: InvoiceStatus;
  issue_date: string;
  due_date: string;
  paid_date?: string;
  subtotal: number;
  tax_amount: number;
  total: number;
  amount_paid: number;
  balance_due: number;
  currency: string;
  notes?: string;
  internal_notes?: string;
  pdf_file_path?: string;
  pdf_generated_at?: string;
  created_by?: string;
  sent_at?: string;
  created_at: string;
  updated_at: string;
}

export interface InvoiceItem {
  id: string;
  invoice_id: string;
  description: string;
  quantity: number;
  unit_price: number;
  amount: number;
  tax_rate?: number;
  tax_amount?: number;
  fee_schedule_id?: string;
  meter_reading_id?: string;
  sort_order: number;
  created_at: string;
}

export interface CreateInvoice {
  unit_id: string;
  billing_period_start?: string;
  billing_period_end?: string;
  due_date: string;
  currency?: string;
  notes?: string;
  items: CreateInvoiceItem[];
}

export interface CreateInvoiceItem {
  description: string;
  quantity?: number;
  unit_price: number;
  tax_rate?: number;
  fee_schedule_id?: string;
  meter_reading_id?: string;
}

// ============================================================================
// PAYMENTS
// ============================================================================

export interface Payment {
  id: string;
  organization_id: string;
  unit_id: string;
  amount: number;
  currency: string;
  payment_method: PaymentMethod;
  status: PaymentStatus;
  reference?: string;
  external_reference?: string;
  payment_date: string;
  notes?: string;
  recorded_by?: string;
  created_at: string;
  updated_at: string;
}

export interface RecordPayment {
  unit_id: string;
  amount: number;
  currency?: string;
  payment_method: PaymentMethod;
  reference?: string;
  payment_date?: string;
  notes?: string;
  invoice_ids?: string[];
}

export interface PaymentAllocation {
  id: string;
  payment_id: string;
  invoice_id: string;
  amount: number;
  created_at: string;
}

/** Params for the org-wide payments list (Payment Management, #975.5). */
export interface ListPaymentsParams {
  organization_id: string;
  status?: PaymentStatus;
  limit?: number;
  offset?: number;
}

/** Paginated payments list response. */
export interface PaymentListResponse {
  payments: Payment[];
  total: number;
}

/** Body for allocating an existing payment to an invoice (manual match). */
export interface AllocatePaymentRequest {
  organization_id: string;
  invoice_id: string;
  amount?: number;
}

/** Result of a bulk auto-match. */
export interface AutoMatchResult {
  matched: number;
}

// ============================================================================
// REMINDERS & LATE FEES
// ============================================================================

export interface ReminderSchedule {
  id: string;
  organization_id: string;
  name: string;
  days_before_due?: number;
  days_after_due?: number;
  email_template_id?: string;
  notification_template?: string;
  is_active: boolean;
  include_sms: boolean;
  created_at: string;
  updated_at: string;
}

export interface LateFeeConfig {
  id: string;
  organization_id: string;
  enabled: boolean;
  grace_period_days: number;
  fee_type: string;
  fee_amount?: number;
  max_fee_amount?: number;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// RESPONSES
// ============================================================================

export interface FinancialAccountResponse {
  account: FinancialAccount;
  recent_transactions: AccountTransaction[];
}

export interface InvoiceResponse {
  invoice: Invoice;
  items: InvoiceItem[];
  payments: PaymentAllocation[];
}

export interface ListInvoicesResponse {
  invoices: Invoice[];
  total: number;
}

export interface PaymentResponse {
  payment: Payment;
  allocations: PaymentAllocation[];
}

export interface ARReportEntry {
  unit_id: string;
  designation: string;
  current: number;
  days_30: number;
  days_60: number;
  days_90_plus: number;
  total: number;
}

export interface ARReportTotals {
  current: number;
  days_30: number;
  days_60: number;
  days_90_plus: number;
  total: number;
}

export interface AccountsReceivableReport {
  as_of_date: string;
  entries: ARReportEntry[];
  totals: ARReportTotals;
}

// ============================================================================
// FINANCIAL STATEMENT REPORTS (Story 11.7)
// ============================================================================

export interface IncomeStatementLine {
  category: string;
  amount: number;
}

export interface IncomeStatement {
  from_date: string;
  to_date: string;
  revenue: IncomeStatementLine[];
  expenses: IncomeStatementLine[];
  total_revenue: number;
  total_expenses: number;
  net_income: number;
}

export interface BalanceSheetLine {
  account_id: string;
  account_name: string;
  account_type: string;
  balance: number;
}

export interface BalanceSheetReport {
  as_of_date: string;
  accounts: BalanceSheetLine[];
  total_assets: number;
  total_liabilities: number;
  net_equity: number;
}

export interface CashFlowLine {
  category: string;
  amount: number;
}

export interface CashFlowReport {
  from_date: string;
  to_date: string;
  inflows: CashFlowLine[];
  outflows: CashFlowLine[];
  total_inflows: number;
  total_outflows: number;
  net_cash_flow: number;
}

export interface DateRangeReportParams {
  organization_id: string;
  from: string;
  to: string;
}

export interface BalanceSheetParams {
  organization_id: string;
  as_of?: string;
}

export type ReportExportFormat = 'pdf' | 'xlsx';
export type ReportType = 'income-statement' | 'balance-sheet' | 'cash-flow';

export interface ExportReportParams {
  organization_id: string;
  format: ReportExportFormat;
  from?: string;
  to?: string;
  as_of?: string;
}

// ============================================================================
// QUERY PARAMS
// ============================================================================

export interface ListAccountsParams {
  organization_id: string;
  building_id?: string;
}

export interface ListTransactionsParams {
  from?: string;
  to?: string;
  limit?: number;
  offset?: number;
}

export interface ListInvoicesParams {
  organization_id: string;
  status?: InvoiceStatus;
  unit_id?: string;
  limit?: number;
  offset?: number;
}

export interface ListFeeSchedulesParams {
  building_id: string;
  active_only?: boolean;
}

export interface ARReportParams {
  organization_id: string;
  building_id?: string;
}

// ============================================================================
// DASHBOARD METRICS
// ============================================================================

export interface FinancialDashboardMetrics {
  total_balance: number;
  total_outstanding: number;
  total_overdue: number;
  invoices_count: {
    draft: number;
    sent: number;
    overdue: number;
    paid: number;
  };
  recent_payments: Payment[];
  overdue_invoices: Invoice[];
  ar_summary: ARReportTotals;
}
