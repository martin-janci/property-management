/**
 * Create/edit form for a system announcement (#521).
 *
 * Form state stays in `useState` (no `react-hook-form` runtime dep yet),
 * but validation runs through a shared zod schema (`../lib/schema.ts`)
 * instead of native HTML5 hints, giving us field-level error display and
 * a tested validation surface.
 */

import type { SystemAnnouncement, SystemAnnouncementSeverity } from '@ppt/api-client';
import type { CSSProperties, FormEvent } from 'react';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toLocalDatetimeInput } from '../lib/formatters';
import { type AnnFormFieldErrors, type AnnFormValues, validateAnnForm } from '../lib/schema';
import { BannerPreview } from './BannerPreview';

export type { AnnFormValues } from '../lib/schema';

export const EMPTY_FORM: AnnFormValues = {
  title: '',
  message: '',
  severity: 'info',
  start_at: toLocalDatetimeInput(new Date().toISOString()),
  end_at: '',
  is_dismissible: true,
  requires_acknowledgment: false,
};

export function annToForm(ann: SystemAnnouncement): AnnFormValues {
  return {
    title: ann.title,
    message: ann.message,
    severity: ann.severity,
    start_at: toLocalDatetimeInput(ann.start_at),
    end_at: toLocalDatetimeInput(ann.end_at),
    is_dismissible: ann.is_dismissible,
    requires_acknowledgment: ann.requires_acknowledgment,
  };
}

const labelStyle: CSSProperties = {
  display: 'block',
  fontSize: 13,
  fontWeight: 500,
  marginBottom: 4,
  color: 'var(--ppt-fg-secondary, #374151)',
};

const inputStyle: CSSProperties = {
  width: '100%',
  padding: '7px 10px',
  border: '1px solid var(--ppt-border-default, #d1d5db)',
  borderRadius: 6,
  fontSize: 13,
  background: 'var(--ppt-bg-surface, #fff)',
  boxSizing: 'border-box',
};

const errorTextStyle: CSSProperties = {
  display: 'block',
  marginTop: 4,
  fontSize: 12,
  color: '#b91c1c',
};

interface AnnFormProps {
  initialValues?: AnnFormValues;
  onSubmit: (values: AnnFormValues) => void;
  onCancel: () => void;
  isSubmitting: boolean;
  mode: 'create' | 'edit';
}

export function AnnForm({ initialValues, onSubmit, onCancel, isSubmitting, mode }: AnnFormProps) {
  const { t } = useTranslation();
  const [values, setValues] = useState<AnnFormValues>(initialValues ?? EMPTY_FORM);
  const [errors, setErrors] = useState<AnnFormFieldErrors>({});
  const formId = useId();

  const set = (patch: Partial<AnnFormValues>) => {
    setValues((v) => ({ ...v, ...patch }));
    setErrors((e) => {
      const next = { ...e };
      for (const k of Object.keys(patch)) delete next[k as keyof AnnFormValues];
      return next;
    });
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const fieldErrors = validateAnnForm(values);
    if (Object.keys(fieldErrors).length > 0) {
      setErrors(fieldErrors);
      return;
    }
    onSubmit(values);
  };

  const renderError = (key: keyof AnnFormValues) =>
    errors[key] ? (
      <span role="alert" style={errorTextStyle}>
        {errors[key]}
      </span>
    ) : null;

  return (
    <form
      id={formId}
      onSubmit={handleSubmit}
      noValidate
      style={{ display: 'flex', flexDirection: 'column', gap: 14 }}
    >
      {/* Preview */}
      <div>
        <div style={{ ...labelStyle, marginBottom: 6 }}>
          {t('admin.announcements.form.preview', 'Banner preview')}
        </div>
        <BannerPreview
          title={values.title}
          message={values.message}
          severity={values.severity}
          isDismissible={values.is_dismissible}
        />
      </div>

      {/* Title */}
      <div>
        <label htmlFor={`${formId}-title`} style={labelStyle}>
          {t('admin.announcements.form.title', 'Title')} *
        </label>
        <input
          id={`${formId}-title`}
          type="text"
          maxLength={200}
          value={values.title}
          onChange={(e) => set({ title: e.target.value })}
          style={inputStyle}
          aria-invalid={!!errors.title}
          placeholder={t(
            'admin.announcements.form.titlePlaceholder',
            'Short headline visible in the banner'
          )}
        />
        {renderError('title')}
      </div>

      {/* Message */}
      <div>
        <label htmlFor={`${formId}-message`} style={labelStyle}>
          {t('admin.announcements.form.message', 'Message')} *
        </label>
        <textarea
          id={`${formId}-message`}
          rows={3}
          maxLength={2000}
          value={values.message}
          onChange={(e) => set({ message: e.target.value })}
          style={{ ...inputStyle, resize: 'vertical' }}
          aria-invalid={!!errors.message}
          placeholder={t(
            'admin.announcements.form.messagePlaceholder',
            'Detailed description shown below the title'
          )}
        />
        {renderError('message')}
      </div>

      {/* Severity */}
      <div>
        <label htmlFor={`${formId}-severity`} style={labelStyle}>
          {t('admin.announcements.form.severity', 'Severity')}
        </label>
        <select
          id={`${formId}-severity`}
          value={values.severity}
          onChange={(e) => set({ severity: e.target.value as SystemAnnouncementSeverity })}
          style={inputStyle}
        >
          <option value="info">{t('admin.announcements.severity.info', 'Info')}</option>
          <option value="warning">{t('admin.announcements.severity.warning', 'Warning')}</option>
          <option value="critical">{t('admin.announcements.severity.critical', 'Critical')}</option>
        </select>
      </div>

      {/* Dates */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <div>
          <label htmlFor={`${formId}-start`} style={labelStyle}>
            {t('admin.announcements.form.startAt', 'Publish at')} *
          </label>
          <input
            id={`${formId}-start`}
            type="datetime-local"
            value={values.start_at}
            onChange={(e) => set({ start_at: e.target.value })}
            style={inputStyle}
            aria-invalid={!!errors.start_at}
          />
          {renderError('start_at')}
        </div>
        <div>
          <label htmlFor={`${formId}-end`} style={labelStyle}>
            {t('admin.announcements.form.endAt', 'Expire at')}
            <span style={{ fontWeight: 400, opacity: 0.6 }}>
              {' '}
              ({t('admin.announcements.form.endAtOptional', 'optional')})
            </span>
          </label>
          <input
            id={`${formId}-end`}
            type="datetime-local"
            value={values.end_at}
            onChange={(e) => set({ end_at: e.target.value })}
            style={inputStyle}
            aria-invalid={!!errors.end_at}
          />
          {renderError('end_at')}
        </div>
      </div>

      {/* Flags */}
      <div style={{ display: 'flex', gap: 20 }}>
        <label
          style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, cursor: 'pointer' }}
        >
          <input
            type="checkbox"
            checked={values.is_dismissible}
            onChange={(e) => set({ is_dismissible: e.target.checked })}
          />
          {t('admin.announcements.form.isDismissible', 'Users can dismiss')}
        </label>
        <label
          style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, cursor: 'pointer' }}
        >
          <input
            type="checkbox"
            checked={values.requires_acknowledgment}
            onChange={(e) => set({ requires_acknowledgment: e.target.checked })}
          />
          {t('admin.announcements.form.requiresAck', 'Requires acknowledgment')}
        </label>
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
        <button
          type="button"
          onClick={onCancel}
          style={{
            padding: '7px 16px',
            border: '1px solid var(--ppt-border-default, #d1d5db)',
            borderRadius: 6,
            background: 'var(--ppt-bg-surface, #fff)',
            cursor: 'pointer',
            fontSize: 13,
          }}
        >
          {t('common.cancel', 'Cancel')}
        </button>
        <button
          type="submit"
          form={formId}
          disabled={isSubmitting}
          style={{
            padding: '7px 16px',
            border: 'none',
            borderRadius: 6,
            background: 'var(--ppt-brand-600, #2563eb)',
            color: '#fff',
            cursor: isSubmitting ? 'not-allowed' : 'pointer',
            fontSize: 13,
            fontWeight: 500,
            opacity: isSubmitting ? 0.6 : 1,
          }}
        >
          {isSubmitting
            ? t('common.saving', 'Saving…')
            : mode === 'create'
              ? t('admin.announcements.form.create', 'Create announcement')
              : t('admin.announcements.form.save', 'Save changes')}
        </button>
      </div>
    </form>
  );
}
