import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate as formatLocaleDate } from '../../i18n/format';
import { colors } from '../shared/screenStyles';

export type FaultStatus = 'open' | 'in_progress' | 'resolved' | 'closed';
export type FaultPriority = 'low' | 'medium' | 'high' | 'urgent';
export type FaultCategory =
  | 'plumbing'
  | 'electrical'
  | 'structural'
  | 'hvac'
  | 'elevator'
  | 'security'
  | 'other';

export interface Fault {
  id: string;
  title: string;
  description: string;
  status: FaultStatus;
  priority: FaultPriority;
  category: FaultCategory;
  location: string;
  createdAt: string;
  updatedAt: string;
  photos: string[];
  reportedBy: string;
  assignedTo?: string;
}

interface FaultsListScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

/** Subset of `FaultSummary` from the api-server that the list view needs. */
interface ApiFaultSummary {
  id: string;
  title: string;
  description?: string | null;
  status: string;
  priority: string;
  category: string;
  location_description?: string | null;
  created_at: string;
  updated_at: string;
  reporter_name?: string | null;
  assignee_name?: string | null;
}

interface ApiFaultListResponse {
  faults: ApiFaultSummary[];
  count: number;
}

const FAULT_STATUSES: readonly FaultStatus[] = ['open', 'in_progress', 'resolved', 'closed'];
const FAULT_PRIORITIES: readonly FaultPriority[] = ['low', 'medium', 'high', 'urgent'];
const FAULT_CATEGORIES: readonly FaultCategory[] = [
  'plumbing',
  'electrical',
  'structural',
  'hvac',
  'elevator',
  'security',
  'other',
];

// Narrow an arbitrary string to a known enum value, or fall back. Plain
// `as Foo ?? 'default'` would let an unknown value flow through, which then
// blows up the colour-lookup tables (Record<Foo, …>) further down.
const narrow = <T extends string>(value: string, allowed: readonly T[], fallback: T): T =>
  (allowed as readonly string[]).includes(value) ? (value as T) : fallback;

function toUiFault(f: ApiFaultSummary): Fault {
  return {
    id: f.id,
    title: f.title,
    description: f.description ?? '',
    status: narrow(f.status, FAULT_STATUSES, 'open'),
    priority: narrow(f.priority, FAULT_PRIORITIES, 'medium'),
    category: narrow(f.category, FAULT_CATEGORIES, 'other'),
    location: f.location_description ?? '',
    createdAt: f.created_at,
    updatedAt: f.updated_at,
    photos: [],
    reportedBy: f.reporter_name ?? 'Resident',
    assignedTo: f.assignee_name ?? undefined,
  };
}

const statusBgColor: Record<FaultStatus, string> = {
  open: colors.statusNewBg,
  in_progress: colors.statusInProgressBg,
  resolved: colors.statusResolvedBg,
  closed: colors.statusClosedBg,
};

const statusInkColor: Record<FaultStatus, string> = {
  open: colors.statusNewInk,
  in_progress: colors.statusInProgressInk,
  resolved: colors.statusResolvedInk,
  closed: colors.statusClosedInk,
};

const priorityInkColor: Record<FaultPriority, string> = {
  low: colors.priorityLowInk,
  medium: colors.priorityMediumInk,
  high: colors.priorityHighInk,
  urgent: colors.priorityUrgentInk,
};

const categoryIcon: Record<FaultCategory, string> = {
  plumbing: '🚿',
  electrical: '⚡',
  structural: '🏗️',
  hvac: '❄️',
  elevator: '🛗',
  security: '🔒',
  other: '🔧',
};

const formatDate = (dateString: string): string =>
  formatLocaleDate(dateString, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });

export function FaultsListScreen({ onNavigate }: FaultsListScreenProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<'all' | 'open' | 'resolved'>('all');

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiFaultListResponse>(
    ['faults', 'list'],
    '/api/v1/faults',
    { staleTime: 30_000 }
  );

  const faults: Fault[] = (data?.faults ?? []).map(toUiFault);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const filteredFaults = faults.filter((fault) => {
    if (filter === 'all') return true;
    if (filter === 'open') return fault.status === 'open' || fault.status === 'in_progress';
    if (filter === 'resolved') return fault.status === 'resolved' || fault.status === 'closed';
    return true;
  });

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.headerTitle}>Faults</Text>
        <Pressable style={styles.addButton} onPress={() => onNavigate?.('ReportFault')}>
          <Text style={styles.addButtonText}>+ Report</Text>
        </Pressable>
      </View>

      <View style={styles.filters}>
        {(['all', 'open', 'resolved'] as const).map((option) => (
          <Pressable
            key={option}
            style={[styles.filterButton, filter === option && styles.filterButtonActive]}
            onPress={() => setFilter(option)}
          >
            <Text style={[styles.filterText, filter === option && styles.filterTextActive]}>
              {option.charAt(0).toUpperCase() + option.slice(1)}
            </Text>
          </Pressable>
        ))}
      </View>

      <ScrollView
        style={styles.scrollView}
        refreshControl={
          <RefreshControl refreshing={isFetching} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {isLoading ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>⚠️</Text>
            <Text style={styles.emptyTitle}>{t('faults.loadError')}</Text>
          </View>
        ) : filteredFaults.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>🔧</Text>
            <Text style={styles.emptyTitle}>{t('faults.emptyTitle')}</Text>
            <Text style={styles.emptyText}>{t('faults.emptyText')}</Text>
          </View>
        ) : (
          filteredFaults.map((fault) => (
            <Pressable
              key={fault.id}
              style={styles.faultCard}
              onPress={() => onNavigate?.('FaultDetail', { faultId: fault.id })}
            >
              <View style={styles.faultHeader}>
                <Text style={styles.categoryIcon}>{categoryIcon[fault.category]}</Text>
                <View style={styles.faultTitleContainer}>
                  <Text style={styles.faultTitle} numberOfLines={1}>
                    {fault.title}
                  </Text>
                  <Text style={styles.faultLocation}>{fault.location}</Text>
                </View>
                <View
                  style={[styles.statusBadge, { backgroundColor: statusBgColor[fault.status] }]}
                >
                  <Text style={[styles.statusText, { color: statusInkColor[fault.status] }]}>
                    {fault.status.replace('_', ' ')}
                  </Text>
                </View>
              </View>

              <Text style={styles.faultDescription} numberOfLines={2}>
                {fault.description}
              </Text>

              <View style={styles.faultFooter}>
                <View
                  style={[styles.priorityBadge, { borderColor: priorityInkColor[fault.priority] }]}
                >
                  <Text style={[styles.priorityText, { color: priorityInkColor[fault.priority] }]}>
                    {fault.priority}
                  </Text>
                </View>
                <Text style={styles.faultDate}>{formatDate(fault.updatedAt)}</Text>
              </View>

              {fault.assignedTo && (
                <View style={styles.assignedRow}>
                  <Text style={styles.assignedLabel}>Assigned to:</Text>
                  <Text style={styles.assignedValue}>{fault.assignedTo}</Text>
                </View>
              )}
            </Pressable>
          ))
        )}

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: colors.background },
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
  headerTitle: { fontSize: 24, fontWeight: 'bold', color: colors.text },
  addButton: {
    backgroundColor: colors.accent,
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 8,
  },
  addButtonText: { color: colors.surface, fontWeight: '600', fontSize: 14 },
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
  filterButtonActive: { backgroundColor: colors.accent },
  filterText: { fontSize: 14, color: colors.textMuted, fontWeight: '500' },
  filterTextActive: { color: colors.surface },
  scrollView: { flex: 1, padding: 16 },
  emptyState: { alignItems: 'center', justifyContent: 'center', paddingVertical: 60 },
  emptyIcon: { fontSize: 48, marginBottom: 16 },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 4,
  },
  emptyText: { fontSize: 14, color: colors.textMuted },
  faultCard: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  faultHeader: { flexDirection: 'row', alignItems: 'flex-start', marginBottom: 8 },
  categoryIcon: { fontSize: 24, marginRight: 12 },
  faultTitleContainer: { flex: 1 },
  faultTitle: { fontSize: 16, fontWeight: '600', color: colors.text },
  faultLocation: { fontSize: 13, color: colors.textMuted, marginTop: 2 },
  statusBadge: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 99,
    marginLeft: 8,
  },
  statusText: {
    fontSize: 11,
    fontWeight: '700',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  faultDescription: {
    fontSize: 14,
    color: colors.textMuted,
    lineHeight: 20,
    marginBottom: 12,
    marginLeft: 36,
  },
  faultFooter: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginLeft: 36,
  },
  priorityBadge: {
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 4,
    borderWidth: 1,
  },
  priorityText: { fontSize: 11, fontWeight: '600', textTransform: 'uppercase' },
  faultDate: { fontSize: 12, color: colors.textSubtle },
  assignedRow: {
    flexDirection: 'row',
    marginTop: 8,
    marginLeft: 36,
    paddingTop: 8,
    borderTopWidth: 1,
    borderTopColor: colors.divider,
  },
  assignedLabel: { fontSize: 12, color: colors.textMuted },
  assignedValue: { fontSize: 12, color: colors.accent, marginLeft: 4, fontWeight: '500' },
  bottomSpacer: { height: 100 },
});
