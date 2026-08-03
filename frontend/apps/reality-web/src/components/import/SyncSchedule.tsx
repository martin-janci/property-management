/**
 * SyncSchedule Component
 *
 * Configure automatic sync schedules (Epic 46, Story 46.3).
 */

'use client';

import type { SyncFrequency, SyncHistoryItem } from '@ppt/reality-api-client';
import { useSyncHistory, useSyncSchedule, useUpdateSyncSchedule } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useState } from 'react';

const FREQUENCY_VALUES: SyncFrequency[] = ['manual', 'hourly', 'daily', 'weekly'];

const TIME_OPTIONS = Array.from({ length: 24 }, (_, i) => {
  const hour = i.toString().padStart(2, '0');
  return { value: `${hour}:00`, label: `${hour}:00` };
});

const DAY_KEYS = [
  'sunday',
  'monday',
  'tuesday',
  'wednesday',
  'thursday',
  'friday',
  'saturday',
] as const;

interface SyncScheduleProps {
  agencyId: string;
  connectionId: string;
  connectionName: string;
}

export function SyncSchedule({ agencyId, connectionId, connectionName }: SyncScheduleProps) {
  const t = useTranslations('import.schedule');
  const { data: schedule, isLoading } = useSyncSchedule(agencyId, connectionId);
  const { data: history } = useSyncHistory(agencyId, connectionId);
  const updateMutation = useUpdateSyncSchedule(agencyId, connectionId);

  const frequencyOptions = FREQUENCY_VALUES.map((value) => ({
    value,
    label: t(`freq.${value}`),
    description: t(`freq.${value}Desc`),
  }));
  const dayOptions = DAY_KEYS.map((key, value) => ({ value, label: t(`days.${key}`) }));

  const [isEditing, setIsEditing] = useState(false);
  const [frequency, setFrequency] = useState<SyncFrequency>(schedule?.frequency || 'daily');
  const [preferredTime, setPreferredTime] = useState(schedule?.preferredTime || '09:00');
  const [preferredDay, setPreferredDay] = useState(schedule?.preferredDay || 1);
  const [enabled, setEnabled] = useState(schedule?.enabled ?? true);

  const handleSave = async () => {
    await updateMutation.mutateAsync({
      frequency,
      preferredTime: frequency === 'daily' || frequency === 'weekly' ? preferredTime : undefined,
      preferredDay: frequency === 'weekly' ? preferredDay : undefined,
      enabled,
    });
    setIsEditing(false);
  };

  if (isLoading) {
    return <ScheduleSkeleton />;
  }

  return (
    <div className="sync-schedule">
      <div className="section">
        <div className="section-header">
          <div>
            <h3>{t('title')}</h3>
            <p className="subtitle">{t('subtitle', { name: connectionName })}</p>
          </div>
          {!isEditing && (
            <button type="button" className="edit-button" onClick={() => setIsEditing(true)}>
              {t('editSchedule')}
            </button>
          )}
        </div>

        {isEditing ? (
          <div className="edit-form">
            {/* Enable/Disable Toggle */}
            <label className="toggle-row">
              <span>{t('enableAuto')}</span>
              <input
                type="checkbox"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
              />
            </label>

            {/* Frequency Selection */}
            <fieldset className="frequency-section">
              <legend>{t('syncFrequency')}</legend>
              <div className="frequency-options">
                {frequencyOptions.map((opt) => (
                  <button
                    key={opt.value}
                    type="button"
                    className={`frequency-option ${frequency === opt.value ? 'selected' : ''}`}
                    onClick={() => setFrequency(opt.value)}
                    disabled={!enabled}
                  >
                    <span className="freq-label">{opt.label}</span>
                    <span className="freq-desc">{opt.description}</span>
                  </button>
                ))}
              </div>
            </fieldset>

            {/* Time Picker (for daily/weekly) */}
            {(frequency === 'daily' || frequency === 'weekly') && enabled && (
              <div className="time-section">
                <label htmlFor="sync-time">{t('preferredTime')}</label>
                <select
                  id="sync-time"
                  value={preferredTime}
                  onChange={(e) => setPreferredTime(e.target.value)}
                >
                  {TIME_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
            )}

            {/* Day Picker (for weekly) */}
            {frequency === 'weekly' && enabled && (
              <div className="day-section">
                <label htmlFor="sync-day">{t('preferredDay')}</label>
                <select
                  id="sync-day"
                  value={preferredDay}
                  onChange={(e) => setPreferredDay(Number(e.target.value))}
                >
                  {dayOptions.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
            )}

            {/* Actions */}
            <div className="form-actions">
              <button type="button" className="cancel-button" onClick={() => setIsEditing(false)}>
                {t('cancel')}
              </button>
              <button
                type="button"
                className="save-button"
                onClick={handleSave}
                disabled={updateMutation.isPending}
              >
                {updateMutation.isPending ? t('saving') : t('saveChanges')}
              </button>
            </div>
          </div>
        ) : (
          <div className="schedule-display">
            <div className="schedule-info">
              <div className="info-row">
                <span className="info-label">{t('status')}</span>
                <span className={`status-badge ${schedule?.enabled ? 'enabled' : 'disabled'}`}>
                  {schedule?.enabled ? t('enabled') : t('disabled')}
                </span>
              </div>
              <div className="info-row">
                <span className="info-label">{t('frequency')}</span>
                <span className="info-value">
                  {schedule?.frequency ? t(`freq.${schedule.frequency}`) : t('notSet')}
                </span>
              </div>
              {schedule?.preferredTime && (
                <div className="info-row">
                  <span className="info-label">{t('time')}</span>
                  <span className="info-value">{schedule.preferredTime}</span>
                </div>
              )}
              {schedule?.preferredDay !== undefined && schedule.frequency === 'weekly' && (
                <div className="info-row">
                  <span className="info-label">{t('day')}</span>
                  <span className="info-value">
                    {dayOptions.find((d) => d.value === schedule.preferredDay)?.label}
                  </span>
                </div>
              )}
              {schedule?.nextRunAt && (
                <div className="info-row">
                  <span className="info-label">{t('nextRun')}</span>
                  <span className="info-value">
                    {new Date(schedule.nextRunAt).toLocaleString()}
                  </span>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Sync History */}
      <div className="section">
        <h3>{t('syncHistory')}</h3>
        {history && history.length > 0 ? (
          <div className="history-list">
            {history.map((item) => (
              <HistoryItem key={item.id} item={item} />
            ))}
          </div>
        ) : (
          <p className="no-history">{t('noHistory')}</p>
        )}
      </div>

      <style jsx>{`
        .sync-schedule {
          display: flex;
          flex-direction: column;
          gap: 24px;
        }

        .section {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          padding: 24px;
        }

        .section-header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 20px;
        }

        h3 {
          font-size: 1.125rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 4px;
        }

        .subtitle {
          color: var(--ppt-fg-muted);
          font-size: 14px;
          margin: 0;
        }

        .edit-button {
          padding: 8px 16px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
        }

        .edit-button:hover {
          background: var(--ppt-bg-app);
        }

        .edit-form {
          display: flex;
          flex-direction: column;
          gap: 20px;
        }

        .toggle-row {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 16px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
          cursor: pointer;
        }

        .toggle-row span {
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .toggle-row input[type="checkbox"] {
          width: 20px;
          height: 20px;
        }

        .frequency-section {
          border: none;
          padding: 0;
          margin: 0;
        }

        .frequency-section legend {
          font-size: 13px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          margin-bottom: 8px;
          padding: 0;
        }

        .time-section label,
        .day-section label {
          display: block;
          font-size: 13px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          margin-bottom: 8px;
        }

        .frequency-options {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 12px;
        }

        .frequency-option {
          display: flex;
          flex-direction: column;
          align-items: flex-start;
          padding: 16px;
          border: 2px solid var(--ppt-border-default);
          border-radius: 10px;
          background: var(--ppt-bg-surface);
          cursor: pointer;
          text-align: left;
          transition: all 0.2s;
        }

        .frequency-option:hover:not(:disabled) {
          border-color: var(--ppt-color-primary);
        }

        .frequency-option.selected {
          border-color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        .frequency-option:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .freq-label {
          font-size: 14px;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .freq-desc {
          font-size: 12px;
          color: var(--ppt-fg-muted);
          margin-top: 4px;
        }

        .time-section select,
        .day-section select {
          padding: 10px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
          width: 200px;
        }

        .form-actions {
          display: flex;
          justify-content: flex-end;
          gap: 12px;
          padding-top: 16px;
          border-top: 1px solid var(--ppt-border-default);
        }

        .cancel-button {
          padding: 10px 20px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
          cursor: pointer;
        }

        .save-button {
          padding: 10px 20px;
          background: var(--ppt-color-primary);
          border: none;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-on-accent);
          cursor: pointer;
        }

        .save-button:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .schedule-display {
          padding: 16px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
        }

        .schedule-info {
          display: flex;
          flex-direction: column;
          gap: 12px;
        }

        .info-row {
          display: flex;
          justify-content: space-between;
        }

        .info-label {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }

        .info-value {
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .status-badge {
          padding: 4px 10px;
          border-radius: 12px;
          font-size: 12px;
          font-weight: 500;
        }

        .status-badge.enabled {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-hover);
        }

        .status-badge.disabled {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-muted);
        }

        .history-list {
          display: flex;
          flex-direction: column;
          gap: 12px;
          margin-top: 16px;
        }

        .no-history {
          color: var(--ppt-fg-muted);
          font-size: 14px;
          text-align: center;
          padding: 24px;
        }

        @media (max-width: 640px) {
          .frequency-options {
            grid-template-columns: 1fr;
          }
        }
      `}</style>
    </div>
  );
}

function HistoryItem({ item }: { item: SyncHistoryItem }) {
  const t = useTranslations('import.schedule');
  const statusConfig = getHistoryStatusConfig(item.status);

  return (
    <div className="history-item">
      <div className="history-header">
        <span className="history-date">{new Date(item.startedAt).toLocaleString()}</span>
        <span
          className="history-status"
          style={{ background: statusConfig.bg, color: statusConfig.color }}
        >
          {t(`historyStatus.${item.status}`)}
        </span>
      </div>
      <div className="history-stats">
        <span className="stat">
          <strong>{item.recordsProcessed}</strong> {t('processed')}
        </span>
        <span className="stat">
          <strong>{item.recordsCreated}</strong> {t('created')}
        </span>
        <span className="stat">
          <strong>{item.recordsUpdated}</strong> {t('updated')}
        </span>
        {item.recordsFailed > 0 && (
          <span className="stat error">
            <strong>{item.recordsFailed}</strong> {t('failed')}
          </span>
        )}
      </div>
      {item.errors && item.errors.length > 0 && (
        <div className="history-errors">
          {item.errors.slice(0, 3).map((error, i) => (
            <span key={`err-${error.slice(0, 20)}-${i}`} className="error-text">
              {error}
            </span>
          ))}
        </div>
      )}

      <style jsx>{`
        .history-item {
          padding: 16px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
        }

        .history-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 12px;
        }

        .history-date {
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .history-status {
          padding: 4px 10px;
          border-radius: 12px;
          font-size: 12px;
          font-weight: 500;
        }

        .history-stats {
          display: flex;
          gap: 16px;
          flex-wrap: wrap;
        }

        .stat {
          font-size: 13px;
          color: var(--ppt-fg-muted);
        }

        .stat strong {
          color: var(--ppt-fg-secondary);
        }

        .stat.error strong {
          color: var(--ppt-color-danger);
        }

        .history-errors {
          margin-top: 12px;
          padding-top: 12px;
          border-top: 1px solid var(--ppt-border-default);
        }

        .error-text {
          display: block;
          font-size: 12px;
          color: var(--ppt-color-danger);
          margin-bottom: 4px;
        }
      `}</style>
    </div>
  );
}

function ScheduleSkeleton() {
  return (
    <div className="skeleton">
      <div className="skeleton-box" style={{ height: '200px' }} />
      <div className="skeleton-box" style={{ height: '150px', marginTop: '24px' }} />
      <style jsx>{`
        .skeleton-box {
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
      `}</style>
    </div>
  );
}

function getHistoryStatusConfig(status: SyncHistoryItem['status']) {
  const configs = {
    running: {
      color: 'var(--ppt-color-primary)',
      bg: 'var(--ppt-color-primary-soft-bg)',
    },
    completed: {
      color: 'var(--ppt-color-success-hover)',
      bg: 'var(--ppt-color-success-light)',
    },
    failed: {
      color: 'var(--ppt-color-danger)',
      bg: 'var(--ppt-color-danger-light)',
    },
    cancelled: {
      color: 'var(--ppt-fg-muted)',
      bg: 'var(--ppt-border-default)',
    },
  };
  return configs[status];
}
