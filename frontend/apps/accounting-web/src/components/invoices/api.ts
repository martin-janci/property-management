/**
 * Typed REST hooks for the accounting-server invoices surface.
 *
 * Covers EPIC-ACC-05 (UC-ACC-05.1, .2, .16, .17) read/write flows the screens
 * consume: list, get, list-items, create draft, update draft, delete draft,
 * issue, duplicate.
 *
 * CONTRACT NOTE (FE-Screens): the contract (§2 FE boundary, §5e) says screens
 * should import the SDK from `@ppt/accounting-api-client`. That package is
 * owned by FE-Scaffold and is not generated yet, so — per HARD RULE 3 — these
 * thin typed hooks live in FE-Screens-owned files and mirror the
 * `reality-api-client` hook pattern (fetch + TanStack Query) against the
 * documented REST surface. Phase 2 can re-point these imports to the generated
 * SDK without touching the screen components (they import from this module and
 * from `@ppt/accounting-api-client` types only). See contract-notes/fe-screens.md.
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAccountingApiBase } from '@/lib/accountingClient';
import type {
  AccInvoiceExt,
  CreateInvoice,
  Invoice,
  InvoiceItem,
  IssueInvoiceRequest,
  UpdateInvoice,
} from './types';

const API = () => `${getAccountingApiBase()}/api/v1`;

async function authedFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API()}${path}`, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  });
  if (!res.ok) {
    throw new Error(`Request failed (${res.status})`);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const invoiceKeys = {
  all: ['acc', 'invoices'] as const,
  list: () => [...invoiceKeys.all, 'list'] as const,
  detail: (id: string) => [...invoiceKeys.all, 'detail', id] as const,
  items: (id: string) => [...invoiceKeys.all, 'items', id] as const,
};

/** List invoices for the active company (UC-ACC-05.1). */
export function useInvoices() {
  return useQuery({
    queryKey: invoiceKeys.list(),
    queryFn: () => authedFetch<AccInvoiceExt[]>('/invoices'),
  });
}

/** Single invoice (UC-ACC-05.1). */
export function useInvoice(id: string | undefined) {
  return useQuery({
    queryKey: invoiceKeys.detail(id ?? ''),
    queryFn: () => authedFetch<AccInvoiceExt>(`/invoices/${id}`),
    enabled: !!id,
  });
}

/** Line items for an invoice (UC-ACC-05.2). */
export function useInvoiceItems(id: string | undefined) {
  return useQuery({
    queryKey: invoiceKeys.items(id ?? ''),
    queryFn: () => authedFetch<InvoiceItem[]>(`/invoices/${id}/items`),
    enabled: !!id,
  });
}

/** Create a draft invoice with line items (UC-ACC-05.1/.2). */
export function useCreateInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateInvoice) =>
      authedFetch<Invoice>('/invoices', { method: 'POST', body: JSON.stringify(body) }),
    onSuccess: () => qc.invalidateQueries({ queryKey: invoiceKeys.list() }),
  });
}

/** Update a draft invoice (UC-ACC-05.1). Issued invoices are immutable. */
export function useUpdateInvoice(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: UpdateInvoice) =>
      authedFetch<Invoice>(`/invoices/${id}`, { method: 'PATCH', body: JSON.stringify(body) }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: invoiceKeys.list() });
      qc.invalidateQueries({ queryKey: invoiceKeys.detail(id) });
    },
  });
}

/** Delete a draft invoice (UC-ACC-05.1). */
export function useDeleteInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => authedFetch<void>(`/invoices/${id}`, { method: 'DELETE' }),
    onSuccess: () => qc.invalidateQueries({ queryKey: invoiceKeys.list() }),
  });
}

/** Issue a draft — allocate the next gapless number and lock it (UC-ACC-05.1/.17). */
export function useIssueInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: IssueInvoiceRequest }) =>
      authedFetch<AccInvoiceExt>(`/invoices/${id}/issue`, {
        method: 'POST',
        body: JSON.stringify(body),
      }),
    onSuccess: (_data, { id }) => {
      qc.invalidateQueries({ queryKey: invoiceKeys.list() });
      qc.invalidateQueries({ queryKey: invoiceKeys.detail(id) });
    },
  });
}

/** Duplicate an invoice into a fresh draft (UC-ACC-05.16). */
export function useDuplicateInvoice() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      authedFetch<Invoice>(`/invoices/${id}/duplicate`, { method: 'POST' }),
    onSuccess: () => qc.invalidateQueries({ queryKey: invoiceKeys.list() }),
  });
}
