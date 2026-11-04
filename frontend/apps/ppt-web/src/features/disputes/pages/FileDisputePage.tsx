/**
 * FileDisputePage — dispute filing form (Epic 80, Story 80.2).
 *
 * Provides:
 *  - Dispute type / reason selector (radio-card grid)
 *  - Subject + description fields (zod-validated via react-hook-form)
 *  - Evidence uploader (EvidenceUploader component — AC-2)
 *  - API wiring delegated to the route wrapper in App.tsx (useCreateDispute +
 *    useUploadEvidence); this component is a pure presentational form.
 */

import { zodResolver } from '@hookform/resolvers/zod';
import React from 'react';
import { Controller, useForm, useWatch } from 'react-hook-form';
import { Trans, useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { z } from 'zod';
import { DraftSavedIndicator } from '../components/DraftSavedIndicator';
import { EvidenceUploader, type PendingEvidence } from '../components/EvidenceUploader';
import { useDraftStorage } from '../hooks/useDraftStorage';

/** localStorage key for the in-progress dispute-filing draft (AC-4). */
export const DISPUTE_DRAFT_KEY = 'ppt-dispute-filing-draft';

// ============================================
// Validation schema
// ============================================

const DISPUTE_TYPES = ['noise', 'damage', 'payment', 'lease', 'maintenance', 'other'] as const;
export type DisputeTypeValue = (typeof DISPUTE_TYPES)[number];

// Validation messages are i18n keys; they are translated at render time.
const disputeSchema = z.object({
  type: z.enum(DISPUTE_TYPES, { required_error: 'disputes.validation.typeRequired' }),
  subject: z
    .string()
    .min(5, 'disputes.validation.subjectMin')
    .max(200, 'disputes.validation.subjectMax'),
  description: z
    .string()
    .min(30, 'disputes.validation.descriptionMin')
    .max(5000, 'disputes.validation.descriptionMax'),
  unitId: z.string().min(1, 'disputes.validation.unitRequired'),
  respondentId: z.string().optional(),
});

export type DisputeFormValues = z.infer<typeof disputeSchema>;

// ============================================
// Type metadata
// ============================================

// Label/description are rendered from `disputes.types.<value>.{label,description}`.
const TYPE_ICONS: Record<DisputeTypeValue, string> = {
  noise: '🔊',
  damage: '🔨',
  payment: '💶',
  lease: '📄',
  maintenance: '🔧',
  other: '⚙️',
};

// ============================================
// Props
// ============================================

export interface FileDisputeSubmitPayload {
  values: DisputeFormValues;
  evidence: PendingEvidence[];
}

interface FileDisputePageProps {
  /** Available units for the selector (fetched by the route wrapper) */
  units?: Array<{ id: string; label: string }>;
  /** Available respondents / other residents */
  respondents?: Array<{ id: string; name: string }>;
  /** True while the mutation is in flight */
  isSubmitting?: boolean;
  /** Called when the user submits the complete form */
  onSubmit: (payload: FileDisputeSubmitPayload) => void | Promise<void>;
}

// ============================================
// Component
// ============================================

export function FileDisputePage({
  units = [],
  respondents = [],
  isSubmitting = false,
  onSubmit,
}: FileDisputePageProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  // AC-4: draft auto-save. Restore any previously-typed draft synchronously so
  // it can seed the form's default values, and persist edits (debounced) so an
  // accidental tab close / navigation doesn't lose a long dispute description.
  // Evidence files are intentionally NOT persisted (File objects aren't
  // serialisable and re-attaching them is the user's call on restore).
  const { restored, savedAt, save, clear } =
    useDraftStorage<Partial<DisputeFormValues>>(DISPUTE_DRAFT_KEY);

  const {
    register,
    control,
    handleSubmit,
    watch,
    formState: { errors },
  } = useForm<DisputeFormValues>({
    resolver: zodResolver(disputeSchema),
    defaultValues: {
      type: restored?.type ?? undefined,
      subject: restored?.subject ?? '',
      description: restored?.description ?? '',
      unitId: restored?.unitId ?? '',
      respondentId: restored?.respondentId ?? '',
    },
  });

  // Evidence state is managed outside react-hook-form (files are not
  // serialisable by zod; they are passed alongside the validated values).
  const [evidence, setEvidence] = React.useState<PendingEvidence[]>([]);

  // Keep isSubmitting in a ref so the watch subscription closure doesn't
  // re-register every time the flag toggles.
  const isSubmittingRef = React.useRef(isSubmitting);
  React.useEffect(() => {
    isSubmittingRef.current = isSubmitting;
  }, [isSubmitting]);

  // Subscribe to field changes (fires only on real changes, not on every render)
  // and persist the draft debounced. Skips while a submit is in flight.
  React.useEffect(() => {
    const sub = watch((values) => {
      if (!isSubmittingRef.current) save(values as Partial<DisputeFormValues>);
    });
    return () => sub.unsubscribe();
  }, [watch, save]);

  const descriptionValue = useWatch({ control, name: 'description' });
  const descriptionLength = descriptionValue?.length ?? 0;

  const handleFormSubmit = handleSubmit(async (values) => {
    try {
      await onSubmit({ values, evidence });
    } catch {
      // The route wrapper already surfaces the error toast. Swallow here so a
      // failed filing keeps the auto-saved draft (we deliberately skip clear())
      // and react-hook-form doesn't bubble an unhandled rejection.
      return;
    }
    // Clear the persisted draft only once the submit handler resolves without
    // throwing — a failed filing keeps the draft so the user can retry.
    clear();
  });

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={() => navigate('/disputes')}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          ← {t('common.backToDisputes')}
        </button>
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">
              {t('disputes.filePageTitle', 'File a Dispute')}
            </h1>
            <p className="text-gray-500 mt-1">
              <Trans i18nKey="disputes.form.pageSubtitle">
                Submit a formal dispute. All fields marked <span className="text-red-500">*</span>{' '}
                are required.
              </Trans>
            </p>
          </div>
          <DraftSavedIndicator savedAt={savedAt} className="mt-1 shrink-0" />
        </div>
        {restored && (
          <p className="mt-3 text-sm text-gray-500" role="status">
            {t('disputes.draftRestored', 'Restored your saved draft.')}
          </p>
        )}
      </div>

      <form onSubmit={handleFormSubmit} noValidate className="space-y-8">
        {/* ── Section 1: Dispute type ── */}
        <section aria-labelledby="type-heading">
          <h2 id="type-heading" className="text-base font-semibold text-gray-800 mb-3">
            {t('disputes.form.disputeType')} <span className="text-red-500">*</span>
          </h2>
          <Controller
            name="type"
            control={control}
            render={({ field }) => (
              <div
                role="radiogroup"
                aria-required="true"
                aria-label={t('aria.disputeType')}
                className="grid grid-cols-1 sm:grid-cols-2 gap-3"
              >
                {DISPUTE_TYPES.map((type) => {
                  const label = t(`disputes.types.${type}.label`);
                  const checked = field.value === type;
                  return (
                    <label
                      key={type}
                      className={[
                        'relative flex items-start gap-3 rounded-lg border p-4 cursor-pointer',
                        'transition-colors duration-150',
                        checked
                          ? 'border-blue-600 bg-blue-50 ring-1 ring-blue-600'
                          : 'border-gray-200 bg-white hover:border-gray-300',
                      ].join(' ')}
                    >
                      <input
                        type="radio"
                        value={type}
                        checked={checked}
                        onChange={() => field.onChange(type)}
                        className="sr-only"
                        aria-label={label}
                      />
                      <span className="text-2xl leading-none" aria-hidden="true">
                        {TYPE_ICONS[type]}
                      </span>
                      <div>
                        <p className="text-sm font-medium text-gray-900">{label}</p>
                        <p className="text-xs text-gray-500 mt-0.5">
                          {t(`disputes.types.${type}.description`)}
                        </p>
                      </div>
                      {checked && (
                        <div className="absolute top-3 right-3 w-4 h-4 rounded-full bg-blue-600 flex items-center justify-center">
                          <svg
                            className="w-2.5 h-2.5 text-white"
                            fill="currentColor"
                            viewBox="0 0 8 8"
                            aria-hidden="true"
                          >
                            <circle cx="4" cy="4" r="3" />
                          </svg>
                        </div>
                      )}
                    </label>
                  );
                })}
              </div>
            )}
          />
          {errors.type && (
            <p className="mt-1 text-sm text-red-600" role="alert">
              {errors.type.message ? t(errors.type.message) : null}
            </p>
          )}
        </section>

        {/* ── Section 2: Location (unit) ── */}
        <section aria-labelledby="location-heading">
          <h2 id="location-heading" className="text-base font-semibold text-gray-800 mb-3">
            {t('disputes.form.location')} <span className="text-red-500">*</span>
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label htmlFor="unitId" className="block text-sm font-medium text-gray-700 mb-1">
                {t('disputes.form.unit')} <span className="text-red-500">*</span>
              </label>
              <select
                id="unitId"
                {...register('unitId')}
                className={[
                  'w-full rounded-md border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500',
                  errors.unitId ? 'border-red-400' : 'border-gray-300',
                ].join(' ')}
              >
                <option value="">{t('disputes.form.selectUnit')}</option>
                {units.map((u) => (
                  <option key={u.id} value={u.id}>
                    {u.label}
                  </option>
                ))}
              </select>
              {errors.unitId && (
                <p className="mt-1 text-xs text-red-600" role="alert">
                  {errors.unitId.message ? t(errors.unitId.message) : null}
                </p>
              )}
            </div>

            {/* Other party (optional) */}
            {respondents.length > 0 && (
              <div>
                <label
                  htmlFor="respondentId"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('disputes.form.otherPartyOptional')}
                </label>
                <select
                  id="respondentId"
                  {...register('respondentId')}
                  className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">{t('disputes.form.notSpecified')}</option>
                  {respondents.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>
        </section>

        {/* ── Section 3: Details ── */}
        <section aria-labelledby="details-heading">
          <h2 id="details-heading" className="text-base font-semibold text-gray-800 mb-3">
            {t('disputes.form.details')}
          </h2>
          <div className="space-y-4">
            {/* Subject */}
            <div>
              <label htmlFor="subject" className="block text-sm font-medium text-gray-700 mb-1">
                {t('disputes.form.subject')} <span className="text-red-500">*</span>
              </label>
              <input
                id="subject"
                type="text"
                maxLength={200}
                placeholder={t('disputes.subjectPlaceholder', 'Brief summary of the dispute')}
                {...register('subject')}
                aria-describedby={errors.subject ? 'subject-error' : undefined}
                className={[
                  'w-full rounded-md border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500',
                  errors.subject ? 'border-red-400' : 'border-gray-300',
                ].join(' ')}
              />
              {errors.subject && (
                <p id="subject-error" className="mt-1 text-xs text-red-600" role="alert">
                  {errors.subject.message ? t(errors.subject.message) : null}
                </p>
              )}
            </div>

            {/* Description */}
            <div>
              <label htmlFor="description" className="block text-sm font-medium text-gray-700 mb-1">
                {t('disputes.form.description')} <span className="text-red-500">*</span>
              </label>
              <textarea
                id="description"
                rows={6}
                maxLength={5000}
                placeholder={t(
                  'disputes.descriptionPlaceholder',
                  'Describe the dispute in detail. Include dates, times, and specific incidents (minimum 30 characters).'
                )}
                {...register('description')}
                aria-describedby={errors.description ? 'description-error' : 'description-hint'}
                className={[
                  'w-full rounded-md border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500',
                  errors.description ? 'border-red-400' : 'border-gray-300',
                ].join(' ')}
              />
              <div className="mt-1 flex justify-between">
                {errors.description ? (
                  <p id="description-error" className="text-xs text-red-600" role="alert">
                    {errors.description.message ? t(errors.description.message) : null}
                  </p>
                ) : (
                  <span id="description-hint" className="text-xs text-gray-400">
                    {t('disputes.form.minCharacters', { count: 30 })}
                  </span>
                )}
                <span className="text-xs text-gray-400">{descriptionLength} / 5000</span>
              </div>
            </div>
          </div>
        </section>

        {/* ── Section 4: Evidence ── */}
        <section aria-labelledby="evidence-heading">
          <h2 id="evidence-heading" className="text-base font-semibold text-gray-800 mb-1">
            {t('disputes.form.evidenceOptional')}
          </h2>
          <p className="text-sm text-gray-500 mb-3">{t('disputes.form.evidenceHint')}</p>
          <EvidenceUploader files={evidence} onChange={setEvidence} disabled={isSubmitting} />
        </section>

        {/* ── Info box ── */}
        <div className="rounded-lg bg-blue-50 border border-blue-100 p-4">
          <h3 className="text-sm font-semibold text-blue-900 mb-1">
            {t('disputes.form.nextStepsTitle')}
          </h3>
          <ol className="text-sm text-blue-800 list-decimal list-inside space-y-0.5">
            <li>{t('disputes.form.nextStep1')}</li>
            <li>{t('disputes.form.nextStep2')}</li>
            <li>{t('disputes.form.nextStep3')}</li>
            <li>{t('disputes.form.nextStep4')}</li>
            <li>{t('disputes.form.nextStep5')}</li>
          </ol>
        </div>

        {/* ── Actions ── */}
        <div className="flex justify-end gap-3 pt-4 border-t">
          <button
            type="button"
            onClick={() => navigate('/disputes')}
            disabled={isSubmitting}
            className="px-4 py-2 border border-gray-300 rounded-lg text-sm text-gray-700 hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            className="px-5 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? (
              <span className="flex items-center gap-2">
                <span
                  className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"
                  aria-hidden="true"
                />
                {t('disputes.form.filing')}
              </span>
            ) : (
              t('disputes.form.submit')
            )}
          </button>
        </div>
      </form>
    </div>
  );
}
