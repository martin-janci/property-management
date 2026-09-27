/**
 * EditRulePage
 *
 * Page for editing an existing automation rule.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type { AutomationRule } from '@ppt/api-client';
import { useAutomationRule, useUpdateAutomationRule } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';

import { RuleBuilder } from '../components/RuleBuilder';

const skeletonKeys = ['skeleton-step-1', 'skeleton-step-2', 'skeleton-step-3', 'skeleton-step-4'];

export function EditRulePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const { data: rule, isLoading, error } = useAutomationRule(id ?? '');
  const updateRule = useUpdateAutomationRule();

  const handleSave = async (updatedRule: Partial<AutomationRule>) => {
    if (id) {
      await updateRule.mutateAsync({ id, data: updatedRule });
      navigate('/automations/rules');
    }
  };

  const handleCancel = () => {
    navigate('/automations/rules');
  };

  if (!id) {
    return (
      <div className="max-w-6xl mx-auto px-4 py-8">
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-700">{t('automation.editPage.invalidId')}</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-6xl mx-auto px-4 py-8">
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-700">{t('automation.editPage.loadError')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="mb-8">
        <button
          type="button"
          onClick={handleCancel}
          className="inline-flex items-center text-sm text-gray-500 hover:text-gray-700 mb-4"
        >
          <svg
            className="w-4 h-4 mr-1"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          {t('automation.editPage.back')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">
          {isLoading
            ? t('automation.editPage.loading')
            : t('automation.editPage.titlePrefix', {
                name: rule?.name ?? t('automation.editPage.fallbackName'),
              })}
        </h1>
        <p className="mt-1 text-sm text-gray-500">{t('automation.editPage.subtitle')}</p>
      </div>

      {/* Loading State */}
      {isLoading ? (
        <div className="max-w-4xl mx-auto">
          <div className="flex items-center gap-4 mb-8">
            {skeletonKeys.map((key) => (
              <div key={key} className="w-10 h-10 bg-gray-200 rounded-full animate-pulse" />
            ))}
          </div>
          <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 animate-pulse">
            <div className="h-6 bg-gray-200 rounded w-1/4 mb-4" />
            <div className="h-4 bg-gray-200 rounded w-1/2 mb-8" />
            <div className="grid grid-cols-2 gap-4">
              <div className="h-24 bg-gray-200 rounded" />
              <div className="h-24 bg-gray-200 rounded" />
              <div className="h-24 bg-gray-200 rounded" />
              <div className="h-24 bg-gray-200 rounded" />
            </div>
          </div>
        </div>
      ) : rule ? (
        <RuleBuilder
          initialRule={rule}
          onSave={handleSave}
          onCancel={handleCancel}
          isLoading={updateRule.isPending}
        />
      ) : null}

      {/* Error Display */}
      {updateRule.error && (
        <div className="mt-4 bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-700">{t('automation.editPage.updateError')}</p>
        </div>
      )}
    </div>
  );
}
