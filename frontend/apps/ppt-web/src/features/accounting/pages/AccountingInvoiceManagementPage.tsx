/**
 * AccountingInvoiceManagementPage for managing native issued invoices.
 */

import {
  type AccountingCreateInvoiceRequest,
  contactsApiList,
  invoicesApiCreate,
  invoicesApiDelete,
  invoicesApiList,
} from '@ppt/api-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog, useToast } from '../../../components';
import { parseApiError } from '../../../lib/errorHandler';
import { queryKeys } from '../../../lib/queryKeys';
import { AccountingInvoiceForm } from '../components/AccountingInvoiceForm';
import { AccountingInvoiceList } from '../components/AccountingInvoiceList';

export function AccountingInvoiceManagementPage() {
  const { t } = useTranslation();
  const [isCreating, setIsCreating] = useState(false);
  const [deleteTargetId, setDeleteTargetId] = useState<string | null>(null);
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
    mutationFn: (data: AccountingCreateInvoiceRequest) => invoicesApiCreate({ body: data }),
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
    mutationFn: (id: string) => invoicesApiDelete({ path: { id } }),
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

  return (
    <div className="p-8">
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

      {isCreating ? (
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
          onViewInvoice={() => {
            // TODO(#1522): wire invoice-detail navigation once the detail route
            // exists. Intentionally a no-op (not console.log) so nothing logs in
            // production.
          }}
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
