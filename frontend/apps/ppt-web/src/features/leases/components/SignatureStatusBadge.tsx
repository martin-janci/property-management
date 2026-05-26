/**
 * SignatureStatusBadge — inline badge showing the e-signature status of a lease document.
 * Epic 84, Story 84.2.
 */

import type { SignatureRequestStatus } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface SignatureStatusBadgeProps {
  status: SignatureRequestStatus | undefined | null;
  className?: string;
}

const STATUS_CLASSES: Record<SignatureRequestStatus, string> = {
  pending: 'bg-amber-100 text-amber-800',
  in_progress: 'bg-amber-100 text-amber-800',
  completed: 'bg-green-100 text-green-800',
  declined: 'bg-red-100 text-red-800',
  expired: 'bg-gray-100 text-gray-600',
  cancelled: 'bg-gray-100 text-gray-600',
};

export function SignatureStatusBadge({ status, className = '' }: SignatureStatusBadgeProps) {
  const { t } = useTranslation();

  if (!status) {
    return null;
  }

  const statusLabels: Record<SignatureRequestStatus, string> = {
    pending: t('leases.esign.status.pending', 'Pending Signature'),
    in_progress: t('leases.esign.status.in_progress', 'Signing In Progress'),
    completed: t('leases.esign.status.completed', 'Signed'),
    declined: t('leases.esign.status.declined', 'Declined'),
    expired: t('leases.esign.status.expired', 'Expired'),
    cancelled: t('leases.esign.status.cancelled', 'Cancelled'),
  };

  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded-full ${STATUS_CLASSES[status]} ${className}`}
    >
      <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
        <path d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z" />
      </svg>
      {statusLabels[status]}
    </span>
  );
}
