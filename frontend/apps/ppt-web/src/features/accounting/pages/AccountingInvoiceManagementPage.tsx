/**
 * AccountingInvoiceManagementPage for managing native issued invoices.
 */

import {
  type AccountingUpdateInvoiceRequest,
  contactsApiList,
  invoicesApiCreate,
  invoicesApiDelete,
  invoicesApiGet,
  invoicesApiList,
  invoicesApiListItems,
  invoicesApiUpdate,
} from '@ppt/api-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog, useToast } from '../../../components';
import { useOrganization } from '../../../hooks/useOrganization';
import { parseApiError } from '../../../lib/errorHandler';
import { queryKeys } from '../../../lib/queryKeys';
import { AccountingInvoiceDetail } from '../components/AccountingInvoiceDetail';
import { AccountingInvoiceForm, type InvoiceFormSubmit } from '../components/AccountingInvoiceForm';
import { AccountingInvoiceList } from '../components/AccountingInvoiceList';
import { AccountingSubNav } from '../components/AccountingSubNav';

export function AccountingInvoiceManagementPage() {
  const { t } = useTranslation();
  const [isCreating, setIsCreating] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const { organizationId } = useOrganization();
  const queryClient = useQueryClient();
  const { showToast } = useToast();

  // Auth (Authorization + X-Tenant-ID) is injected centrally by the api-client
  // request interceptor (#1522) — no per-call headers / casts needed here.
  const { data: invoices, isLoading: invoicesLoading } = useQuery({
    queryKey: queryKeys.accounting.invoices(),
    queryFn: () => invoicesApiList(),
  });

  const { data: contacts, isLoading: contactsLoading } = useQuery({
    queryKey: queryKeys.accounting.contacts(),
    queryFn: () => contactsApiList(),
  });

  const isLoading = invoicesLoading || contactsLoading;

  const createMutation = useMutation({
    // The server overwrites tenant_id with the session tenant, but its body
    // shape requires the field. throwOnError so failures reach onError.
    mutationFn: (data: InvoiceFormSubmit) =>
      invoicesApiCreate({
        body: { ...data, tenant_id: organizationId },
        throwOnError: true,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounting.invoices() });
      setIsCreating(false);
    },
    onError: (error) => {
      const parsed = parseApiError(error);
      showToast({
        type: 'error',
        title: t('accounting.createFailed'),
        message: parsed.message,
      });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => invoicesApiDelete({ path: { id }, throwOnError: true }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounting.invoices() });
      setDeleteTargetId(null);
    },
    onError: (error) => {
      const parsed = parseApiError(error);
      showToast({
        type: 'error',
        title: t('accounting.deleteFailed'),
        message: parsed.message,
      });
    },
  });

  const selectedQuery = useQuery({
    queryKey: [...queryKeys.accounting.invoices(), 'detail', selectedId],
    queryFn: async () =>
      (await invoicesApiGet({ path: { id: selectedId as string }, throwOnError: true })).data,
    enabled: selectedId !== null,
  });
  const itemsQuery = useQuery({
    queryKey: [...queryKeys.accounting.invoices(), 'items', selectedId],
    queryFn: async () =>
      (await invoicesApiListItems({ path: { id: selectedId as string }, throwOnError: true }))
        .data ?? [],
    enabled: selectedId !== null,
  });

  const updateMutation = useMutation({
    mutationFn: ({
      id,
      body,
    }: {
      id: string;
      body: AccountingUpdateInvoiceRequest;
      successKey: string;
    }) => invoicesApiUpdate({ path: { id }, body, throwOnError: true }),
    onSuccess: (_data, { successKey }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounting.invoices() });
      showToast({ type: 'success', title: t(successKey) });
    },
    onError: (error) => {
      showToast({
        type: 'error',
        title: t('accounting.detail.updateFailed'),
        message: parseApiError(error).message,
      });
    },
  });

  const selectedInvoice =
    selectedQuery.data ?? (invoices?.data || []).find((inv) => inv.id === selectedId);

  return (
    <div className="p-8">
      <AccountingSubNav />
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('accounting.title')}</h1>
          <p className="text-sm text-gray-500">{t('accounting.subtitle')}</p>
        </div>
        {!isCreating && (
          <button
            type="button"
            onClick={() => setIsCreating(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700"
          >
            {t('accounting.createInvoice')}
          </button>
        )}
      </div>

      {selectedId !== null && selectedInvoice ? (
        <AccountingInvoiceDetail
          invoice={selectedInvoice}
          items={itemsQuery.data ?? []}
          contacts={contacts?.data || []}
          isLoadingItems={itemsQuery.isLoading}
          isUpdating={updateMutation.isPending}
          onUpdate={(body, successKey) =>
            updateMutation.mutate({ id: selectedId, body, successKey })
          }
          onBack={() => setSelectedId(null)}
        />
      ) : isCreating ? (
        <div className="max-w-4xl mx-auto">
          <h2 className="text-xl font-semibold mb-4">{t('accounting.newInvoice')}</h2>
          <AccountingInvoiceForm
            contacts={contacts?.data || []}
            onSubmit={(data) => createMutation.mutate(data)}
            onCancel={() => setIsCreating(false)}
            isSubmitting={createMutation.isPending}
          />
        </div>
      ) : (
        <AccountingInvoiceList
          invoices={invoices?.data || []}
          isLoading={isLoading}
          onViewInvoice={(id) => setSelectedId(id)}
          onDeleteInvoice={(id) => setDeleteTargetId(id)}
        />
      )}

      <ConfirmationDialog
        isOpen={deleteTargetId !== null}
        title={t('accounting.deleteConfirmTitle')}
        message={t('accounting.deleteConfirmMessage')}
        confirmLabel={t('common.delete')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        onConfirm={() => {
          if (deleteTargetId !== null) {
            deleteMutation.mutate(deleteTargetId);
          }
        }}
        onCancel={() => setDeleteTargetId(null)}
        isLoading={deleteMutation.isPending}
      />
    </div>
  );
}
