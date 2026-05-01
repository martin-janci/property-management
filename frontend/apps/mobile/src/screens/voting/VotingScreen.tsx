import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Alert, Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors } from '../shared/screenStyles';

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

// Mock data
const mockVotes: Vote[] = [
  {
    id: '1',
    title: 'Elevator Renovation Proposal',
    description:
      'Vote on the proposed elevator modernization project. The renovation includes new control systems, improved safety features, and energy-efficient motors.',
    status: 'active',
    type: 'simple',
    options: [
      { id: 'yes', label: 'Yes - Approve renovation', votes: 45, percentage: 60 },
      { id: 'no', label: 'No - Decline proposal', votes: 30, percentage: 40 },
    ],
    startsAt: '2025-12-20T00:00:00Z',
    endsAt: '2025-12-30T23:59:59Z',
    totalVotes: 75,
    requiredQuorum: 100,
    currentQuorum: 75,
    hasVoted: false,
    createdBy: 'Building Management',
  },
  {
    id: '2',
    title: 'New Board Member Election',
    description: 'Vote for the new board member position. Each owner can vote for one candidate.',
    status: 'active',
    type: 'simple',
    options: [
      { id: 'a', label: 'John Smith', votes: 35, percentage: 44 },
      { id: 'b', label: 'Maria Garcia', votes: 28, percentage: 35 },
      { id: 'c', label: 'Peter Johnson', votes: 17, percentage: 21 },
    ],
    startsAt: '2025-12-15T00:00:00Z',
    endsAt: '2025-12-28T23:59:59Z',
    totalVotes: 80,
    requiredQuorum: 100,
    currentQuorum: 80,
    hasVoted: true,
    userVote: 'a',
    createdBy: 'Building Management',
  },
  {
    id: '3',
    title: 'Parking Space Allocation',
    description: 'Closed vote on new parking space allocation policy.',
    status: 'closed',
    type: 'simple',
    options: [
      { id: 'rotate', label: 'Rotating system', votes: 65, percentage: 65 },
      { id: 'fixed', label: 'Fixed assignment', votes: 35, percentage: 35 },
    ],
    startsAt: '2025-12-01T00:00:00Z',
    endsAt: '2025-12-15T23:59:59Z',
    totalVotes: 100,
    requiredQuorum: 80,
    currentQuorum: 100,
    hasVoted: true,
    userVote: 'rotate',
    createdBy: 'Building Management',
  },
];

interface VotingScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function VotingScreen({ onNavigate: _onNavigate }: VotingScreenProps) {
  const { t } = useTranslation();
  const [refreshing, setRefreshing] = useState(false);
  const [votes, setVotes] = useState<Vote[]>(mockVotes);
  const [filter, setFilter] = useState<'all' | 'active' | 'closed'>('all');

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    setRefreshing(false);
  }, []);

  const getStatusColor = (status: VoteStatus): string => {
    switch (status) {
      case 'active':
        return colors.success;
      case 'closed':
        return colors.textMuted;
      case 'pending':
        return colors.warning;
      default:
        return colors.textMuted;
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

  const handleVote = (voteId: string, optionId: string) => {
    Alert.alert(t('voting.confirmVoteTitle'), t('voting.confirmVoteMessage'), [
      { text: t('common.cancel'), style: 'cancel' },
      {
        text: t('voting.vote'),
        onPress: () => {
          setVotes((prev) =>
            prev.map((v) => {
              if (v.id !== voteId) return v;
              return {
                ...v,
                hasVoted: true,
                userVote: optionId,
                options: v.options.map((o) => ({
                  ...o,
                  votes: o.id === optionId ? o.votes + 1 : o.votes,
                  percentage: Math.round(
                    ((o.id === optionId ? o.votes + 1 : o.votes) / (v.totalVotes + 1)) * 100
                  ),
                })),
                totalVotes: v.totalVotes + 1,
                currentQuorum: v.currentQuorum + 1,
              };
            })
          );
          Alert.alert(t('voting.successTitle'), t('voting.successMessage'));
        },
      },
    ]);
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
          <RefreshControl refreshing={refreshing} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {filteredVotes.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>🗳️</Text>
            <Text style={styles.emptyTitle}>{t('voting.emptyTitle')}</Text>
            <Text style={styles.emptyText}>{t('voting.emptyText')}</Text>
          </View>
        ) : (
          filteredVotes.map((vote) => (
            <View key={vote.id} style={styles.voteCard}>
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

              {/* Vote Options */}
              <View style={styles.optionsSection}>
                {vote.options.map((option) => (
                  <Pressable
                    key={option.id}
                    style={[
                      styles.optionRow,
                      vote.hasVoted && vote.userVote === option.id && styles.selectedOption,
                      vote.status !== 'active' && styles.optionDisabled,
                    ]}
                    onPress={() => {
                      if (!vote.hasVoted && vote.status === 'active') {
                        handleVote(vote.id, option.id);
                      }
                    }}
                    disabled={vote.hasVoted || vote.status !== 'active'}
                  >
                    <View style={styles.optionLeft}>
                      <View
                        style={[
                          styles.radioButton,
                          vote.hasVoted && vote.userVote === option.id && styles.radioSelected,
                        ]}
                      >
                        {vote.hasVoted && vote.userVote === option.id && (
                          <View style={styles.radioInner} />
                        )}
                      </View>
                      <Text
                        style={[
                          styles.optionLabel,
                          vote.hasVoted &&
                            vote.userVote === option.id &&
                            styles.optionLabelSelected,
                        ]}
                      >
                        {option.label}
                      </Text>
                    </View>
                    {(vote.hasVoted || vote.status === 'closed') && (
                      <View style={styles.optionRight}>
                        <Text style={styles.percentageText}>{option.percentage}%</Text>
                        <View style={styles.percentageBar}>
                          <View
                            style={[styles.percentageProgress, { width: `${option.percentage}%` }]}
                          />
                        </View>
                      </View>
                    )}
                  </Pressable>
                ))}
              </View>

              {/* Vote Status Message */}
              {vote.hasVoted && (
                <View style={styles.votedMessage}>
                  <Text style={styles.votedIcon}>✓</Text>
                  <Text style={styles.votedText}>{t('voting.youHaveVoted')}</Text>
                </View>
              )}

              {/* Footer */}
              <View style={styles.voteFooter}>
                <Text style={styles.createdBy}>
                  {t('voting.createdBy', { name: vote.createdBy })}
                </Text>
                <Text style={styles.dateRange}>
                  {formatDate(vote.startsAt)} - {formatDate(vote.endsAt)}
                </Text>
              </View>
            </View>
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
    backgroundColor: colors.background,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  headerTitle: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
  },
  activeCountBadge: {
    backgroundColor: colors.successBg,
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 12,
  },
  activeCountText: {
    color: colors.successDark,
    fontSize: 13,
    fontWeight: '600',
  },
  filters: {
    flexDirection: 'row',
    padding: 16,
    gap: 8,
    backgroundColor: colors.surface,
  },
  filterButton: {
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: colors.surfaceMuted,
  },
  filterButtonActive: {
    backgroundColor: colors.accent,
  },
  filterText: {
    fontSize: 14,
    color: colors.textMuted,
    fontWeight: '500',
  },
  filterTextActive: {
    color: colors.surface,
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
    color: colors.textSecondary,
    marginBottom: 4,
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  voteCard: {
    backgroundColor: colors.surface,
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
    color: colors.surface,
    textTransform: 'uppercase',
  },
  timeRemaining: {
    fontSize: 13,
    color: colors.warning,
    fontWeight: '600',
  },
  voteTitle: {
    fontSize: 17,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 6,
  },
  voteDescription: {
    fontSize: 14,
    color: colors.textMuted,
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
    color: colors.textMuted,
    fontWeight: '500',
  },
  quorumValue: {
    fontSize: 12,
    color: colors.textSecondary,
    fontWeight: '500',
  },
  quorumBar: {
    height: 6,
    backgroundColor: colors.border,
    borderRadius: 3,
    overflow: 'hidden',
  },
  quorumProgress: {
    height: '100%',
    backgroundColor: colors.warning,
    borderRadius: 3,
  },
  quorumMet: {
    backgroundColor: colors.success,
  },
  optionsSection: {
    gap: 8,
    marginBottom: 12,
  },
  optionRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: 12,
    backgroundColor: colors.background,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.border,
  },
  selectedOption: {
    backgroundColor: colors.accentSoft,
    borderColor: colors.accent,
  },
  optionDisabled: {
    opacity: 0.8,
  },
  optionLeft: {
    flexDirection: 'row',
    alignItems: 'center',
    flex: 1,
  },
  radioButton: {
    width: 20,
    height: 20,
    borderRadius: 10,
    borderWidth: 2,
    borderColor: colors.borderStrong,
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: 10,
  },
  radioSelected: {
    borderColor: colors.accent,
  },
  radioInner: {
    width: 10,
    height: 10,
    borderRadius: 5,
    backgroundColor: colors.accent,
  },
  optionLabel: {
    fontSize: 14,
    color: colors.textSecondary,
    flex: 1,
  },
  optionLabelSelected: {
    color: colors.accent,
    fontWeight: '500',
  },
  optionRight: {
    alignItems: 'flex-end',
    minWidth: 80,
  },
  percentageText: {
    fontSize: 13,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 4,
  },
  percentageBar: {
    height: 4,
    width: 60,
    backgroundColor: colors.border,
    borderRadius: 2,
    overflow: 'hidden',
  },
  percentageProgress: {
    height: '100%',
    backgroundColor: colors.accent,
    borderRadius: 2,
  },
  votedMessage: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 8,
    backgroundColor: colors.successBg,
    borderRadius: 6,
    marginTop: 4,
    marginBottom: 12,
    gap: 6,
  },
  votedIcon: {
    fontSize: 14,
    color: colors.successDark,
  },
  votedText: {
    fontSize: 13,
    color: colors.successDark,
    fontWeight: '500',
  },
  voteFooter: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingTop: 12,
    borderTopWidth: 1,
    borderTopColor: colors.surfaceMuted,
  },
  createdBy: {
    fontSize: 12,
    color: colors.textSubtle,
  },
  dateRange: {
    fontSize: 12,
    color: colors.textSubtle,
  },
  bottomSpacer: {
    height: 100,
  },
});
