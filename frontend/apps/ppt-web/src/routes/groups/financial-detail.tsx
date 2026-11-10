/**
 * Financial create / detail routes (Epic 52).
 *
 * The financial list pages (`./financial.tsx`) navigate to
 * `/financial/invoices/new`, `/financial/invoices/:invoiceId` and
 * `/financial/payments/new`, but those paths had no `<Route>` and fell through
 * to not-found. This module wires them to the existing feature components
 * (`InvoiceForm`, `InvoiceDetail`, `PaymentForm`) and the api-server endpoints:
 *
 *   POST /api/v1/financial/invoices          — createInvoice
 *   GET  /api/v1/financial/invoices/{id}     — getInvoice
 *   POST /api/v1/financial/invoices/{id}/send — sendInvoice
 *   GET  /api/v1/financial/invoices/{id}/pdf — downloadInvoicePdf
 *   POST /api/v1/financial/payments          — recordPayment
 *
 * plus the budget create / detail screens (`/financial/budgets/new`,
 * `/financial/budgets/:budgetId`) on the `/api/v1/budgets` endpoints wrapped by
 * the `@ppt/api-client` budgets module (create, get, items, categories,
 * summary, submit / approve / activate / close, delete, record actuals).
 *
 * All of these are manager actions, so every route is wrapped in
 * `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`.
 *
 * Kept in its own module (registered from `AppRoutes`) so it doesn't touch the
 * list-route table in `financial.tsx`.
 */
import type { CreateInvoice, RecordPayment } from '@ppt/api-client';
import {
  activateBudget,
  addBudgetItem,
  approveBudget,
  budgetKeys,
  closeBudget,
  createBudget,
  createBudgetCategory,
  createBuildingsApi,
  createInvoice,
  deleteBudget,
  deleteBudgetItem,
  downloadInvoicePdf,
  getBudget,
  getBudgetSummary,
  getInvoice,
  getOrg,
  getToken,
  listBudgetCategories,
  listBudgetItems,
  listInvoices,
  recordBudgetActual,
  recordPayment,
  sendInvoice,
  submitBudget,
  useBuildings,
} from '@ppt/api-client';
import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import { lazy, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Route, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import { useAuth } from '../../contexts';
import type {
  BudgetActualValues,
  BudgetDetailAction,
  BudgetFormValues,
  NewBudgetItemValues,
} from '../../features/financial';
import { MANAGER_ROLES } from '../shared';

const InvoiceForm = lazy(() =>
  import('../../features/financial').then((m) => ({ default: m.InvoiceForm }))
);
const InvoiceDetail = lazy(() =>
  import('../../features/financial').then((m) => ({ default: m.InvoiceDetail }))
);
const PaymentForm = lazy(() =>
  import('../../features/financial').then((m) => ({ default: m.PaymentForm }))
);
const BudgetForm = lazy(() =>
  import('../../features/financial').then((m) => ({ default: m.BudgetForm }))
);
const BudgetDetail = lazy(() =>
  import('../../features/financial').then((m) => ({ default: m.BudgetDetail }))
);

/** Units list limit — the api-server clamps `/buildings/{id}/units` to 100. */
const UNITS_PAGE_LIMIT = 100;

interface FinancialUnitOption {
  id: string;
  unit_number: string;
  building_name: string;
}

/**
 * Every unit across the organisation's buildings, in the `{ id, unit_number,
 * building_name }` shape the invoice/payment forms render in their unit picker.
 * There is no org-wide units endpoint, so this lists buildings then fans out
 * one `/buildings/{id}/units` query per building.
 */
function useOrgUnitOptions() {
  const { data: buildingsData, isLoading: buildingsLoading } = useBuildings({ pageSize: 100 });
  const buildings = buildingsData?.items ?? [];

  const unitQueries = useQueries({
    queries: buildings.map((building) => ({
      queryKey: ['financial', 'building-units', building.id],
      queryFn: () =>
        createBuildingsApi({
          baseUrl: import.meta.env.VITE_API_BASE_URL ?? '',
          accessToken: getToken() ?? undefined,
          tenantId: getOrg() ?? undefined,
        }).listUnits(building.id, { limit: UNITS_PAGE_LIMIT }),
    })),
  });

  const units: FinancialUnitOption[] = buildings.flatMap((building, i) =>
    (unitQueries[i]?.data?.units ?? []).map((unit) => ({
      id: unit.id,
      unit_number: unit.designation,
      building_name: building.name,
    }))
  );

  return {
    units,
    isLoading: buildingsLoading || unitQueries.some((q) => q.isLoading),
  };
}

function Spinner() {
  return (
    <div className="flex items-center justify-center py-12">
      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
    </div>
  );
}

/** Shared header + card shell for the create forms. */
function FormPageShell({
  title,
  subtitle,
  backTo,
  backLabel,
  children,
}: {
  title: string;
  subtitle: string;
  backTo: string;
  backLabel: string;
  children: ReactNode;
}) {
  return (
    <div className="min-h-screen bg-gray-100">
      <div className="bg-white shadow">
        <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <Link to={backTo} className="text-sm text-blue-600 hover:text-blue-800">
            &larr; {backLabel}
          </Link>
          <h1 className="mt-2 text-2xl font-bold text-gray-900">{title}</h1>
          <p className="mt-1 text-sm text-gray-500">{subtitle}</p>
        </div>
      </div>
      <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <div className="bg-white rounded-lg shadow p-6">{children}</div>
      </div>
    </div>
  );
}

/** `/financial/invoices/new` — create an invoice for a unit. */
function NewInvoicePageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';
  const { units, isLoading } = useOrgUnitOptions();

  const createMutation = useMutation({
    mutationFn: (data: CreateInvoice) => createInvoice(orgId, user?.id ?? '', data),
    onSuccess: (created) => {
      queryClient.invalidateQueries({ queryKey: ['financial'] });
      showToast({ type: 'success', title: t('financial.pages.invoiceCreated'), message: '' });
      navigate(`/financial/invoices/${created.invoice.id}`);
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.pages.createInvoiceFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  return (
    <FormPageShell
      title={t('financial.pages.newInvoiceTitle')}
      subtitle={t('financial.pages.newInvoiceSubtitle')}
      backTo="/financial/invoices"
      backLabel={t('financial.pages.backToInvoices')}
    >
      {isLoading ? (
        <Spinner />
      ) : units.length === 0 ? (
        <p className="text-sm text-gray-600">{t('financial.pages.noUnits')}</p>
      ) : (
        <InvoiceForm
          units={units}
          onSubmit={(data) => createMutation.mutate(data)}
          onCancel={() => navigate('/financial/invoices')}
          isSubmitting={createMutation.isPending}
        />
      )}
    </FormPageShell>
  );
}

/** `/financial/invoices/:invoiceId` — invoice detail with send / PDF / record-payment. */
function InvoiceDetailPageRoute() {
  const { invoiceId = '' } = useParams<{ invoiceId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  const { data, isLoading, error } = useQuery({
    queryKey: ['financial', 'invoice', invoiceId],
    queryFn: () => getInvoice(invoiceId),
    enabled: !!invoiceId,
  });

  const sendMutation = useMutation({
    mutationFn: () => sendInvoice(invoiceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['financial'] });
      showToast({ type: 'success', title: t('financial.pages.invoiceSent'), message: '' });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.pages.sendInvoiceFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  const pdfMutation = useMutation({
    mutationFn: () => downloadInvoicePdf(invoiceId),
    onSuccess: (blob) => {
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `invoice-${data?.invoice.invoice_number ?? invoiceId}.pdf`;
      document.body.appendChild(anchor);
      anchor.click();
      document.body.removeChild(anchor);
      URL.revokeObjectURL(url);
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.pages.pdfFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  if (isLoading) return <Spinner />;

  if (error || !data) {
    return (
      <div className="max-w-3xl mx-auto px-4 py-12 text-center">
        <p className="text-gray-700">{t('financial.pages.invoiceNotFound')}</p>
        <Link
          to="/financial/invoices"
          className="mt-4 inline-block text-sm text-blue-600 hover:text-blue-800"
        >
          &larr; {t('financial.pages.backToInvoices')}
        </Link>
      </div>
    );
  }

  return (
    <InvoiceDetail
      invoice={data}
      onSend={() => sendMutation.mutate()}
      isSending={sendMutation.isPending}
      onDownloadPdf={() => {
        if (!pdfMutation.isPending) pdfMutation.mutate();
      }}
      onRecordPayment={() =>
        navigate(`/financial/payments/new?unitId=${encodeURIComponent(data.invoice.unit_id)}`)
      }
      onClose={() => navigate('/financial/invoices')}
    />
  );
}

/** `/financial/payments/new` — record a received payment (optionally `?unitId=`). */
function NewPaymentPageRoute() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';
  const { units, isLoading: unitsLoading } = useOrgUnitOptions();

  const { data: invoicesData, isLoading: invoicesLoading } = useQuery({
    queryKey: ['financial', 'invoices', orgId],
    queryFn: () => listInvoices({ organization_id: orgId }),
    enabled: !!orgId,
  });
  const outstandingInvoices = (invoicesData?.invoices ?? []).filter(
    (inv) => (inv.balance_due ?? 0) > 0
  );

  const recordMutation = useMutation({
    mutationFn: (data: RecordPayment) => recordPayment(orgId, user?.id ?? '', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['financial'] });
      showToast({ type: 'success', title: t('financial.pages.paymentRecorded'), message: '' });
      navigate('/financial/payments');
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.pages.recordPaymentFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  return (
    <FormPageShell
      title={t('financial.pages.newPaymentTitle')}
      subtitle={t('financial.pages.newPaymentSubtitle')}
      backTo="/financial/payments"
      backLabel={t('financial.pages.backToPayments')}
    >
      {unitsLoading || (invoicesLoading && !!orgId) ? (
        <Spinner />
      ) : units.length === 0 ? (
        <p className="text-sm text-gray-600">{t('financial.pages.noUnits')}</p>
      ) : (
        <PaymentForm
          units={units}
          outstandingInvoices={outstandingInvoices}
          initialUnitId={searchParams.get('unitId') ?? undefined}
          onSubmit={(data) => recordMutation.mutate(data)}
          onCancel={() => navigate('/financial/payments')}
          isSubmitting={recordMutation.isPending}
        />
      )}
    </FormPageShell>
  );
}

/** `/financial/budgets/new` — create a budget, then open its detail to add lines. */
function NewBudgetPageRoute() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';
  const { data: buildingsData, isLoading } = useBuildings({ pageSize: 100 });
  const buildings = (buildingsData?.items ?? []).map((b) => ({ id: b.id, name: b.name }));

  const createMutation = useMutation({
    mutationFn: (values: BudgetFormValues) =>
      createBudget({
        organizationId: orgId,
        name: values.name,
        fiscalYear: values.fiscalYear,
        buildingId: values.buildingId,
        notes: values.notes,
      }),
    onSuccess: (created) => {
      queryClient.invalidateQueries({ queryKey: budgetKeys.all });
      showToast({ type: 'success', title: t('financial.budgetPages.toast.created'), message: '' });
      navigate(`/financial/budgets/${created.id}`);
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('financial.budgetPages.toast.createFailed'),
        message: err instanceof Error ? err.message : '',
      });
    },
  });

  return (
    <FormPageShell
      title={t('financial.budgetPages.newTitle')}
      subtitle={t('financial.budgetPages.newSubtitle')}
      backTo="/financial/budgets"
      backLabel={t('financial.budgetPages.backToBudgets')}
    >
      {isLoading ? (
        <Spinner />
      ) : (
        <BudgetForm
          year={new Date().getFullYear()}
          buildings={buildings}
          onSubmit={(values) => createMutation.mutate(values)}
          onCancel={() => navigate('/financial/budgets')}
          isSubmitting={createMutation.isPending || !orgId}
        />
      )}
    </FormPageShell>
  );
}

/** Lifecycle transition call for a detail-page action (resolved at call time). */
function budgetTransition(action: Exclude<BudgetDetailAction, 'delete'>) {
  switch (action) {
    case 'submit':
      return submitBudget;
    case 'approve':
      return approveBudget;
    case 'activate':
      return activateBudget;
    case 'close':
      return closeBudget;
  }
}

/** `/financial/budgets/:budgetId` — budget detail: lines, totals and lifecycle actions. */
function BudgetDetailPageRoute() {
  const { budgetId = '' } = useParams<{ budgetId: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { user } = useAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const orgId = user?.organizationId ?? '';

  const budgetQuery = useQuery({
    queryKey: budgetKeys.detail(budgetId),
    queryFn: () => getBudget(orgId, budgetId),
    enabled: !!orgId && !!budgetId,
  });
  const itemsQuery = useQuery({
    queryKey: budgetKeys.items(budgetId),
    queryFn: () => listBudgetItems(budgetId),
    enabled: !!budgetId && budgetQuery.isSuccess,
  });
  const summaryQuery = useQuery({
    queryKey: budgetKeys.summary(budgetId),
    queryFn: () => getBudgetSummary(budgetId),
    enabled: !!budgetId && budgetQuery.isSuccess,
  });
  const categoriesQuery = useQuery({
    queryKey: budgetKeys.categories(orgId),
    queryFn: () => listBudgetCategories(orgId),
    enabled: !!orgId && budgetQuery.isSuccess,
  });
  const { data: buildingsData } = useBuildings({ pageSize: 100 });

  const refresh = () => queryClient.invalidateQueries({ queryKey: budgetKeys.all });
  const fail = (titleKey: string) => (err: unknown) =>
    showToast({
      type: 'error',
      title: t(titleKey),
      message: err instanceof Error ? err.message : '',
    });

  const actionMutation = useMutation({
    mutationFn: async (action: BudgetDetailAction) => {
      if (action === 'delete') {
        await deleteBudget(orgId, budgetId);
        return action;
      }
      await budgetTransition(action)(orgId, budgetId);
      return action;
    },
    onSuccess: (action) => {
      showToast({
        type: 'success',
        title: t(`financial.budgetPages.toast.${action}Done`),
        message: '',
      });
      if (action === 'delete') {
        queryClient.removeQueries({ queryKey: budgetKeys.detail(budgetId) });
        queryClient.invalidateQueries({ queryKey: budgetKeys.lists() });
        navigate('/financial/budgets');
        return;
      }
      refresh();
    },
    onError: fail('financial.budgetPages.toast.actionFailed'),
  });

  const addItemMutation = useMutation({
    mutationFn: async (values: NewBudgetItemValues) => {
      let categoryId = values.categoryId;
      if (!categoryId) {
        const category = await createBudgetCategory({
          organizationId: orgId,
          name: values.newCategoryName ?? '',
        });
        categoryId = category.id;
      }
      return addBudgetItem(budgetId, {
        categoryId,
        name: values.name,
        budgetedAmount: values.budgetedAmount,
        description: values.description,
      });
    },
    onSuccess: () => {
      showToast({
        type: 'success',
        title: t('financial.budgetPages.toast.itemAdded'),
        message: '',
      });
      refresh();
    },
    onError: (err) => {
      // A category may have been created before the item failed — refresh either way.
      refresh();
      fail('financial.budgetPages.toast.itemAddFailed')(err);
    },
  });

  const deleteItemMutation = useMutation({
    mutationFn: (itemId: string) => deleteBudgetItem(itemId),
    onSuccess: refresh,
    onError: fail('financial.budgetPages.toast.itemRemoveFailed'),
  });

  const actualMutation = useMutation({
    mutationFn: ({ itemId, values }: { itemId: string; values: BudgetActualValues }) =>
      recordBudgetActual(itemId, values),
    onSuccess: () => {
      showToast({
        type: 'success',
        title: t('financial.budgetPages.toast.actualRecorded'),
        message: '',
      });
      refresh();
    },
    onError: fail('financial.budgetPages.toast.actualFailed'),
  });

  if (budgetQuery.isLoading || (!orgId && !budgetQuery.error)) return <Spinner />;

  const budget = budgetQuery.data;
  if (budgetQuery.error || !budget) {
    return (
      <div className="max-w-3xl mx-auto px-4 py-12 text-center">
        <p className="text-gray-700">{t('financial.budgetPages.notFound')}</p>
        <Link
          to="/financial/budgets"
          className="mt-4 inline-block text-sm text-blue-600 hover:text-blue-800"
        >
          &larr; {t('financial.budgetPages.backToBudgets')}
        </Link>
      </div>
    );
  }

  if (itemsQuery.isLoading || categoriesQuery.isLoading) return <Spinner />;

  const buildingName = budget.buildingId
    ? buildingsData?.items?.find((b) => b.id === budget.buildingId)?.name
    : undefined;

  return (
    <BudgetDetail
      budget={budget}
      buildingName={buildingName}
      items={itemsQuery.data ?? []}
      categories={categoriesQuery.data ?? []}
      summary={summaryQuery.data}
      pendingAction={actionMutation.isPending ? actionMutation.variables : null}
      onAction={(action) => actionMutation.mutate(action)}
      onAddItem={(values) => addItemMutation.mutate(values)}
      isAddingItem={addItemMutation.isPending}
      onDeleteItem={(itemId) => deleteItemMutation.mutate(itemId)}
      onRecordActual={(itemId, values) => actualMutation.mutate({ itemId, values })}
      isRecordingActual={actualMutation.isPending}
      onBack={() => navigate('/financial/budgets')}
    />
  );
}

/** Financial create / detail routes (manager-only). */
export function financialDetailRoutes() {
  return (
    <>
      <Route
        path="/financial/invoices/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <NewInvoicePageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/financial/invoices/:invoiceId"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <InvoiceDetailPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/financial/payments/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <NewPaymentPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/financial/budgets/new"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <NewBudgetPageRoute />
          </ProtectedRoute>
        }
      />
      <Route
        path="/financial/budgets/:budgetId"
        element={
          <ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>
            <BudgetDetailPageRoute />
          </ProtectedRoute>
        }
      />
    </>
  );
}
