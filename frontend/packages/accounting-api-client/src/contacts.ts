/**
 * Contacts hooks for @ppt/accounting-api-client (UC-ACC-03).
 *
 * tanstack-query hooks over the accounting-server contacts REST surface
 * (CONTRACT §5e):
 *   GET /contacts, GET /contacts/{id}, POST /contacts,
 *   PATCH /contacts/{id}, DELETE /contacts/{id}.
 *
 * Modeled on reality-api-client hooks; all calls carry the Bearer JWT via the
 * shared client (resource-server auth).
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { request } from './client';
import { contactKeys } from './query-keys';
import type { Contact, ContactRequest } from './types';

/** List contacts (RLS scopes to the active company). UC-ACC-03.1 */
export function useContacts(filters?: { kind?: string; active?: boolean }) {
  return useQuery({
    queryKey: contactKeys.list(filters),
    queryFn: ({ signal }) =>
      request<Contact[]>('/contacts', {
        query: { kind: filters?.kind, active: filters?.active },
        signal,
      }),
  });
}

/** Fetch a single contact. UC-ACC-03.1 */
export function useContact(id: string | undefined) {
  return useQuery({
    queryKey: contactKeys.detail(id ?? ''),
    queryFn: ({ signal }) => request<Contact>(`/contacts/${id}`, { signal }),
    enabled: !!id,
  });
}

/** Create a contact. UC-ACC-03.2 */
export function useCreateContact() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: ContactRequest) => request<Contact>('/contacts', { method: 'POST', body }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: contactKeys.lists() });
    },
  });
}

/** Update a contact. UC-ACC-03.2 */
export function useUpdateContact(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: ContactRequest) =>
      request<Contact>(`/contacts/${id}`, { method: 'PATCH', body }),
    onSuccess: (updated) => {
      qc.invalidateQueries({ queryKey: contactKeys.lists() });
      qc.setQueryData(contactKeys.detail(id), updated);
    },
  });
}

/** Delete (deactivate) a contact. UC-ACC-03.4 */
export function useDeleteContact() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => request<void>(`/contacts/${id}`, { method: 'DELETE' }),
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: contactKeys.lists() });
      qc.removeQueries({ queryKey: contactKeys.detail(id) });
    },
  });
}
