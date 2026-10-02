/**
 * RuleCard Component
 *
 * Display automation rule summary in a card format.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type { AutomationRule } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';

interface RuleCardProps {
  rule: AutomationRule;
  onEdit: (rule: AutomationRule) => void;
  onDelete: (rule: AutomationRule) => void;
  onToggle: (rule: AutomationRule, enabled: boolean) => void;
  onRun?: (rule: AutomationRule) => void;
}

const triggerIcons: Record<string, string> = {
  time_based: '🕐',
  event_based: '⚡',
  condition_based: '🔀',
  manual: '👆',
};

const statusColors = {
  active: 'bg-green-100 text-green-800',
  paused: 'bg-yellow-100 text-yellow-800',
  error: 'bg-red-100 text-red-800',
};

export function RuleCard({ rule, onEdit, onDelete, onToggle, onRun }: RuleCardProps) {
  const { t } = useTranslation();
  const triggerIcon = triggerIcons[rule.trigger?.type ?? ''] ?? '⚙️';
  const status = rule.isEnabled ? 'active' : 'paused';

  return (
    <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3 flex-1 min-w-0">
          <span className="text-2xl flex-shrink-0">{triggerIcon}</span>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-base font-semibold text-gray-900 truncate">{rule.name}</h3>
              <span
                className={`px-2 py-0.5 text-xs font-medium rounded-full ${statusColors[status]}`}
              >
                {status === 'active'
                  ? t('automation.status.active')
                  : t('automation.status.paused')}
              </span>
            </div>
            {rule.description && (
              <p className="text-sm text-gray-500 line-clamp-2 mb-2">{rule.description}</p>
            )}
            <div className="flex flex-wrap gap-2 text-xs text-gray-400">
              <span className="flex items-center gap-1">
                <svg
                  className="w-3 h-3"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M13 10V3L4 14h7v7l9-11h-7z"
                  />
                </svg>
                {rule.trigger?.name ??
                  rule.trigger?.type ??
                  t('automation.ruleCard.unknownTrigger')}
              </span>
              <span>•</span>
              <span>{t('automation.ruleCard.actions', { count: rule.actions?.length ?? 0 })}</span>
              {rule.trigger?.conditions && rule.trigger.conditions.length > 0 && (
                <>
                  <span>•</span>
                  <span>
                    {t('automation.ruleCard.conditions', {
                      count: rule.trigger.conditions.length,
                    })}
                  </span>
                </>
              )}
            </div>
          </div>
        </div>

        {/* Toggle Switch */}
        <div className="flex items-center ml-4">
          <button
            type="button"
            onClick={() => onToggle(rule, !rule.isEnabled)}
            className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
              rule.isEnabled ? 'bg-blue-600' : 'bg-gray-200'
            }`}
            role="switch"
            aria-checked={rule.isEnabled}
            aria-label={t('automation.ruleCard.toggle', { name: rule.name })}
          >
            <span
              className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                rule.isEnabled ? 'translate-x-5' : 'translate-x-0'
              }`}
            />
          </button>
        </div>
      </div>

      {/* Execution Stats */}
      {rule.executionStats && (
        <div className="mt-4 pt-3 border-t border-gray-100">
          <div className="flex gap-4 text-xs">
            <div>
              <span className="text-gray-400">{t('automation.ruleCard.totalRuns')}</span>{' '}
              <span className="font-medium text-gray-700">{rule.executionStats.totalRuns}</span>
            </div>
            <div>
              <span className="text-gray-400">{t('automation.ruleCard.success')}</span>{' '}
              <span className="font-medium text-green-600">
                {rule.executionStats.successfulRuns}
              </span>
            </div>
            <div>
              <span className="text-gray-400">{t('automation.ruleCard.failed')}</span>{' '}
              <span className="font-medium text-red-600">{rule.executionStats.failedRuns}</span>
            </div>
            {rule.executionStats.lastRunAt && (
              <div>
                <span className="text-gray-400">{t('automation.ruleCard.lastRun')}</span>{' '}
                <span className="font-medium text-gray-700">
                  {new Date(rule.executionStats.lastRunAt).toLocaleDateString()}
                </span>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Action Buttons */}
      <div className="mt-4 pt-3 border-t border-gray-100 flex items-center gap-2">
        {rule.trigger?.type === 'manual' && onRun && (
          <button
            type="button"
            onClick={() => onRun(rule)}
            className="inline-flex items-center px-3 py-1.5 text-xs font-medium text-green-700 bg-green-50 rounded-md hover:bg-green-100"
          >
            <svg
              className="w-3.5 h-3.5 mr-1"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
              />
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            {t('automation.ruleCard.runNow')}
          </button>
        )}
        <button
          type="button"
          onClick={() => onEdit(rule)}
          className="inline-flex items-center px-3 py-1.5 text-xs font-medium text-gray-700 bg-gray-50 rounded-md hover:bg-gray-100"
        >
          <svg
            className="w-3.5 h-3.5 mr-1"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
            />
          </svg>
          {t('automation.ruleCard.edit')}
        </button>
        <button
          type="button"
          onClick={() => onDelete(rule)}
          className="inline-flex items-center px-3 py-1.5 text-xs font-medium text-red-700 bg-red-50 rounded-md hover:bg-red-100"
        >
          <svg
            className="w-3.5 h-3.5 mr-1"
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
          {t('automation.ruleCard.delete')}
        </button>
      </div>
    </div>
  );
}
