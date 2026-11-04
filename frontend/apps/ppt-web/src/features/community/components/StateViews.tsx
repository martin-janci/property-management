/**
 * Loading / empty / error blocks shared by the Community pages.
 */

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components';

export function LoadingState() {
  const { t } = useTranslation();
  return (
    <div className="flex justify-center py-16">
      <Spinner size="lg" label={t('community.common.loading')} />
    </div>
  );
}

interface EmptyStateProps {
  title: string;
  body?: string;
  action?: ReactNode;
}

export function EmptyState({ title, body, action }: EmptyStateProps) {
  return (
    <div className="bg-white rounded-lg shadow p-8 text-center">
      <h3 className="text-lg font-medium text-gray-900">{title}</h3>
      {body && <p className="mt-2 text-sm text-gray-500">{body}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

interface ErrorStateProps {
  message: string;
  onRetry?: () => void;
}

export function ErrorState({ message, onRetry }: ErrorStateProps) {
  const { t } = useTranslation();
  return (
    <div role="alert" className="bg-red-50 border border-red-200 rounded-lg p-6 text-center">
      <p className="text-sm text-red-700">{message}</p>
      {onRetry && (
        <button
          type="button"
          onClick={onRetry}
          className="mt-3 px-4 py-2 text-sm font-medium text-red-700 border border-red-300 rounded-md hover:bg-red-100"
        >
          {t('community.hub.retry')}
        </button>
      )}
    </div>
  );
}
