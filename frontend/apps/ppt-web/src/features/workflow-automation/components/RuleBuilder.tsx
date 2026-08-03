/**
 * RuleBuilder Component
 *
 * Main component for creating and editing automation rules.
 * Combines TriggerSelector, ConditionBuilder, and ActionBuilder.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type {
  AutomationAction,
  AutomationRule,
  AutomationTrigger,
  TriggerCondition,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ActionBuilder } from './ActionBuilder';
import { ConditionBuilder } from './ConditionBuilder';
import { TriggerSelector } from './TriggerSelector';

interface RuleBuilderProps {
  initialRule?: Partial<AutomationRule>;
  onSave: (rule: Partial<AutomationRule>) => void;
  onCancel: () => void;
  isLoading?: boolean;
}

type BuilderStep = 'trigger' | 'conditions' | 'actions' | 'review';

const steps: { id: BuilderStep; label: string; description: string }[] = [
  { id: 'trigger', label: 'Trigger', description: 'When should this run?' },
  { id: 'conditions', label: 'Conditions', description: 'Filter when to run' },
  { id: 'actions', label: 'Actions', description: 'What should happen?' },
  { id: 'review', label: 'Review', description: 'Confirm and save' },
];

export function RuleBuilder({ initialRule, onSave, onCancel, isLoading }: RuleBuilderProps) {
  const { t } = useTranslation();
  const [currentStep, setCurrentStep] = useState<BuilderStep>('trigger');
  const [rule, setRule] = useState<Partial<AutomationRule>>({
    name: '',
    description: '',
    isEnabled: true,
    ...initialRule,
  });

  const currentStepIndex = steps.findIndex((s) => s.id === currentStep);

  const updateRule = (updates: Partial<AutomationRule>) => {
    setRule((prev) => ({ ...prev, ...updates }));
  };

  const handleTriggerChange = (trigger: Partial<AutomationTrigger>) => {
    updateRule({ trigger: trigger as AutomationTrigger });
  };

  const handleConditionsChange = (conditions: TriggerCondition[]) => {
    updateRule({
      trigger: {
        ...rule.trigger,
        conditions,
      } as AutomationTrigger,
    });
  };

  const handleActionsChange = (actions: AutomationAction[]) => {
    updateRule({ actions });
  };

  const goToStep = (step: BuilderStep) => {
    setCurrentStep(step);
  };

  const goNext = () => {
    const nextIndex = currentStepIndex + 1;
    if (nextIndex < steps.length) {
      setCurrentStep(steps[nextIndex].id);
    }
  };

  const goPrevious = () => {
    const prevIndex = currentStepIndex - 1;
    if (prevIndex >= 0) {
      setCurrentStep(steps[prevIndex].id);
    }
  };

  const canProceed = () => {
    switch (currentStep) {
      case 'trigger':
        return !!rule.trigger?.type;
      case 'conditions':
        return true; // Conditions are optional
      case 'actions':
        return (rule.actions?.length ?? 0) > 0;
      case 'review':
        return !!rule.name && !!rule.trigger?.type && (rule.actions?.length ?? 0) > 0;
      default:
        return false;
    }
  };

  const handleSave = () => {
    if (canProceed()) {
      onSave(rule);
    }
  };

  return (
    <div className="max-w-4xl mx-auto">
      {/* Step Indicator */}
      <nav className="mb-8" aria-label={t('aria.progress')}>
        <ol className="flex items-center">
          {steps.map((step, index) => (
            <li
              key={step.id}
              className={`relative ${index !== steps.length - 1 ? 'pr-8 sm:pr-20 flex-1' : ''}`}
            >
              <div className="flex items-center">
                <button
                  type="button"
                  onClick={() => goToStep(step.id)}
                  disabled={isLoading}
                  className={`relative flex h-10 w-10 items-center justify-center rounded-full ${
                    index < currentStepIndex
                      ? 'bg-blue-600 hover:bg-blue-700'
                      : index === currentStepIndex
                        ? 'border-2 border-blue-600 bg-white'
                        : 'border-2 border-gray-300 bg-white'
                  }`}
                >
                  {index < currentStepIndex ? (
                    <svg
                      className="h-5 w-5 text-white"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                      aria-hidden="true"
                    >
                      <path
                        fillRule="evenodd"
                        d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                        clipRule="evenodd"
                      />
                    </svg>
                  ) : (
                    <span
                      className={
                        index === currentStepIndex ? 'text-blue-600 font-medium' : 'text-gray-500'
                      }
                    >
                      {index + 1}
                    </span>
                  )}
                </button>
                <span className="ml-4 text-sm font-medium text-gray-900 hidden sm:block">
                  {step.label}
                </span>
              </div>
              {index !== steps.length - 1 && (
                <div
                  className="absolute top-5 left-10 -ml-px mt-0.5 h-0.5 w-full sm:w-20 bg-gray-300"
                  aria-hidden="true"
                >
                  <div
                    className={`h-full ${index < currentStepIndex ? 'bg-blue-600' : ''}`}
                    style={{ width: index < currentStepIndex ? '100%' : '0%' }}
                  />
                </div>
              )}
            </li>
          ))}
        </ol>
      </nav>

      {/* Step Content */}
      <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6 mb-6">
        <div className="mb-6">
          <h2 className="text-lg font-semibold text-gray-900">{steps[currentStepIndex].label}</h2>
          <p className="text-sm text-gray-500">{steps[currentStepIndex].description}</p>
        </div>

        {currentStep === 'trigger' && (
          <TriggerSelector
            value={rule.trigger}
            onChange={handleTriggerChange}
            disabled={isLoading}
          />
        )}

        {currentStep === 'conditions' && (
          <ConditionBuilder
            conditions={rule.trigger?.conditions ?? []}
            onChange={handleConditionsChange}
            disabled={isLoading}
          />
        )}

        {currentStep === 'actions' && (
          <ActionBuilder
            actions={rule.actions ?? []}
            onChange={handleActionsChange}
            disabled={isLoading}
          />
        )}

        {currentStep === 'review' && (
          <div className="space-y-6">
            {/* Rule Name & Description */}
            <div className="space-y-4">
              <div>
                <label htmlFor="rule-name" className="block text-sm font-medium text-gray-700">
                  Rule Name *
                </label>
                <input
                  id="rule-name"
                  type="text"
                  value={rule.name ?? ''}
                  onChange={(e) => updateRule({ name: e.target.value })}
                  disabled={isLoading}
                  placeholder="e.g., Auto-assign high priority faults"
                  className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div>
                <label
                  htmlFor="rule-description"
                  className="block text-sm font-medium text-gray-700"
                >
                  Description
                </label>
                <textarea
                  id="rule-description"
                  value={rule.description ?? ''}
                  onChange={(e) => updateRule({ description: e.target.value })}
                  disabled={isLoading}
                  rows={2}
                  placeholder="Describe what this automation does..."
                  className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>
              <div className="flex items-center gap-2">
                <input
                  id="rule-enabled"
                  type="checkbox"
                  checked={rule.isEnabled ?? true}
                  onChange={(e) => updateRule({ isEnabled: e.target.checked })}
                  disabled={isLoading}
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label htmlFor="rule-enabled" className="text-sm text-gray-700">
                  Enable this automation immediately
                </label>
              </div>
            </div>

            {/* Summary */}
            <div className="border-t pt-6">
              <h3 className="text-sm font-medium text-gray-900 mb-4">Summary</h3>
              <div className="bg-gray-50 rounded-lg p-4 space-y-4">
                {/* Trigger Summary */}
                <div className="flex items-start gap-3">
                  <span className="text-lg">⚡</span>
                  <div>
                    <p className="text-sm font-medium text-gray-700">Trigger</p>
                    <p className="text-sm text-gray-500">
                      {rule.trigger?.name ?? rule.trigger?.type ?? 'Not configured'}
                    </p>
                  </div>
                </div>

                {/* Conditions Summary */}
                <div className="flex items-start gap-3">
                  <span className="text-lg">🔀</span>
                  <div>
                    <p className="text-sm font-medium text-gray-700">Conditions</p>
                    <p className="text-sm text-gray-500">
                      {(rule.trigger?.conditions?.length ?? 0) > 0
                        ? `${rule.trigger?.conditions?.length} condition(s)`
                        : 'No conditions (runs for all triggers)'}
                    </p>
                  </div>
                </div>

                {/* Actions Summary */}
                <div className="flex items-start gap-3">
                  <span className="text-lg">🎯</span>
                  <div>
                    <p className="text-sm font-medium text-gray-700">Actions</p>
                    <p className="text-sm text-gray-500">
                      {(rule.actions?.length ?? 0) > 0
                        ? `${rule.actions?.length} action(s)`
                        : 'No actions configured'}
                    </p>
                    {rule.actions && rule.actions.length > 0 && (
                      <ul className="mt-1 text-xs text-gray-400">
                        {rule.actions.map((action, i) => (
                          <li key={`action-summary-${action.type}-${action.order}`}>
                            {i + 1}. {action.name ?? action.type}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Navigation Buttons */}
      <div className="flex justify-between">
        <div>
          {currentStepIndex > 0 ? (
            <button
              type="button"
              onClick={goPrevious}
              disabled={isLoading}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
            >
              Previous
            </button>
          ) : (
            <button
              type="button"
              onClick={onCancel}
              disabled={isLoading}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
            >
              Cancel
            </button>
          )}
        </div>

        <div className="flex gap-3">
          {currentStep !== 'review' ? (
            <button
              type="button"
              onClick={goNext}
              disabled={isLoading || !canProceed()}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Next
            </button>
          ) : (
            <button
              type="button"
              onClick={handleSave}
              disabled={isLoading || !canProceed()}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {isLoading && (
                <svg
                  className="animate-spin h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  />
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                  />
                </svg>
              )}
              {initialRule?.id ? 'Save Changes' : 'Create Automation'}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
