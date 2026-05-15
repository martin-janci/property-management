/**
 * Phase 5 (B6 / N9) — `/admin/agencies` page.
 *
 * Wired to `GET /api/v1/admin/agencies` via the typed `useAgencies` hook
 * from `@ppt/api-client`. Rendering is capability-gated row-by-row through
 * `<ResourceTable>` (the Suspend action only renders for principals with
 * `agencies_suspend`).
 *
 * N9: the Suspend action calls `POST /api/v1/admin/agencies/{id}/suspend`
 * for real. The capability extractor on api-server requires recent MFA;
 * api-client's mfa_required interceptor pops `<MfaChallengeModal>` and
 * retries the request transparently on success. The toast surface still
 * fires on failure so QA can see what happened.
 */

import { type Agency, suspendAgency, useAgencies, adminKeys } from '@ppt/api-client';
import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import { useQueryClient } from '@tanstack/react-query';
import type React from 'react';
import { useCallback } from 'react';
import { useToast } from '../../../components';

const columns: ReadonlyArray<ResourceTableColumn<Agency>> = [
  { key: 'name', header: 'Name', render: (a) => a.name },
  { key: 'slug', header: 'Slug', render: (a) => a.slug },
  { key: 'status', header: 'Status', render: (a) => a.status },
  { key: 'members', header: 'Members', render: (a) => String(a.member_count) },
];

const AgenciesPage: React.FC = () => {
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const query = useAgencies({ page: 1, page_size: 50 });

  // N9: the suspend handler is the canonical "this works under MFA"
  // example. The api-client's mfa_required interceptor handles the
  // challenge + retry, so this code only sees the final outcome.
  const handleSuspend = useCallback(
    async (agency: Agency) => {
      try {
        await suspendAgency(agency.id);
        showToast({
          type: 'success',
          title: 'Agency suspended',
          message: `${agency.name} has been suspended.`,
        });
        await queryClient.invalidateQueries({ queryKey: adminKeys.agencies() });
      } catch (e) {
        const message = e instanceof Error ? e.message : 'Suspend failed';
        showToast({
          type: 'error',
          title: 'Suspend failed',
          message,
        });
      }
    },
    [queryClient, showToast],
  );

  if (query.isLoading) {
    // Lightweight inline spinner — the global PageLoading lives elsewhere
    // and would be overkill for a sub-route partial fetch.
    return (
      <section>
        <h1>Agencies</h1>
        <div role="status" aria-live="polite">
          Loading agencies…
        </div>
      </section>
    );
  }

  if (query.isError) {
    const message = query.error instanceof Error ? query.error.message : 'Unknown error';
    return (
      <section>
        <h1>Agencies</h1>
        <div role="alert" className="ppt-admin-error">
          <p>Failed to load agencies: {message}</p>
          <button type="button" onClick={() => query.refetch()}>
            Retry
          </button>
        </div>
      </section>
    );
  }

  const items = query.data?.items ?? [];

  return (
    <section>
      <h1>Agencies</h1>
      <ResourceTable<Agency>
        columns={columns}
        data={items}
        rowKey={(a) => a.id}
        emptyMessage="No agencies found."
        actions={[
          {
            label: 'Suspend',
            capability: 'agencies_suspend',
            variant: 'danger',
            // Real call — api-client interceptor opens the MFA modal on
            // 401 mfa_required and retries on success.
            onClick: (a) => {
              void handleSuspend(a);
            },
          },
          {
            label: 'Add domain',
            capability: 'agencies_write',
            // TODO: separate endpoint not yet wired (out of scope for N9).
            onClick: (a) =>
              showToast({
                type: 'warning',
                title: 'Not yet wired',
                message: `TODO: POST /admin/agencies/${a.id}/domains/add`,
              }),
          },
        ]}
      />
    </section>
  );
};

export default AgenciesPage;
