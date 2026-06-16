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
import { AccountingInvoiceForm } from '../components/AccountingInvoiceForm';
import { AccountingInvoiceList } from '../components/AccountingInvoiceList';

export function AccountingInvoiceManagementPage() {
  const [isCreating, setIsCreating] = useState(false);
  const queryClient = useQueryClient();

  const { data: invoices, isLoading: invoicesLoading } = useQuery({
    queryKey: ['accounting', 'invoices'],
    queryFn: () => invoicesApiList({}),
  });

  const { data: contacts, isLoading: contactsLoading } = useQuery({
    queryKey: ['accounting', 'contacts'],
    queryFn: () => contactsApiList({}),
  });

  const isLoading = invoicesLoading || contactsLoading;

  const createMutation = useMutation({
    mutationFn: (data: AccountingCreateInvoiceRequest) => invoicesApiCreate({ body: data }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounting', 'invoices'] });
      setIsCreating(false);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => invoicesApiDelete({ path: { id } }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['accounting', 'invoices'] });
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
          onViewInvoice={(id) => console.log('View', id)}
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
