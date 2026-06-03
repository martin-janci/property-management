/**
 * MediationSessionDialog — schedule or reschedule a mediation session.
 * Epic 80: Dispute Resolution (Story 80.3)
 *
 * Used in two modes:
 *   - schedule:   no `session` prop — collects type/date/time/duration/location
 *   - reschedule: `session` prop present — pre-fills the form; on confirm the
 *                 parent issues a PATCH that also sets status to `rescheduled`.
 */

import type {
  MediationSession,
  ScheduleSessionRequest,
  SessionType,
  UpdateSessionRequest,
} from '@ppt/api-client';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';

const SESSION_TYPES: SessionType[] = ['in_person', 'video_call', 'phone', 'written'];

const sessionTypeLabelKeys: Record<SessionType, string> = {
  in_person: 'disputes.mediation.sessions.typeInPerson',
  video_call: 'disputes.mediation.sessions.typeVideoCall',
  phone: 'disputes.mediation.sessions.typePhone',
  written: 'disputes.mediation.sessions.typeWritten',
};

/**
 * Split an ISO timestamp into the `value` strings an <input type="datetime-local">
 * expects (local time, no timezone suffix). Returns '' for an absent date.
 */
function toLocalInputValue(iso?: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  // datetime-local wants YYYY-MM-DDTHH:mm in local time.
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

export interface MediationSessionDialogProps {
  /** When present, the dialog is in reschedule mode and pre-fills the form. */
  session?: MediationSession;
  onScheduleConfirm: (data: ScheduleSessionRequest) => void;
  onRescheduleConfirm: (sessionId: string, data: UpdateSessionRequest) => void;
  onCancel: () => void;
  isSubmitting: boolean;
}

export function MediationSessionDialog({
  session,
  onScheduleConfirm,
  onRescheduleConfirm,
  onCancel,
  isSubmitting,
}: MediationSessionDialogProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onCancel);

  const isReschedule = !!session;

  const [sessionType, setSessionType] = useState<SessionType>(
    session?.session_type ?? 'video_call'
  );
  const [scheduledAt, setScheduledAt] = useState(toLocalInputValue(session?.scheduled_at));
  const [durationMinutes, setDurationMinutes] = useState(
    session?.duration_minutes != null ? String(session.duration_minutes) : '60'
  );
  const [location, setLocation] = useState(session?.location ?? '');
  const [meetingUrl, setMeetingUrl] = useState(session?.meeting_url ?? '');

  const titleId = 'session-dialog-title';
  const canSubmit = scheduledAt.trim().length > 0 && !isSubmitting;

  const handleConfirm = () => {
    if (!canSubmit) return;
    // datetime-local gives a local time string; Date() interprets it as local
    // and toISOString() converts to UTC for the backend (which expects UTC).
    const scheduledIso = new Date(scheduledAt).toISOString();
    const duration = durationMinutes.trim() === '' ? null : Number(durationMinutes);
    const trimmedLocation = location.trim() === '' ? null : location.trim();
    const trimmedUrl = meetingUrl.trim() === '' ? null : meetingUrl.trim();

    if (isReschedule && session) {
      onRescheduleConfirm(session.id, {
        scheduled_at: scheduledIso,
        duration_minutes: duration,
        location: trimmedLocation,
        meeting_url: trimmedUrl,
        status: 'rescheduled',
      });
    } else {
      onScheduleConfirm({
        session_type: sessionType,
        scheduled_at: scheduledIso,
        duration_minutes: duration,
        location: trimmedLocation,
        meeting_url: trimmedUrl,
      });
    }
  };

  const inputClass =
    'w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-violet-500 focus:border-transparent';

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabIndex={-1}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
    >
      <div className="bg-white rounded-xl shadow-xl w-full max-w-md p-6 max-h-[90vh] overflow-y-auto">
        <h2 id={titleId} className="text-lg font-semibold text-gray-900 mb-2">
          {isReschedule
            ? t('disputes.mediation.sessions.rescheduleTitle')
            : t('disputes.mediation.sessions.scheduleTitle')}
        </h2>
        <p className="text-sm text-gray-500 mb-4">
          {isReschedule
            ? t('disputes.mediation.sessions.rescheduleDescription')
            : t('disputes.mediation.sessions.scheduleDescription')}
        </p>

        <div className="space-y-4">
          {!isReschedule && (
            <div>
              <label
                htmlFor="session-dialog-type"
                className="block text-sm font-medium text-gray-700 mb-1"
              >
                {t('disputes.mediation.sessions.sessionType')}
              </label>
              <select
                id="session-dialog-type"
                value={sessionType}
                onChange={(e) => setSessionType(e.target.value as SessionType)}
                className={inputClass}
              >
                {SESSION_TYPES.map((type) => (
                  <option key={type} value={type}>
                    {t(sessionTypeLabelKeys[type])}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div>
            <label
              htmlFor="session-dialog-datetime"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('disputes.mediation.sessions.dateTime')}
            </label>
            <input
              id="session-dialog-datetime"
              type="datetime-local"
              value={scheduledAt}
              onChange={(e) => setScheduledAt(e.target.value)}
              className={inputClass}
            />
          </div>

          <div>
            <label
              htmlFor="session-dialog-duration"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('disputes.mediation.sessions.durationMinutes')}
            </label>
            <input
              id="session-dialog-duration"
              type="number"
              min={0}
              step={15}
              value={durationMinutes}
              onChange={(e) => setDurationMinutes(e.target.value)}
              className={inputClass}
            />
          </div>

          <div>
            <label
              htmlFor="session-dialog-location"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('disputes.mediation.sessions.location')}
            </label>
            <input
              id="session-dialog-location"
              type="text"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              placeholder={t('disputes.mediation.sessions.locationPlaceholder')}
              className={inputClass}
            />
          </div>

          <div>
            <label
              htmlFor="session-dialog-url"
              className="block text-sm font-medium text-gray-700 mb-1"
            >
              {t('disputes.mediation.sessions.meetingUrl')}
            </label>
            <input
              id="session-dialog-url"
              type="url"
              value={meetingUrl}
              onChange={(e) => setMeetingUrl(e.target.value)}
              placeholder={t('disputes.mediation.sessions.meetingUrlPlaceholder')}
              className={inputClass}
            />
          </div>
        </div>

        <div className="flex justify-end gap-3 mt-6">
          <button
            type="button"
            onClick={onCancel}
            disabled={isSubmitting}
            className="px-4 py-2 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
          >
            {t('disputes.mediation.cancel')}
          </button>
          <button
            type="button"
            onClick={handleConfirm}
            disabled={!canSubmit}
            className="px-4 py-2 text-sm bg-violet-600 text-white rounded-lg hover:bg-violet-700 disabled:opacity-50 disabled:cursor-not-allowed font-medium"
          >
            {isSubmitting
              ? t('disputes.mediation.sessions.saving')
              : isReschedule
                ? t('disputes.mediation.sessions.rescheduleBtn')
                : t('disputes.mediation.sessions.scheduleBtn')}
          </button>
        </div>
      </div>
    </div>
  );
}
