/**
 * OcrPreviewCard component
 * Epic 128: OCR Meter Preview
 *
 * Displays OCR-extracted meter reading value for user verification.
 * Shows the image with bounding box around detected digits.
 */

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';

/** OCR detection result from backend */
export interface OcrResult {
  extractedValue: number;
  confidence: number;
  boundingBox: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  rawText: string;
  processingTimeMs: number;
}

/** OCR correction feedback for model training */
export interface OcrCorrection {
  originalValue: number;
  correctedValue: number;
  imageUrl: string;
  boundingBox?: OcrResult['boundingBox'];
  timestamp: string;
}

interface OcrPreviewCardProps {
  imageUrl: string;
  ocrResult: OcrResult;
  meterUnit: string;
  lastReadingValue?: number;
  onAccept: (value: number) => void;
  onCorrect: (correction: OcrCorrection) => void;
  onRetake: () => void;
  isLoading?: boolean;
}

function getConfidenceColor(confidence: number): string {
  if (confidence >= 0.9) return 'text-green-600 bg-green-100';
  if (confidence >= 0.7) return 'text-yellow-600 bg-yellow-100';
  return 'text-red-600 bg-red-100';
}

function getConfidenceLabel(confidence: number, t: (key: string) => string): string {
  if (confidence >= 0.9) return t('meters.ocr.confidenceHigh');
  if (confidence >= 0.7) return t('meters.ocr.confidenceMedium');
  return t('meters.ocr.confidenceLow');
}

export function OcrPreviewCard({
  imageUrl,
  ocrResult,
  meterUnit,
  lastReadingValue,
  onAccept,
  onCorrect,
  onRetake,
  isLoading = false,
}: OcrPreviewCardProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [correctedValue, setCorrectedValue] = useState<string>(ocrResult.extractedValue.toString());

  const handleAccept = useCallback(() => {
    if (isEditing) {
      const numValue = Number.parseFloat(correctedValue);
      if (!Number.isNaN(numValue) && numValue !== ocrResult.extractedValue) {
        onCorrect({
          originalValue: ocrResult.extractedValue,
          correctedValue: numValue,
          imageUrl,
          boundingBox: ocrResult.boundingBox,
          timestamp: new Date().toISOString(),
        });
        onAccept(numValue);
      } else {
        onAccept(ocrResult.extractedValue);
      }
    } else {
      onAccept(ocrResult.extractedValue);
    }
  }, [isEditing, correctedValue, ocrResult, imageUrl, onCorrect, onAccept]);

  const handleEditToggle = () => {
    setIsEditing(!isEditing);
    if (!isEditing) {
      setCorrectedValue(ocrResult.extractedValue.toString());
    }
  };

  const confidenceColor = getConfidenceColor(ocrResult.confidence);
  const confidenceLabel = getConfidenceLabel(ocrResult.confidence, t);
  const confidencePercent = Math.round(ocrResult.confidence * 100);

  // Calculate bounding box overlay position
  const { x, y, width, height } = ocrResult.boundingBox;

  // Check if value seems valid compared to last reading
  const valueDiff =
    lastReadingValue !== undefined ? ocrResult.extractedValue - lastReadingValue : null;
  const hasUnusualChange =
    valueDiff !== null &&
    lastReadingValue !== undefined &&
    (valueDiff < 0 || valueDiff > lastReadingValue * 0.5);

  return (
    <div className="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
      {/* Header */}
      <div className="px-4 py-3 bg-gray-50 border-b border-gray-200">
        <div className="flex items-center justify-between">
          <h3 className="font-medium text-gray-900">{t('meters.ocr.previewTitle')}</h3>
          <span className={`px-2 py-1 text-xs font-medium rounded-full ${confidenceColor}`}>
            {confidenceLabel} ({confidencePercent}%)
          </span>
        </div>
      </div>

      {/* Image with bounding box overlay */}
      <div className="relative">
        <img src={imageUrl} alt={t('meters.ocr.meterPhoto')} className="w-full h-auto" />
        {/* Bounding box overlay */}
        <div
          className="absolute border-2 border-blue-500 bg-blue-500/10 pointer-events-none"
          style={{
            left: `${x}%`,
            top: `${y}%`,
            width: `${width}%`,
            height: `${height}%`,
          }}
          aria-hidden="true"
        >
          <div className="absolute -top-6 left-0 bg-blue-500 text-white text-xs px-2 py-0.5 rounded">
            {t('meters.ocr.detectedArea')}
          </div>
        </div>
      </div>

      {/* Extracted value display */}
      <div className="p-4 space-y-4">
        <div className="text-center">
          <p className="text-sm text-gray-500 mb-1">{t('meters.ocr.extractedValue')}</p>
          {isEditing ? (
            <div className="flex items-center justify-center gap-2">
              <input
                type="number"
                value={correctedValue}
                onChange={(e) => setCorrectedValue(e.target.value)}
                step="0.01"
                min="0"
                className="w-40 text-center text-3xl font-bold border-2 border-blue-500 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                aria-label={t('meters.ocr.correctedValue')}
              />
              <span className="text-xl text-gray-600">{meterUnit}</span>
            </div>
          ) : (
            <p className="text-3xl font-bold text-gray-900">
              {ocrResult.extractedValue.toLocaleString(getActiveLocale())} {meterUnit}
            </p>
          )}
        </div>

        {/* Warning for unusual changes */}
        {hasUnusualChange && (
          <div
            className="flex items-start gap-2 p-3 bg-amber-50 border border-amber-200 rounded-lg text-sm"
            role="alert"
          >
            <svg
              className="w-5 h-5 text-amber-600 flex-shrink-0 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"
              />
            </svg>
            <div>
              <p className="font-medium text-amber-800">{t('meters.ocr.unusualChange')}</p>
              <p className="text-amber-700">
                {valueDiff !== null && valueDiff < 0
                  ? t('meters.ocr.valueLowerThanLast')
                  : t('meters.ocr.unusuallyHighChange')}
              </p>
            </div>
          </div>
        )}

        {/* Last reading comparison */}
        {lastReadingValue !== undefined && valueDiff !== null && valueDiff >= 0 && (
          <div className="text-sm text-gray-600 text-center">
            <p>
              {t('meters.ocr.lastReading')}: {lastReadingValue.toLocaleString(getActiveLocale())}{' '}
              {meterUnit}
            </p>
            <p className="text-green-600 font-medium">
              {t('meters.ocr.consumption')}: +{valueDiff.toLocaleString(getActiveLocale())}{' '}
              {meterUnit}
            </p>
          </div>
        )}

        {/* Processing info */}
        <div className="text-xs text-gray-400 text-center">
          {t('meters.ocr.processingTime', { ms: ocrResult.processingTimeMs })}
        </div>

        {/* Action buttons */}
        <div className="flex flex-col gap-2 pt-2">
          <button
            type="button"
            onClick={handleAccept}
            disabled={isLoading}
            className="w-full px-4 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 font-medium flex items-center justify-center gap-2"
          >
            {isLoading ? (
              <div className="w-5 h-5 border-2 border-white border-t-transparent rounded-full animate-spin" />
            ) : (
              <svg
                className="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            )}
            {isEditing ? t('meters.ocr.acceptCorrected') : t('meters.ocr.acceptValue')}
          </button>

          <div className="flex gap-2">
            <button
              type="button"
              onClick={handleEditToggle}
              disabled={isLoading}
              className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 disabled:opacity-50 flex items-center justify-center gap-2"
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
                  d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
                />
              </svg>
              {isEditing ? t('common.cancel') : t('meters.ocr.correctValue')}
            </button>

            <button
              type="button"
              onClick={onRetake}
              disabled={isLoading}
              className="flex-1 px-4 py-2 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 disabled:opacity-50 flex items-center justify-center gap-2"
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
                  d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                />
              </svg>
              {t('meters.ocr.retakePhoto')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
