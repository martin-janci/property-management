/** A single vote row/card for the voting list (Epic 5). */
import type { VoteSummary } from '@ppt/api-client';
import { useTranslation } from 'react-i18next';
import { QuorumTile } from './QuorumTile';
import { VoteStatusBadge } from './VoteStatusBadge';

interface VoteCardProps {
  vote: VoteSummary;
  onOpen: (voteId: string) => void;
}

function formatDate(value: string | null, locale: string): string {
  if (!value) return '—';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleDateString(locale);
}

export function VoteCard({ vote, onOpen }: VoteCardProps) {
  const { t, i18n } = useTranslation();
  return (
    <button
      type="button"
      onClick={() => onOpen(vote.id)}
      className="flex w-full items-start justify-between gap-4 rounded-lg bg-white p-4 text-left shadow transition-shadow hover:shadow-md"
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <VoteStatusBadge status={vote.status} />
          <span className="text-xs text-gray-400">
            {t('voting.card.questionCount', { count: vote.question_count })}
          </span>
        </div>
        <h3 className="mt-1.5 truncate text-base font-semibold text-gray-900">{vote.title}</h3>
        {vote.description ? (
          <p className="mt-1 line-clamp-2 text-sm text-gray-500">{vote.description}</p>
        ) : null}
        <div className="mt-2 text-xs text-gray-500">
          {t('voting.detail.closes', { date: formatDate(vote.end_at, i18n.language) })}
        </div>
      </div>
      <div className="shrink-0">
        <QuorumTile
          participation={vote.participation_count}
          eligible={vote.eligible_count}
          quorumMet={vote.quorum_met}
          compact
        />
      </div>
    </button>
  );
}
