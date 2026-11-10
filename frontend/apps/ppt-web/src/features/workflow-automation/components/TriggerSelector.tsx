/**
 * TriggerSelector Component
 *
 * Select and configure automation triggers.
 * Part of Story 43.1: Automation Rule Builder.
 */

import type {
  AutomationTrigger,
  EventTriggerType,
  TimeTriggerConfig,
  TriggerType,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

interface TriggerSelectorProps {
  value?: Partial<AutomationTrigger>;
  onChange: (trigger: Partial<AutomationTrigger>) => void;
  disabled?: boolean;
}

const triggerTypes: {
  value: TriggerType;
  labelKey: string;
  descriptionKey: string;
  icon: string;
}[] = [
  {
    value: 'time_based',
    labelKey: 'automation.triggerType.time_based',
    descriptionKey: 'automation.triggerSelector.timeBasedDesc',
    icon: '🕐',
  },
  {
    value: 'event_based',
    labelKey: 'automation.triggerType.event_based',
    descriptionKey: 'automation.triggerSelector.eventBasedDesc',
    icon: '⚡',
  },
  {
    value: 'condition_based',
    labelKey: 'automation.triggerType.condition_based',
    descriptionKey: 'automation.triggerSelector.conditionBasedDesc',
    icon: '🔀',
  },
  {
    value: 'manual',
    labelKey: 'automation.triggerType.manual',
    descriptionKey: 'automation.triggerSelector.manualDesc',
    icon: '👆',
  },
];

/** Stable event-category ids, each mapped to its heading i18n key. */
const eventCategories: { id: string; labelKey: string }[] = [
  { id: 'faults', labelKey: 'automation.triggerSelector.categoryFaults' },
  { id: 'payments', labelKey: 'automation.triggerSelector.categoryPayments' },
  { id: 'documents', labelKey: 'automation.triggerSelector.categoryDocuments' },
  { id: 'announcements', labelKey: 'automation.triggerSelector.categoryAnnouncements' },
  { id: 'voting', labelKey: 'automation.triggerSelector.categoryVoting' },
  { id: 'guests', labelKey: 'automation.triggerSelector.categoryGuests' },
  { id: 'maintenance', labelKey: 'automation.triggerSelector.categoryMaintenance' },
  { id: 'meters', labelKey: 'automation.triggerSelector.categoryMeters' },
  { id: 'leases', labelKey: 'automation.triggerSelector.categoryLeases' },
];

const eventTypes: { value: EventTriggerType; labelKey: string; category: string }[] = [
  {
    value: 'fault_created',
    labelKey: 'automation.triggerSelector.eventFaultCreated',
    category: 'faults',
  },
  {
    value: 'fault_status_changed',
    labelKey: 'automation.triggerSelector.eventFaultStatusChanged',
    category: 'faults',
  },
  {
    value: 'payment_received',
    labelKey: 'automation.triggerSelector.eventPaymentReceived',
    category: 'payments',
  },
  {
    value: 'payment_overdue',
    labelKey: 'automation.triggerSelector.eventPaymentOverdue',
    category: 'payments',
  },
  {
    value: 'document_uploaded',
    labelKey: 'automation.triggerSelector.eventDocumentUploaded',
    category: 'documents',
  },
  {
    value: 'announcement_published',
    labelKey: 'automation.triggerSelector.eventAnnouncementPublished',
    category: 'announcements',
  },
  {
    value: 'vote_started',
    labelKey: 'automation.triggerSelector.eventVoteStarted',
    category: 'voting',
  },
  {
    value: 'vote_ended',
    labelKey: 'automation.triggerSelector.eventVoteEnded',
    category: 'voting',
  },
  {
    value: 'guest_registered',
    labelKey: 'automation.triggerSelector.eventGuestRegistered',
    category: 'guests',
  },
  {
    value: 'maintenance_scheduled',
    labelKey: 'automation.triggerSelector.eventMaintenanceScheduled',
    category: 'maintenance',
  },
  {
    value: 'meter_reading_due',
    labelKey: 'automation.triggerSelector.eventMeterReadingDue',
    category: 'meters',
  },
  {
    value: 'lease_expiring',
    labelKey: 'automation.triggerSelector.eventLeaseExpiring',
    category: 'leases',
  },
];

const schedulePresets = [
  { labelKey: 'automation.triggerSelector.presetEveryHour', value: '0 * * * *' },
  { labelKey: 'automation.triggerSelector.presetEveryDay9', value: '0 9 * * *' },
  { labelKey: 'automation.triggerSelector.presetEveryMonday9', value: '0 9 * * 1' },
  { labelKey: 'automation.triggerSelector.presetFirstOfMonth', value: '0 9 1 * *' },
  { labelKey: 'automation.triggerSelector.presetCustom', value: 'custom' },
];

export function TriggerSelector({ value, onChange, disabled }: TriggerSelectorProps) {
  const { t } = useTranslation();
  const [showScheduleCustom, setShowScheduleCustom] = useState(false);
  const [customCron, setCustomCron] = useState('');

  const handleTypeSelect = (type: TriggerType) => {
    const labelKey = triggerTypes.find((item) => item.value === type)?.labelKey;
    onChange({
      ...value,
      type,
      name: labelKey ? t(labelKey) : '',
      timeConfig:
        type === 'time_based'
          ? { schedule: '0 9 * * *', timezone: 'Europe/Bratislava' }
          : undefined,
      eventConfig: type === 'event_based' ? { eventType: 'fault_created' } : undefined,
    });
  };

  const handleEventTypeChange = (eventType: EventTriggerType) => {
    onChange({
      ...value,
      eventConfig: { ...value?.eventConfig, eventType },
    });
  };

  const handleScheduleChange = (schedule: string) => {
    if (schedule === 'custom') {
      setShowScheduleCustom(true);
      return;
    }
    setShowScheduleCustom(false);
    onChange({
      ...value,
      timeConfig: {
        ...value?.timeConfig,
        schedule,
        timezone: value?.timeConfig?.timezone ?? 'Europe/Bratislava',
      } as TimeTriggerConfig,
    });
  };

  const handleCustomCronSave = () => {
    if (customCron) {
      onChange({
        ...value,
        timeConfig: {
          schedule: customCron,
          timezone: value?.timeConfig?.timezone ?? 'Europe/Bratislava',
        },
      });
      setShowScheduleCustom(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Trigger Type Selection */}
      <div>
        <span className="block text-sm font-medium text-gray-700 mb-3">
          {t('automation.triggerSelector.triggerTypeLabel')}
        </span>
        <div className="grid grid-cols-2 gap-3">
          {triggerTypes.map((trigger) => (
            <button
              key={trigger.value}
              type="button"
              disabled={disabled}
              onClick={() => handleTypeSelect(trigger.value)}
              className={`flex flex-col items-start p-4 border rounded-lg transition-colors text-left ${
                value?.type === trigger.value
                  ? 'border-blue-500 bg-blue-50 ring-1 ring-blue-500'
                  : 'border-gray-200 hover:border-gray-300'
              } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            >
              <span className="text-2xl mb-2">{trigger.icon}</span>
              <span className="font-medium text-gray-900">{t(trigger.labelKey)}</span>
              <span className="text-xs text-gray-500 mt-1">{t(trigger.descriptionKey)}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Time-based Configuration */}
      {value?.type === 'time_based' && (
        <div className="border-t pt-6">
          <span className="block text-sm font-medium text-gray-700 mb-3">
            {t('automation.triggerSelector.scheduleLabel')}
          </span>
          <div className="space-y-3">
            {schedulePresets.map((preset) => (
              <label
                key={preset.value}
                className={`flex items-center p-3 border rounded-lg cursor-pointer transition-colors ${
                  value?.timeConfig?.schedule === preset.value ||
                  (preset.value === 'custom' && showScheduleCustom)
                    ? 'border-blue-500 bg-blue-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <input
                  type="radio"
                  name="schedule"
                  value={preset.value}
                  checked={value?.timeConfig?.schedule === preset.value}
                  onChange={() => handleScheduleChange(preset.value)}
                  disabled={disabled}
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
                />
                <span className="ml-3 text-sm text-gray-900">{t(preset.labelKey)}</span>
              </label>
            ))}

            {showScheduleCustom && (
              <div className="flex gap-2 mt-3">
                <input
                  type="text"
                  value={customCron}
                  onChange={(e) => setCustomCron(e.target.value)}
                  placeholder="0 9 * * *"
                  disabled={disabled}
                  className="flex-1 px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
                />
                <button
                  type="button"
                  onClick={handleCustomCronSave}
                  disabled={disabled || !customCron}
                  className="px-4 py-2 bg-blue-600 text-white rounded-md text-sm hover:bg-blue-700 disabled:opacity-50"
                >
                  {t('automation.triggerSelector.apply')}
                </button>
              </div>
            )}

            <div className="mt-4">
              <label htmlFor="timezone" className="block text-sm font-medium text-gray-700 mb-1">
                {t('automation.triggerSelector.timezoneLabel')}
              </label>
              <select
                id="timezone"
                value={value?.timeConfig?.timezone ?? 'Europe/Bratislava'}
                onChange={(e) =>
                  onChange({
                    ...value,
                    timeConfig: {
                      ...value?.timeConfig,
                      timezone: e.target.value,
                    } as TimeTriggerConfig,
                  })
                }
                disabled={disabled}
                className="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:ring-blue-500 focus:border-blue-500"
              >
                <option value="Europe/Bratislava">Europe/Bratislava (CET)</option>
                <option value="Europe/Prague">Europe/Prague (CET)</option>
                <option value="Europe/Vienna">Europe/Vienna (CET)</option>
                <option value="Europe/Berlin">Europe/Berlin (CET)</option>
                <option value="UTC">UTC</option>
              </select>
            </div>
          </div>
        </div>
      )}

      {/* Event-based Configuration */}
      {value?.type === 'event_based' && (
        <div className="border-t pt-6">
          <span className="block text-sm font-medium text-gray-700 mb-3">
            {t('automation.triggerSelector.eventTypeLabel')}
          </span>
          <div className="space-y-4">
            {eventCategories.map((category) => {
              const categoryEvents = eventTypes.filter((e) => e.category === category.id);
              if (categoryEvents.length === 0) return null;

              return (
                <div key={category.id}>
                  <h4 className="text-xs font-medium text-gray-500 uppercase mb-2">
                    {t(category.labelKey)}
                  </h4>
                  <div className="grid grid-cols-2 gap-2">
                    {categoryEvents.map((event) => (
                      <label
                        key={event.value}
                        className={`flex items-center p-2 border rounded cursor-pointer transition-colors ${
                          value?.eventConfig?.eventType === event.value
                            ? 'border-blue-500 bg-blue-50'
                            : 'border-gray-200 hover:border-gray-300'
                        }`}
                      >
                        <input
                          type="radio"
                          name="eventType"
                          value={event.value}
                          checked={value?.eventConfig?.eventType === event.value}
                          onChange={() => handleEventTypeChange(event.value)}
                          disabled={disabled}
                          className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
                        />
                        <span className="ml-2 text-sm text-gray-900">{t(event.labelKey)}</span>
                      </label>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Manual Trigger Info */}
      {value?.type === 'manual' && (
        <div className="border-t pt-6">
          <div className="bg-gray-50 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <span className="text-2xl">👆</span>
              <div>
                <p className="text-sm text-gray-700">
                  {t('automation.triggerSelector.manualInfo')}
                </p>
                <p className="text-xs text-gray-500 mt-2">
                  {t('automation.triggerSelector.manualInfoHint')}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Condition-based Info */}
      {value?.type === 'condition_based' && (
        <div className="border-t pt-6">
          <div className="bg-amber-50 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <span className="text-2xl">🔀</span>
              <div>
                <p className="text-sm text-gray-700">
                  {t('automation.triggerSelector.conditionInfo')}
                </p>
                <p className="text-xs text-gray-500 mt-2">
                  {t('automation.triggerSelector.conditionInfoHint')}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
