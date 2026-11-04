/**
 * Voting list page (Epic 5, screen `ppt/voting`).
 *
 * Lists votes for the manager's current building with a status filter and an
 * entry point into the create wizard. Data comes from the `@ppt/api-client`
 * voting hooks; navigation is delegated to the route wrapper via props.
 */
import { useBuildings, useVotes, type VoteStatus } from '@ppt/api-client';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Spinner } from '../../../components';
import { VoteCard } from '../components';
import { VOTE_STATUS_LABEL_KEYS } from '../types';

interface VotingPageProps {
  onOpenVote: (voteId: string) => void;
  onCreateVote: () => void;
}

const STATUS_FILTERS: Array<VoteStatus | 'all'> = [
  'all',
  'draft',
  'scheduled',
  'active',
  'closed',
  'cancelled',
];

export function VotingPage({ onOpenVote, onCreateVote }: VotingPageProps) {
  const { t } = useTranslation();
  const { data: buildingsData, isLoading: isLoadingBuildings } = useBuildings();
  const buildingId = buildingsData?.items?.[0]?.id ?? '';

  const [statusFilter, setStatusFilter] = useState<VoteStatus | 'all'>('all');

  const query = useMemo(
    () => ({
      building_id: buildingId || undefined,
      status: statusFilter === 'all' ? undefined : statusFilter,
    }),
    [buildingId, statusFilter]
  );

  const { data: votes, isLoading, error } = useVotes(query);

  const counts = useMemo(() => {
    const open = votes?.filter((v) => v.status === 'active').length ?? 0;
    const closed = votes?.filter((v) => v.status === 'closed').length ?? 0;
    return { open, closed, total: votes?.length ?? 0 };
  }, [votes]);

  return (
    <div className="mx-auto max-w-5xl px-4 py-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('voting.title')}</h1>
          <p className="mt-1 text-sm text-gray-500">
            {t('voting.list.voteCount', { count: counts.total })} ·{' '}
            {t('voting.list.openCount', { count: counts.open })} ·{' '}
            {t('voting.list.closedCount', { count: counts.closed })}
          </p>
        </div>
        <button
          type="button"
          onClick={onCreateVote}
          className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
        >
          + {t('voting.list.newVote')}
        </button>
      </div>

      <div className="mt-6 flex flex-wrap gap-2">
        {STATUS_FILTERS.map((s) => (
          <button
            key={s}
            type="button"
            onClick={() => setStatusFilter(s)}
            className={`rounded-full px-3 py-1 text-sm ${
              statusFilter === s
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
            }`}
          >
            {s === 'all' ? t('common.all') : t(VOTE_STATUS_LABEL_KEYS[s])}
          </button>
        ))}
      </div>

      <div className="mt-6">
        {isLoading || isLoadingBuildings ? (
          <div className="flex justify-center py-16">
            <Spinner size="lg" color="blue" />
          </div>
        ) : error ? (
          <div className="rounded-lg border border-red-200 bg-red-50 p-6 text-center">
            <p className="text-sm text-red-700">
              {t('voting.list.loadFailed', {
                error: error instanceof Error ? error.message : t('voting.detail.unexpectedError'),
              })}
            </p>
          </div>
        ) : !votes || votes.length === 0 ? (
          <div className="rounded-lg bg-white p-12 text-center shadow">
            <p className="text-gray-500">{t('voting.list.empty')}</p>
            <button
              type="button"
              onClick={onCreateVote}
              className="mt-4 text-sm font-medium text-blue-600 hover:text-blue-800"
            >
              {t('voting.list.createFirst')}
            </button>
          </div>
        ) : (
          <div className="space-y-3">
            {votes.map((vote) => (
              <VoteCard key={vote.id} vote={vote} onOpen={onOpenVote} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
