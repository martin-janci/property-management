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
 * All three are manager actions, so every route is wrapped in
 * `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`.
 *
 * Kept in its own module (registered from `AppRoutes`) so it doesn't touch the
 * list-route table in `financial.tsx`.
 */
import type { CreateInvoice, RecordPayment } from '@ppt/api-client';
import {
  createBuildingsApi,
  createInvoice,
  downloadInvoicePdf,
  getInvoice,
  getOrg,
  getToken,
  listInvoices,
  recordPayment,
  sendInvoice,
  useBuildings,
} from '@ppt/api-client';
import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query';
import { lazy, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Route, useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { ProtectedRoute, useToast } from '../../components';
import { useAuth } from '../../contexts';
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
    </>
  );
}
