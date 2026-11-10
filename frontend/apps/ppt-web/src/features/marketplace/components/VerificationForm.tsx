/**
 * VerificationForm component - form for submitting verification documents.
 * Epic 68: Service Provider Marketplace (Story 68.4)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { VerificationType } from './VerificationBadge';

export interface VerificationFormData {
  type: VerificationType;
  documentName: string;
  documentNumber?: string;
  issuingAuthority?: string;
  issueDate?: string;
  expiryDate?: string;
  documentUrl?: string;
}

interface VerificationFormProps {
  onSubmit: (data: VerificationFormData) => void;
  onCancel: () => void;
  isLoading?: boolean;
}

const verificationTypes: { value: VerificationType; labelKey: string; descKey: string }[] = [
  {
    value: 'business_registration',
    labelKey: 'marketplace.verificationForm.typeBusinessLabel',
    descKey: 'marketplace.verificationForm.typeBusinessDesc',
  },
  {
    value: 'insurance',
    labelKey: 'marketplace.verificationForm.typeInsuranceLabel',
    descKey: 'marketplace.verificationForm.typeInsuranceDesc',
  },
  {
    value: 'certification',
    labelKey: 'marketplace.verificationForm.typeCertificationLabel',
    descKey: 'marketplace.verificationForm.typeCertificationDesc',
  },
  {
    value: 'license',
    labelKey: 'marketplace.verificationForm.typeLicenseLabel',
    descKey: 'marketplace.verificationForm.typeLicenseDesc',
  },
  {
    value: 'identity',
    labelKey: 'marketplace.verificationForm.typeIdentityLabel',
    descKey: 'marketplace.verificationForm.typeIdentityDesc',
  },
];

const defaultFormData: VerificationFormData = {
  type: 'business_registration',
  documentName: '',
};

export function VerificationForm({ onSubmit, onCancel, isLoading }: VerificationFormProps) {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<VerificationFormData>(defaultFormData);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const updateField = <K extends keyof VerificationFormData>(
    key: K,
    value: VerificationFormData[K]
  ) => {
    setFormData((prev) => ({ ...prev, [key]: value }));
    if (errors[key]) {
      setErrors((prev) => {
        const next = { ...prev };
        delete next[key];
        return next;
      });
    }
  };

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formData.documentName.trim()) {
      newErrors.documentName = t('marketplace.verificationForm.errDocumentName');
    }

    // Insurance and licenses typically require expiry dates
    if ((formData.type === 'insurance' || formData.type === 'license') && !formData.expiryDate) {
      newErrors.expiryDate = t('marketplace.verificationForm.errExpiryDate');
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

  const selectedType = verificationTypes.find((type) => type.value === formData.type);
  const selectedTypeLabel = selectedType
    ? t(selectedType.labelKey)
    : t('marketplace.verificationForm.documentFallback');

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <div className="bg-white rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold text-gray-900 mb-6">
          {t('marketplace.verificationForm.title')}
        </h2>

        {/* Document Type Selection */}
        <div className="mb-6">
          <label className="block text-sm font-medium text-gray-700 mb-3">
            {t('marketplace.verificationForm.verificationType')}
          </label>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {verificationTypes.map((type) => (
              <label
                key={type.value}
                className={`flex items-start p-4 border rounded-lg cursor-pointer transition-colors ${
                  formData.type === type.value
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-300 hover:bg-gray-50'
                }`}
              >
                <input
                  type="radio"
                  name="verificationType"
                  value={type.value}
                  checked={formData.type === type.value}
                  onChange={(e) => updateField('type', e.target.value as VerificationType)}
                  className="mt-1 w-4 h-4 text-blue-600 border-gray-300 focus:ring-blue-500"
                />
                <div className="ml-3">
                  <span className="block font-medium text-gray-900">{t(type.labelKey)}</span>
                  <span className="text-sm text-gray-500">{t(type.descKey)}</span>
                </div>
              </label>
            ))}
          </div>
        </div>

        {/* Document Details */}
        <div className="space-y-4">
          <div>
            <label htmlFor="documentName" className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.verificationForm.documentName')}
            </label>
            <input
              id="documentName"
              type="text"
              value={formData.documentName}
              onChange={(e) => updateField('documentName', e.target.value)}
              placeholder={t('marketplace.verificationForm.documentNamePlaceholder', {
                type: selectedTypeLabel,
              })}
              className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                errors.documentName ? 'border-red-500' : 'border-gray-300'
              }`}
            />
            {errors.documentName && (
              <p className="mt-1 text-sm text-red-600">{errors.documentName}</p>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label
                htmlFor="documentNumber"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.verificationForm.documentNumber')}
              </label>
              <input
                id="documentNumber"
                type="text"
                value={formData.documentNumber || ''}
                onChange={(e) => updateField('documentNumber', e.target.value)}
                placeholder={t('marketplace.verificationForm.documentNumberPlaceholder')}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label
                htmlFor="issuingAuthority"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.verificationForm.issuingAuthority')}
              </label>
              <input
                id="issuingAuthority"
                type="text"
                value={formData.issuingAuthority || ''}
                onChange={(e) => updateField('issuingAuthority', e.target.value)}
                placeholder={t('marketplace.verificationForm.issuingAuthorityPlaceholder')}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label htmlFor="issueDate" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.verificationForm.issueDate')}
              </label>
              <input
                id="issueDate"
                type="date"
                value={formData.issueDate || ''}
                onChange={(e) => updateField('issueDate', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label htmlFor="expiryDate" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.verificationForm.expiryDate')}{' '}
                {(formData.type === 'insurance' || formData.type === 'license') && '*'}
              </label>
              <input
                id="expiryDate"
                type="date"
                value={formData.expiryDate || ''}
                onChange={(e) => updateField('expiryDate', e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                  errors.expiryDate ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {errors.expiryDate && (
                <p className="mt-1 text-sm text-red-600">{errors.expiryDate}</p>
              )}
            </div>
          </div>

          {/* Document Upload */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.verificationForm.uploadDocument')}
            </label>
            <div className="border-2 border-dashed border-gray-300 rounded-lg p-6 text-center">
              <svg
                className="mx-auto w-12 h-12 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>{t('marketplace.verificationForm.uploadTitle')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                />
              </svg>
              <p className="mt-2 text-sm text-gray-600">
                {t('marketplace.verificationForm.dragDrop')}
              </p>
              <p className="mt-1 text-xs text-gray-500">
                {t('marketplace.verificationForm.fileTypes')}
              </p>
              <input
                type="file"
                accept=".pdf,.jpg,.jpeg,.png"
                className="hidden"
                id="documentUpload"
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) {
                    // In production, would upload to server and get URL
                    updateField('documentUrl', URL.createObjectURL(file));
                  }
                }}
              />
              <label
                htmlFor="documentUpload"
                className="mt-4 inline-flex px-4 py-2 bg-white border border-gray-300 rounded-lg text-sm font-medium text-gray-700 hover:bg-gray-50 cursor-pointer"
              >
                {t('marketplace.verificationForm.selectFile')}
              </label>
            </div>
            {formData.documentUrl && (
              <p className="mt-2 text-sm text-green-600">
                {t('marketplace.verificationForm.documentSelected')}
              </p>
            )}
          </div>
        </div>

        {/* Info Box */}
        <div className="mt-6 p-4 bg-blue-50 rounded-lg">
          <h3 className="text-sm font-medium text-blue-800">
            {t('marketplace.verificationForm.processTitle')}
          </h3>
          <ul className="mt-2 text-sm text-blue-700 list-disc list-inside space-y-1">
            <li>{t('marketplace.verificationForm.process1')}</li>
            <li>{t('marketplace.verificationForm.process2')}</li>
            <li>{t('marketplace.verificationForm.process3')}</li>
            <li>{t('marketplace.verificationForm.process4')}</li>
          </ul>
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-3">
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 text-gray-700 font-medium hover:text-gray-900"
        >
          {t('marketplace.common.cancel')}
        </button>
        <button
          type="submit"
          disabled={isLoading}
          className="px-6 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50"
        >
          {isLoading
            ? t('marketplace.verificationForm.submitting')
            : t('marketplace.verificationForm.submitForVerification')}
        </button>
      </div>
    </form>
  );
}
