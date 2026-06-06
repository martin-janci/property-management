/**
 * MediationSessionsPanel — sidebar list of mediation sessions for a dispute,
 * with upcoming sessions surfaced first and reschedule / cancel actions.
 * Epic 80: Dispute Resolution (Story 80.3)
 */

import type { MediationSession, SessionStatus, SessionType } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components/Spinner';

const sessionTypeLabelKeys: Record<SessionType, string> = {
  in_person: 'disputes.mediation.sessions.typeInPerson',
  video_call: 'disputes.mediation.sessions.typeVideoCall',
  phone: 'disputes.mediation.sessions.typePhone',
  written: 'disputes.mediation.sessions.typeWritten',
};

const statusColors: Record<SessionStatus, string> = {
  scheduled: 'bg-blue-100 text-blue-800',
  in_progress: 'bg-yellow-100 text-yellow-800',
  completed: 'bg-green-100 text-green-800',
  cancelled: 'bg-red-100 text-red-800',
  rescheduled: 'bg-orange-100 text-orange-800',
};

const statusLabelKeys: Record<SessionStatus, string> = {
  scheduled: 'disputes.mediation.sessions.statusScheduled',
  in_progress: 'disputes.mediation.sessions.statusInProgress',
  completed: 'disputes.mediation.sessions.statusCompleted',
  cancelled: 'disputes.mediation.sessions.statusCancelled',
  rescheduled: 'disputes.mediation.sessions.statusRescheduled',
};

/** Sort upcoming scheduled/rescheduled sessions first (soonest first), then the rest. */
function sortSessions(sessions: MediationSession[]): MediationSession[] {
  const now = Date.now();
  const isUpcoming = (s: MediationSession) =>
    (s.status === 'scheduled' || s.status === 'rescheduled') &&
    new Date(s.scheduled_at).getTime() >= now;

  return [...sessions].sort((a, b) => {
    const aUp = isUpcoming(a);
    const bUp = isUpcoming(b);
    if (aUp !== bUp) return aUp ? -1 : 1;
    const aTime = new Date(a.scheduled_at).getTime();
    const bTime = new Date(b.scheduled_at).getTime();
    // Upcoming: soonest first. Past/other: most recent first.
    return aUp ? aTime - bTime : bTime - aTime;
  });
}

export interface MediationSessionsPanelProps {
  sessions: MediationSession[];
  isLoading: boolean;
  canManage: boolean;
  onSchedule: () => void;
  onReschedule: (session: MediationSession) => void;
  onCancel: (session: MediationSession) => void;
}

export function MediationSessionsPanel({
  sessions,
  isLoading,
  canManage,
  onSchedule,
  onReschedule,
  onCancel,
}: MediationSessionsPanelProps) {
  const { t } = useTranslation();

  const formatDateTime = (iso: string) =>
    new Date(iso).toLocaleString(undefined, {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });

  const ordered = sortSessions(sessions);

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-gray-700">
          {t('disputes.mediation.sessions.title')}
        </h3>
        {canManage && (
          <button
            type="button"
            onClick={onSchedule}
            className="text-xs font-medium text-violet-700 hover:text-violet-900"
          >
            {t('disputes.mediation.sessions.schedule')}
          </button>
        )}
      </div>

      {isLoading ? (
        <div className="flex justify-center py-4">
          <Spinner size="sm" />
        </div>
      ) : ordered.length === 0 ? (
        <p className="text-xs text-gray-400">{t('disputes.mediation.sessions.empty')}</p>
      ) : (
        <ul className="space-y-2.5">
          {ordered.map((session) => {
            const isActionable = session.status === 'scheduled' || session.status === 'rescheduled';
            return (
              <li key={session.id} className="p-2.5 border border-gray-100 rounded-lg bg-gray-50">
                <div className="flex items-center justify-between gap-2 mb-1">
                  <span className="text-sm font-medium text-gray-900">
                    {t(sessionTypeLabelKeys[session.session_type])}
                  </span>
                  <span
                    className={`px-1.5 py-0.5 text-[10px] font-medium rounded ${
                      statusColors[session.status]
                    }`}
                  >
                    {t(statusLabelKeys[session.status])}
                  </span>
                </div>
                <p className="text-xs text-gray-600">{formatDateTime(session.scheduled_at)}</p>
                {session.duration_minutes != null && (
                  <p className="text-xs text-gray-400">
                    {t('disputes.mediation.sessions.minutesShort', {
                      count: session.duration_minutes,
                    })}
                  </p>
                )}
                {session.location && (
                  <p className="text-xs text-gray-500 truncate" title={session.location}>
                    {session.location}
                  </p>
                )}
                {session.meeting_url && (
                  <a
                    href={session.meeting_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs text-violet-600 hover:text-violet-800"
                  >
                    {t('disputes.mediation.sessions.joinLink')}
                  </a>
                )}

                {canManage && isActionable && (
                  <div className="flex gap-2 mt-2 pt-2 border-t border-gray-100">
                    <button
                      type="button"
                      onClick={() => onReschedule(session)}
                      className="text-xs text-violet-700 hover:text-violet-900"
                    >
                      {t('disputes.mediation.sessions.reschedule')}
                    </button>
                    <button
                      type="button"
                      onClick={() => onCancel(session)}
                      className="text-xs text-red-600 hover:text-red-800"
                    >
                      {t('disputes.mediation.sessions.cancel')}
                    </button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
