/**
 * SubmissionStatusBadge - Status badge for regulatory submissions.
 * Epic 41: Government Portal UI (Story 41.2)
 */

import type { SubmissionStatus } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface SubmissionStatusBadgeProps {
  status: SubmissionStatus;
  showIcon?: boolean;
  /** Size variant for the badge */
  size?: 'sm' | 'md' | 'lg';
}

interface StatusConfig {
  /** i18n key under `submissionStatus.status` for the badge label */
  labelKey: string;
  className: string;
  icon: React.ReactNode;
}

const statusConfig: Record<SubmissionStatus, StatusConfig> = {
  draft: {
    labelKey: 'draft',
    className: 'bg-gray-100 text-gray-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
        />
      </svg>
    ),
  },
  pending_validation: {
    labelKey: 'pending_validation',
    className: 'bg-blue-100 text-blue-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
        />
      </svg>
    ),
  },
  validated: {
    labelKey: 'validated',
    className: 'bg-cyan-100 text-cyan-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
        />
      </svg>
    ),
  },
  submitted: {
    labelKey: 'submitted',
    className: 'bg-indigo-100 text-indigo-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
        />
      </svg>
    ),
  },
  acknowledged: {
    labelKey: 'acknowledged',
    className: 'bg-purple-100 text-purple-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
      </svg>
    ),
  },
  processing: {
    labelKey: 'processing',
    className: 'bg-yellow-100 text-yellow-700',
    icon: (
      <svg
        className="h-3.5 w-3.5 animate-spin"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
        />
      </svg>
    ),
  },
  accepted: {
    labelKey: 'accepted',
    className: 'bg-green-100 text-green-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
    ),
  },
  rejected: {
    labelKey: 'rejected',
    className: 'bg-red-100 text-red-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
    ),
  },
  requires_correction: {
    labelKey: 'requires_correction',
    className: 'bg-orange-100 text-orange-700',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        />
      </svg>
    ),
  },
  cancelled: {
    labelKey: 'cancelled',
    className: 'bg-gray-100 text-gray-500',
    icon: (
      <svg
        className="h-3.5 w-3.5"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        aria-hidden="true"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
        />
      </svg>
    ),
  },
};

export function SubmissionStatusBadge({
  status,
  showIcon = true,
  size = 'md',
}: SubmissionStatusBadgeProps) {
  const { t } = useTranslation();
  const config = statusConfig[status];

  const sizeClasses = {
    sm: 'px-2 py-0.5 text-xs gap-1',
    md: 'px-2.5 py-1 text-sm gap-1.5',
    lg: 'px-3 py-1.5 text-base gap-2',
  };

  return (
    <span
      className={`inline-flex items-center rounded-full font-medium ${config.className} ${sizeClasses[size]}`}
    >
      {showIcon && config.icon}
      {t(`submissionStatus.status.${config.labelKey}`)}
    </span>
  );
}
