/**
 * ConditionBuilder Component
 *
 * Build and configure automation conditions.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type { ConditionOperator, TriggerCondition } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface ConditionBuilderProps {
  conditions: TriggerCondition[];
  onChange: (conditions: TriggerCondition[]) => void;
  disabled?: boolean;
}

const fieldOptions = [
  {
    value: 'fault.category',
    labelKey: 'automation.conditionBuilder.fieldFaultCategory',
    type: 'select',
  },
  {
    value: 'fault.priority',
    labelKey: 'automation.conditionBuilder.fieldFaultPriority',
    type: 'select',
  },
  {
    value: 'fault.status',
    labelKey: 'automation.conditionBuilder.fieldFaultStatus',
    type: 'select',
  },
  {
    value: 'fault.location',
    labelKey: 'automation.conditionBuilder.fieldFaultLocation',
    type: 'text',
  },
  {
    value: 'payment.amount',
    labelKey: 'automation.conditionBuilder.fieldPaymentAmount',
    type: 'number',
  },
  {
    value: 'payment.daysOverdue',
    labelKey: 'automation.conditionBuilder.fieldDaysOverdue',
    type: 'number',
  },
  {
    value: 'document.type',
    labelKey: 'automation.conditionBuilder.fieldDocumentType',
    type: 'select',
  },
  {
    value: 'document.size',
    labelKey: 'automation.conditionBuilder.fieldDocumentSize',
    type: 'number',
  },
  { value: 'user.role', labelKey: 'automation.conditionBuilder.fieldUserRole', type: 'select' },
  { value: 'building.id', labelKey: 'automation.conditionBuilder.fieldBuilding', type: 'select' },
  { value: 'unit.id', labelKey: 'automation.conditionBuilder.fieldUnit', type: 'select' },
];

const operatorOptions: { value: ConditionOperator; labelKey: string; types: string[] }[] = [
  {
    value: 'equals',
    labelKey: 'automation.conditionBuilder.opEquals',
    types: ['text', 'number', 'select'],
  },
  {
    value: 'not_equals',
    labelKey: 'automation.conditionBuilder.opNotEquals',
    types: ['text', 'number', 'select'],
  },
  { value: 'contains', labelKey: 'automation.conditionBuilder.opContains', types: ['text'] },
  { value: 'not_contains', labelKey: 'automation.conditionBuilder.opNotContains', types: ['text'] },
  {
    value: 'greater_than',
    labelKey: 'automation.conditionBuilder.opGreaterThan',
    types: ['number'],
  },
  { value: 'less_than', labelKey: 'automation.conditionBuilder.opLessThan', types: ['number'] },
  {
    value: 'greater_than_or_equals',
    labelKey: 'automation.conditionBuilder.opGreaterThanOrEquals',
    types: ['number'],
  },
  {
    value: 'less_than_or_equals',
    labelKey: 'automation.conditionBuilder.opLessThanOrEquals',
    types: ['number'],
  },
  {
    value: 'is_empty',
    labelKey: 'automation.conditionBuilder.opIsEmpty',
    types: ['text', 'select'],
  },
  {
    value: 'is_not_empty',
    labelKey: 'automation.conditionBuilder.opIsNotEmpty',
    types: ['text', 'select'],
  },
  { value: 'in_list', labelKey: 'automation.conditionBuilder.opInList', types: ['text', 'select'] },
  {
    value: 'not_in_list',
    labelKey: 'automation.conditionBuilder.opNotInList',
    types: ['text', 'select'],
  },
];

const fieldValueOptions: Record<string, { value: string; labelKey: string }[]> = {
  'fault.category': [
    { value: 'electrical', labelKey: 'automation.conditionBuilder.valFaultCategoryElectrical' },
    { value: 'plumbing', labelKey: 'automation.conditionBuilder.valFaultCategoryPlumbing' },
    { value: 'hvac', labelKey: 'automation.conditionBuilder.valFaultCategoryHvac' },
    { value: 'structural', labelKey: 'automation.conditionBuilder.valFaultCategoryStructural' },
    { value: 'other', labelKey: 'automation.conditionBuilder.valFaultCategoryOther' },
  ],
  'fault.priority': [
    { value: 'low', labelKey: 'automation.conditionBuilder.valFaultPriorityLow' },
    { value: 'medium', labelKey: 'automation.conditionBuilder.valFaultPriorityMedium' },
    { value: 'high', labelKey: 'automation.conditionBuilder.valFaultPriorityHigh' },
    { value: 'critical', labelKey: 'automation.conditionBuilder.valFaultPriorityCritical' },
  ],
  'fault.status': [
    { value: 'reported', labelKey: 'automation.conditionBuilder.valFaultStatusReported' },
    { value: 'acknowledged', labelKey: 'automation.conditionBuilder.valFaultStatusAcknowledged' },
    { value: 'in_progress', labelKey: 'automation.conditionBuilder.valFaultStatusInProgress' },
    { value: 'resolved', labelKey: 'automation.conditionBuilder.valFaultStatusResolved' },
    { value: 'closed', labelKey: 'automation.conditionBuilder.valFaultStatusClosed' },
  ],
  'document.type': [
    { value: 'contract', labelKey: 'automation.conditionBuilder.valDocumentTypeContract' },
    { value: 'invoice', labelKey: 'automation.conditionBuilder.valDocumentTypeInvoice' },
    { value: 'report', labelKey: 'automation.conditionBuilder.valDocumentTypeReport' },
    { value: 'minutes', labelKey: 'automation.conditionBuilder.valDocumentTypeMinutes' },
    { value: 'other', labelKey: 'automation.conditionBuilder.valDocumentTypeOther' },
  ],
  'user.role': [
    { value: 'owner', labelKey: 'automation.conditionBuilder.valUserRoleOwner' },
    { value: 'tenant', labelKey: 'automation.conditionBuilder.valUserRoleTenant' },
    { value: 'manager', labelKey: 'automation.conditionBuilder.valUserRoleManager' },
    { value: 'admin', labelKey: 'automation.conditionBuilder.valUserRoleAdmin' },
  ],
};

export function ConditionBuilder({ conditions, onChange, disabled }: ConditionBuilderProps) {
  const { t } = useTranslation();
  const addCondition = () => {
    onChange([...conditions, { field: 'fault.category', operator: 'equals', value: '' }]);
  };

  const updateCondition = (index: number, updates: Partial<TriggerCondition>) => {
    const newConditions = [...conditions];
    newConditions[index] = { ...newConditions[index], ...updates };
    onChange(newConditions);
  };

  const removeCondition = (index: number) => {
    onChange(conditions.filter((_, i) => i !== index));
  };

  const getFieldType = (field: string) => {
    return fieldOptions.find((f) => f.value === field)?.type ?? 'text';
  };

  const getAvailableOperators = (field: string) => {
    const fieldType = getFieldType(field);
    return operatorOptions.filter((op) => op.types.includes(fieldType));
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="block text-sm font-medium text-gray-700">
          {t('automation.conditionBuilder.title')}
        </span>
        <button
          type="button"
          onClick={addCondition}
          disabled={disabled}
          className="inline-flex items-center px-3 py-1 text-sm bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 disabled:opacity-50"
        >
          <svg
            className="w-4 h-4 mr-1"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          {t('automation.conditionBuilder.add')}
        </button>
      </div>

      {conditions.length === 0 ? (
        <div className="bg-gray-50 rounded-lg p-6 text-center">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
            />
          </svg>
          <p className="mt-2 text-sm text-gray-500">
            {t('automation.conditionBuilder.emptyTitle')}
          </p>
          <p className="text-xs text-gray-400">{t('automation.conditionBuilder.emptyBody')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {conditions.map((condition, index) => (
            <div
              key={`condition-${condition.field}-${condition.operator}-${index}`}
              className="flex items-start gap-2 p-3 bg-gray-50 rounded-lg"
            >
              {index > 0 && (
                <span className="self-center px-2 py-1 text-xs font-medium text-gray-500 bg-gray-200 rounded">
                  {t('automation.conditionBuilder.and')}
                </span>
              )}

              <div className="flex-1 grid grid-cols-3 gap-2">
                {/* Field Selector */}
                <select
                  value={condition.field}
                  onChange={(e) => updateCondition(index, { field: e.target.value, value: '' })}
                  disabled={disabled}
                  className="px-2 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                >
                  {fieldOptions.map((field) => (
                    <option key={field.value} value={field.value}>
                      {t(field.labelKey)}
                    </option>
                  ))}
                </select>

                {/* Operator Selector */}
                <select
                  value={condition.operator}
                  onChange={(e) =>
                    updateCondition(index, { operator: e.target.value as ConditionOperator })
                  }
                  disabled={disabled}
                  className="px-2 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                >
                  {getAvailableOperators(condition.field).map((op) => (
                    <option key={op.value} value={op.value}>
                      {t(op.labelKey)}
                    </option>
                  ))}
                </select>

                {/* Value Input */}
                {!['is_empty', 'is_not_empty'].includes(condition.operator) &&
                  (fieldValueOptions[condition.field] ? (
                    <select
                      value={condition.value as string}
                      onChange={(e) => updateCondition(index, { value: e.target.value })}
                      disabled={disabled}
                      className="px-2 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                    >
                      <option value="">{t('automation.conditionBuilder.selectPlaceholder')}</option>
                      {fieldValueOptions[condition.field].map((opt) => (
                        <option key={opt.value} value={opt.value}>
                          {t(opt.labelKey)}
                        </option>
                      ))}
                    </select>
                  ) : getFieldType(condition.field) === 'number' ? (
                    <input
                      type="number"
                      value={condition.value as number}
                      onChange={(e) => updateCondition(index, { value: Number(e.target.value) })}
                      disabled={disabled}
                      placeholder={t('automation.conditionBuilder.valuePlaceholder')}
                      className="px-2 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                    />
                  ) : (
                    <input
                      type="text"
                      value={condition.value as string}
                      onChange={(e) => updateCondition(index, { value: e.target.value })}
                      disabled={disabled}
                      placeholder={t('automation.conditionBuilder.valuePlaceholder')}
                      className="px-2 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-blue-500 focus:border-blue-500"
                    />
                  ))}
              </div>

              <button
                type="button"
                onClick={() => removeCondition(index)}
                disabled={disabled}
                className="p-1 text-gray-400 hover:text-red-500 disabled:opacity-50"
                title={t('automation.conditionBuilder.removeCondition')}
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              </button>
            </div>
          ))}
        </div>
      )}

      {conditions.length > 0 && (
        <p className="text-xs text-gray-500 mt-2">
          {t('automation.conditionBuilder.allMustBeTrue')}
        </p>
      )}
    </div>
  );
}
