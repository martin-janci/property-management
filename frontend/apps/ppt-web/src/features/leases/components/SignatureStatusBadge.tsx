/**
 * SignatureStatusBadge — inline badge showing the e-signature status of a lease document.
 * Epic 84, Story 84.2.
 */

import type { SignatureRequestStatus } from '@ppt/api-client';

interface SignatureStatusBadgeProps {
  status: SignatureRequestStatus | undefined | null;
  className?: string;
}

const STATUS_CONFIG: Record<SignatureRequestStatus, { label: string; classes: string }> = {
  pending: {
    label: 'Pending Signature',
    classes: 'bg-amber-100 text-amber-800',
  },
  in_progress: {
    label: 'Signing In Progress',
    classes: 'bg-amber-100 text-amber-800',
  },
  completed: {
    label: 'Signed',
    classes: 'bg-green-100 text-green-800',
  },
  declined: {
    label: 'Declined',
    classes: 'bg-red-100 text-red-800',
  },
  expired: {
    label: 'Expired',
    classes: 'bg-gray-100 text-gray-600',
  },
  cancelled: {
    label: 'Cancelled',
    classes: 'bg-gray-100 text-gray-600',
  },
};

export function SignatureStatusBadge({ status, className = '' }: SignatureStatusBadgeProps) {
  if (!status) {
    return null;
  }

  const config = STATUS_CONFIG[status];

  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded-full ${config.classes} ${className}`}
    >
      <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
        <path d="M13.586 3.586a2 2 0 112.828 2.828l-.793.793-2.828-2.828.793-.793zM11.379 5.793L3 14.172V17h2.828l8.38-8.379-2.83-2.828z" />
      </svg>
      {config.label}
    </span>
  );
}
