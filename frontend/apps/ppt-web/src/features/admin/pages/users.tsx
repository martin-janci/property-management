/**
 * Phase 5 — `/admin/users` page.
 *
 * Stub: capability-gated global user search.
 */

import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import type React from 'react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

interface UserRow {
  id: string;
  email: string;
  display_name: string | null;
}

const UsersPage: React.FC = () => {
  const { t } = useTranslation();

  const columns = useMemo<ReadonlyArray<ResourceTableColumn<UserRow>>>(
    () => [
      { key: 'email', header: t('admin.users.columns.email'), render: (u) => u.email },
      {
        key: 'name',
        header: t('admin.users.columns.displayName'),
        render: (u) => u.display_name ?? '',
      },
    ],
    [t],
  );

  // TODO(phase-5-followup): wire to GET /api/v1/admin/users?q=...
  const data: UserRow[] = [];
  return (
    <section>
      <h1>{t('admin.users.title')}</h1>
      <ResourceTable<UserRow>
        columns={columns}
        data={data}
        rowKey={(u) => u.id}
        emptyMessage={t('admin.users.empty')}
        actions={[
          {
            label: t('admin.users.actions.impersonate'),
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
