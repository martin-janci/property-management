/**
 * ReadingValidationCard component - card for manager to approve/reject a reading.
 * Meters feature: Self-readings and consumption tracking.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { MeterReading, ValidationResult } from '../types';

interface ReadingValidationCardProps {
  reading: MeterReading;
  previousReading?: number;
  onValidate: (result: ValidationResult) => void;
  isProcessing?: boolean;
}

const statusColors: Record<string, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  validated: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  corrected: 'bg-blue-100 text-blue-800',
};

export function ReadingValidationCard({
  reading,
  previousReading,
  onValidate,
  isProcessing,
}: ReadingValidationCardProps) {
  const { t, i18n } = useTranslation();
  const [showRejectForm, setShowRejectForm] = useState(false);
  const [showCorrectForm, setShowCorrectForm] = useState(false);
  const [rejectionReason, setRejectionReason] = useState('');
  const [correctedValue, setCorrectedValue] = useState(reading.value);
  const [correctionNotes, setCorrectionNotes] = useState('');

  const consumption = previousReading !== undefined ? reading.value - previousReading : null;

  const handleApprove = () => {
    onValidate({
      readingId: reading.id,
      status: 'validated',
    });
  };

  const handleReject = () => {
    if (!rejectionReason.trim()) return;
    onValidate({
      readingId: reading.id,
      status: 'rejected',
      rejectionReason: rejectionReason.trim(),
    });
    setShowRejectForm(false);
    setRejectionReason('');
  };

  const handleCorrect = () => {
    if (correctedValue === reading.value) {
      handleApprove();
      return;
    }
    onValidate({
      readingId: reading.id,
      status: 'corrected',
      correctedValue,
      notes: correctionNotes.trim() || undefined,
    });
    setShowCorrectForm(false);
    setCorrectionNotes('');
  };

  const isPending = reading.status === 'pending';

  return (
    <div className="bg-white rounded-lg shadow p-4 border-l-4 border-yellow-400">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-2">
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[reading.status]}`}
            >
              {t(`meters.status.${reading.status}`)}
            </span>
            <span className="text-sm text-gray-500">{t(`meters.types.${reading.meterType}`)}</span>
          </div>

          <div className="mb-3">
            <p className="text-sm text-gray-600">
              {t('meters.serialNumber')}: {reading.meterSerialNumber}
            </p>
            <p className="text-2xl font-bold text-gray-900">
              {reading.value.toLocaleString()} {reading.meterUnit}
            </p>
            {consumption !== null && (
              <p className="text-sm text-gray-600">
                {t('meters.consumption')}: {consumption.toLocaleString()} {reading.meterUnit}
              </p>
            )}
          </div>

          <div className="grid grid-cols-2 gap-2 text-sm">
            <div>
              <span className="text-gray-500">{t('meters.readingDate')}:</span>
              <p className="font-medium">
                {new Date(reading.readingDate).toLocaleDateString(i18n.language)}
              </p>
            </div>
            <div>
              <span className="text-gray-500">{t('meters.submittedBy')}:</span>
              <p className="font-medium">{reading.submittedByName || t('common.unknown')}</p>
            </div>
            <div>
              <span className="text-gray-500">{t('meters.submittedAt')}:</span>
              <p className="font-medium">
                {new Date(reading.submittedAt).toLocaleString(i18n.language)}
              </p>
            </div>
          </div>

          {reading.photoUrl && (
            <div className="mt-3">
              <img
                src={reading.photoUrl}
                alt={t('meters.readingPhoto')}
                className="w-24 h-24 object-cover rounded-lg border cursor-pointer hover:opacity-90"
                onClick={() => window.open(reading.photoUrl, '_blank')}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    window.open(reading.photoUrl, '_blank');
                  }
                }}
              />
            </div>
          )}

          {reading.notes && (
            <p className="mt-2 text-sm text-gray-600 bg-gray-50 p-2 rounded">{reading.notes}</p>
          )}
        </div>
      </div>

      {/* Rejection reason if already rejected */}
      {reading.status === 'rejected' && reading.rejectionReason && (
        <div className="mt-3 p-3 bg-red-50 rounded-lg">
          <p className="text-sm font-medium text-red-800">{t('meters.rejectionReason')}:</p>
          <p className="text-sm text-red-700">{reading.rejectionReason}</p>
        </div>
      )}

      {/* Correction info if corrected */}
      {reading.status === 'corrected' && reading.correctedValue !== undefined && (
        <div className="mt-3 p-3 bg-blue-50 rounded-lg">
          <p className="text-sm font-medium text-blue-800">{t('meters.correctedValue')}:</p>
          <p className="text-sm text-blue-700">
            {reading.correctedValue.toLocaleString()} {reading.meterUnit}
          </p>
        </div>
      )}

      {/* Actions for pending readings */}
      {isPending && (
        <>
          {!showRejectForm && !showCorrectForm && (
            <div className="mt-4 flex items-center gap-2 border-t pt-3">
              <button
                type="button"
                onClick={handleApprove}
                disabled={isProcessing}
                className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50"
              >
                {t('meters.approve')}
              </button>
              <button
                type="button"
                onClick={() => setShowCorrectForm(true)}
                disabled={isProcessing}
                className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {t('meters.correct')}
              </button>
              <button
                type="button"
                onClick={() => setShowRejectForm(true)}
                disabled={isProcessing}
                className="px-3 py-1.5 text-sm text-red-600 border border-red-300 rounded-lg hover:bg-red-50 disabled:opacity-50"
              >
                {t('meters.reject')}
              </button>
            </div>
          )}

          {/* Reject Form */}
          {showRejectForm && (
            <div className="mt-4 border-t pt-3">
              <label
                htmlFor="rejectionReason"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('meters.rejectionReason')} *
              </label>
              <textarea
                id="rejectionReason"
                value={rejectionReason}
                onChange={(e) => setRejectionReason(e.target.value)}
                rows={2}
                placeholder={t('meters.rejectionReasonPlaceholder')}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-red-500"
              />
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={handleReject}
                  disabled={isProcessing || !rejectionReason.trim()}
                  className="px-3 py-1.5 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
                >
                  {t('meters.confirmReject')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowRejectForm(false);
                    setRejectionReason('');
                  }}
                  disabled={isProcessing}
                  className="px-3 py-1.5 text-sm text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
              </div>
            </div>
          )}

          {/* Correct Form */}
          {showCorrectForm && (
            <div className="mt-4 border-t pt-3">
              <label
                htmlFor="correctedValue"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('meters.correctedValue')} ({reading.meterUnit}) *
              </label>
              <input
                type="number"
                id="correctedValue"
                value={correctedValue}
                onChange={(e) => setCorrectedValue(Number.parseFloat(e.target.value) || 0)}
                step="0.01"
                min="0"
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <label
                htmlFor="correctionNotes"
                className="block text-sm font-medium text-gray-700 mt-2 mb-1"
              >
                {t('meters.form.notes')} ({t('common.optional')})
              </label>
              <textarea
                id="correctionNotes"
                value={correctionNotes}
                onChange={(e) => setCorrectionNotes(e.target.value)}
                rows={2}
                placeholder={t('meters.correctionNotesPlaceholder')}
                className="w-full rounded-md border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <div className="mt-2 flex gap-2">
                <button
                  type="button"
                  onClick={handleCorrect}
                  disabled={isProcessing}
                  className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                >
                  {t('meters.confirmCorrection')}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCorrectForm(false);
                    setCorrectedValue(reading.value);
                    setCorrectionNotes('');
                  }}
                  disabled={isProcessing}
                  className="px-3 py-1.5 text-sm text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50"
                >
                  {t('common.cancel')}
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
