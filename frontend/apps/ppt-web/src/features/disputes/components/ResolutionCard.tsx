/**
 * ResolutionCard - displays a proposed resolution with voting.
 * Epic 77: Dispute Resolution (Story 77.3)
 */

import { useTranslation } from 'react-i18next';
import { formatDisputeDate, resolutionStatusKeys } from '../utils/labels';

export type ResolutionStatus =
  | 'proposed'
  | 'accepted'
  | 'rejected'
  | 'partially_accepted'
  | 'implemented';

export interface ResolutionTerm {
  id: string;
  description: string;
  responsiblePartyId?: string;
  responsiblePartyName?: string;
  deadline?: string;
  completed: boolean;
  completedAt?: string;
}

export interface Resolution {
  id: string;
  disputeId: string;
  proposedBy: string;
  proposedByName: string;
  resolutionText: string;
  terms: ResolutionTerm[];
  status: ResolutionStatus;
  proposedAt: string;
  acceptedAt?: string;
  implementedAt?: string;
}

export interface ResolutionVote {
  partyId: string;
  partyName: string;
  accepted: boolean;
  comments?: string;
  votedAt: string;
}

const statusColors: Record<ResolutionStatus, string> = {
  proposed: 'bg-blue-100 text-blue-800',
  accepted: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  partially_accepted: 'bg-yellow-100 text-yellow-800',
  implemented: 'bg-emerald-100 text-emerald-800',
};

interface ResolutionCardProps {
  resolution: Resolution;
  votes?: ResolutionVote[];
  acceptanceRate?: number;
  canVote?: boolean;
  canAccept?: boolean;
  canImplement?: boolean;
  onVote?: (resolutionId: string, accepted: boolean, comments?: string) => void;
  onAccept?: (resolutionId: string) => void;
  onImplement?: (resolutionId: string) => void;
  onCompleteItem?: (resolutionId: string, termId: string) => void;
}

export function ResolutionCard({
  resolution,
  votes,
  acceptanceRate,
  canVote,
  canAccept,
  canImplement,
  onVote,
  onAccept,
  onImplement,
  onCompleteItem,
}: ResolutionCardProps) {
  const { t, i18n } = useTranslation();
  const formatDate = (dateString: string) => formatDisputeDate(dateString, i18n.language);

  const completedTerms = resolution.terms.filter((term) => term.completed).length;
  const totalTerms = resolution.terms.length;
  const progressPercent = totalTerms > 0 ? (completedTerms / totalTerms) * 100 : 0;

  return (
    <div className="bg-white rounded-lg shadow border border-gray-200 p-4">
      {/* Header */}
      <div className="flex items-start justify-between mb-3">
        <div>
          <div className="flex items-center gap-2">
            <span
              className={`px-2 py-0.5 text-xs font-medium rounded ${statusColors[resolution.status]}`}
            >
              {t(resolutionStatusKeys[resolution.status])}
            </span>
            {acceptanceRate !== undefined && (
              <span className="text-sm text-gray-500">
                {t('disputes.resolution.approval', {
                  rate: new Intl.NumberFormat(i18n.language, {
                    style: 'percent',
                    maximumFractionDigits: 0,
                  }).format(acceptanceRate),
                })}
              </span>
            )}
          </div>
          <p className="text-sm text-gray-500 mt-1">
            {t('disputes.resolution.proposedBy', {
              name: resolution.proposedByName,
              date: formatDate(resolution.proposedAt),
            })}
          </p>
        </div>
      </div>

      {/* Resolution Text */}
      <div className="mb-4">
        <h4 className="font-medium text-gray-900 mb-2">
          {t('disputes.resolution.proposalHeading')}
        </h4>
        <p className="text-gray-700 whitespace-pre-wrap">{resolution.resolutionText}</p>
      </div>

      {/* Terms */}
      {resolution.terms.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <h4 className="font-medium text-gray-900">
              {t('disputes.resolution.terms', { completed: completedTerms, total: totalTerms })}
            </h4>
            {totalTerms > 0 && (
              <div className="w-32 h-2 bg-gray-200 rounded-full overflow-hidden">
                <div
                  className="h-full bg-green-500 transition-all"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
            )}
          </div>
          <div className="space-y-2">
            {resolution.terms.map((term) => (
              <div
                key={term.id}
                className={`flex items-start gap-2 p-2 rounded ${
                  term.completed ? 'bg-green-50' : 'bg-gray-50'
                }`}
              >
                <input
                  type="checkbox"
                  checked={term.completed}
                  disabled={term.completed || !onCompleteItem}
                  onChange={() => onCompleteItem?.(resolution.id, term.id)}
                  className="mt-1 rounded border-gray-300"
                />
                <div className="flex-1">
                  <p
                    className={`text-sm ${term.completed ? 'text-gray-500 line-through' : 'text-gray-700'}`}
                  >
                    {term.description}
                  </p>
                  <div className="flex items-center gap-3 mt-1 text-xs text-gray-500">
                    {term.responsiblePartyName && (
                      <span>
                        {t('disputes.resolution.termAssignedTo', {
                          name: term.responsiblePartyName,
                        })}
                      </span>
                    )}
                    {term.deadline && (
                      <span>
                        {t('disputes.resolution.termDue', { date: formatDate(term.deadline) })}
                      </span>
                    )}
                    {term.completedAt && (
                      <span className="text-green-600">
                        {t('disputes.resolution.termCompleted', {
                          date: formatDate(term.completedAt),
                        })}
                      </span>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Votes */}
      {votes && votes.length > 0 && (
        <div className="mb-4">
          <h4 className="font-medium text-gray-900 mb-2">{t('disputes.resolution.votes')}</h4>
          <div className="space-y-2">
            {votes.map((vote) => (
              <div
                key={vote.partyId}
                className={`flex items-center justify-between p-2 rounded ${
                  vote.accepted ? 'bg-green-50' : 'bg-red-50'
                }`}
              >
                <div>
                  <span className="font-medium">{vote.partyName}</span>
                  {vote.comments && <p className="text-sm text-gray-600 mt-1">{vote.comments}</p>}
                </div>
                <span className={vote.accepted ? 'text-green-600' : 'text-red-600'}>
                  {vote.accepted
                    ? t('disputes.resolution.voteAccepted')
                    : t('disputes.resolution.voteRejected')}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-2 pt-3 border-t">
        {canVote && onVote && resolution.status === 'proposed' && (
          <>
            <button
              type="button"
              onClick={() => onVote(resolution.id, false)}
              className="px-3 py-1.5 text-sm border border-red-300 text-red-600 rounded-lg hover:bg-red-50"
            >
              {t('disputes.resolution.reject')}
            </button>
            <button
              type="button"
              onClick={() => onVote(resolution.id, true)}
              className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
            >
              {t('disputes.resolution.accept')}
            </button>
          </>
        )}
        {canAccept && onAccept && resolution.status === 'proposed' && (
          <button
            type="button"
            onClick={() => onAccept(resolution.id)}
            className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
          >
            {t('disputes.resolution.finalize')}
          </button>
        )}
        {canImplement &&
          onImplement &&
          resolution.status === 'accepted' &&
          progressPercent === 100 && (
            <button
              type="button"
              onClick={() => onImplement(resolution.id)}
              className="px-3 py-1.5 text-sm bg-emerald-600 text-white rounded-lg hover:bg-emerald-700"
            >
              {t('disputes.resolution.markImplemented')}
            </button>
          )}
      </div>
    </div>
  );
}
