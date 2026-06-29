/**
 * Invoice hooks for @ppt/accounting-api-client (UC-ACC-05).
 *
 * tanstack-query hooks over the accounting-server invoices REST surface
 * (CONTRACT §5e):
 *   GET /invoices, GET /invoices/{id}, GET /invoices/{id}/items,
 *   POST /invoices, PATCH /invoices/{id}, DELETE /invoices/{id},
 *   POST /invoices/{id}/issue, POST /invoices/{id}/duplicate.
 *
 * All calls carry the Bearer JWT (resource-server auth). Issuing an invoice
 * (UC-ACC-05) allocates a gapless number server-side, so on success we
 * invalidate both the detail and the list.
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { request } from './client';
import { invoiceKeys } from './query-keys';
import type {
  CreateInvoice,
  Invoice,
  InvoiceItem,
  InvoiceStatus,
  InvoiceWithItems,
  UpdateInvoice,
} from './types';

/** List invoices (RLS scopes to the active company). UC-ACC-05 */
export function useInvoices(filters?: { status?: InvoiceStatus; contactId?: string }) {
  return useQuery({
    queryKey: invoiceKeys.list(filters),
    queryFn: ({ signal }) =>
      request<Invoice[]>('/invoices', {
        query: { status: filters?.status, contact_id: filters?.contactId },
        signal,
      }),
  });
}

/** Fetch a single invoice with its items. UC-ACC-05 */
export function useInvoice(id: string | undefined) {
  return useQuery({
    queryKey: invoiceKeys.detail(id ?? ''),
    queryFn: ({ signal }) => request<InvoiceWithItems>(`/invoices/${id}`, { signal }),
    enabled: !!id,
  });
}

/** Fetch only the line items of an invoice. UC-ACC-05 */
export function useInvoiceItems(id: string | undefined) {
  return useQuery({
    queryKey: invoiceKeys.items(id ?? ''),
    queryFn: ({ signal }) => request<InvoiceItem[]>(`/invoices/${id}/items`, { signal }),
    enabled: !!id,
  });
}

/** Create a draft invoice. UC-ACC-05 */
export function useCreateInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateInvoice) => request<Invoice>('/invoices', { method: 'POST', body }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: invoiceKeys.lists() });
    },
  });
}

/** Update a draft invoice. UC-ACC-05 */
export function useUpdateInvoice(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: UpdateInvoice) =>
      request<Invoice>(`/invoices/${id}`, { method: 'PATCH', body }),
    onSuccess: (updated) => {
      qc.invalidateQueries({ queryKey: invoiceKeys.lists() });
      qc.invalidateQueries({ queryKey: invoiceKeys.detail(id) });
      qc.setQueryData(invoiceKeys.detail(id), updated);
    },
  });
}

/** Delete a draft invoice. UC-ACC-05 */
export function useDeleteInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => request<void>(`/invoices/${id}`, { method: 'DELETE' }),
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: invoiceKeys.lists() });
      qc.removeQueries({ queryKey: invoiceKeys.detail(id) });
    },
  });
}

/**
 * Issue a draft invoice: allocates a gapless number and freezes the document
 * (UC-ACC-05 / UC-ACC-02.3). Returns the issued invoice with its number.
 */
export function useIssueInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => request<Invoice>(`/invoices/${id}/issue`, { method: 'POST' }),
    onSuccess: (issued, id) => {
      qc.invalidateQueries({ queryKey: invoiceKeys.lists() });
      qc.invalidateQueries({ queryKey: invoiceKeys.detail(id) });
      qc.setQueryData(invoiceKeys.detail(id), issued);
    },
  });
}

/** Duplicate an invoice into a new draft. UC-ACC-05 */
export function useDuplicateInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => request<Invoice>(`/invoices/${id}/duplicate`, { method: 'POST' }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: invoiceKeys.lists() });
    },
  });
}
