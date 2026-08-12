/**
 * OutagesScreen (UC-12).
 *
 * Lists planned and active utility outages affecting the building.
 */

import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { resolveLocale } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type OutageCommodity = 'water' | 'electricity' | 'gas' | 'heat' | 'internet';
export type OutageStatus = 'scheduled' | 'in_progress' | 'resolved';
export type OutageSeverity = 'low' | 'medium' | 'high';

export interface Outage {
  id: string;
  commodity: OutageCommodity;
  title: string;
  description: string;
  status: OutageStatus;
  severity: OutageSeverity;
  startsAt: string;
  endsAt: string;
}

interface ApiOutage {
  id: string;
  commodity: string;
  title: string;
  description?: string | null;
  status: string;
  severity?: string | null;
  starts_at: string;
  ends_at: string;
}

interface ApiOutageListResponse {
  outages: ApiOutage[];
  total?: number;
}

const OUTAGE_COMMODITIES: readonly OutageCommodity[] = [
  'water',
  'electricity',
  'gas',
  'heat',
  'internet',
];
const OUTAGE_STATUSES: readonly OutageStatus[] = ['scheduled', 'in_progress', 'resolved'];
const OUTAGE_SEVERITIES: readonly OutageSeverity[] = ['low', 'medium', 'high'];

// `as Foo ?? default` doesn't actually guard at runtime — an unknown value
// flows through and then blows up `commodityIcon` / `severityColor` /
// `statusBadgeColor`. Narrow against the allow-list instead.
const narrow = <T extends string>(
  value: string | null | undefined,
  allowed: readonly T[],
  fallback: T
): T => (value && (allowed as readonly string[]).includes(value) ? (value as T) : fallback);

function toUiOutage(o: ApiOutage): Outage {
  return {
    id: o.id,
    commodity: narrow(o.commodity, OUTAGE_COMMODITIES, 'water'),
    title: o.title,
    description: o.description ?? '',
    status: narrow(o.status, OUTAGE_STATUSES, 'scheduled'),
    severity: narrow(o.severity, OUTAGE_SEVERITIES, 'low'),
    startsAt: o.starts_at,
    endsAt: o.ends_at,
  };
}

function commodityIcon(commodity: OutageCommodity): string {
  switch (commodity) {
    case 'water':
      return '💧';
    case 'electricity':
      return '⚡';
    case 'gas':
      return '🟦';
    case 'heat':
      return '🌡️';
    case 'internet':
      return '🛰️';
  }
}

function severityColor(severity: OutageSeverity): string {
  return severity === 'high'
    ? colors.danger
    : severity === 'medium'
      ? colors.warning
      : colors.success;
}

function formatRange(startsAt: string, endsAt: string): string {
  const start = new Date(startsAt);
  const end = new Date(endsAt);
  const sameDay = start.toDateString() === end.toDateString();
  const locale = resolveLocale();
  const dateOpts: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' };
  const timeOpts: Intl.DateTimeFormatOptions = { hour: '2-digit', minute: '2-digit' };
  if (sameDay) {
    return `${start.toLocaleDateString(locale, dateOpts)} · ${start.toLocaleTimeString(
      locale,
      timeOpts
    )} – ${end.toLocaleTimeString(locale, timeOpts)}`;
  }
  return `${start.toLocaleDateString(locale, dateOpts)} – ${end.toLocaleDateString(
    locale,
    dateOpts
  )}`;
}

interface OutagesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function OutagesScreen({ onNavigate }: OutagesScreenProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<'all' | OutageStatus>('all');

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiOutageListResponse>(
    ['outages', 'list'],
    '/api/v1/outages',
    { staleTime: 30_000 }
  );

  const outages: Outage[] = (data?.outages ?? []).map(toUiOutage);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const filtered = useMemo(
    () => (filter === 'all' ? outages : outages.filter((o) => o.status === filter)),
    [outages, filter]
  );

  const filters: ReadonlyArray<{ value: 'all' | OutageStatus; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'scheduled', label: 'Scheduled' },
    { value: 'in_progress', label: 'Active' },
    { value: 'resolved', label: 'Resolved' },
  ];

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Outages</Text>
        <Text style={s.headerSubtitle}>Planned and active utility interruptions.</Text>
      </View>

      <View style={s.filtersWrapper}>
        <ScrollView horizontal showsHorizontalScrollIndicator={false}>
          <View style={s.filtersRow}>
            {filters.map((option) => (
              <Pressable
                key={option.value}
                style={[s.filterChip, filter === option.value && s.filterChipActive]}
                onPress={() => setFilter(option.value)}
              >
                <Text style={[s.filterText, filter === option.value && s.filterTextActive]}>
                  {option.label}
                </Text>
              </Pressable>
            ))}
          </View>
        </ScrollView>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('outages.loadError')}</Text>
          </View>
        ) : filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>✅</Text>
            <Text style={s.emptyTitle}>{t('outages.emptyTitle')}</Text>
            <Text style={s.emptyText}>{t('outages.emptyText')}</Text>
          </View>
        ) : (
          filtered.map((outage) => (
            <Pressable
              key={outage.id}
              style={s.card}
              onPress={() => onNavigate?.('OutageDetail', { outageId: outage.id })}
            >
              <View style={s.cardHeader}>
                <Text style={styles.icon}>{commodityIcon(outage.commodity)}</Text>
                <Text style={[s.cardTitle, { marginLeft: 8 }]} numberOfLines={2}>
                  {outage.title}
                </Text>
                <View style={[styles.dot, { backgroundColor: severityColor(outage.severity) }]} />
              </View>
              <Text style={s.cardBody}>{outage.description}</Text>
              <View style={s.cardFooter}>
                <Text style={s.cardMeta}>{formatRange(outage.startsAt, outage.endsAt)}</Text>
                <View style={s.badge}>
                  <Text style={s.badgeText}>{outage.status.replace('_', ' ')}</Text>
                </View>
              </View>
            </Pressable>
          ))
        )}
        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  icon: { fontSize: 20 },
  dot: { width: 12, height: 12, borderRadius: 6, marginLeft: 8 },
});
