import type { ResolutionType, ResolveDisputeRequest } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface MediationResolutionFormProps {
  isSubmitting?: boolean;
  onResolve: (data: ResolveDisputeRequest) => void;
  onCancel: () => void;
}

export function MediationResolutionForm({
  isSubmitting,
  onResolve,
  onCancel,
}: MediationResolutionFormProps) {
  const { t } = useTranslation();
  const [resolutionType, setResolutionType] = useState<ResolutionType>('mutual_agreement');
  const [resolutionDetails, setResolutionDetails] = useState('');
  const [terms, setTerms] = useState('');
  const [requiresConfirmation, setRequiresConfirmation] = useState(false);

  const charCount = resolutionDetails.trim().length;
  const canSubmit = charCount >= 10;

  const resolutionTypeOptions: Array<{
    value: ResolutionType;
    label: string;
    description: string;
  }> = [
    {
      value: 'mutual_agreement',
      label: t('disputes.mediation.resolutionMutualAgreement'),
      description: t('disputes.mediation.resolutionMutualAgreementDesc'),
    },
    {
      value: 'favor_filer',
      label: t('disputes.mediation.resolutionFavorFiler'),
      description: t('disputes.mediation.resolutionFavorFilerDesc'),
    },
    {
      value: 'favor_respondent',
      label: t('disputes.mediation.resolutionFavorRespondent'),
      description: t('disputes.mediation.resolutionFavorRespondentDesc'),
    },
    {
      value: 'withdrawn',
      label: t('disputes.mediation.resolutionWithdrawn'),
      description: t('disputes.mediation.resolutionWithdrawnDesc'),
    },
    {
      value: 'dismissed',
      label: t('disputes.mediation.resolutionDismissed'),
      description: t('disputes.mediation.resolutionDismissedDesc'),
    },
  ];

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit || isSubmitting) return;

    onResolve({
      resolutionType,
      resolutionDetails: resolutionDetails.trim(),
      terms: terms.trim() || undefined,
      requiresConfirmation,
    });
  };

  const selectedOption = resolutionTypeOptions.find((o) => o.value === resolutionType);

  return (
    <form onSubmit={handleSubmit} className="space-y-5">
      {/* Resolution type */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-2">
          {t('disputes.mediation.resolutionType')} <span className="text-red-500">*</span>
        </label>
        <div className="space-y-2">
          {resolutionTypeOptions.map((option) => (
            <label
              key={option.value}
              className={[
                'flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors',
                resolutionType === option.value
                  ? 'border-violet-500 bg-violet-50'
                  : 'border-gray-200 hover:border-gray-300',
              ].join(' ')}
            >
              <input
                type="radio"
                name="resolutionType"
                value={option.value}
                checked={resolutionType === option.value}
                onChange={() => setResolutionType(option.value)}
                className="mt-1 text-violet-600"
              />
              <div>
                <p className="text-sm font-medium text-gray-900">{option.label}</p>
                <p className="text-xs text-gray-500 mt-0.5">{option.description}</p>
              </div>
            </label>
          ))}
        </div>
        {selectedOption && (
          <p className="mt-2 text-xs text-gray-500 italic">
            {t('disputes.mediation.selectedType', { label: selectedOption.label })}
          </p>
        )}
      </div>

      {/* Resolution details */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          {t('disputes.mediation.resolutionDetails')} <span className="text-red-500">*</span>
        </label>
        <textarea
          value={resolutionDetails}
          onChange={(e) => setResolutionDetails(e.target.value)}
          rows={5}
          placeholder={t('disputes.mediation.resolutionDetailsPlaceholder')}
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent resize-none"
          required
        />
        <p className="mt-1 text-xs text-gray-400">
          {t('disputes.mediation.charMinimum', { count: charCount })}
        </p>
      </div>

      {/* Terms (optional) */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-1">
          {t('disputes.mediation.termsConditions')}{' '}
          <span className="text-xs font-normal text-gray-400">
            ({t('disputes.mediation.optional')})
          </span>
        </label>
        <textarea
          value={terms}
          onChange={(e) => setTerms(e.target.value)}
          rows={3}
          placeholder={t('disputes.mediation.termsPlaceholder')}
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent resize-none"
        />
      </div>

      {/* Requires confirmation */}
      <label className="flex items-start gap-2.5 cursor-pointer">
        <input
          type="checkbox"
          checked={requiresConfirmation}
          onChange={(e) => setRequiresConfirmation(e.target.checked)}
          className="mt-0.5 rounded border-gray-300 text-violet-600"
        />
        <div>
          <p className="text-sm font-medium text-gray-700">
            {t('disputes.mediation.requiresConfirmation')}
          </p>
          <p className="text-xs text-gray-500 mt-0.5">
            {t('disputes.mediation.requiresConfirmationDesc')}
          </p>
        </div>
      </label>

      {/* Actions */}
      <div className="flex justify-end gap-3 pt-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50"
        >
          {t('disputes.mediation.cancel')}
        </button>
        <button
          type="submit"
          disabled={!canSubmit || isSubmitting}
          className="px-5 py-2 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
        >
          {isSubmitting
            ? t('disputes.mediation.recording')
            : t('disputes.mediation.recordResolutionBtn')}
        </button>
      </div>
    </form>
  );
}
