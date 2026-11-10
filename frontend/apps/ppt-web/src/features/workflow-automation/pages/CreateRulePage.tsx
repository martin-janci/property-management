/**
 * CreateRulePage
 *
 * Page for creating a new automation rule.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type { AutomationRule, CreateAutomationRuleInput } from '@ppt/api-client';
import { useCreateAutomationRule } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';

import { useToast } from '../../../components';
import { RuleBuilder } from '../components/RuleBuilder';

export function CreateRulePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { showToast } = useToast();
  const createRule = useCreateAutomationRule();

  const handleSave = async (rule: Partial<AutomationRule>) => {
    // The builder yields a Partial; validate the client-required fields at the
    // boundary and construct a typed CreateAutomationRuleInput instead of a
    // blanket `as AutomationRule` cast (which silently pretends server-owned
    // fields like id/createdAt exist and masks missing required fields).
    if (!rule.name || !rule.trigger || !rule.actions || rule.actions.length === 0) {
      showToast({
        type: 'error',
        title: t('automation.createPage.createFailedTitle'),
        message: t('automation.createPage.incomplete'),
      });
      return;
    }

    const payload: CreateAutomationRuleInput = {
      name: rule.name,
      description: rule.description,
      isEnabled: rule.isEnabled ?? true,
      trigger: rule.trigger,
      actions: rule.actions,
    };

    try {
      await createRule.mutateAsync(payload);
      showToast({
        type: 'success',
        title: t('automation.createPage.createdTitle'),
        message: t('automation.createPage.createdMessage'),
      });
      navigate('/automations/rules');
    } catch (err) {
      showToast({
        type: 'error',
        title: t('automation.createPage.createFailedTitle'),
        message: err instanceof Error ? err.message : t('automation.createPage.createErrorMessage'),
      });
    }
  };

  const handleCancel = () => {
    navigate('/automations/rules');
  };

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
          {t('automation.createPage.back')}
        </button>
        <h1 className="text-2xl font-bold text-gray-900">{t('automation.createPage.title')}</h1>
        <p className="mt-1 text-sm text-gray-500">{t('automation.createPage.subtitle')}</p>
      </div>

      {/* Builder */}
      <RuleBuilder onSave={handleSave} onCancel={handleCancel} isLoading={createRule.isPending} />

      {/* Error Display */}
      {createRule.error && (
        <div className="mt-4 bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-700">{t('automation.createPage.error')}</p>
        </div>
      )}
    </div>
  );
}
