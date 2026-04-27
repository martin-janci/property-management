import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';

export type VoteStatus = 'active' | 'closed' | 'pending';
export type VoteType = 'simple' | 'multiple' | 'weighted' | 'ranked';

export interface VoteOption {
  id: string;
  label: string;
  votes: number;
  percentage: number;
}

export interface Vote {
  id: string;
  title: string;
  description: string;
  status: VoteStatus;
  type: VoteType;
  options: VoteOption[];
  startsAt: string;
  endsAt: string;
  totalVotes: number;
  requiredQuorum: number;
  currentQuorum: number;
  hasVoted: boolean;
  userVote?: string | string[];
  createdBy: string;
}

/** Subset of `VoteSummary` from `GET /api/v1/voting`. */
interface ApiVoteSummary {
  id: string;
  building_id: string;
  title: string;
  status: string;
  end_at: string;
  quorum_type: string;
  participation_count?: number | null;
  eligible_count?: number | null;
  quorum_met?: boolean | null;
}

/** Map the api-server's status string onto the UI's narrowed enum. */
function toUiStatus(status: string): VoteStatus {
  switch (status) {
    case 'active':
    case 'open':
      return 'active';
    case 'closed':
    case 'cancelled':
      return 'closed';
    default:
      return 'pending';
  }
}

/** The list endpoint only returns summaries — no questions/options/results
 *  detail. The screen still renders, but inline voting is disabled until
 *  the user navigates into the (future) detail screen. */
function toUiVote(s: ApiVoteSummary): Vote {
  const total = s.participation_count ?? 0;
  return {
    id: s.id,
    title: s.title,
    description: '',
    status: toUiStatus(s.status),
    type: 'simple',
    options: [],
    startsAt: '',
    endsAt: s.end_at,
    totalVotes: total,
    requiredQuorum: s.eligible_count ?? 0,
    currentQuorum: total,
    hasVoted: false,
    createdBy: 'Building Management',
  };
}

interface VotingScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

interface ApiVoteListResponse {
  votes: ApiVoteSummary[];
  total?: number;
}

export function VotingScreen({ onNavigate }: VotingScreenProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<'all' | 'active' | 'closed'>('all');

  // Voting routes use `RlsConnection` and require both Authorization and
  // X-Tenant-ID — both are sent by `useApiQuery` (the latter pulled from
  // the JWT's `tenant_id` claim).
  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiVoteListResponse>(
    ['voting', 'list'],
    '/api/v1/voting',
    { staleTime: 30_000 }
  );

  // The list endpoint may return either an array or `{ votes: [...] }`
  // depending on backend version — accept both.
  const apiVotes: ApiVoteSummary[] = Array.isArray(data)
    ? (data as unknown as ApiVoteSummary[])
    : (data?.votes ?? []);
  const votes: Vote[] = apiVotes.map(toUiVote);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const getStatusColor = (status: VoteStatus): string => {
    switch (status) {
      case 'active':
        return '#10b981';
      case 'closed':
        return '#6b7280';
      case 'pending':
        return '#f59e0b';
      default:
        return '#6b7280';
    }
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  };

  const getTimeRemaining = (endsAt: string): string => {
    const end = new Date(endsAt);
    const now = new Date();
    const diff = end.getTime() - now.getTime();

    if (diff <= 0) return t('voting.ended');

    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));

    if (days > 0) return t('voting.timeLeftDaysHours', { days, hours });
    if (hours > 0) return t('voting.timeLeftHours', { hours });

    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
    return t('voting.timeLeftMinutes', { minutes });
  };

  // The list endpoint only returns summaries — so tapping a card opens the
  // detail screen that loads questions/options/eligibility and runs the
  // cast-vote flow.
  const openDetail = (voteId: string) => {
    onNavigate?.('VoteDetail', { voteId });
  };

  const filteredVotes = votes.filter((v) => {
    if (filter === 'all') return true;
    return v.status === filter;
  });

  const activeCount = votes.filter((v) => v.status === 'active').length;

  return (
    <View style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <Text style={styles.headerTitle}>{t('voting.title')}</Text>
        {activeCount > 0 && (
          <View style={styles.activeCountBadge}>
            <Text style={styles.activeCountText}>
              {t('voting.activeCount', { count: activeCount })}
            </Text>
          </View>
        )}
      </View>

      {/* Filters */}
      <View style={styles.filters}>
        {(['all', 'active', 'closed'] as const).map((option) => (
          <Pressable
            key={option}
            style={[styles.filterButton, filter === option && styles.filterButtonActive]}
            onPress={() => setFilter(option)}
          >
            <Text style={[styles.filterText, filter === option && styles.filterTextActive]}>
              {t(`voting.filter.${option}`)}
            </Text>
          </Pressable>
        ))}
      </View>

      {/* Votes List */}
      <ScrollView
        style={styles.scrollView}
        refreshControl={
          <RefreshControl refreshing={isFetching} onRefresh={onRefresh} tintColor="#2563eb" />
        }
      >
        {isLoading ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>{t('common.loading') ?? 'Loading…'}</Text>
          </View>
        ) : error ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>⚠️</Text>
            <Text style={styles.emptyTitle}>{t('voting.loadError') ?? "Couldn't load votes"}</Text>
            <Text style={styles.emptyText}>{error.message}</Text>
          </View>
        ) : filteredVotes.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>🗳️</Text>
            <Text style={styles.emptyTitle}>{t('voting.emptyTitle')}</Text>
            <Text style={styles.emptyText}>{t('voting.emptyText')}</Text>
          </View>
        ) : (
          filteredVotes.map((vote) => (
            <Pressable
              key={vote.id}
              style={styles.voteCard}
              onPress={() => openDetail(vote.id)}
              accessibilityRole="button"
              accessibilityLabel={t('voting.openDetail') ?? `Open ${vote.title}`}
            >
              <View style={styles.voteHeader}>
                <View
                  style={[styles.statusBadge, { backgroundColor: getStatusColor(vote.status) }]}
                >
                  <Text style={styles.statusText}>{vote.status}</Text>
                </View>
                {vote.status === 'active' && (
                  <Text style={styles.timeRemaining}>{getTimeRemaining(vote.endsAt)}</Text>
                )}
              </View>

              <Text style={styles.voteTitle}>{vote.title}</Text>
              <Text style={styles.voteDescription} numberOfLines={2}>
                {vote.description}
              </Text>

              {/* Quorum Progress */}
              <View style={styles.quorumSection}>
                <View style={styles.quorumHeader}>
                  <Text style={styles.quorumLabel}>{t('voting.quorum')}</Text>
                  <Text style={styles.quorumValue}>
                    {t('voting.quorumVotes', {
                      current: vote.currentQuorum,
                      required: vote.requiredQuorum,
                    })}
                  </Text>
                </View>
                <View style={styles.quorumBar}>
                  <View
                    style={[
                      styles.quorumProgress,
                      {
                        width: `${Math.min((vote.currentQuorum / vote.requiredQuorum) * 100, 100)}%`,
                      },
                      vote.currentQuorum >= vote.requiredQuorum && styles.quorumMet,
                    ]}
                  />
                </View>
              </View>

              {/* CTA — list view doesn't ship options, detail screen handles
                  the actual ballot. */}
              <View style={styles.openDetailCta}>
                <Text style={styles.openDetailText}>
                  {vote.status === 'active'
                    ? (t('voting.tapToVote') ?? 'Tap to view & cast ballot')
                    : (t('voting.tapToView') ?? 'Tap to view results')}
                </Text>
                <Text style={styles.openDetailChevron}>›</Text>
              </View>

              {/* Footer */}
              <View style={styles.voteFooter}>
                <Text style={styles.createdBy}>
                  {t('voting.createdBy', { name: vote.createdBy })}
                </Text>
                <Text style={styles.dateRange}>
                  {formatDate(vote.startsAt)} - {formatDate(vote.endsAt)}
                </Text>
              </View>
            </Pressable>
          ))
        )}

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingTop: 60,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e5e7eb',
  },
  headerTitle: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#1f2937',
  },
  activeCountBadge: {
    backgroundColor: '#dcfce7',
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 12,
  },
  activeCountText: {
    color: '#16a34a',
    fontSize: 13,
    fontWeight: '600',
  },
  filters: {
    flexDirection: 'row',
    padding: 16,
    gap: 8,
    backgroundColor: '#fff',
  },
  filterButton: {
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: '#f3f4f6',
  },
  filterButtonActive: {
    backgroundColor: '#2563eb',
  },
  filterText: {
    fontSize: 14,
    color: '#6b7280',
    fontWeight: '500',
  },
  filterTextActive: {
    color: '#fff',
  },
  scrollView: {
    flex: 1,
    padding: 16,
  },
  emptyState: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
  },
  emptyIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: '#374151',
    marginBottom: 4,
  },
  emptyText: {
    fontSize: 14,
    color: '#6b7280',
  },
  voteCard: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  voteHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  statusBadge: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 4,
  },
  statusText: {
    fontSize: 11,
    fontWeight: '600',
    color: '#fff',
    textTransform: 'uppercase',
  },
  timeRemaining: {
    fontSize: 13,
    color: '#f59e0b',
    fontWeight: '600',
  },
  voteTitle: {
    fontSize: 17,
    fontWeight: '600',
    color: '#1f2937',
    marginBottom: 6,
  },
  voteDescription: {
    fontSize: 14,
    color: '#6b7280',
    lineHeight: 20,
    marginBottom: 16,
  },
  quorumSection: {
    marginBottom: 16,
  },
  quorumHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 6,
  },
  quorumLabel: {
    fontSize: 12,
    color: '#6b7280',
    fontWeight: '500',
  },
  quorumValue: {
    fontSize: 12,
    color: '#374151',
    fontWeight: '500',
  },
  quorumBar: {
    height: 6,
    backgroundColor: '#e5e7eb',
    borderRadius: 3,
    overflow: 'hidden',
  },
  quorumProgress: {
    height: '100%',
    backgroundColor: '#f59e0b',
    borderRadius: 3,
  },
  quorumMet: {
    backgroundColor: '#10b981',
  },
  openDetailCta: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: 12,
    backgroundColor: '#eff6ff',
    borderRadius: 8,
    marginBottom: 12,
  },
  openDetailText: {
    fontSize: 13,
    color: '#2563eb',
    fontWeight: '500',
  },
  openDetailChevron: {
    fontSize: 20,
    color: '#2563eb',
    fontWeight: '400',
  },
  voteFooter: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingTop: 12,
    borderTopWidth: 1,
    borderTopColor: '#f3f4f6',
  },
  createdBy: {
    fontSize: 12,
    color: '#9ca3af',
  },
  dateRange: {
    fontSize: 12,
    color: '#9ca3af',
  },
  bottomSpacer: {
    height: 100,
  },
});
