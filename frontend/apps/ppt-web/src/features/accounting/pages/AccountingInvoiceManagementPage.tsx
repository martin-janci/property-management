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
import { useToast } from '../../../components';
import { parseApiError } from '../../../lib/errorHandler';
import { queryKeys } from '../../../lib/queryKeys';
import { AccountingInvoiceForm } from '../components/AccountingInvoiceForm';
import { AccountingInvoiceList } from '../components/AccountingInvoiceList';

export function AccountingInvoiceManagementPage() {
  const [isCreating, setIsCreating] = useState(false);
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
        title: 'Failed to create invoice',
        message: parsed.message,
      });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => invoicesApiDelete({ path: { id } }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounting.invoices() });
    },
    onError: (error) => {
      const parsed = parseApiError(error);
      showToast({
        type: 'error',
        title: 'Failed to delete invoice',
        message: parsed.message,
      });
    },
  });

  return (
    <div className="p-8">
      <div className="flex justify-between items-center mb-8">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Issued Invoices</h1>
          <p className="text-sm text-gray-500">Native accounting MVP</p>
        </div>
        {!isCreating && (
          <button
            type="button"
            onClick={() => setIsCreating(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700"
          >
            Create Invoice
          </button>
        )}
      </div>

      {isCreating ? (
        <div className="max-w-4xl mx-auto">
          <h2 className="text-xl font-semibold mb-4">New Invoice</h2>
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
          onDeleteInvoice={(id) => {
            if (confirm('Are you sure you want to delete this invoice?')) {
              deleteMutation.mutate(id);
            }
          }}
        />
      )}
    </div>
  );
}
