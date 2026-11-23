/**
 * FillFormPage Component
 *
 * Form filling page for users (Epic 54, Story 54.3).
 */

import type { FormWithDetails, SubmitFormRequest } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import { FormRenderer } from '../components/FormRenderer';

interface FillFormPageProps {
  form: FormWithDetails;
  isSubmitting?: boolean;
  onSubmit: (formId: string, data: SubmitFormRequest) => void;
  onCancel: () => void;
}

export function FillFormPage({ form, isSubmitting, onSubmit, onCancel }: FillFormPageProps) {
  const { t } = useTranslation();
  const handleSubmit = (data: SubmitFormRequest) => {
    onSubmit(form.id, data);
  };

  const isExpired = form.submissionDeadline && new Date(form.submissionDeadline) < new Date();

  if (isExpired) {
    return (
      <div className="max-w-3xl mx-auto px-4 py-8">
        <div className="bg-red-50 border border-red-200 rounded-lg p-6 text-center">
          <svg
            className="mx-auto h-12 w-12 text-red-400 mb-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{t('forms.fill.expiredIcon')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <h2 className="text-xl font-semibold text-red-800 mb-2">
            {t('forms.fill.expiredTitle')}
          </h2>
          <p className="text-red-600 mb-4">{t('forms.fill.expiredDescription')}</p>
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
          >
            {t('forms.actions.goBack')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-gray-900">{form.title}</h1>
        {form.description && <p className="text-gray-500 mt-2">{form.description}</p>}

        <div className="mt-4 flex flex-wrap gap-4 text-sm text-gray-500">
          {form.category && (
            <span className="flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('forms.card.iconCategory')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
                />
              </svg>
              {form.category}
            </span>
          )}
          {form.submissionDeadline && (
            <span className="flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('forms.card.iconDeadline')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
                />
              </svg>
              {t('forms.card.due', {
                date: new Date(form.submissionDeadline).toLocaleDateString(getActiveLocale()),
              })}
            </span>
          )}
          {form.requireSignatures && (
            <span className="flex items-center gap-1 text-purple-600">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>{t('forms.fill.signatureRequiredIcon')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
                />
              </svg>
              {t('forms.fill.signatureRequired')}
            </span>
          )}
        </div>
      </div>

      {/* Form */}
      <div className="bg-white border border-gray-200 rounded-lg p-6">
        <FormRenderer
          fields={form.fields}
          requireSignature={form.requireSignatures}
          isSubmitting={isSubmitting}
          onSubmit={handleSubmit}
          onCancel={onCancel}
        />
      </div>
    </div>
  );
}
