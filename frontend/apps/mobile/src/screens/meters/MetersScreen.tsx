/**
 * MetersScreen (UC-11.1).
 *
 * Lists meters assigned to the user's unit with their last reading and the
 * next due date. The existing `MeterReadingScreen` handles the submission
 * flow; this screen wraps that flow with a list view.
 */

import { useCallback, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type MeterCommodity = 'water_cold' | 'water_hot' | 'electricity' | 'gas' | 'heat';

export interface Meter {
  id: string;
  label: string;
  commodity: MeterCommodity;
  unit: string;
  lastReading: number | null;
  lastReadingAt: string | null;
  dueAt: string;
}

const MOCK_METERS: Meter[] = [
  {
    id: 'm-w-cold',
    label: 'Cold water',
    commodity: 'water_cold',
    unit: 'm³',
    lastReading: 142.4,
    lastReadingAt: '2026-03-31T09:10:00Z',
    dueAt: '2026-04-30T23:59:00Z',
  },
  {
    id: 'm-w-hot',
    label: 'Hot water',
    commodity: 'water_hot',
    unit: 'm³',
    lastReading: 88.6,
    lastReadingAt: '2026-03-31T09:12:00Z',
    dueAt: '2026-04-30T23:59:00Z',
  },
  {
    id: 'm-elec',
    label: 'Electricity',
    commodity: 'electricity',
    unit: 'kWh',
    lastReading: 4521,
    lastReadingAt: '2026-04-01T08:00:00Z',
    dueAt: '2026-05-01T23:59:00Z',
  },
  {
    id: 'm-heat',
    label: 'Heating',
    commodity: 'heat',
    unit: 'GJ',
    lastReading: null,
    lastReadingAt: null,
    dueAt: '2026-04-30T23:59:00Z',
  },
];

function commodityIcon(commodity: MeterCommodity): string {
  switch (commodity) {
    case 'water_cold':
      return '💧';
    case 'water_hot':
      return '🔥';
    case 'electricity':
      return '⚡';
    case 'gas':
      return '🟦';
    case 'heat':
      return '🌡️';
  }
}

function daysUntil(dueAt: string): number {
  return Math.ceil((new Date(dueAt).getTime() - Date.now()) / (1000 * 60 * 60 * 24));
}

interface MetersScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MetersScreen({ onNavigate }: MetersScreenProps) {
  const [refreshing, setRefreshing] = useState(false);

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 600));
    setRefreshing(false);
  }, []);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Meters</Text>
        <Text style={s.headerSubtitle}>Submit your readings before the due date.</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {MOCK_METERS.map((meter) => {
          const days = daysUntil(meter.dueAt);
          const overdue = days < 0;
          const dueSoon = days >= 0 && days <= 7;
          return (
            <Pressable
              key={meter.id}
              style={s.card}
              onPress={() => onNavigate?.('MeterDetail', { meterId: meter.id })}
            >
              <View style={s.cardHeader}>
                <Text style={styles.icon}>{commodityIcon(meter.commodity)}</Text>
                <Text style={[s.cardTitle, { marginLeft: 8 }]}>{meter.label}</Text>
                <View
                  style={[
                    s.badge,
                    overdue && styles.badgeOverdue,
                    dueSoon && !overdue && styles.badgeDueSoon,
                  ]}
                >
                  <Text
                    style={[
                      s.badgeText,
                      overdue && styles.badgeOverdueText,
                      dueSoon && !overdue && styles.badgeDueSoonText,
                    ]}
                  >
                    {overdue ? `${Math.abs(days)}d overdue` : `${days}d left`}
                  </Text>
                </View>
              </View>

              <Text style={s.cardBody}>
                Last reading:{' '}
                {meter.lastReading != null
                  ? `${meter.lastReading} ${meter.unit}`
                  : '— no reading yet'}
                {meter.lastReadingAt
                  ? ` (${new Date(meter.lastReadingAt).toLocaleDateString()})`
                  : ''}
              </Text>

              <View style={s.cardFooter}>
                <Pressable
                  style={styles.linkButton}
                  onPress={() => onNavigate?.('MeterReading', { meterId: meter.id })}
                >
                  <Text style={styles.linkButtonText}>Submit reading →</Text>
                </Pressable>
              </View>
            </Pressable>
          );
        })}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  icon: { fontSize: 20 },
  badgeOverdue: { backgroundColor: colors.dangerBg },
  badgeOverdueText: { color: colors.danger },
  badgeDueSoon: { backgroundColor: colors.warningBg },
  badgeDueSoonText: { color: colors.warningDark },
  linkButton: { paddingVertical: 4 },
  linkButtonText: { color: colors.accent, fontSize: 14, fontWeight: '500' },
});
