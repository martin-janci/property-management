/**
 * ProviderProfileForm component - form for creating/editing provider profiles.
 * Epic 68: Service Provider Marketplace (Story 68.1)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ServiceCategory } from './ProviderCard';

export interface ProviderProfileFormData {
  companyName: string;
  businessRegistrationNumber?: string;
  taxId?: string;
  description?: string;
  logoUrl?: string;
  website?: string;
  contactName: string;
  contactEmail: string;
  contactPhone?: string;
  address?: string;
  city?: string;
  postalCode?: string;
  country?: string;
  serviceCategories: ServiceCategory[];
  serviceDescription?: string;
  specializations?: string[];
  coveragePostalCodes?: string[];
  coverageRadiusKm?: number;
  coverageRegions?: string[];
  pricingType: 'hourly' | 'project' | 'fixed' | 'quote_required';
  hourlyRateMin?: number;
  hourlyRateMax?: number;
  currency?: string;
  responseTimeHours?: number;
  emergencyAvailable?: boolean;
  portfolioImages?: string[];
  portfolioDescription?: string;
}

interface ProviderProfileFormProps {
  initialData?: Partial<ProviderProfileFormData>;
  onSubmit: (data: ProviderProfileFormData) => void;
  onCancel: () => void;
  isLoading?: boolean;
  isEdit?: boolean;
}

const categoryValues: ServiceCategory[] = [
  'plumbing',
  'electrical',
  'hvac',
  'cleaning',
  'landscaping',
  'security',
  'painting',
  'roofing',
  'carpentry',
  'locksmith',
  'pest_control',
  'general_maintenance',
  'elevator_maintenance',
  'fire_safety',
  'waste_management',
  'other',
];

const defaultFormData: ProviderProfileFormData = {
  companyName: '',
  contactName: '',
  contactEmail: '',
  serviceCategories: [],
  pricingType: 'hourly',
  currency: 'EUR',
};

export function ProviderProfileForm({
  initialData,
  onSubmit,
  onCancel,
  isLoading,
  isEdit,
}: ProviderProfileFormProps) {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<ProviderProfileFormData>({
    ...defaultFormData,
    ...initialData,
  });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [currentStep, setCurrentStep] = useState(1);

  const totalSteps = 4;

  const stepLabels = [
    t('marketplace.providerProfileForm.stepCompanyInfo'),
    t('marketplace.providerProfileForm.stepServices'),
    t('marketplace.providerProfileForm.stepPricing'),
    t('marketplace.providerProfileForm.stepPortfolio'),
  ];

  const pricingTypes: { value: ProviderProfileFormData['pricingType']; labelKey: string }[] = [
    { value: 'hourly', labelKey: 'marketplace.providerProfileForm.pricingHourly' },
    { value: 'project', labelKey: 'marketplace.providerProfileForm.pricingProject' },
    { value: 'fixed', labelKey: 'marketplace.providerProfileForm.pricingFixed' },
    { value: 'quote_required', labelKey: 'marketplace.providerProfileForm.pricingQuote' },
  ];

  const updateField = <K extends keyof ProviderProfileFormData>(
    key: K,
    value: ProviderProfileFormData[K]
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

  const toggleCategory = (cat: ServiceCategory) => {
    const current = formData.serviceCategories;
    if (current.includes(cat)) {
      updateField(
        'serviceCategories',
        current.filter((c) => c !== cat)
      );
    } else {
      updateField('serviceCategories', [...current, cat]);
    }
  };

  const validateStep = (step: number): boolean => {
    const newErrors: Record<string, string> = {};

    if (step === 1) {
      if (!formData.companyName.trim()) {
        newErrors.companyName = t('marketplace.providerProfileForm.errCompanyName');
      }
      if (!formData.contactName.trim()) {
        newErrors.contactName = t('marketplace.providerProfileForm.errContactName');
      }
      if (!formData.contactEmail.trim()) {
        newErrors.contactEmail = t('marketplace.providerProfileForm.errContactEmail');
      } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(formData.contactEmail)) {
        newErrors.contactEmail = t('marketplace.providerProfileForm.errEmailFormat');
      }
    }

    if (step === 2) {
      if (formData.serviceCategories.length === 0) {
        newErrors.serviceCategories = t('marketplace.providerProfileForm.errCategories');
      }
    }

    if (step === 3) {
      if (formData.pricingType === 'hourly') {
        if (formData.hourlyRateMin && formData.hourlyRateMax) {
          if (formData.hourlyRateMin > formData.hourlyRateMax) {
            newErrors.hourlyRateMax = t('marketplace.providerProfileForm.errMaxRate');
          }
        }
      }
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleNext = () => {
    if (validateStep(currentStep)) {
      setCurrentStep((prev) => Math.min(prev + 1, totalSteps));
    }
  };

  const handlePrev = () => {
    setCurrentStep((prev) => Math.max(prev - 1, 1));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validateStep(currentStep)) {
      onSubmit(formData);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="max-w-2xl mx-auto">
      {/* Progress Indicator */}
      <div className="mb-8">
        <div className="flex items-center justify-between">
          {stepLabels.map((label, index) => (
            <div key={label} className="flex items-center">
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium ${
                  index + 1 < currentStep
                    ? 'bg-green-500 text-white'
                    : index + 1 === currentStep
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-200 text-gray-600'
                }`}
              >
                {index + 1 < currentStep ? (
                  <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                    <title>{t('marketplace.common.completed')}</title>
                    <path
                      fillRule="evenodd"
                      d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                      clipRule="evenodd"
                    />
                  </svg>
                ) : (
                  index + 1
                )}
              </div>
              <span
                className={`ml-2 text-sm ${index + 1 === currentStep ? 'font-medium' : 'text-gray-500'}`}
              >
                {label}
              </span>
              {index < 3 && <div className="w-12 h-0.5 bg-gray-200 mx-4" />}
            </div>
          ))}
        </div>
      </div>

      {/* Step 1: Company Information */}
      {currentStep === 1 && (
        <div className="space-y-6">
          <h2 className="text-xl font-semibold text-gray-900">
            {t('marketplace.providerProfileForm.companyInformation')}
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="md:col-span-2">
              <label htmlFor="companyName" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.companyName')}
              </label>
              <input
                id="companyName"
                type="text"
                value={formData.companyName}
                onChange={(e) => updateField('companyName', e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                  errors.companyName ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {errors.companyName && (
                <p className="mt-1 text-sm text-red-600">{errors.companyName}</p>
              )}
            </div>

            <div>
              <label
                htmlFor="businessRegistrationNumber"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.providerProfileForm.businessRegNumber')}
              </label>
              <input
                id="businessRegistrationNumber"
                type="text"
                value={formData.businessRegistrationNumber || ''}
                onChange={(e) => updateField('businessRegistrationNumber', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label htmlFor="taxId" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.taxId')}
              </label>
              <input
                id="taxId"
                type="text"
                value={formData.taxId || ''}
                onChange={(e) => updateField('taxId', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div className="md:col-span-2">
              <label htmlFor="description" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.companyDescription')}
              </label>
              <textarea
                id="description"
                rows={4}
                value={formData.description || ''}
                onChange={(e) => updateField('description', e.target.value)}
                placeholder={t('marketplace.providerProfileForm.companyDescriptionPlaceholder')}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label htmlFor="website" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.website')}
              </label>
              <input
                id="website"
                type="url"
                value={formData.website || ''}
                onChange={(e) => updateField('website', e.target.value)}
                placeholder={t('marketplace.providerProfileForm.websitePlaceholder')}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label htmlFor="logoUrl" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.logoUrl')}
              </label>
              <input
                id="logoUrl"
                type="url"
                value={formData.logoUrl || ''}
                onChange={(e) => updateField('logoUrl', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>

          <h3 className="text-lg font-medium text-gray-900 pt-4">
            {t('marketplace.providerProfileForm.contactInformation')}
          </h3>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label htmlFor="contactName" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.contactName')}
              </label>
              <input
                id="contactName"
                type="text"
                value={formData.contactName}
                onChange={(e) => updateField('contactName', e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                  errors.contactName ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {errors.contactName && (
                <p className="mt-1 text-sm text-red-600">{errors.contactName}</p>
              )}
            </div>

            <div>
              <label
                htmlFor="contactEmail"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.providerProfileForm.contactEmail')}
              </label>
              <input
                id="contactEmail"
                type="email"
                value={formData.contactEmail}
                onChange={(e) => updateField('contactEmail', e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                  errors.contactEmail ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {errors.contactEmail && (
                <p className="mt-1 text-sm text-red-600">{errors.contactEmail}</p>
              )}
            </div>

            <div>
              <label
                htmlFor="contactPhone"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.providerProfileForm.contactPhone')}
              </label>
              <input
                id="contactPhone"
                type="tel"
                value={formData.contactPhone || ''}
                onChange={(e) => updateField('contactPhone', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label htmlFor="city" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.city')}
              </label>
              <input
                id="city"
                type="text"
                value={formData.city || ''}
                onChange={(e) => updateField('city', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div className="md:col-span-2">
              <label htmlFor="address" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.providerProfileForm.address')}
              </label>
              <input
                id="address"
                type="text"
                value={formData.address || ''}
                onChange={(e) => updateField('address', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>
          </div>
        </div>
      )}

      {/* Step 2: Services */}
      {currentStep === 2 && (
        <div className="space-y-6">
          <h2 className="text-xl font-semibold text-gray-900">
            {t('marketplace.providerProfileForm.servicesOffered')}
          </h2>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              {t('marketplace.providerProfileForm.selectCategories')}
            </label>
            {errors.serviceCategories && (
              <p className="mb-2 text-sm text-red-600">{errors.serviceCategories}</p>
            )}
            <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
              {categoryValues.map((cat) => (
                <label
                  key={cat}
                  className={`flex items-center p-3 border rounded-lg cursor-pointer transition-colors ${
                    formData.serviceCategories.includes(cat)
                      ? 'border-blue-500 bg-blue-50'
                      : 'border-gray-300 hover:bg-gray-50'
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={formData.serviceCategories.includes(cat)}
                    onChange={() => toggleCategory(cat)}
                    className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                  />
                  <span className="ml-2 text-sm text-gray-700">
                    {t(`marketplace.serviceCategory.${cat}`)}
                  </span>
                </label>
              ))}
            </div>
          </div>

          <div>
            <label
              htmlFor="serviceDescription"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('marketplace.providerProfileForm.serviceDescription')}
            </label>
            <textarea
              id="serviceDescription"
              rows={4}
              value={formData.serviceDescription || ''}
              onChange={(e) => updateField('serviceDescription', e.target.value)}
              placeholder={t('marketplace.providerProfileForm.serviceDescriptionPlaceholder')}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label
              htmlFor="coverageRadiusKm"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('marketplace.providerProfileForm.coverageRadius')}
            </label>
            <input
              id="coverageRadiusKm"
              type="number"
              min="1"
              value={formData.coverageRadiusKm || ''}
              onChange={(e) =>
                updateField('coverageRadiusKm', e.target.value ? Number(e.target.value) : undefined)
              }
              placeholder={t('marketplace.providerProfileForm.coverageRadiusPlaceholder')}
              className="w-32 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div className="flex items-center gap-4">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={formData.emergencyAvailable || false}
                onChange={(e) => updateField('emergencyAvailable', e.target.checked)}
                className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
              />
              <span className="text-sm text-gray-700">
                {t('marketplace.providerProfileForm.emergencyCalls')}
              </span>
            </label>
          </div>
        </div>
      )}

      {/* Step 3: Pricing */}
      {currentStep === 3 && (
        <div className="space-y-6">
          <h2 className="text-xl font-semibold text-gray-900">
            {t('marketplace.providerProfileForm.pricingStructure')}
          </h2>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              {t('marketplace.providerProfileForm.pricingType')}
            </label>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {pricingTypes.map((opt) => (
                <label
                  key={opt.value}
                  className={`flex items-center justify-center p-3 border rounded-lg cursor-pointer transition-colors ${
                    formData.pricingType === opt.value
                      ? 'border-blue-500 bg-blue-50'
                      : 'border-gray-300 hover:bg-gray-50'
                  }`}
                >
                  <input
                    type="radio"
                    name="pricingType"
                    value={opt.value}
                    checked={formData.pricingType === opt.value}
                    onChange={(e) =>
                      updateField(
                        'pricingType',
                        e.target.value as ProviderProfileFormData['pricingType']
                      )
                    }
                    className="sr-only"
                  />
                  <span className="text-sm text-gray-700">{t(opt.labelKey)}</span>
                </label>
              ))}
            </div>
          </div>

          {formData.pricingType === 'hourly' && (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div>
                <label
                  htmlFor="hourlyRateMin"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('marketplace.providerProfileForm.minHourlyRate')}
                </label>
                <div className="relative">
                  <input
                    id="hourlyRateMin"
                    type="number"
                    min="0"
                    step="0.01"
                    value={formData.hourlyRateMin || ''}
                    onChange={(e) =>
                      updateField(
                        'hourlyRateMin',
                        e.target.value ? Number(e.target.value) : undefined
                      )
                    }
                    className="w-full pl-8 pr-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                  />
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500">
                    {formData.currency === 'EUR' ? 'EUR' : formData.currency || 'EUR'}
                  </span>
                </div>
              </div>

              <div>
                <label
                  htmlFor="hourlyRateMax"
                  className="block text-sm font-medium text-gray-700 mb-1"
                >
                  {t('marketplace.providerProfileForm.maxHourlyRate')}
                </label>
                <div className="relative">
                  <input
                    id="hourlyRateMax"
                    type="number"
                    min="0"
                    step="0.01"
                    value={formData.hourlyRateMax || ''}
                    onChange={(e) =>
                      updateField(
                        'hourlyRateMax',
                        e.target.value ? Number(e.target.value) : undefined
                      )
                    }
                    className={`w-full pl-8 pr-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                      errors.hourlyRateMax ? 'border-red-500' : 'border-gray-300'
                    }`}
                  />
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500">
                    {formData.currency === 'EUR' ? 'EUR' : formData.currency || 'EUR'}
                  </span>
                </div>
                {errors.hourlyRateMax && (
                  <p className="mt-1 text-sm text-red-600">{errors.hourlyRateMax}</p>
                )}
              </div>

              <div>
                <label htmlFor="currency" className="block text-sm font-medium text-gray-700 mb-1">
                  {t('marketplace.providerProfileForm.currency')}
                </label>
                <select
                  id="currency"
                  value={formData.currency || 'EUR'}
                  onChange={(e) => updateField('currency', e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                >
                  <option value="EUR">EUR</option>
                  <option value="CZK">CZK</option>
                  <option value="USD">USD</option>
                  <option value="GBP">GBP</option>
                </select>
              </div>
            </div>
          )}

          <div>
            <label
              htmlFor="responseTimeHours"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('marketplace.providerProfileForm.responseTime')}
            </label>
            <input
              id="responseTimeHours"
              type="number"
              min="1"
              value={formData.responseTimeHours || ''}
              onChange={(e) =>
                updateField(
                  'responseTimeHours',
                  e.target.value ? Number(e.target.value) : undefined
                )
              }
              placeholder={t('marketplace.providerProfileForm.responseTimePlaceholder')}
              className="w-32 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            />
            <p className="mt-1 text-sm text-gray-500">
              {t('marketplace.providerProfileForm.responseTimeHint')}
            </p>
          </div>
        </div>
      )}

      {/* Step 4: Portfolio */}
      {currentStep === 4 && (
        <div className="space-y-6">
          <h2 className="text-xl font-semibold text-gray-900">
            {t('marketplace.providerProfileForm.portfolio')}
          </h2>

          <div>
            <label
              htmlFor="portfolioDescription"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('marketplace.providerProfileForm.portfolioDescription')}
            </label>
            <textarea
              id="portfolioDescription"
              rows={4}
              value={formData.portfolioDescription || ''}
              onChange={(e) => updateField('portfolioDescription', e.target.value)}
              placeholder={t('marketplace.providerProfileForm.portfolioDescriptionPlaceholder')}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              {t('marketplace.providerProfileForm.portfolioImages')}
            </label>
            <p className="text-sm text-gray-500 mb-4">
              {t('marketplace.providerProfileForm.portfolioImagesHint')}
            </p>
            <div className="border-2 border-dashed border-gray-300 rounded-lg p-8 text-center">
              <svg
                className="mx-auto w-12 h-12 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>{t('marketplace.providerProfileForm.uploadImages')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
                />
              </svg>
              <p className="mt-2 text-sm text-gray-600">
                {t('marketplace.providerProfileForm.imageUploadHint')}
              </p>
            </div>
          </div>

          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
            <h3 className="text-sm font-medium text-blue-800">
              {t('marketplace.providerProfileForm.readyToSubmit')}
            </h3>
            <p className="mt-1 text-sm text-blue-700">
              {t('marketplace.providerProfileForm.reviewNote')}
            </p>
          </div>
        </div>
      )}

      {/* Navigation Buttons */}
      <div className="mt-8 flex items-center justify-between pt-6 border-t">
        <button
          type="button"
          onClick={currentStep === 1 ? onCancel : handlePrev}
          className="px-4 py-2 text-gray-700 font-medium hover:text-gray-900"
        >
          {currentStep === 1
            ? t('marketplace.common.cancel')
            : t('marketplace.providerProfileForm.previous')}
        </button>

        <div className="flex items-center gap-3">
          {currentStep < totalSteps ? (
            <button
              type="button"
              onClick={handleNext}
              className="px-6 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
            >
              {t('marketplace.providerProfileForm.next')}
            </button>
          ) : (
            <button
              type="submit"
              disabled={isLoading}
              className="px-6 py-2 bg-green-600 text-white font-medium rounded-lg hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 disabled:opacity-50"
            >
              {isLoading
                ? t('marketplace.providerProfileForm.saving')
                : isEdit
                  ? t('marketplace.providerProfileForm.updateProfile')
                  : t('marketplace.providerProfileForm.createProfile')}
            </button>
          )}
        </div>
      </div>
    </form>
  );
}
