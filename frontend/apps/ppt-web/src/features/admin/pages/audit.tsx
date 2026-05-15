/**
 * Phase 5 — `/admin/audit` page.
 *
 * Audit-log viewer. Reads from `GET /api/v1/admin/audit` (capability
 * `audit_read`).
 */

import { AuditViewer, type AuditEntry, type AuditFilter } from '@ppt/admin-ui';
import { useState, type FC } from 'react';
import { useTranslation } from 'react-i18next';

const AuditPage: FC = () => {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<AuditFilter>({});
  // TODO(phase-5-followup): wire to GET /api/v1/admin/audit?actor_id=...&action=...
  const entries: AuditEntry[] = [];
  return (
    <section>
      <h1>{t('admin.audit.title')}</h1>
      <AuditViewer entries={entries} filter={filter} onFilterChange={setFilter} />
    </section>
  );
};

export default AuditPage;
