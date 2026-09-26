/**
 * CreateRulePage
 *
 * Page for creating a new automation rule.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type { AutomationRule } from '@ppt/api-client';
import { useCreateAutomationRule } from '@ppt/api-client';
import { useNavigate } from 'react-router-dom';

import { useToast } from '../../../components';
import { RuleBuilder } from '../components/RuleBuilder';

export function CreateRulePage() {
  const navigate = useNavigate();
  const { showToast } = useToast();
  const createRule = useCreateAutomationRule();

  const handleSave = async (rule: Partial<AutomationRule>) => {
    try {
      await createRule.mutateAsync(rule as AutomationRule);
      showToast({
        type: 'success',
        title: 'Rule created',
        message: 'The automation rule was created.',
      });
      navigate('/automations/rules');
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Create failed',
        message: err instanceof Error ? err.message : 'The automation rule could not be created.',
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
          Back to Rules
        </button>
        <h1 className="text-2xl font-bold text-gray-900">Create Automation Rule</h1>
        <p className="mt-1 text-sm text-gray-500">
          Set up a new automated workflow for your property management tasks.
        </p>
      </div>

      {/* Builder */}
      <RuleBuilder onSave={handleSave} onCancel={handleCancel} isLoading={createRule.isPending} />

      {/* Error Display */}
      {createRule.error && (
        <div className="mt-4 bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-700">Failed to create automation rule. Please try again.</p>
        </div>
      )}
    </div>
  );
}
