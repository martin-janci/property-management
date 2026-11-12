/**
 * CreateNotificationForm Component (Epic 8A, Story 8A.2)
 *
 * A form for administrators to create new critical notifications.
 */

import type { CreateCriticalNotificationRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface CreateNotificationFormProps {
  onSubmit: (data: CreateCriticalNotificationRequest) => Promise<void>;
  onCancel: () => void;
  isSubmitting?: boolean;
  error?: string | null;
}

export function CreateNotificationForm({
  onSubmit,
  onCancel,
  isSubmitting = false,
  error = null,
}: CreateNotificationFormProps) {
  const { t } = useTranslation();
  const [title, setTitle] = useState('');
  const [message, setMessage] = useState('');
  const [errors, setErrors] = useState<{ title?: string; message?: string }>({});

  const validate = () => {
    const newErrors: { title?: string; message?: string } = {};

    if (!title.trim()) {
      newErrors.title = t('criticalNotifications.form.validation.titleRequired');
    } else if (title.length > 255) {
      newErrors.title = t('criticalNotifications.form.validation.titleTooLong');
    }

    if (!message.trim()) {
      newErrors.message = t('criticalNotifications.form.validation.messageRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) return;

    try {
      await onSubmit({ title: title.trim(), message: message.trim() });
      // Clear form on success
      setTitle('');
      setMessage('');
      setErrors({});
    } catch {
      // If submission fails, keep form values for user correction
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Error alert */}
      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <div className="flex">
            <div className="flex-shrink-0">
              <svg
                className="h-5 w-5 text-red-400"
                viewBox="0 0 20 20"
                fill="currentColor"
                aria-hidden="true"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
            <div className="ml-3">
              <p className="text-sm text-red-700">{error}</p>
            </div>
          </div>
        </div>
      )}

      <div>
        <label
          htmlFor="notification-title"
          className="block text-sm font-medium leading-6 text-gray-900"
        >
          {t('criticalNotifications.form.titleLabel')}
        </label>
        <div className="mt-2">
          <input
            type="text"
            id="notification-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className={`block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ${
              errors.title ? 'ring-red-300' : 'ring-gray-300'
            } placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-red-600 sm:text-sm sm:leading-6`}
            placeholder={t('criticalNotifications.form.titlePlaceholder')}
          />
          {errors.title && <p className="mt-1 text-sm text-red-600">{errors.title}</p>}
        </div>
      </div>

      <div>
        <label
          htmlFor="notification-message"
          className="block text-sm font-medium leading-6 text-gray-900"
        >
          {t('criticalNotifications.form.messageLabel')}
        </label>
        <div className="mt-2">
          <textarea
            id="notification-message"
            rows={6}
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            className={`block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ${
              errors.message ? 'ring-red-300' : 'ring-gray-300'
            } placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-red-600 sm:text-sm sm:leading-6`}
            placeholder={t('criticalNotifications.form.messagePlaceholder')}
          />
          {errors.message && <p className="mt-1 text-sm text-red-600">{errors.message}</p>}
        </div>
        <p className="mt-2 text-sm text-gray-500">{t('criticalNotifications.form.messageHelp')}</p>
      </div>

      <div className="bg-amber-50 border-l-4 border-amber-400 p-4">
        <div className="flex">
          <div className="flex-shrink-0">
            <svg
              className="h-5 w-5 text-amber-400"
              viewBox="0 0 20 20"
              fill="currentColor"
              aria-hidden="true"
            >
              <path
                fillRule="evenodd"
                d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 5zm0 9a1 1 0 100-2 1 1 0 000 2z"
                clipRule="evenodd"
              />
            </svg>
          </div>
          <div className="ml-3">
            <p className="text-sm text-amber-700">
              <strong>{t('criticalNotifications.form.warningStrong')}</strong>{' '}
              {t('criticalNotifications.form.warningText')}
            </p>
          </div>
        </div>
      </div>

      <div className="flex justify-end gap-3">
        <button
          type="button"
          onClick={onCancel}
          disabled={isSubmitting}
          className="rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 disabled:opacity-50"
        >
          {t('criticalNotifications.form.cancel')}
        </button>
        <button
          type="submit"
          disabled={isSubmitting}
          className="rounded-md bg-red-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-red-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-red-600 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSubmitting ? (
            <>
              <svg
                className="animate-spin -ml-1 mr-2 h-4 w-4 text-white inline"
                fill="none"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <circle
                  className="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  strokeWidth="4"
                />
                <path
                  className="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
              {t('criticalNotifications.form.submitting')}
            </>
          ) : (
            t('criticalNotifications.form.submit')
          )}
        </button>
      </div>
    </form>
  );
}
