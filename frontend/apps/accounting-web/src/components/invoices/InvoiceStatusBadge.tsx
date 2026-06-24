/**
 * Status badge for an invoice (UC-ACC-05.1 lifecycle:
 * draft → issued → sent → (partially_)paid / overdue / cancelled).
 */

'use client';

import { Badge, type BadgeVariant } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';

const VARIANT: Record<string, BadgeVariant> = {
  draft: 'secondary',
  issued: 'info',
  sent: 'primary',
  paid: 'success',
  partially_paid: 'warning',
  overdue: 'danger',
  cancelled: 'default',
};

const KNOWN = new Set(Object.keys(VARIANT));

export function InvoiceStatusBadge({ status }: { status: string }) {
  const t = useTranslations('app.invoices.status');
  const variant = VARIANT[status] ?? 'default';
  // Only translate known statuses; fall back to the raw backend value for any
  // status not yet mapped (avoids next-intl missing-key noise).
  const label = KNOWN.has(status) ? t(status as never) : status;
  return (
    <Badge variant={variant} rounded>
      {label}
    </Badge>
  );
}
