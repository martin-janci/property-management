/**
 * CreateRfqPage - page for creating a new RFQ.
 * Epic 68: Service Provider Marketplace (Story 68.3)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ProviderCard, type ProviderSummary } from '../components/ProviderCard';
import { ProviderSearchFilters, type SearchFilters } from '../components/ProviderSearchFilters';
import { RfqForm, type RfqFormData } from '../components/RfqForm';

interface CreateRfqPageProps {
  buildingId?: string;
  buildingName?: string;
  availableProviders: ProviderSummary[];
  isLoadingProviders?: boolean;
  isSaving?: boolean;
  onSearchProviders: (filters: SearchFilters) => void;
  onSubmit: (data: RfqFormData) => void;
  onCancel: () => void;
}

type StepType = 'select-providers' | 'create-rfq';

export function CreateRfqPage({
  buildingId,
  buildingName,
  availableProviders,
  isLoadingProviders,
  isSaving,
  onSearchProviders,
  onSubmit,
  onCancel,
}: CreateRfqPageProps) {
  const { t } = useTranslation();
  const [currentStep, setCurrentStep] = useState<StepType>('select-providers');
  const [selectedProviders, setSelectedProviders] = useState<ProviderSummary[]>([]);
  const [searchFilters, setSearchFilters] = useState<SearchFilters>({});

  const handleSelectProvider = (provider: ProviderSummary) => {
    if (selectedProviders.find((p) => p.id === provider.id)) {
      setSelectedProviders((prev) => prev.filter((p) => p.id !== provider.id));
    } else {
      setSelectedProviders((prev) => [...prev, provider]);
    }
  };

  const handleSearch = () => {
    onSearchProviders(searchFilters);
  };

  const handleReset = () => {
    setSearchFilters({});
    onSearchProviders({});
  };

  const handleProceedToRfq = () => {
    setCurrentStep('create-rfq');
  };

  const handleBackToProviders = () => {
    setCurrentStep('select-providers');
  };

  // Provider Selection Step
  if (currentStep === 'select-providers') {
    return (
      <div className="max-w-5xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-2xl font-bold text-gray-900">{t('marketplace.rfqForm.title')}</h1>
          <p className="mt-1 text-gray-600">{t('marketplace.createRfqPage.step1')}</p>
        </div>

        {/* Progress Indicator */}
        <div className="mb-8">
          <div className="flex items-center">
            <div className="flex items-center">
              <div className="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center text-sm font-medium">
                1
              </div>
              <span className="ml-2 text-sm font-medium text-blue-600">
                {t('marketplace.createRfqPage.selectProviders')}
              </span>
            </div>
            <div className="flex-1 mx-4 h-0.5 bg-gray-200" />
            <div className="flex items-center">
              <div className="w-8 h-8 rounded-full bg-gray-200 text-gray-600 flex items-center justify-center text-sm font-medium">
                2
              </div>
              <span className="ml-2 text-sm text-gray-500">
                {t('marketplace.createRfqPage.createRfqStep')}
              </span>
            </div>
          </div>
        </div>

        {/* Search Filters */}
        <ProviderSearchFilters
          filters={searchFilters}
          onFiltersChange={setSearchFilters}
          onSearch={handleSearch}
          onReset={handleReset}
          isLoading={isLoadingProviders}
        />

        {/* Selected Providers Summary */}
        {selectedProviders.length > 0 && (
          <div className="mt-6 p-4 bg-blue-50 rounded-lg">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-blue-900">
                  {selectedProviders.length === 1
                    ? t('marketplace.createRfqPage.providersSelectedOne', {
                        count: selectedProviders.length,
                      })
                    : t('marketplace.createRfqPage.providersSelectedOther', {
                        count: selectedProviders.length,
                      })}
                </p>
                <p className="text-sm text-blue-700">
                  {selectedProviders.map((p) => p.companyName).join(', ')}
                </p>
              </div>
              <button
                type="button"
                onClick={() => setSelectedProviders([])}
                className="text-sm text-blue-600 hover:text-blue-800"
              >
                {t('marketplace.common.clearAll')}
              </button>
            </div>
          </div>
        )}

        {/* Provider Grid */}
        <div className="mt-6">
          {isLoadingProviders ? (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {[...Array(4)].map((_, i) => (
                <div key={`skeleton-${i}`} className="bg-white rounded-lg shadow p-6 animate-pulse">
                  <div className="flex items-start gap-4">
                    <div className="w-16 h-16 bg-gray-200 rounded-lg" />
                    <div className="flex-1">
                      <div className="h-5 bg-gray-200 rounded w-3/4" />
                      <div className="mt-2 h-4 bg-gray-200 rounded w-1/2" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : availableProviders.length === 0 ? (
            <div className="text-center py-12 bg-gray-50 rounded-lg">
              <p className="text-gray-500">{t('marketplace.createRfqPage.noProvidersFound')}</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {availableProviders.map((provider) => {
                const isSelected = selectedProviders.some((p) => p.id === provider.id);
                return (
                  <div
                    key={provider.id}
                    className={`relative ${isSelected ? 'ring-2 ring-blue-500 rounded-lg' : ''}`}
                  >
                    {isSelected && (
                      <div className="absolute top-2 right-2 z-10 w-6 h-6 bg-blue-500 rounded-full flex items-center justify-center">
                        <svg className="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 20 20">
                          <title>{t('marketplace.common.selected')}</title>
                          <path
                            fillRule="evenodd"
                            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                            clipRule="evenodd"
                          />
                        </svg>
                      </div>
                    )}
                    <ProviderCard
                      provider={provider}
                      onView={() => handleSelectProvider(provider)}
                      onRequestQuote={() => handleSelectProvider(provider)}
                    />
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Navigation */}
        <div className="mt-8 flex items-center justify-between pt-6 border-t">
          <button
            type="button"
            onClick={onCancel}
            className="px-4 py-2 text-gray-700 font-medium hover:text-gray-900"
          >
            {t('marketplace.common.cancel')}
          </button>
          <button
            type="button"
            onClick={handleProceedToRfq}
            disabled={selectedProviders.length === 0}
            className="px-6 py-2 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {selectedProviders.length === 1
              ? t('marketplace.createRfqPage.continueWithOne', { count: selectedProviders.length })
              : t('marketplace.createRfqPage.continueWithOther', {
                  count: selectedProviders.length,
                })}
          </button>
        </div>
      </div>
    );
  }

  // RFQ Creation Step
  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">{t('marketplace.rfqForm.title')}</h1>
        <p className="mt-1 text-gray-600">{t('marketplace.createRfqPage.step2')}</p>
      </div>

      {/* Progress Indicator */}
      <div className="mb-8">
        <div className="flex items-center">
          <div className="flex items-center">
            <div className="w-8 h-8 rounded-full bg-green-500 text-white flex items-center justify-center">
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <title>{t('marketplace.common.completed')}</title>
                <path
                  fillRule="evenodd"
                  d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                  clipRule="evenodd"
                />
              </svg>
            </div>
            <span className="ml-2 text-sm text-green-600">
              {t('marketplace.createRfqPage.selectProviders')}
            </span>
          </div>
          <div className="flex-1 mx-4 h-0.5 bg-blue-600" />
          <div className="flex items-center">
            <div className="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center text-sm font-medium">
              2
            </div>
            <span className="ml-2 text-sm font-medium text-blue-600">
              {t('marketplace.createRfqPage.createRfqStep')}
            </span>
          </div>
        </div>
      </div>

      {/* RFQ Form */}
      <RfqForm
        buildingId={buildingId}
        buildingName={buildingName}
        selectedProviders={selectedProviders.map((p) => ({
          id: p.id,
          companyName: p.companyName,
        }))}
        onSubmit={onSubmit}
        onCancel={handleBackToProviders}
        onSelectProviders={handleBackToProviders}
        isLoading={isSaving}
      />
    </div>
  );
}
