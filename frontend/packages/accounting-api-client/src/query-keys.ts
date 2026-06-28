/**
 * Centralised tanstack-query keys for @ppt/accounting-api-client.
 *
 * A single source of truth for query keys so hooks and manual
 * invalidations stay in sync (mirrors the keying discipline in
 * reality-api-client's hooks). Keys are hierarchical: invalidating
 * `contactKeys.all` clears every contact query.
 */

export const contactKeys = {
  all: ['acc', 'contacts'] as const,
  lists: () => [...contactKeys.all, 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...contactKeys.lists(), filters ?? {}] as const,
  details: () => [...contactKeys.all, 'detail'] as const,
  detail: (id: string) => [...contactKeys.details(), id] as const,
};

export const invoiceKeys = {
  all: ['acc', 'invoices'] as const,
  lists: () => [...invoiceKeys.all, 'list'] as const,
  list: (filters?: Record<string, unknown>) => [...invoiceKeys.lists(), filters ?? {}] as const,
  details: () => [...invoiceKeys.all, 'detail'] as const,
  detail: (id: string) => [...invoiceKeys.details(), id] as const,
  items: (id: string) => [...invoiceKeys.detail(id), 'items'] as const,
};
