/**
 * Financial route group (Epic 52).
 *
 * Owns the financial route-wrapper components and the `<Route>` table fragment.
 * Extracted from App.tsx to isolate financial work.
 */
import type { InvoiceStatus, ReportExportFormat, ReportType } from '@ppt/api-client';
import {
  allocatePayment,
  autoMatchPayments,
  budgetKeys,
  downloadInvoicePdf,
  exportReport,
  getARAgingReport,
  getBalanceSheet,
  getBudgetSummary,
  getCashFlowReport,
  getIncomeStatement,
  getOverdueInvoices,
  listBudgets,
  listInvoices,
  listPayments,
  listUnallocatedPayments,
  sendInvoice,
  useBuildings,
} from '@ppt/api-client';
import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Route, useNavigate } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import { useAuth } from '../../contexts';
import {
  BudgetManagementPage,
  FinancialDashboardPage,
  FinancialReportsPage,
  InvoiceManagementPage,
  PaymentManagementPage,
} from '../lazyRoutes';
import { isManagerRole, MANAGER_ROLES } from '../shared';

/** Trigger a browser download for a fetched report blob. */
function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
}

/** YYYY-MM-DD for a Date (local-safe via the same slice the date filters use). */
function isoDate(d: Date): string {
  return d.toISOString().split('T')[0];
}

/**
 * Route wrapper for financial dashboard (Epic 52, #975.1).
 *
 * Wires the hand-written financial api-client functions via TanStack Query:
 *   getARAgingReport({ organization_id })  — AR aging buckets + totals
 *   getOverdueInvoices(orgId)              — overdue invoice list
 *   listInvoices({ organization_id })      — full invoice list (for status counts)
 *
 * Metrics are derived from `arReport.totals` plus invoice sums; invoiceCounts are
 * derived client-side from the invoice list. recentPayments stays `[]` — there is
 * no org-wide payments endpoint in the api-client.
 */
function FinancialDashboardPageRoute() {
  const { user } = useAuth();
  const orgId = user?.organizationId ?? '';

  const { data: arReportData, isLoading: arLoading } = useQuery({
    queryKey: ['financial', 'ar-aging', orgId],
    queryFn: () => getARAgingReport({ organization_id: orgId }),
    enabled: !!orgId,
  });
  const { data: overdueData, isLoading: overdueLoading } = useQuery({
    queryKey: ['financial', 'overdue-invoices', orgId],
    queryFn: () => getOverdueInvoices(orgId),
    enabled: !!orgId,
  });
  const { data: invoicesData, isLoading: invoicesLoading } = useQuery({
    queryKey: ['financial', 'invoices', orgId],
    queryFn: () => listInvoices({ organization_id: orgId }),
    enabled: !!orgId,
  });

  const isLoading = arLoading || overdueLoading || invoicesLoading;

  const totals = arReportData?.totals ?? {
    current: 0,
    days_30: 0,
    days_60: 0,
    days_90_plus: 0,
    total: 0,
  };
  const invoices = invoicesData?.invoices ?? [];
  const overdueInvoices = overdueData ?? [];

  // Outstanding = sum of balances still due across all invoices.
  const totalOutstanding = invoices.reduce((sum, inv) => sum + (inv.balance_due ?? 0), 0);
  const totalOverdue = overdueInvoices.reduce((sum, inv) => sum + (inv.balance_due ?? 0), 0);
  const currency = invoices[0]?.currency ?? 'EUR';

  // Derive per-status counts client-side from the invoice list.
  const invoiceCounts = invoices.reduce(
    (acc, inv) => {
      if (inv.status === 'draft') acc.draft += 1;
      else if (inv.status === 'sent') acc.sent += 1;
      else if (inv.status === 'overdue') acc.overdue += 1;
      else if (inv.status === 'paid') acc.paid += 1;
      return acc;
    },
    { draft: 0, sent: 0, overdue: 0, paid: 0 }
  );

  return (
    <FinancialDashboardPage
      organizationId={orgId}
      buildings={[]}
      metrics={{
        totalBalance: totals.total,
        totalOutstanding,
        totalOverdue,
        currency,
      }}
      invoiceCounts={invoiceCounts}
      recentPayments={[]}
      overdueInvoices={overdueInvoices}
      arReport={{
        entries: arReportData?.entries ?? [],
        totals,
      }}
      isLoading={isLoading}
    />
  );
}

/**
 * Route wrapper for invoice management (Epic 52, #975.2).
 *
 * Lists invoices via listInvoices({ organization_id, status, limit, offset }) with
 * status + pagination state, and sends invoices via sendInvoice (invalidating the
 * list on success). ListInvoicesParams only supports status + unit_id + limit/offset
 * (no building or search), so only status + pagination are wired; buildings stays `[]`.
 */
function InvoiceManagementPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';

  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);
  const [statusFilter, setStatusFilter] = useState<InvoiceStatus | undefined>();

  const { data, isLoading } = useQuery({
    queryKey: ['financial', 'invoices', orgId, statusFilter, page, pageSize],
    queryFn: () =>
      listInvoices({
        organization_id: orgId,
        status: statusFilter,
        limit: pageSize,
        offset: (page - 1) * pageSize,
      }),
    enabled: !!orgId,
  });

  const sendInvoiceMutation = useMutation({
    mutationFn: (invoiceId: string) => sendInvoice(invoiceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['financial', 'invoices', orgId] });
      showToast({
        type: 'success',
        title: t('financial.invoices.sent'),
        message: '',
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.invoices.sendFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  // Route the PDF download through a TanStack mutation (mirrors sendInvoiceMutation)
  // so we get an in-flight `isPending` + `variables` (the invoice id) for free.
  // This drives the per-row loading affordance and the double-click guard below.
  const downloadPdfMutation = useMutation({
    mutationFn: (id: string) => downloadInvoicePdf(id),
    onSuccess: (blob, id) => {
      const number = data?.invoices.find((inv) => inv.id === id)?.invoice_number ?? id;
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `invoice-${number}.pdf`;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.invoices.pdfFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  const handleDownloadPdf = (id: string) => {
    // Double-click guard: ignore new clicks while any download is in flight.
    if (downloadPdfMutation.isPending) return;
    downloadPdfMutation.mutate(id);
  };

  return (
    <InvoiceManagementPage
      invoices={data?.invoices ?? []}
      total={data?.total ?? 0}
      buildings={[]}
      isLoading={isLoading}
      onNavigateToCreate={() => navigate('/financial/invoices/new')}
      onNavigateToDetail={(id: string) => navigate(`/financial/invoices/${id}`)}
      onSendInvoice={(id: string) => sendInvoiceMutation.mutate(id)}
      onDownloadPdf={handleDownloadPdf}
      downloadingPdfId={
        downloadPdfMutation.isPending ? (downloadPdfMutation.variables ?? null) : null
      }
      onFilterChange={(params) => {
        setStatusFilter(params.status);
        setPage(params.page);
        setPageSize(params.pageSize);
      }}
    />
  );
}

/**
 * Route wrapper for payment management (Epic 11/52, #975.5).
 *
 * Wires the org-wide payment endpoints (#1628):
 *   listPayments               — paginated org payments (+ total)
 *   listUnallocatedPayments    — the reconciliation queue
 *   listInvoices               — outstanding invoices (matching targets, balance>0)
 *   allocatePayment            — manual match (onMatch)
 *   autoMatchPayments          — bulk auto-match (onAutoMatch)
 *
 * Metrics are derived client-side: totalReceived = sum of listed payments,
 * pendingReconciliation = sum of unallocated payments.
 */
function PaymentManagementPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';

  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);

  const { data: paymentsData, isLoading } = useQuery({
    queryKey: ['financial', 'payments', orgId, page, pageSize],
    queryFn: () =>
      listPayments({
        organization_id: orgId,
        limit: pageSize,
        offset: (page - 1) * pageSize,
      }),
    enabled: !!orgId,
  });
  const { data: unallocatedData } = useQuery({
    queryKey: ['financial', 'payments', 'unallocated', orgId],
    queryFn: () => listUnallocatedPayments(orgId),
    enabled: !!orgId,
  });
  const { data: invoicesData } = useQuery({
    queryKey: ['financial', 'invoices', orgId],
    queryFn: () => listInvoices({ organization_id: orgId }),
    enabled: !!orgId,
  });

  const payments = paymentsData?.payments ?? [];
  const unallocatedPayments = unallocatedData ?? [];
  const unpaidInvoices = (invoicesData?.invoices ?? []).filter((inv) => (inv.balance_due ?? 0) > 0);

  const totalReceived = payments.reduce((sum, p) => sum + (p.amount ?? 0), 0);
  const pendingReconciliation = unallocatedPayments.reduce((sum, p) => sum + (p.amount ?? 0), 0);
  const currency = payments[0]?.currency ?? 'EUR';

  const invalidatePayments = () => {
    queryClient.invalidateQueries({ queryKey: ['financial', 'payments'] });
    queryClient.invalidateQueries({ queryKey: ['financial', 'invoices'] });
  };
  const matchMutation = useMutation({
    mutationFn: (vars: { paymentId: string; invoiceId: string; amount: number }) =>
      allocatePayment(vars.paymentId, {
        organization_id: orgId,
        invoice_id: vars.invoiceId,
        amount: vars.amount,
      }),
    onSuccess: invalidatePayments,
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.payments.matchFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });
  const autoMatchMutation = useMutation({
    mutationFn: () => autoMatchPayments(orgId),
    onSuccess: invalidatePayments,
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.payments.autoMatchFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  return (
    <PaymentManagementPage
      payments={payments}
      total={paymentsData?.total ?? 0}
      buildings={[]}
      unallocatedPayments={unallocatedPayments}
      unpaidInvoices={unpaidInvoices}
      metrics={{ totalReceived, pendingReconciliation, currency }}
      isLoading={isLoading}
      // Recording a payment is a manager action; residents only see the list.
      onNavigateToRecord={
        isManagerRole(user?.role) ? () => navigate('/financial/payments/new') : undefined
      }
      onMatch={(paymentId: string, invoiceId: string, amount: number) =>
        matchMutation.mutate({ paymentId, invoiceId, amount })
      }
      onAutoMatch={() => autoMatchMutation.mutate()}
      onFilterChange={(params) => {
        setPage(params.page);
        setPageSize(params.pageSize);
      }}
    />
  );
}

/**
 * Budget overview (Epic 24 / Story 52.4).
 *
 *   listBudgets({ organizationId, fiscalYear, buildingId }) — GET /api/v1/budgets
 *   getBudgetSummary(id) per budget                         — GET /api/v1/budgets/{id}/summary
 *
 * The list endpoint only returns each budget's `total_amount`, so actual spend
 * comes from one summary query per listed budget (the list is capped at the
 * server's default page of 50). "Remaining" is budgeted minus actual, so a
 * positive value is under budget (green) and a negative one overspent (red).
 */
function BudgetManagementPageRoute() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const orgId = user?.organizationId ?? '';
  const currentYear = new Date().getFullYear();
  const [fiscalYear, setFiscalYear] = useState(currentYear);
  const [buildingId, setBuildingId] = useState<string>();

  const { data: buildingsData } = useBuildings({ pageSize: 100 });
  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));
  const buildingNames = new Map(buildings.map((b) => [b.id, b.name]));

  const params = { organizationId: orgId, fiscalYear, buildingId };
  const budgetsQuery = useQuery({
    queryKey: budgetKeys.list(params),
    queryFn: () => listBudgets(params),
    enabled: !!orgId,
  });
  const budgetList = budgetsQuery.data ?? [];

  const summaryQueries = useQueries({
    queries: budgetList.map((budget) => ({
      queryKey: budgetKeys.summary(budget.id),
      queryFn: () => getBudgetSummary(budget.id),
    })),
  });

  const budgets = budgetList.map((budget, i) => {
    const totalBudget = budget.totalAmount;
    const totalActual = summaryQueries[i]?.data?.totalActual ?? 0;
    return {
      id: budget.id,
      name: budget.name,
      year: budget.fiscalYear,
      buildingName: budget.buildingId ? buildingNames.get(budget.buildingId) : undefined,
      totalBudget,
      totalActual,
      totalVariance: totalBudget - totalActual,
      currency: 'EUR',
      status: budget.status,
    };
  });
  const totalBudget = budgets.reduce((sum, b) => sum + b.totalBudget, 0);
  const totalActual = budgets.reduce((sum, b) => sum + b.totalActual, 0);

  return (
    <BudgetManagementPage
      budgets={budgets}
      currentYear={currentYear}
      summary={{
        totalBudget,
        totalActual,
        overallVariance: totalBudget - totalActual,
        currency: 'EUR',
      }}
      buildings={buildings}
      isLoading={budgetsQuery.isLoading}
      error={budgetsQuery.error instanceof Error ? budgetsQuery.error.message : null}
      onRetry={() => budgetsQuery.refetch()}
      onNavigateToCreate={() => navigate('/financial/budgets/new')}
      onNavigateToDetail={(id) => navigate(`/financial/budgets/${id}`)}
      onYearChange={setFiscalYear}
      onBuildingChange={setBuildingId}
    />
  );
}

/**
 * Route wrapper for financial statement reports (Epic 11, Story 11.7).
 *
 * Wires the three statement endpoints via TanStack Query, fetching only the
 * report for the active tab:
 *   getIncomeStatement({ organization_id, from, to })
 *   getBalanceSheet({ organization_id, as_of })
 *   getCashFlowReport({ organization_id, from, to })
 *
 * Export buttons call exportReport(report, { format, ... }) and stream the
 * returned PDF / xlsx blob to a browser download. Date defaults are year-to-date
 * for the range reports and today for the balance sheet's as-of.
 */
function FinancialReportsPageRoute() {
  const { user } = useAuth();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const orgId = user?.organizationId ?? '';

  const now = new Date();
  const todayStr = isoDate(now);
  const yearStartStr = isoDate(new Date(now.getFullYear(), 0, 1));

  const [activeTab, setActiveTab] = useState<ReportType>('income-statement');
  const [fromDate, setFromDate] = useState(yearStartStr);
  const [toDate, setToDate] = useState(todayStr);
  const [asOfDate, setAsOfDate] = useState(todayStr);
  const [exportingFormat, setExportingFormat] = useState<ReportExportFormat | null>(null);

  const rangeReady = !!orgId && !!fromDate && !!toDate;

  const incomeQuery = useQuery({
    queryKey: ['financial', 'income-statement', orgId, fromDate, toDate],
    queryFn: () => getIncomeStatement({ organization_id: orgId, from: fromDate, to: toDate }),
    enabled: rangeReady && activeTab === 'income-statement',
  });
  const balanceQuery = useQuery({
    queryKey: ['financial', 'balance-sheet', orgId, asOfDate],
    queryFn: () => getBalanceSheet({ organization_id: orgId, as_of: asOfDate }),
    enabled: !!orgId && !!asOfDate && activeTab === 'balance-sheet',
  });
  const cashFlowQuery = useQuery({
    queryKey: ['financial', 'cash-flow', orgId, fromDate, toDate],
    queryFn: () => getCashFlowReport({ organization_id: orgId, from: fromDate, to: toDate }),
    enabled: rangeReady && activeTab === 'cash-flow',
  });

  const activeQuery =
    activeTab === 'income-statement'
      ? incomeQuery
      : activeTab === 'balance-sheet'
        ? balanceQuery
        : cashFlowQuery;

  const handleExport = (format: ReportExportFormat) => {
    if (!orgId) return;
    setExportingFormat(format);
    const params =
      activeTab === 'balance-sheet'
        ? { organization_id: orgId, format, as_of: asOfDate }
        : { organization_id: orgId, format, from: fromDate, to: toDate };
    const ext = format === 'pdf' ? 'pdf' : 'xlsx';
    exportReport(activeTab, params)
      .then((blob) => downloadBlob(blob, `${activeTab}.${ext}`))
      .catch((err) => {
        showToast({
          type: 'error',
          title: t('financial.reports.exportFailed'),
          message: err instanceof Error ? err.message : '',
        });
      })
      .finally(() => setExportingFormat(null));
  };

  return (
    <FinancialReportsPage
      activeTab={activeTab}
      onTabChange={setActiveTab}
      fromDate={fromDate}
      toDate={toDate}
      onFromDateChange={setFromDate}
      onToDateChange={setToDate}
      asOfDate={asOfDate}
      onAsOfDateChange={setAsOfDate}
      incomeStatement={incomeQuery.data}
      balanceSheet={balanceQuery.data}
      cashFlow={cashFlowQuery.data}
      isLoading={activeQuery.isLoading && activeQuery.fetchStatus !== 'idle'}
      error={activeQuery.error instanceof Error ? activeQuery.error.message : null}
      onExport={handleExport}
      exportingFormat={exportingFormat}
    />
  );
}

/** Financial routes (Epic 52). */
export function financialRoutes() {
  return (
    <>
      <Route path="/financial" element={<FinancialDashboardPageRoute />} />
      <Route
        path="/financial/invoices"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <InvoiceManagementPageRoute />
          </ProtectedRoute>
        }
      />
      <Route path="/financial/payments" element={<PaymentManagementPageRoute />} />
      <Route
        path="/financial/budgets"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <BudgetManagementPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/financial/reports"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <FinancialReportsPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
