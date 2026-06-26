/**
 * Typed REST hooks for the accounting-server contacts surface (EPIC-ACC-03).
 *
 * Covers UC-ACC-03.1 / .9 CRUD flows the screens consume: list, get, create,
 * update, delete (soft-deactivate is modelled via PATCH `is_active` when the
 * backend exposes it; the contract's DELETE soft-deactivates referenced
 * contacts per STORY-ACC-03-001 technical notes).
 *
 * CONTRACT NOTE (FE-Screens): mirrors the `reality-api-client` hook pattern.
 * Re-pointable to `@ppt/accounting-api-client` in Phase 2. See
 * contract-notes/fe-screens.md.
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAccountingApiBase } from '@/lib/accountingClient';
import type { AccContactExt, ContactRequest } from './types';

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

export const contactKeys = {
  all: ['acc', 'contacts'] as const,
  list: () => [...contactKeys.all, 'list'] as const,
  detail: (id: string) => [...contactKeys.all, 'detail', id] as const,
};

/** List contacts for the active company (UC-ACC-03.1). */
export function useContacts() {
  return useQuery({
    queryKey: contactKeys.list(),
    queryFn: () => authedFetch<AccContactExt[]>('/contacts'),
  });
}

/** Single contact (UC-ACC-03.1). */
export function useContact(id: string | undefined) {
  return useQuery({
    queryKey: contactKeys.detail(id ?? ''),
    queryFn: () => authedFetch<AccContactExt>(`/contacts/${id}`),
    enabled: !!id,
  });
}

/** Create a contact (UC-ACC-03.1). */
export function useCreateContact() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: ContactRequest) =>
      authedFetch<AccContactExt>('/contacts', { method: 'POST', body: JSON.stringify(body) }),
    onSuccess: () => qc.invalidateQueries({ queryKey: contactKeys.list() }),
  });
}

/** Update a contact (UC-ACC-03.1/.9). */
export function useUpdateContact(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: ContactRequest) =>
      authedFetch<AccContactExt>(`/contacts/${id}`, {
        method: 'PATCH',
        body: JSON.stringify(body),
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: contactKeys.list() });
      qc.invalidateQueries({ queryKey: contactKeys.detail(id) });
    },
  });
}

/** Deactivate / soft-delete a contact (UC-ACC-03.5 — never hard-delete referenced). */
export function useDeleteContact() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => authedFetch<void>(`/contacts/${id}`, { method: 'DELETE' }),
    onSuccess: () => qc.invalidateQueries({ queryKey: contactKeys.list() }),
  });
}
