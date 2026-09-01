/**
 * EditReadingPage - page to edit an existing meter reading.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { EditReadingFormData, Meter, MeterReading } from '../types';

interface EditReadingPageProps {
  meter: Meter;
  reading: MeterReading;
  isSubmitting?: boolean;
  isLoading?: boolean;
  onSubmit: (data: EditReadingFormData) => void;
  onCancel: () => void;
}

export function EditReadingPage({
  meter,
  reading,
  isSubmitting,
  isLoading,
  onSubmit,
  onCancel,
}: EditReadingPageProps) {
  const { t, i18n } = useTranslation();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [formData, setFormData] = useState<EditReadingFormData>({
    readingId: reading.id,
    meterId: meter.id,
    value: reading.value,
    readingDate: reading.readingDate.split('T')[0],
    notes: reading.notes || '',
    reason: '',
    previousValue: reading.value,
  });
  const [photoPreview, setPhotoPreview] = useState<string | null>(reading.photoUrl || null);
  const [errors, setErrors] = useState<Partial<Record<keyof EditReadingFormData, string>>>({});

  if (isLoading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600" />
      </div>
    );
  }

  const validate = (): boolean => {
    const newErrors: Partial<Record<keyof EditReadingFormData, string>> = {};

    if (formData.value < 0) {
      newErrors.value = t('meters.form.errors.valueNegative');
    }

    if (!formData.readingDate) {
      newErrors.readingDate = t('meters.form.errors.dateRequired');
    }

    const readingDate = new Date(formData.readingDate);
    const today = new Date();
    if (readingDate > today) {
      newErrors.readingDate = t('meters.form.errors.futureDate');
    }

    if (!formData.reason || formData.reason.trim().length === 0) {
      newErrors.reason = t('meters.edit.errors.reasonRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmit(formData);
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value, type } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: type === 'number' ? Number.parseFloat(value) || 0 : value,
    }));
    if (errors[name as keyof EditReadingFormData]) {
      setErrors((prev) => ({ ...prev, [name]: undefined }));
    }
  };

  const handlePhotoChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setFormData((prev) => ({ ...prev, photo: file }));
      const reader = new FileReader();
      reader.onloadend = () => {
        setPhotoPreview(reader.result as string);
      };
      reader.readAsDataURL(file);
    }
  };

  const handleRemovePhoto = () => {
    setFormData((prev) => ({ ...prev, photo: undefined }));
    setPhotoPreview(null);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const valueChanged = formData.value !== reading.value;
  const valueDifference = formData.value - reading.value;

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <button
          type="button"
          onClick={onCancel}
          className="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 mb-4"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('meters.backToMeter')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('meters.edit.title')}</h1>
        <p className="text-gray-600 mt-1">
          {t(`meters.types.${meter.meterType}`)} - {meter.serialNumber}
        </p>
      </div>

      {/* Form */}
      <div className="bg-white rounded-lg shadow p-6">
        <form onSubmit={handleSubmit} className="space-y-6">
          {/* Previous Value Reference */}
          <div className="bg-amber-50 rounded-lg p-4 border border-amber-200">
            <h3 className="text-sm font-medium text-amber-800 mb-2">
              {t('meters.edit.previousValue')}
            </h3>
            <p className="text-2xl font-bold text-amber-900">
              {reading.value.toLocaleString()} {meter.unit}
            </p>
            <p className="text-sm text-amber-700 mt-1">
              {t('meters.edit.recordedOn')}{' '}
              {new Date(reading.readingDate).toLocaleDateString(i18n.language)}
            </p>
          </div>

          {/* Reading Value */}
          <div>
            <label htmlFor="value" className="block text-sm font-medium text-gray-700">
              {t('meters.edit.newValue')} ({meter.unit}) *
            </label>
            <input
              type="number"
              id="value"
              name="value"
              value={formData.value}
              onChange={handleChange}
              step="0.01"
              min="0"
              className={`mt-1 block w-full rounded-md border ${
                errors.value ? 'border-red-500' : 'border-gray-300'
              } px-3 py-2 text-lg font-semibold focus:outline-none focus:ring-2 focus:ring-blue-500`}
            />
            {errors.value && <p className="mt-1 text-sm text-red-500">{errors.value}</p>}
            {valueChanged && (
              <p
                className={`mt-1 text-sm ${valueDifference > 0 ? 'text-green-600' : 'text-red-600'}`}
              >
                {t('meters.edit.difference')}: {valueDifference > 0 ? '+' : ''}
                {valueDifference.toLocaleString()} {meter.unit}
              </p>
            )}
          </div>

          {/* Reading Date */}
          <div>
            <label htmlFor="readingDate" className="block text-sm font-medium text-gray-700">
              {t('meters.form.readingDate')} *
            </label>
            <input
              type="date"
              id="readingDate"
              name="readingDate"
              value={formData.readingDate}
              onChange={handleChange}
              max={new Date().toISOString().split('T')[0]}
              className={`mt-1 block w-full rounded-md border ${
                errors.readingDate ? 'border-red-500' : 'border-gray-300'
              } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
            />
            {errors.readingDate && (
              <p className="mt-1 text-sm text-red-500">{errors.readingDate}</p>
            )}
          </div>

          {/* Reason for Edit */}
          <div>
            <label htmlFor="reason" className="block text-sm font-medium text-gray-700">
              {t('meters.edit.reason')} *
            </label>
            <textarea
              id="reason"
              name="reason"
              value={formData.reason || ''}
              onChange={handleChange}
              rows={3}
              placeholder={t('meters.edit.reasonPlaceholder')}
              className={`mt-1 block w-full rounded-md border ${
                errors.reason ? 'border-red-500' : 'border-gray-300'
              } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
            />
            {errors.reason && <p className="mt-1 text-sm text-red-500">{errors.reason}</p>}
            <p className="mt-1 text-xs text-gray-500">{t('meters.edit.reasonHelp')}</p>
          </div>

          {/* Photo Upload */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              {t('meters.form.photo')} ({t('common.optional')})
            </label>
            {photoPreview ? (
              <div className="relative inline-block">
                <img
                  src={photoPreview}
                  alt={t('meters.form.photoPreview')}
                  className="w-48 h-48 object-cover rounded-lg border"
                />
                <button
                  type="button"
                  onClick={handleRemovePhoto}
                  className="absolute top-1 right-1 p-1 bg-red-500 text-white rounded-full hover:bg-red-600"
                  aria-label={t('common.delete')}
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </button>
              </div>
            ) : (
              <label className="flex flex-col items-center justify-center w-full h-32 border-2 border-gray-300 border-dashed rounded-lg cursor-pointer hover:bg-gray-50">
                <div className="flex flex-col items-center justify-center pt-5 pb-6">
                  <svg
                    className="w-8 h-8 mb-2 text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z"
                    />
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 13a3 3 0 11-6 0 3 3 0 016 0z"
                    />
                  </svg>
                  <p className="text-sm text-gray-500">{t('meters.form.uploadPhoto')}</p>
                </div>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  className="hidden"
                  onChange={handlePhotoChange}
                />
              </label>
            )}
            <p className="mt-1 text-xs text-gray-500">{t('meters.form.photoHelp')}</p>
          </div>

          {/* Notes */}
          <div>
            <label htmlFor="notes" className="block text-sm font-medium text-gray-700">
              {t('meters.form.notes')} ({t('common.optional')})
            </label>
            <textarea
              id="notes"
              name="notes"
              value={formData.notes || ''}
              onChange={handleChange}
              rows={3}
              placeholder={t('meters.form.notesPlaceholder')}
              className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          {/* Actions */}
          <div className="flex items-center justify-end gap-3 pt-4 border-t">
            <button
              type="button"
              onClick={onCancel}
              disabled={isSubmitting}
              className="px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
            >
              {isSubmitting && (
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
              )}
              {isSubmitting ? t('common.saving') : t('common.save')}
            </button>
          </div>
        </form>
      </div>

      {/* Warning */}
      <div className="mt-6 bg-gray-50 rounded-lg p-4 border border-gray-200">
        <div className="flex items-start gap-3">
          <svg
            className="w-5 h-5 text-gray-500 mt-0.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <div>
            <h3 className="text-sm font-medium text-gray-700">{t('meters.edit.noteTitle')}</h3>
            <p className="text-sm text-gray-600 mt-1">{t('meters.edit.noteDescription')}</p>
          </div>
        </div>
      </div>
    </div>
  );
}
