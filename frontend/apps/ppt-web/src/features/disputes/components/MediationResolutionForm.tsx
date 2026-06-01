import { zodResolver } from '@hookform/resolvers/zod';
import type { ResolutionType, ResolveDisputeRequest } from '@ppt/api-client';
import { Controller, useForm } from 'react-hook-form';
import { useTranslation } from 'react-i18next';
import { z } from 'zod';

const RESOLUTION_TYPES = [
  'mutual_agreement',
  'favor_filer',
  'favor_respondent',
  'withdrawn',
  'dismissed',
] as const satisfies readonly ResolutionType[];

const resolutionSchema = z.object({
  resolutionType: z.enum(RESOLUTION_TYPES),
  resolutionDetails: z.string().min(10, { message: 'disputes.mediation.errors.detailsMinLength' }),
  terms: z.string().optional(),
  requiresConfirmation: z.boolean(),
});

type ResolutionFormValues = z.infer<typeof resolutionSchema>;

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

  const {
    register,
    control,
    handleSubmit,
    watch,
    formState: { errors },
  } = useForm<ResolutionFormValues>({
    resolver: zodResolver(resolutionSchema),
    defaultValues: {
      resolutionType: 'mutual_agreement',
      resolutionDetails: '',
      terms: '',
      requiresConfirmation: false,
    },
    mode: 'onTouched',
  });

  const selectedType = watch('resolutionType');

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

  const selectedOption = resolutionTypeOptions.find((o) => o.value === selectedType);

  const onSubmit = (data: ResolutionFormValues) => {
    onResolve({
      resolutionType: data.resolutionType,
      resolutionDetails: data.resolutionDetails.trim(),
      terms: data.terms?.trim() || undefined,
      requiresConfirmation: data.requiresConfirmation,
    });
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
      {/* Resolution type */}
      <fieldset>
        <legend className="block text-sm font-medium text-gray-700 mb-2">
          {t('disputes.mediation.resolutionType')}{' '}
          <span className="text-red-500" aria-hidden="true">
            *
          </span>
        </legend>
        <div className="space-y-2">
          <Controller
            name="resolutionType"
            control={control}
            render={({ field }) => (
              <>
                {resolutionTypeOptions.map((option) => (
                  <label
                    key={option.value}
                    className={[
                      'flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors',
                      field.value === option.value
                        ? 'border-violet-500 bg-violet-50'
                        : 'border-gray-200 hover:border-gray-300',
                    ].join(' ')}
                  >
                    <input
                      type="radio"
                      name={field.name}
                      value={option.value}
                      checked={field.value === option.value}
                      onChange={() => field.onChange(option.value)}
                      className="mt-1 text-violet-600"
                    />
                    <div>
                      <p className="text-sm font-medium text-gray-900">{option.label}</p>
                      <p className="text-xs text-gray-500 mt-0.5">{option.description}</p>
                    </div>
                  </label>
                ))}
              </>
            )}
          />
        </div>
        {selectedOption && (
          <p className="mt-2 text-xs text-gray-500 italic">
            {t('disputes.mediation.selectedType', { label: selectedOption.label })}
          </p>
        )}
      </fieldset>

      {/* Resolution details */}
      <div>
        <label htmlFor="resolutionDetails" className="block text-sm font-medium text-gray-700 mb-1">
          {t('disputes.mediation.resolutionDetails')}{' '}
          <span className="text-red-500" aria-hidden="true">
            *
          </span>
        </label>
        <textarea
          id="resolutionDetails"
          {...register('resolutionDetails')}
          rows={5}
          placeholder={t('disputes.mediation.resolutionDetailsPlaceholder')}
          aria-invalid={!!errors.resolutionDetails}
          aria-describedby={errors.resolutionDetails ? 'resolutionDetails-error' : undefined}
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent resize-none aria-[invalid=true]:border-red-400"
        />
        {errors.resolutionDetails ? (
          <p id="resolutionDetails-error" role="alert" className="mt-1 text-xs text-red-600">
            {t(errors.resolutionDetails.message ?? 'disputes.mediation.errors.detailsMinLength')}
          </p>
        ) : (
          <p className="mt-1 text-xs text-gray-400">
            {t('disputes.mediation.charMinimum', {
              count: watch('resolutionDetails').trim().length,
            })}
          </p>
        )}
      </div>

      {/* Terms (optional) */}
      <div>
        <label htmlFor="terms" className="block text-sm font-medium text-gray-700 mb-1">
          {t('disputes.mediation.termsConditions')}{' '}
          <span className="text-xs font-normal text-gray-400">
            ({t('disputes.mediation.optional')})
          </span>
        </label>
        <textarea
          id="terms"
          {...register('terms')}
          rows={3}
          placeholder={t('disputes.mediation.termsPlaceholder')}
          className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent resize-none"
        />
      </div>

      {/* Requires confirmation */}
      <label className="flex items-start gap-2.5 cursor-pointer">
        <input
          type="checkbox"
          {...register('requiresConfirmation')}
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
          disabled={isSubmitting}
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
