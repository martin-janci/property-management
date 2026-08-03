/**
 * TemplatePreviewModal Component
 *
 * Preview automation template details before using.
 * Part of Story 43.2: Template Library.
 */

import type { AutomationTemplate } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface TemplatePreviewModalProps {
  template: AutomationTemplate;
  onClose: () => void;
  onUse: (template: AutomationTemplate) => void;
}

const categoryColors: Record<string, string> = {
  faults: 'bg-red-100 text-red-800',
  payments: 'bg-green-100 text-green-800',
  communications: 'bg-blue-100 text-blue-800',
  documents: 'bg-purple-100 text-purple-800',
  maintenance: 'bg-orange-100 text-orange-800',
  general: 'bg-gray-100 text-gray-800',
};

const categoryIcons: Record<string, string> = {
  faults: '🔧',
  payments: '💰',
  communications: '📢',
  documents: '📄',
  maintenance: '🛠️',
  general: '⚙️',
};

export function TemplatePreviewModal({ template, onClose, onUse }: TemplatePreviewModalProps) {
  const { t } = useTranslation();
  const categoryColor = categoryColors[template.category] ?? categoryColors.general;
  const categoryIcon = categoryIcons[template.category] ?? categoryIcons.general;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-start justify-between">
            <div className="flex items-start gap-3">
              <span className="text-4xl">{categoryIcon}</span>
              <div>
                <div className="flex items-center gap-2 flex-wrap">
                  <h2 className="text-xl font-bold text-gray-900">{template.name}</h2>
                  <span
                    className={`px-2 py-0.5 text-xs font-medium rounded-full capitalize ${categoryColor}`}
                  >
                    {template.category}
                  </span>
                  {template.isPopular && (
                    <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-yellow-100 text-yellow-800">
                      Popular
                    </span>
                  )}
                </div>
                <p className="text-sm text-gray-500 mt-1">{template.description}</p>
              </div>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="text-gray-400 hover:text-gray-500"
              aria-label={t('aria.closeTemplatePreview')}
            >
              <svg
                className="w-6 h-6"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Trigger Section */}
          <div>
            <h3 className="text-sm font-medium text-gray-900 mb-3 flex items-center gap-2">
              <span className="text-lg">⚡</span>
              Trigger
            </h3>
            <div className="bg-gray-50 rounded-lg p-4">
              <p className="text-sm text-gray-700">
                {template.triggerType === 'time_based' && (
                  <>
                    <span className="font-medium">Time Based</span> - Runs on a schedule
                    {template.triggerDetails?.schedule && (
                      <span className="text-gray-500"> ({template.triggerDetails.schedule})</span>
                    )}
                  </>
                )}
                {template.triggerType === 'event_based' && (
                  <>
                    <span className="font-medium">Event Based</span> - Runs when{' '}
                    {template.triggerDetails?.eventType ?? 'an event occurs'}
                  </>
                )}
                {template.triggerType === 'condition_based' && (
                  <>
                    <span className="font-medium">Condition Based</span> - Runs when conditions are
                    met
                  </>
                )}
                {template.triggerType === 'manual' && (
                  <>
                    <span className="font-medium">Manual</span> - Runs when triggered manually
                  </>
                )}
              </p>
            </div>
          </div>

          {/* Conditions Section */}
          {template.conditionsPreview && template.conditionsPreview.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-gray-900 mb-3 flex items-center gap-2">
                <span className="text-lg">🔀</span>
                Conditions
              </h3>
              <div className="bg-gray-50 rounded-lg p-4 space-y-2">
                {template.conditionsPreview.map((condition, index) => (
                  <div
                    key={`condition-preview-${condition.slice(0, 20)}-${index}`}
                    className="flex items-center gap-2"
                  >
                    {index > 0 && (
                      <span className="text-xs font-medium text-gray-400 uppercase">AND</span>
                    )}
                    <p className="text-sm text-gray-700">{condition}</p>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Actions Section */}
          <div>
            <h3 className="text-sm font-medium text-gray-900 mb-3 flex items-center gap-2">
              <span className="text-lg">🎯</span>
              Actions ({template.actionsCount ?? 0})
            </h3>
            <div className="bg-gray-50 rounded-lg p-4 space-y-3">
              {template.actionsPreview && template.actionsPreview.length > 0 ? (
                template.actionsPreview.map((action, index) => (
                  <div
                    key={`action-preview-${action.slice(0, 20)}-${index}`}
                    className="flex items-start gap-3"
                  >
                    <span className="flex-shrink-0 w-6 h-6 bg-blue-100 text-blue-700 rounded-full flex items-center justify-center text-xs font-medium">
                      {index + 1}
                    </span>
                    <p className="text-sm text-gray-700">{action}</p>
                  </div>
                ))
              ) : (
                <p className="text-sm text-gray-500">No actions configured</p>
              )}
            </div>
          </div>

          {/* Tags */}
          {template.tags && template.tags.length > 0 && (
            <div>
              <h3 className="text-sm font-medium text-gray-900 mb-3">Tags</h3>
              <div className="flex flex-wrap gap-2">
                {template.tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-3 py-1 text-sm bg-gray-100 text-gray-600 rounded-full"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Usage Stats */}
          {template.usageCount !== undefined && (
            <div className="text-sm text-gray-500">
              Used {template.usageCount} time{template.usageCount !== 1 ? 's' : ''} by other users
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-gray-200 flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => onUse(template)}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
          >
            Use This Template
          </button>
        </div>
      </div>
    </div>
  );
}
