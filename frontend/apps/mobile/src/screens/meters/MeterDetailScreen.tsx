/**
 * MeterDetailScreen (UC-11.4 / 11.6).
 *
 * Single meter view: history of readings, consumption summary and a CTA to
 * submit a new reading.
 */

import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

interface MeterReadingRecord {
  id: string;
  value: number;
  unit: string;
  takenAt: string;
}

const MOCK_READINGS: MeterReadingRecord[] = [
  { id: 'r1', value: 142.4, unit: 'm³', takenAt: '2026-03-31T09:10:00Z' },
  { id: 'r2', value: 141.2, unit: 'm³', takenAt: '2026-02-28T08:45:00Z' },
  { id: 'r3', value: 139.7, unit: 'm³', takenAt: '2026-01-31T09:05:00Z' },
  { id: 'r4', value: 138.0, unit: 'm³', takenAt: '2025-12-31T09:00:00Z' },
];

interface MeterDetailScreenProps {
  meterId?: string;
  meterLabel?: string;
  onBack?: () => void;
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MeterDetailScreen({
  meterId,
  meterLabel = 'Cold water',
  onBack,
  onNavigate,
}: MeterDetailScreenProps) {
  const lastTwo = MOCK_READINGS.slice(0, 2);
  const monthDelta = lastTwo.length === 2 ? Math.max(0, lastTwo[0].value - lastTwo[1].value) : null;

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← Meters</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>{meterLabel}</Text>
        {meterId && <Text style={s.headerSubtitle}>ID: {meterId}</Text>}
      </View>

      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          <Text style={styles.metricLabel}>Latest reading</Text>
          <Text style={styles.metricValue}>
            {MOCK_READINGS[0].value} {MOCK_READINGS[0].unit}
          </Text>
          <Text style={s.cardMeta}>{new Date(MOCK_READINGS[0].takenAt).toLocaleDateString()}</Text>
          {monthDelta != null && (
            <Text style={[s.cardBody, { marginTop: 8 }]}>
              Used in last month: {monthDelta.toFixed(2)} {MOCK_READINGS[0].unit}
            </Text>
          )}
        </View>

        <Text style={styles.sectionTitle}>Reading history</Text>
        {MOCK_READINGS.map((r) => (
          <View key={r.id} style={s.card}>
            <View style={s.cardHeader}>
              <Text style={s.cardTitle}>
                {r.value} {r.unit}
              </Text>
              <Text style={s.cardMeta}>{new Date(r.takenAt).toLocaleDateString()}</Text>
            </View>
          </View>
        ))}

        <Pressable
          style={s.primaryButton}
          onPress={() => onNavigate?.('MeterReading', { meterId })}
        >
          <Text style={s.primaryButtonText}>Submit new reading</Text>
        </Pressable>

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  metricLabel: {
    fontSize: 12,
    color: colors.textMuted,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  metricValue: { fontSize: 32, fontWeight: '700', color: colors.text, marginTop: 8 },
  sectionTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.textMuted,
    marginTop: 16,
    marginBottom: 8,
    marginHorizontal: 4,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
});
