/**
 * NewMessagePage - Start a new conversation.
 *
 * Allows users to select recipients and compose an initial message
 * to start a new conversation thread.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RecipientSelector } from '../components/RecipientSelector';
import type { CreateThreadRequest, RecipientOption } from '../types';

interface NewMessagePageProps {
  recipients: RecipientOption[];
  /** Recipient ids to preselect (e.g. when arriving from "Contact" on a neighbor). */
  initialRecipientIds?: string[];
  isLoadingRecipients?: boolean;
  isSubmitting?: boolean;
  onSubmit: (data: CreateThreadRequest) => void;
  onCancel: () => void;
}

export function NewMessagePage({
  recipients,
  initialRecipientIds,
  isLoadingRecipients,
  isSubmitting,
  onSubmit,
  onCancel,
}: NewMessagePageProps) {
  const { t } = useTranslation();
  const [selectedRecipientIds, setSelectedRecipientIds] = useState<string[]>(
    initialRecipientIds ?? []
  );
  const [subject, setSubject] = useState('');
  const [message, setMessage] = useState('');
  const [errors, setErrors] = useState<{
    recipients?: string;
    message?: string;
  }>({});

  const validate = (): boolean => {
    const newErrors: typeof errors = {};

    if (selectedRecipientIds.length === 0) {
      newErrors.recipients = t('messaging.errors.recipientRequired');
    }

    if (!message.trim()) {
      newErrors.message = t('messaging.errors.messageRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) return;

    onSubmit({
      recipientIds: selectedRecipientIds,
      subject: subject.trim() || undefined,
      initialMessage: message.trim(),
    });
  };

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onCancel}
          className="mb-4 text-blue-600 hover:text-blue-800 flex items-center gap-1"
        >
          <svg
            className="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('messaging.backToMessages')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('messaging.newConversation')}</h1>
        <p className="mt-1 text-gray-600">{t('messaging.newConversationDescription')}</p>
      </div>

      {/* Form */}
      <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow p-6">
        {/* Recipients */}
        <div className="mb-6">
          <label htmlFor="recipients" className="block text-sm font-medium text-gray-700 mb-2">
            {t('messaging.to')} <span className="text-red-500">*</span>
          </label>
          <RecipientSelector
            recipients={recipients}
            selectedIds={selectedRecipientIds}
            onSelectionChange={setSelectedRecipientIds}
            isLoading={isLoadingRecipients}
          />
          {errors.recipients && <p className="mt-1 text-sm text-red-600">{errors.recipients}</p>}
        </div>

        {/* Subject (Optional) */}
        <div className="mb-6">
          <label htmlFor="subject" className="block text-sm font-medium text-gray-700 mb-2">
            {t('messaging.subject')} <span className="text-gray-400">({t('common.optional')})</span>
          </label>
          <input
            id="subject"
            type="text"
            value={subject}
            onChange={(e) => setSubject(e.target.value)}
            placeholder={t('messaging.subjectPlaceholder')}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
            maxLength={255}
          />
        </div>

        {/* Message */}
        <div className="mb-6">
          <label htmlFor="message" className="block text-sm font-medium text-gray-700 mb-2">
            {t('messaging.message')} <span className="text-red-500">*</span>
          </label>
          <textarea
            id="message"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder={t('messaging.messagePlaceholder')}
            rows={6}
            className="w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500 resize-none"
          />
          {errors.message && <p className="mt-1 text-sm text-red-600">{errors.message}</p>}
          <p className="mt-1 text-sm text-gray-500 text-right">
            {message.length} {t('announcements.form.characters')}
          </p>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-3 pt-4 border-t">
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="px-4 py-2 border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
          >
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed inline-flex items-center gap-2"
          >
            {isSubmitting ? (
              <>
                <svg
                  className="w-4 h-4 animate-spin"
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
                {t('common.saving')}
              </>
            ) : (
              <>
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                  />
                </svg>
                {t('messaging.sendMessage')}
              </>
            )}
          </button>
        </div>
      </form>
    </div>
  );
}
