/**
 * ActionBuilder Component
 *
 * Build and configure automation actions.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type { ActionType, AutomationAction } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface ActionBuilderProps {
  actions: AutomationAction[];
  onChange: (actions: AutomationAction[]) => void;
  disabled?: boolean;
}

const actionTypes: { value: ActionType; labelKey: string; icon: string; descriptionKey: string }[] =
  [
    {
      value: 'send_notification',
      labelKey: 'automation.actionBuilder.typeSendNotification',
      icon: '🔔',
      descriptionKey: 'automation.actionBuilder.typeSendNotificationDesc',
    },
    {
      value: 'send_email',
      labelKey: 'automation.actionBuilder.typeSendEmail',
      icon: '📧',
      descriptionKey: 'automation.actionBuilder.typeSendEmailDesc',
    },
    {
      value: 'create_task',
      labelKey: 'automation.actionBuilder.typeCreateTask',
      icon: '✅',
      descriptionKey: 'automation.actionBuilder.typeCreateTaskDesc',
    },
    {
      value: 'update_status',
      labelKey: 'automation.actionBuilder.typeUpdateStatus',
      icon: '🔄',
      descriptionKey: 'automation.actionBuilder.typeUpdateStatusDesc',
    },
    {
      value: 'assign_user',
      labelKey: 'automation.actionBuilder.typeAssignUser',
      icon: '👤',
      descriptionKey: 'automation.actionBuilder.typeAssignUserDesc',
    },
    {
      value: 'add_tag',
      labelKey: 'automation.actionBuilder.typeAddTag',
      icon: '🏷️',
      descriptionKey: 'automation.actionBuilder.typeAddTagDesc',
    },
    {
      value: 'webhook',
      labelKey: 'automation.actionBuilder.typeWebhook',
      icon: '🔗',
      descriptionKey: 'automation.actionBuilder.typeWebhookDesc',
    },
    {
      value: 'delay',
      labelKey: 'automation.actionBuilder.typeDelay',
      icon: '⏱️',
      descriptionKey: 'automation.actionBuilder.typeDelayDesc',
    },
  ];

export function ActionBuilder({ actions, onChange, disabled }: ActionBuilderProps) {
  const { t } = useTranslation();
  const [showActionPicker, setShowActionPicker] = useState(false);

  const addAction = (type: ActionType) => {
    const actionConfig = actionTypes.find((a) => a.value === type);
    onChange([
      ...actions,
      {
        type,
        name: actionConfig?.labelKey ? t(actionConfig.labelKey) : type,
        config: {},
        order: actions.length,
      },
    ]);
    setShowActionPicker(false);
  };

  const updateAction = (index: number, updates: Partial<AutomationAction>) => {
    const newActions = [...actions];
    newActions[index] = { ...newActions[index], ...updates };
    onChange(newActions);
  };

  const removeAction = (index: number) => {
    onChange(actions.filter((_, i) => i !== index));
  };

  const moveAction = (fromIndex: number, toIndex: number) => {
    if (toIndex < 0 || toIndex >= actions.length) return;
    const newActions = [...actions];
    const [moved] = newActions.splice(fromIndex, 1);
    newActions.splice(toIndex, 0, moved);
    onChange(newActions.map((a, i) => ({ ...a, order: i })));
  };

  const getActionIcon = (type: ActionType) => {
    return actionTypes.find((a) => a.value === type)?.icon ?? '⚙️';
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <span className="block text-sm font-medium text-gray-700">
          {t('automation.actionBuilder.title')}
        </span>
        <button
          type="button"
          onClick={() => setShowActionPicker(true)}
          disabled={disabled}
          className="inline-flex items-center px-3 py-1 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
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
          {t('automation.actionBuilder.add')}
        </button>
      </div>

      {actions.length === 0 ? (
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
              d="M13 10V3L4 14h7v7l9-11h-7z"
            />
          </svg>
          <p className="mt-2 text-sm text-gray-500">{t('automation.actionBuilder.emptyTitle')}</p>
          <p className="text-xs text-gray-400">{t('automation.actionBuilder.emptyBody')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {actions.map((action, index) => (
            <div
              key={`action-${action.type}-${action.name}-${action.order}`}
              className="flex items-start gap-3 p-4 bg-gray-50 rounded-lg border border-gray-200"
            >
              <div className="flex flex-col items-center gap-1">
                <span className="text-2xl">{getActionIcon(action.type)}</span>
                <span className="text-xs text-gray-400 font-medium">{index + 1}</span>
              </div>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-2">
                  <h4 className="font-medium text-gray-900">{action.name}</h4>
                  <span className="text-xs text-gray-400 capitalize">
                    {action.type.replace(/_/g, ' ')}
                  </span>
                </div>

                {/* Action-specific config UI */}
                {action.type === 'send_notification' && (
                  <div className="space-y-2">
                    <input
                      type="text"
                      value={(action.config?.title as string) ?? ''}
                      onChange={(e) =>
                        updateAction(index, { config: { ...action.config, title: e.target.value } })
                      }
                      disabled={disabled}
                      placeholder={t('automation.actionBuilder.notificationTitlePlaceholder')}
                      className="w-full px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                    />
                    <textarea
                      value={(action.config?.message as string) ?? ''}
                      onChange={(e) =>
                        updateAction(index, {
                          config: { ...action.config, message: e.target.value },
                        })
                      }
                      disabled={disabled}
                      placeholder={t('automation.actionBuilder.notificationMessagePlaceholder')}
                      rows={2}
                      className="w-full px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                    />
                  </div>
                )}

                {action.type === 'send_email' && (
                  <div className="space-y-2">
                    <input
                      type="text"
                      value={(action.config?.to as string) ?? ''}
                      onChange={(e) =>
                        updateAction(index, { config: { ...action.config, to: e.target.value } })
                      }
                      disabled={disabled}
                      placeholder={t('automation.actionBuilder.emailToPlaceholder')}
                      className="w-full px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                    />
                    <input
                      type="text"
                      value={(action.config?.subject as string) ?? ''}
                      onChange={(e) =>
                        updateAction(index, {
                          config: { ...action.config, subject: e.target.value },
                        })
                      }
                      disabled={disabled}
                      placeholder={t('automation.actionBuilder.emailSubjectPlaceholder')}
                      className="w-full px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                    />
                  </div>
                )}

                {action.type === 'delay' && (
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={(action.config?.duration as number) ?? 0}
                      onChange={(e) =>
                        updateAction(index, {
                          config: { ...action.config, duration: Number(e.target.value) },
                        })
                      }
                      disabled={disabled}
                      min={1}
                      className="w-20 px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                    />
                    <select
                      value={(action.config?.unit as string) ?? 'minutes'}
                      onChange={(e) =>
                        updateAction(index, { config: { ...action.config, unit: e.target.value } })
                      }
                      disabled={disabled}
                      className="px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                    >
                      <option value="minutes">{t('automation.actionBuilder.unitMinutes')}</option>
                      <option value="hours">{t('automation.actionBuilder.unitHours')}</option>
                      <option value="days">{t('automation.actionBuilder.unitDays')}</option>
                    </select>
                  </div>
                )}

                {action.type === 'webhook' && (
                  <input
                    type="url"
                    value={(action.config?.url as string) ?? ''}
                    onChange={(e) =>
                      updateAction(index, { config: { ...action.config, url: e.target.value } })
                    }
                    disabled={disabled}
                    placeholder={t('automation.actionBuilder.webhookPlaceholder')}
                    pattern="https://.*"
                    title={t('automation.actionBuilder.webhookTitle')}
                    className="w-full px-2 py-1.5 text-sm border border-gray-300 rounded-md"
                  />
                )}
              </div>

              {/* Action controls */}
              <div className="flex flex-col gap-1">
                <button
                  type="button"
                  onClick={() => moveAction(index, index - 1)}
                  disabled={disabled || index === 0}
                  className="p-1 text-gray-400 hover:text-gray-600 disabled:opacity-30"
                  title={t('automation.actionBuilder.moveUp')}
                >
                  <svg
                    className="w-4 h-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M5 15l7-7 7 7"
                    />
                  </svg>
                </button>
                <button
                  type="button"
                  onClick={() => moveAction(index, index + 1)}
                  disabled={disabled || index === actions.length - 1}
                  className="p-1 text-gray-400 hover:text-gray-600 disabled:opacity-30"
                  title={t('automation.actionBuilder.moveDown')}
                >
                  <svg
                    className="w-4 h-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M19 9l-7 7-7-7"
                    />
                  </svg>
                </button>
                <button
                  type="button"
                  onClick={() => removeAction(index)}
                  disabled={disabled}
                  className="p-1 text-gray-400 hover:text-red-500 disabled:opacity-50"
                  title={t('automation.actionBuilder.removeAction')}
                >
                  <svg
                    className="w-4 h-4"
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
            </div>
          ))}
        </div>
      )}

      {/* Action Picker Modal */}
      {showActionPicker && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg shadow-xl max-w-lg w-full mx-4 p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-900">
                {t('automation.actionBuilder.add')}
              </h3>
              <button
                type="button"
                onClick={() => setShowActionPicker(false)}
                className="text-gray-400 hover:text-gray-500"
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
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>
            <div className="grid grid-cols-2 gap-3">
              {actionTypes.map((actionType) => (
                <button
                  key={actionType.value}
                  type="button"
                  onClick={() => addAction(actionType.value)}
                  className="flex items-start gap-3 p-3 border border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors text-left"
                >
                  <span className="text-2xl">{actionType.icon}</span>
                  <div>
                    <p className="font-medium text-gray-900">{t(actionType.labelKey)}</p>
                    <p className="text-xs text-gray-500">{t(actionType.descriptionKey)}</p>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
