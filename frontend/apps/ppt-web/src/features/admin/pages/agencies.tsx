/**
 * Phase 5 (B6) — `/admin/agencies` page.
 *
 * Wired to `GET /api/v1/admin/agencies` via the typed `useAgencies` hook
 * from `@ppt/api-client`. Rendering is capability-gated row-by-row through
 * `<ResourceTable>` (the Suspend action only renders for principals with
 * `agencies_suspend`).
 *
 * Mutations (Suspend, Add domain) are stubbed for this PR — they emit a
 * toast so QA can spot un-wired affordances during exploratory testing.
 *
 * TODO(N9): When the backend returns `401 mfa_required`, surface an MFA
 * challenge modal before retrying the action. Out of scope for B6.
 */

import { type Agency, useAgencies } from '@ppt/api-client';
import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import type React from 'react';
import { useToast } from '../../../components';

const columns: ReadonlyArray<ResourceTableColumn<Agency>> = [
  { key: 'name', header: 'Name', render: (a) => a.name },
  { key: 'slug', header: 'Slug', render: (a) => a.slug },
  { key: 'status', header: 'Status', render: (a) => a.status },
  { key: 'members', header: 'Members', render: (a) => String(a.member_count) },
];

const AgenciesPage: React.FC = () => {
  const { showToast } = useToast();
  const query = useAgencies({ page: 1, page_size: 50 });

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
            // TODO(N9): MFA challenge modal when 401 mfa_required
            onClick: (a) =>
              showToast({
                type: 'warning',
                title: 'Not yet wired',
                message: `TODO: POST /admin/agencies/${a.id}/suspend`,
              }),
          },
          {
            label: 'Add domain',
            capability: 'agencies_write',
            // TODO(N9): MFA challenge modal when 401 mfa_required
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
