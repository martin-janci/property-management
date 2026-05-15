/**
 * Phase 5 — `/admin/agencies` page.
 *
 * Stub: lists agencies via the future `GET /api/v1/admin/agencies` endpoint.
 * Concrete data fetching is intentionally deferred until the typed admin
 * API client is generated; this file provides the route surface.
 */

import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import type React from 'react';

interface AgencyRow {
  id: string;
  name: string;
  slug: string;
  status: string;
  member_count: number;
}

const columns: ReadonlyArray<ResourceTableColumn<AgencyRow>> = [
  { key: 'name', header: 'Name', render: (a) => a.name },
  { key: 'slug', header: 'Slug', render: (a) => a.slug },
  { key: 'status', header: 'Status', render: (a) => a.status },
  { key: 'members', header: 'Members', render: (a) => String(a.member_count) },
];

const AgenciesPage: React.FC = () => {
  // TODO(phase-5-followup): wire to GET /api/v1/admin/agencies.
  const data: AgencyRow[] = [];
  return (
    <section>
      <h1>Agencies</h1>
      <ResourceTable<AgencyRow>
        columns={columns}
        data={data}
        rowKey={(a) => a.id}
        emptyMessage="No agencies loaded yet."
        actions={[
          {
            label: 'Suspend',
            capability: 'agencies_suspend',
            variant: 'danger',
            onClick: (a) => console.warn('TODO: suspend', a.id),
          },
        ]}
      />
    </section>
  );
};

export default AgenciesPage;
