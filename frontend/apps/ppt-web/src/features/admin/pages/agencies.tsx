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

import { ResourceTable, type ResourceTableColumn } from '@ppt/admin-ui';
import { type Agency, adminKeys, suspendAgency, useAgencies } from '@ppt/api-client';
import { useQueryClient } from '@tanstack/react-query';
import type React from 'react';
import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../../../components';

const AgenciesPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const queryClient = useQueryClient();
  const query = useAgencies({ page: 1, page_size: 50 });

  const columns = useMemo<ReadonlyArray<ResourceTableColumn<Agency>>>(
    () => [
      { key: 'name', header: t('admin.agencies.columns.name'), render: (a) => a.name },
      { key: 'slug', header: t('admin.agencies.columns.slug'), render: (a) => a.slug },
      { key: 'status', header: t('admin.agencies.columns.status'), render: (a) => a.status },
      {
        key: 'members',
        header: t('admin.agencies.columns.members'),
        render: (a) => String(a.member_count),
      },
    ],
    [t]
  );

  // N9: the suspend handler is the canonical "this works under MFA"
  // example. The api-client's mfa_required interceptor handles the
  // challenge + retry, so this code only sees the final outcome.
  const handleSuspend = useCallback(
    async (agency: Agency) => {
      try {
        await suspendAgency(agency.id);
        showToast({
          type: 'success',
          title: t('admin.agencies.toast.suspendedTitle'),
          message: t('admin.agencies.toast.suspendedMessage', { name: agency.name }),
        });
        await queryClient.invalidateQueries({ queryKey: adminKeys.agencies() });
      } catch (e) {
        const message =
          e instanceof Error ? e.message : t('admin.agencies.toast.suspendFailedMessage');
        showToast({
          type: 'error',
          title: t('admin.agencies.toast.suspendFailedTitle'),
          message,
        });
      }
    },
    [queryClient, showToast, t]
  );

  if (query.isLoading) {
    // Lightweight inline spinner — the global PageLoading lives elsewhere
    // and would be overkill for a sub-route partial fetch.
    return (
      <section>
        <h1>{t('admin.agencies.title')}</h1>
        <div role="status" aria-live="polite">
          {t('admin.agencies.loading')}
        </div>
      </section>
    );
  }

  if (query.isError) {
    const message =
      query.error instanceof Error ? query.error.message : t('admin.agencies.unknownError');
    return (
      <section>
        <h1>{t('admin.agencies.title')}</h1>
        <div role="alert" className="ppt-admin-error">
          <p>{t('admin.agencies.loadError', { message })}</p>
          <button type="button" onClick={() => query.refetch()}>
            {t('admin.common.retry')}
          </button>
        </div>
      </section>
    );
  }

  const items = query.data?.items ?? [];

  return (
    <section>
      <h1>{t('admin.agencies.title')}</h1>
      <ResourceTable<Agency>
        columns={columns}
        data={items}
        rowKey={(a) => a.id}
        emptyMessage={t('admin.agencies.empty')}
        actions={[
          {
            label: t('admin.agencies.actions.suspend'),
            capability: 'agencies_suspend',
            variant: 'danger',
            // Real call — api-client interceptor opens the MFA modal on
            // 401 mfa_required and retries on success.
            onClick: (a) => {
              void handleSuspend(a);
            },
          },
          {
            label: t('admin.agencies.actions.addDomain'),
            capability: 'agencies_write',
            // TODO: separate endpoint not yet wired (out of scope for N9).
            onClick: (a) =>
              showToast({
                type: 'warning',
                title: t('admin.agencies.toast.notWiredTitle'),
                message: t('admin.agencies.toast.notWiredMessage', { id: a.id }),
              }),
          },
        ]}
      />
    </section>
  );
};

export default AgenciesPage;
