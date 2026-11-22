/**
 * ReadingForm component - form to submit a new meter reading.
 * Meters feature: Self-readings and consumption tracking.
 * Epic 128: OCR Meter Preview integration.
 */

import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import { useOcrMeterReading } from '../hooks';
import type { Meter, ReadingFormData } from '../types';
import { type OcrCorrection, OcrPreviewCard, type OcrResult } from './OcrPreviewCard';

interface ReadingFormProps {
  meter: Meter;
  isSubmitting?: boolean;
  enableOcr?: boolean;
  onSubmit: (data: ReadingFormData) => void;
  onCancel: () => void;
}

export function ReadingForm({
  meter,
  isSubmitting,
  enableOcr = false,
  onSubmit,
  onCancel,
}: ReadingFormProps) {
  const { t } = useTranslation();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [formData, setFormData] = useState<Omit<ReadingFormData, 'value'>>({
    meterId: meter.id,
    readingDate: new Date().toISOString().split('T')[0],
    notes: '',
  });
  // The reading value is kept as the raw input string so an empty or invalid
  // entry can be told apart from a genuine `0`. A controlled number input that
  // stored `Number.parseFloat(value) || 0` would silently turn a blank field
  // into 0 and submit it. We only coerce to a number at validation/submit time.
  const [valueInput, setValueInput] = useState<string>(
    meter.lastReadingValue !== undefined ? String(meter.lastReadingValue) : ''
  );
  const [photoPreview, setPhotoPreview] = useState<string | null>(null);
  const [errors, setErrors] = useState<Partial<Record<keyof ReadingFormData, string>>>({});

  // OCR state
  const [ocrResult, setOcrResult] = useState<OcrResult | null>(null);
  const [ocrImageUrl, setOcrImageUrl] = useState<string | null>(null);
  const [showOcrPreview, setShowOcrPreview] = useState(false);

  const {
    process: processOcr,
    correctAndSubmit: submitOcrCorrection,
    isProcessing: isOcrProcessing,
    processingError: ocrError,
    reset: resetOcr,
  } = useOcrMeterReading(meter.id);

  // A trimmed-empty field parses to NaN so it is treated as "missing", never 0.
  const parsedValue = valueInput.trim() === '' ? Number.NaN : Number(valueInput);

  const validate = (): boolean => {
    const newErrors: Partial<Record<keyof ReadingFormData, string>> = {};

    if (Number.isNaN(parsedValue)) {
      // Empty or non-numeric input — do not silently coerce to 0.
      newErrors.value = t('meters.form.errors.valueRequired');
    } else if (parsedValue < 0) {
      newErrors.value = t('meters.form.errors.valueNegative');
    } else if (meter.lastReadingValue !== undefined && parsedValue < meter.lastReadingValue) {
      newErrors.value = t('meters.form.errors.valueLowerThanLast');
    }

    if (!formData.readingDate) {
      newErrors.readingDate = t('meters.form.errors.dateRequired');
    }

    const readingDate = new Date(formData.readingDate);
    const today = new Date();
    if (readingDate > today) {
      newErrors.readingDate = t('meters.form.errors.futureDate');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmit({ ...formData, value: parsedValue });
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
    if (errors[name as keyof ReadingFormData]) {
      setErrors((prev) => ({ ...prev, [name]: undefined }));
    }
  };

  const handleValueChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    // Store the raw string; empty/invalid is preserved rather than coerced to 0.
    setValueInput(e.target.value);
    if (errors.value) {
      setErrors((prev) => ({ ...prev, value: undefined }));
    }
  };

  const handlePhotoChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setFormData((prev) => ({ ...prev, photo: file }));
      const reader = new FileReader();
      reader.onloadend = () => {
        setPhotoPreview(reader.result as string);
      };
      reader.readAsDataURL(file);

      // If OCR is enabled, process the image
      if (enableOcr) {
        try {
          const result = await processOcr(file);
          setOcrResult(result.result);
          setOcrImageUrl(result.imageUrl);
          setShowOcrPreview(true);
        } catch {
          // OCR failed, user can still enter value manually
          console.error('OCR processing failed');
        }
      }
    }
  };

  const handleRemovePhoto = () => {
    setFormData((prev) => ({ ...prev, photo: undefined }));
    setPhotoPreview(null);
    setOcrResult(null);
    setOcrImageUrl(null);
    setShowOcrPreview(false);
    resetOcr();
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleOcrAccept = (value: number) => {
    setValueInput(String(value));
    setShowOcrPreview(false);
  };

  const handleOcrCorrect = (correction: OcrCorrection) => {
    submitOcrCorrection(correction);
  };

  const handleOcrRetake = () => {
    setOcrResult(null);
    setOcrImageUrl(null);
    setShowOcrPreview(false);
    setPhotoPreview(null);
    setFormData((prev) => ({ ...prev, photo: undefined }));
    resetOcr();
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
      fileInputRef.current.click();
    }
  };

  const consumption =
    meter.lastReadingValue !== undefined && !Number.isNaN(parsedValue)
      ? parsedValue - meter.lastReadingValue
      : null;

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Meter Info */}
      <div className="bg-gray-50 rounded-lg p-4">
        <h3 className="text-sm font-medium text-gray-700 mb-2">{t('meters.meterInfo')}</h3>
        <dl className="grid grid-cols-2 gap-2 text-sm">
          <dt className="text-gray-500">{t('meters.type')}</dt>
          <dd className="font-medium">{t(`meters.types.${meter.meterType}`)}</dd>
          <dt className="text-gray-500">{t('meters.serialNumber')}</dt>
          <dd className="font-medium">{meter.serialNumber}</dd>
          {meter.lastReadingValue !== undefined && (
            <>
              <dt className="text-gray-500">{t('meters.lastReading')}</dt>
              <dd className="font-medium">
                {meter.lastReadingValue.toLocaleString(getActiveLocale())} {meter.unit}
              </dd>
            </>
          )}
        </dl>
      </div>

      {/* Reading Value */}
      <div>
        <label htmlFor="value" className="block text-sm font-medium text-gray-700">
          {t('meters.form.currentReading')} ({meter.unit}) *
        </label>
        <input
          type="number"
          id="value"
          name="value"
          value={valueInput}
          onChange={handleValueChange}
          step="0.01"
          min="0"
          inputMode="decimal"
          className={`mt-1 block w-full rounded-md border ${
            errors.value ? 'border-red-500' : 'border-gray-300'
          } px-3 py-2 text-lg font-semibold focus:outline-none focus:ring-2 focus:ring-blue-500`}
        />
        {errors.value && <p className="mt-1 text-sm text-red-500">{errors.value}</p>}
        {consumption !== null && consumption >= 0 && (
          <p className="mt-1 text-sm text-gray-600">
            {t('meters.form.consumption')}: {consumption.toLocaleString(getActiveLocale())}{' '}
            {meter.unit}
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
        {errors.readingDate && <p className="mt-1 text-sm text-red-500">{errors.readingDate}</p>}
      </div>

      {/* OCR Preview Modal */}
      {showOcrPreview && ocrResult && ocrImageUrl && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="max-w-md w-full">
            <OcrPreviewCard
              imageUrl={ocrImageUrl}
              ocrResult={ocrResult}
              meterUnit={meter.unit}
              lastReadingValue={meter.lastReadingValue}
              onAccept={handleOcrAccept}
              onCorrect={handleOcrCorrect}
              onRetake={handleOcrRetake}
              isLoading={isOcrProcessing}
            />
          </div>
        </div>
      )}

      {/* Photo Upload */}
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-2">
          {t('meters.form.photo')} ({t('common.optional')})
          {enableOcr && (
            <span className="ml-2 text-xs text-blue-600 font-normal">
              {t('meters.ocr.ocrEnabled')}
            </span>
          )}
        </label>

        {/* OCR Processing Indicator */}
        {isOcrProcessing && (
          <div className="flex items-center gap-2 p-3 mb-2 bg-blue-50 border border-blue-200 rounded-lg">
            <div className="w-5 h-5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />
            <span className="text-sm text-blue-700">{t('meters.ocr.processing')}</span>
          </div>
        )}

        {/* OCR Error */}
        {ocrError && (
          <div className="p-3 mb-2 bg-red-50 border border-red-200 rounded-lg text-sm text-red-700">
            {t('meters.ocr.processingFailed')}
          </div>
        )}

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
              <p className="text-sm text-gray-500">
                {enableOcr ? t('meters.ocr.uploadForOcr') : t('meters.form.uploadPhoto')}
              </p>
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
          {isSubmitting ? t('common.saving') : t('meters.form.submitReading')}
        </button>
      </div>
    </form>
  );
}
