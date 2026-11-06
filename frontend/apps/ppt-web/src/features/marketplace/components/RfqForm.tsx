/**
 * RfqForm component - form for creating Request for Quote.
 * Epic 68: Service Provider Marketplace (Story 68.3)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ServiceCategory } from './ProviderCard';

export interface RfqFormData {
  title: string;
  description: string;
  serviceCategory: ServiceCategory;
  scopeOfWork?: string;
  preferredStartDate?: string;
  preferredEndDate?: string;
  isUrgent?: boolean;
  budgetMin?: number;
  budgetMax?: number;
  currency?: string;
  quoteDeadline?: string;
  contactPreference?: 'email' | 'phone' | 'any';
  siteVisitRequired?: boolean;
  providerIds: string[];
}

interface RfqFormProps {
  initialData?: Partial<RfqFormData>;
  buildingId?: string;
  buildingName?: string;
  selectedProviders?: Array<{ id: string; companyName: string }>;
  onSubmit: (data: RfqFormData) => void;
  onCancel: () => void;
  onSelectProviders: () => void;
  isLoading?: boolean;
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

const defaultFormData: RfqFormData = {
  title: '',
  description: '',
  serviceCategory: 'general_maintenance',
  currency: 'EUR',
  contactPreference: 'any',
  providerIds: [],
};

export function RfqForm({
  initialData,
  buildingName,
  selectedProviders = [],
  onSubmit,
  onCancel,
  onSelectProviders,
  isLoading,
}: RfqFormProps) {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<RfqFormData>({
    ...defaultFormData,
    ...initialData,
    providerIds: selectedProviders.map((p) => p.id),
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const updateField = <K extends keyof RfqFormData>(key: K, value: RfqFormData[K]) => {
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

    if (!formData.title.trim()) {
      newErrors.title = t('marketplace.rfqForm.errTitle');
    }
    if (!formData.description.trim()) {
      newErrors.description = t('marketplace.rfqForm.errDescription');
    }
    if (selectedProviders.length === 0) {
      newErrors.providers = t('marketplace.rfqForm.errProviders');
    }
    if (formData.budgetMin && formData.budgetMax && formData.budgetMin > formData.budgetMax) {
      newErrors.budgetMax = t('marketplace.rfqForm.errBudgetMax');
    }
    if (formData.preferredStartDate && formData.preferredEndDate) {
      if (new Date(formData.preferredStartDate) > new Date(formData.preferredEndDate)) {
        newErrors.preferredEndDate = t('marketplace.rfqForm.errEndDate');
      }
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmit({
        ...formData,
        providerIds: selectedProviders.map((p) => p.id),
      });
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <div className="bg-white rounded-lg shadow p-6">
        <h2 className="text-xl font-semibold text-gray-900 mb-6">
          {t('marketplace.rfqForm.title')}
        </h2>

        {buildingName && (
          <div className="mb-6 p-4 bg-gray-50 rounded-lg">
            <p className="text-sm text-gray-600">
              {t('marketplace.rfqForm.building')}{' '}
              <span className="font-medium text-gray-900">{buildingName}</span>
            </p>
          </div>
        )}

        {/* Basic Info */}
        <div className="space-y-4">
          <div>
            <label htmlFor="title" className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.rfqForm.titleLabel')}
            </label>
            <input
              id="title"
              type="text"
              value={formData.title}
              onChange={(e) => updateField('title', e.target.value)}
              placeholder={t('marketplace.rfqForm.titlePlaceholder')}
              className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                errors.title ? 'border-red-500' : 'border-gray-300'
              }`}
            />
            {errors.title && <p className="mt-1 text-sm text-red-600">{errors.title}</p>}
          </div>

          <div>
            <label
              htmlFor="serviceCategory"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('marketplace.rfqForm.serviceCategory')}
            </label>
            <select
              id="serviceCategory"
              value={formData.serviceCategory}
              onChange={(e) => updateField('serviceCategory', e.target.value as ServiceCategory)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            >
              {categoryValues.map((cat) => (
                <option key={cat} value={cat}>
                  {t(`marketplace.serviceCategory.${cat}`)}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label htmlFor="description" className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.rfqForm.description')}
            </label>
            <textarea
              id="description"
              rows={4}
              value={formData.description}
              onChange={(e) => updateField('description', e.target.value)}
              placeholder={t('marketplace.rfqForm.descriptionPlaceholder')}
              className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                errors.description ? 'border-red-500' : 'border-gray-300'
              }`}
            />
            {errors.description && (
              <p className="mt-1 text-sm text-red-600">{errors.description}</p>
            )}
          </div>

          <div>
            <label htmlFor="scopeOfWork" className="block text-sm font-medium text-gray-700 mb-1">
              {t('marketplace.rfqForm.scopeOfWork')}
            </label>
            <textarea
              id="scopeOfWork"
              rows={3}
              value={formData.scopeOfWork || ''}
              onChange={(e) => updateField('scopeOfWork', e.target.value)}
              placeholder={t('marketplace.rfqForm.scopePlaceholder')}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>
        </div>

        {/* Timeline */}
        <div className="mt-6 pt-6 border-t">
          <h3 className="text-lg font-medium text-gray-900 mb-4">
            {t('marketplace.rfqForm.timeline')}
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label
                htmlFor="preferredStartDate"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.rfqForm.preferredStartDate')}
              </label>
              <input
                id="preferredStartDate"
                type="date"
                value={formData.preferredStartDate || ''}
                onChange={(e) => updateField('preferredStartDate', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label
                htmlFor="preferredEndDate"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.rfqForm.preferredEndDate')}
              </label>
              <input
                id="preferredEndDate"
                type="date"
                value={formData.preferredEndDate || ''}
                onChange={(e) => updateField('preferredEndDate', e.target.value)}
                className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                  errors.preferredEndDate ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {errors.preferredEndDate && (
                <p className="mt-1 text-sm text-red-600">{errors.preferredEndDate}</p>
              )}
            </div>

            <div>
              <label
                htmlFor="quoteDeadline"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.rfqForm.quoteDeadline')}
              </label>
              <input
                id="quoteDeadline"
                type="datetime-local"
                value={formData.quoteDeadline || ''}
                onChange={(e) => updateField('quoteDeadline', e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
              <p className="mt-1 text-xs text-gray-500">
                {t('marketplace.rfqForm.quoteDeadlineHint')}
              </p>
            </div>

            <div className="flex items-end">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.isUrgent || false}
                  onChange={(e) => updateField('isUrgent', e.target.checked)}
                  className="w-4 h-4 text-red-600 border-gray-300 rounded focus:ring-red-500"
                />
                <span className="text-sm text-gray-700">{t('marketplace.rfqForm.markUrgent')}</span>
              </label>
            </div>
          </div>
        </div>

        {/* Budget */}
        <div className="mt-6 pt-6 border-t">
          <h3 className="text-lg font-medium text-gray-900 mb-4">
            {t('marketplace.rfqForm.budgetOptional')}
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label htmlFor="budgetMin" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.rfqForm.minimumBudget')}
              </label>
              <input
                id="budgetMin"
                type="number"
                min="0"
                step="0.01"
                value={formData.budgetMin || ''}
                onChange={(e) =>
                  updateField('budgetMin', e.target.value ? Number(e.target.value) : undefined)
                }
                placeholder="0.00"
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label htmlFor="budgetMax" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.rfqForm.maximumBudget')}
              </label>
              <input
                id="budgetMax"
                type="number"
                min="0"
                step="0.01"
                value={formData.budgetMax || ''}
                onChange={(e) =>
                  updateField('budgetMax', e.target.value ? Number(e.target.value) : undefined)
                }
                placeholder="0.00"
                className={`w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 ${
                  errors.budgetMax ? 'border-red-500' : 'border-gray-300'
                }`}
              />
              {errors.budgetMax && <p className="mt-1 text-sm text-red-600">{errors.budgetMax}</p>}
            </div>

            <div>
              <label htmlFor="currency" className="block text-sm font-medium text-gray-700 mb-1">
                {t('marketplace.rfqForm.currency')}
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
        </div>

        {/* Preferences */}
        <div className="mt-6 pt-6 border-t">
          <h3 className="text-lg font-medium text-gray-900 mb-4">
            {t('marketplace.rfqForm.preferences')}
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label
                htmlFor="contactPreference"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('marketplace.rfqForm.contactPreference')}
              </label>
              <select
                id="contactPreference"
                value={formData.contactPreference || 'any'}
                onChange={(e) =>
                  updateField(
                    'contactPreference',
                    e.target.value as RfqFormData['contactPreference']
                  )
                }
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              >
                <option value="any">{t('marketplace.rfqForm.any')}</option>
                <option value="email">{t('marketplace.rfqForm.emailOnly')}</option>
                <option value="phone">{t('marketplace.rfqForm.phoneOnly')}</option>
              </select>
            </div>

            <div className="flex items-end">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.siteVisitRequired || false}
                  onChange={(e) => updateField('siteVisitRequired', e.target.checked)}
                  className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span className="text-sm text-gray-700">
                  {t('marketplace.rfqForm.siteVisitRequired')}
                </span>
              </label>
            </div>
          </div>
        </div>

        {/* Selected Providers */}
        <div className="mt-6 pt-6 border-t">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-medium text-gray-900">
              {t('marketplace.rfqForm.sendRfqTo')}
            </h3>
            <button
              type="button"
              onClick={onSelectProviders}
              className="text-sm font-medium text-blue-600 hover:text-blue-800"
            >
              {selectedProviders.length > 0
                ? t('marketplace.rfqForm.modifySelection')
                : t('marketplace.rfqForm.selectProviders')}
            </button>
          </div>

          {errors.providers && <p className="mb-2 text-sm text-red-600">{errors.providers}</p>}

          {selectedProviders.length === 0 ? (
            <div className="text-center py-8 bg-gray-50 rounded-lg border-2 border-dashed border-gray-300">
              <p className="text-gray-500">{t('marketplace.rfqForm.noProvidersSelected')}</p>
              <button
                type="button"
                onClick={onSelectProviders}
                className="mt-2 text-sm font-medium text-blue-600 hover:text-blue-800"
              >
                {t('marketplace.rfqForm.browseSelect')}
              </button>
            </div>
          ) : (
            <div className="space-y-2">
              {selectedProviders.map((provider) => (
                <div
                  key={provider.id}
                  className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                >
                  <span className="font-medium text-gray-900">{provider.companyName}</span>
                  <button
                    type="button"
                    onClick={() => {
                      // Remove provider from selection
                      const newProviders = selectedProviders.filter((p) => p.id !== provider.id);
                      updateField(
                        'providerIds',
                        newProviders.map((p) => p.id)
                      );
                    }}
                    className="text-sm text-red-600 hover:text-red-800"
                  >
                    {t('marketplace.common.remove')}
                  </button>
                </div>
              ))}
              <p className="text-sm text-gray-500">
                {selectedProviders.length === 1
                  ? t('marketplace.rfqForm.providersWillReceiveOne', {
                      count: selectedProviders.length,
                    })
                  : t('marketplace.rfqForm.providersWillReceiveOther', {
                      count: selectedProviders.length,
                    })}
              </p>
            </div>
          )}
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
          {isLoading ? t('marketplace.rfqForm.sending') : t('marketplace.rfqForm.sendRfq')}
        </button>
      </div>
    </form>
  );
}
