/**
 * OutagesScreen (UC-12).
 *
 * Lists planned and active utility outages affecting the building.
 */

import { useCallback, useMemo, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
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

const MOCK_OUTAGES: Outage[] = [
  {
    id: 'o1',
    commodity: 'water',
    title: 'Cold water shutdown — pipe replacement',
    description: 'Cold water supply will be unavailable in floors 1–6 during the maintenance.',
    status: 'scheduled',
    severity: 'medium',
    startsAt: '2026-04-26T08:00:00Z',
    endsAt: '2026-04-26T13:00:00Z',
  },
  {
    id: 'o2',
    commodity: 'electricity',
    title: 'Common-area electricity test',
    description: 'Brief electricity interruption in hallways and the lobby (about 15 minutes).',
    status: 'scheduled',
    severity: 'low',
    startsAt: '2026-05-02T09:00:00Z',
    endsAt: '2026-05-02T09:30:00Z',
  },
  {
    id: 'o3',
    commodity: 'heat',
    title: 'Annual heating-system shutdown',
    description: 'End-of-season shutdown of the central heating loop. No heating until October.',
    status: 'in_progress',
    severity: 'low',
    startsAt: '2026-04-15T00:00:00Z',
    endsAt: '2026-10-01T00:00:00Z',
  },
];

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
  const dateOpts: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' };
  const timeOpts: Intl.DateTimeFormatOptions = { hour: '2-digit', minute: '2-digit' };
  if (sameDay) {
    return `${start.toLocaleDateString('en-US', dateOpts)} · ${start.toLocaleTimeString(
      'en-US',
      timeOpts
    )} – ${end.toLocaleTimeString('en-US', timeOpts)}`;
  }
  return `${start.toLocaleDateString('en-US', dateOpts)} – ${end.toLocaleDateString(
    'en-US',
    dateOpts
  )}`;
}

interface OutagesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function OutagesScreen({ onNavigate }: OutagesScreenProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [filter, setFilter] = useState<'all' | OutageStatus>('all');

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 500));
    setRefreshing(false);
  }, []);

  const filtered = useMemo(
    () => (filter === 'all' ? MOCK_OUTAGES : MOCK_OUTAGES.filter((o) => o.status === filter)),
    [filter]
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
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>✅</Text>
            <Text style={s.emptyTitle}>No outages</Text>
            <Text style={s.emptyText}>Nothing scheduled or active right now.</Text>
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
