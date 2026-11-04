/**
 * CreateViolationPage - Report a new lease violation.
 * UC-34: Lease Violations Tracking
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CreateViolationData, LeaseSummary, ViolationSeverity, ViolationType } from '../types';

interface CreateViolationPageProps {
  leases: LeaseSummary[];
  isLoading?: boolean;
  isSubmitting?: boolean;
  onBack: () => void;
  onSubmit: (data: CreateViolationData, evidence: FileList | null) => void;
}

const violationTypes: ViolationType[] = [
  'noise',
  'damage',
  'unauthorized_occupant',
  'late_payment',
  'pet_policy',
  'parking',
  'cleanliness',
  'illegal_activity',
  'lease_terms',
  'other',
];

const violationSeverities: ViolationSeverity[] = ['minor', 'moderate', 'severe'];

interface FormErrors {
  leaseId?: string;
  violationType?: string;
  severity?: string;
  description?: string;
  violationDate?: string;
}

export function CreateViolationPage({
  leases,
  isLoading,
  isSubmitting,
  onBack,
  onSubmit,
}: CreateViolationPageProps) {
  const { t } = useTranslation();

  const [leaseId, setLeaseId] = useState('');
  const [violationType, setViolationType] = useState<ViolationType | ''>('');
  const [severity, setSeverity] = useState<ViolationSeverity | ''>('');
  const [description, setDescription] = useState('');
  const [violationDate, setViolationDate] = useState('');
  const [evidence, setEvidence] = useState<FileList | null>(null);
  const [errors, setErrors] = useState<FormErrors>({});

  const selectedLease = leases.find((l) => l.id === leaseId);

  const validateForm = (): boolean => {
    const newErrors: FormErrors = {};

    if (!leaseId) {
      newErrors.leaseId = t('leases.violations.form.errors.leaseRequired');
    }
    if (!violationType) {
      newErrors.violationType = t('leases.violations.form.errors.typeRequired');
    }
    if (!severity) {
      newErrors.severity = t('leases.violations.form.errors.severityRequired');
    }
    if (!description.trim()) {
      newErrors.description = t('leases.violations.form.errors.descriptionRequired');
    } else if (description.length < 10) {
      newErrors.description = t('leases.violations.form.errors.descriptionTooShort');
    }
    if (!violationDate) {
      newErrors.violationDate = t('leases.violations.form.errors.dateRequired');
    } else {
      const date = new Date(violationDate);
      const today = new Date();
      today.setHours(23, 59, 59, 999);
      if (date > today) {
        newErrors.violationDate = t('leases.violations.form.errors.dateInFuture');
      }
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!validateForm() || !violationType || !severity) return;

    const data: CreateViolationData = {
      leaseId,
      violationType,
      severity,
      description: description.trim(),
      violationDate,
    };

    onSubmit(data, evidence);
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setEvidence(e.target.files);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto px-4 py-8">
      {/* Back Button */}
      <button
        type="button"
        onClick={onBack}
        className="flex items-center gap-2 text-gray-600 hover:text-gray-800 mb-6"
      >
        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        {t('leases.violations.backToViolations')}
      </button>

      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">{t('leases.violations.createTitle')}</h1>
        <p className="text-gray-600 mt-1">{t('leases.violations.createSubtitle')}</p>
      </div>

      {/* Form */}
      <form onSubmit={handleSubmit} className="bg-white rounded-lg shadow p-6 space-y-6">
        {/* Lease Selector */}
        <div>
          <label htmlFor="lease" className="block text-sm font-medium text-gray-700 mb-1">
            {t('leases.violations.form.lease')} <span className="text-red-500">*</span>
          </label>
          <select
            id="lease"
            value={leaseId}
            onChange={(e) => setLeaseId(e.target.value)}
            className={`block w-full py-2 px-3 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm ${
              errors.leaseId ? 'border-red-300' : 'border-gray-300'
            }`}
          >
            <option value="">{t('leases.violations.form.selectLease')}</option>
            {leases.map((lease) => (
              <option key={lease.id} value={lease.id}>
                {lease.buildingName} - {lease.unitNumber} ({lease.tenantName})
              </option>
            ))}
          </select>
          {errors.leaseId && <p className="mt-1 text-sm text-red-600">{errors.leaseId}</p>}
          {selectedLease && (
            <div className="mt-2 p-3 bg-gray-50 rounded-md text-sm">
              <p className="font-medium text-gray-900">{selectedLease.tenantName}</p>
              <p className="text-gray-500">{selectedLease.tenantEmail}</p>
            </div>
          )}
        </div>

        {/* Violation Type */}
        <div>
          <label htmlFor="violation-type" className="block text-sm font-medium text-gray-700 mb-1">
            {t('leases.violations.form.violationType')} <span className="text-red-500">*</span>
          </label>
          <select
            id="violation-type"
            value={violationType}
            onChange={(e) => setViolationType(e.target.value as ViolationType)}
            className={`block w-full py-2 px-3 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm ${
              errors.violationType ? 'border-red-300' : 'border-gray-300'
            }`}
          >
            <option value="">{t('leases.violations.form.selectType')}</option>
            {violationTypes.map((type) => (
              <option key={type} value={type}>
                {t(`leases.violations.type.${type}`)}
              </option>
            ))}
          </select>
          {errors.violationType && (
            <p className="mt-1 text-sm text-red-600">{errors.violationType}</p>
          )}
        </div>

        {/* Severity */}
        <div>
          <label htmlFor="severity" className="block text-sm font-medium text-gray-700 mb-1">
            {t('leases.violations.form.severity')} <span className="text-red-500">*</span>
          </label>
          <div className="grid grid-cols-3 gap-3">
            {violationSeverities.map((sev) => (
              <button
                key={sev}
                type="button"
                onClick={() => setSeverity(sev)}
                className={`py-3 px-4 text-sm font-medium rounded-md border ${
                  severity === sev
                    ? sev === 'minor'
                      ? 'bg-blue-100 border-blue-500 text-blue-800'
                      : sev === 'moderate'
                        ? 'bg-orange-100 border-orange-500 text-orange-800'
                        : 'bg-red-100 border-red-500 text-red-800'
                    : 'bg-white border-gray-300 text-gray-700 hover:bg-gray-50'
                }`}
              >
                {t(`leases.violations.severity.${sev}`)}
              </button>
            ))}
          </div>
          {errors.severity && <p className="mt-1 text-sm text-red-600">{errors.severity}</p>}
        </div>

        {/* Violation Date */}
        <div>
          <label htmlFor="violation-date" className="block text-sm font-medium text-gray-700 mb-1">
            {t('leases.violations.form.violationDate')} <span className="text-red-500">*</span>
          </label>
          <input
            id="violation-date"
            type="date"
            value={violationDate}
            onChange={(e) => setViolationDate(e.target.value)}
            max={new Date().toISOString().split('T')[0]}
            className={`block w-full py-2 px-3 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm ${
              errors.violationDate ? 'border-red-300' : 'border-gray-300'
            }`}
          />
          {errors.violationDate && (
            <p className="mt-1 text-sm text-red-600">{errors.violationDate}</p>
          )}
        </div>

        {/* Description */}
        <div>
          <label htmlFor="description" className="block text-sm font-medium text-gray-700 mb-1">
            {t('leases.violations.form.description')} <span className="text-red-500">*</span>
          </label>
          <textarea
            id="description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            rows={4}
            placeholder={t('leases.violations.form.descriptionPlaceholder')}
            className={`block w-full py-2 px-3 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm ${
              errors.description ? 'border-red-300' : 'border-gray-300'
            }`}
          />
          {errors.description && <p className="mt-1 text-sm text-red-600">{errors.description}</p>}
          <p className="mt-1 text-xs text-gray-500">
            {t('leases.violations.form.characters', { count: description.length })}
          </p>
        </div>

        {/* Evidence Upload */}
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">
            {t('leases.violations.form.evidence')}
          </label>
          <div className="mt-1 flex justify-center px-6 pt-5 pb-6 border-2 border-gray-300 border-dashed rounded-md">
            <div className="space-y-1 text-center">
              <svg
                className="mx-auto h-12 w-12 text-gray-400"
                stroke="currentColor"
                fill="none"
                viewBox="0 0 48 48"
              >
                <path
                  d="M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02"
                  strokeWidth={2}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
              <div className="flex text-sm text-gray-600">
                <label
                  htmlFor="evidence-upload"
                  className="relative cursor-pointer rounded-md font-medium text-blue-600 hover:text-blue-500 focus-within:outline-none"
                >
                  <span>{t('leases.violations.form.uploadFiles')}</span>
                  <input
                    id="evidence-upload"
                    type="file"
                    multiple
                    accept="image/*,.pdf,.doc,.docx"
                    onChange={handleFileChange}
                    className="sr-only"
                  />
                </label>
                <p className="pl-1">{t('leases.violations.form.orDragDrop')}</p>
              </div>
              <p className="text-xs text-gray-500">{t('leases.violations.form.fileTypesHint')}</p>
            </div>
          </div>
          {evidence && evidence.length > 0 && (
            <div className="mt-2">
              <p className="text-sm text-gray-600">
                {t('leases.violations.form.filesSelected', { count: evidence.length })}
              </p>
              <ul className="mt-1 text-xs text-gray-500">
                {Array.from(evidence).map((file, idx) => (
                  <li key={idx}>{file.name}</li>
                ))}
              </ul>
            </div>
          )}
        </div>

        {/* Submit Button */}
        <div className="flex justify-end gap-3 pt-4 border-t">
          <button
            type="button"
            onClick={onBack}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
          >
            {t('common.cancel')}
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? t('common.saving') : t('leases.violations.form.submit')}
          </button>
        </div>
      </form>
    </div>
  );
}
