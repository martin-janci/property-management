/**
 * Phase 5 — `/admin/users` page.
 *
 * Stub: capability-gated global user search.
 */

import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import type React from 'react';

interface UserRow {
  id: string;
  email: string;
  display_name: string | null;
}

const columns: ReadonlyArray<ResourceTableColumn<UserRow>> = [
  { key: 'email', header: 'Email', render: (u) => u.email },
  { key: 'name', header: 'Display Name', render: (u) => u.display_name ?? '' },
];

const UsersPage: React.FC = () => {
  // TODO(phase-5-followup): wire to GET /api/v1/admin/users?q=...
  const data: UserRow[] = [];
  return (
    <section>
      <h1>Users</h1>
      <ResourceTable<UserRow>
        columns={columns}
        data={data}
        rowKey={(u) => u.id}
        emptyMessage="Search to load users."
        actions={[
          {
            label: 'Impersonate',
            capability: 'users_impersonate',
            variant: 'danger',
            onClick: (u) => console.warn('TODO: impersonate', u.id),
          },
        ]}
      />
    </section>
  );
};

export default UsersPage;
