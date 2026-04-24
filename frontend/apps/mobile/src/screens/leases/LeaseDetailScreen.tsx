/**
 * LeaseDetailScreen (UC-34.4).
 *
 * Single-lease view with payment summary and document downloads.
 */

import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';
import type { Lease } from './LeasesScreen';

const MOCK_LEASE: Lease = {
  id: 'l1',
  unitLabel: 'Apt 3B',
  buildingName: 'Lipová Residence',
  role: 'tenant',
  rentAmount: 850,
  currency: 'EUR',
  startsAt: '2024-09-01T00:00:00Z',
  endsAt: '2026-08-31T00:00:00Z',
  status: 'active',
  counterpartyName: 'Anna Novak',
};

const MOCK_PAYMENTS: Array<{ id: string; period: string; amount: number; paidAt: string | null }> = [
  { id: 'p1', period: 'April 2026', amount: 850, paidAt: '2026-04-01T09:00:00Z' },
  { id: 'p2', period: 'March 2026', amount: 850, paidAt: '2026-03-01T09:00:00Z' },
  { id: 'p3', period: 'February 2026', amount: 850, paidAt: '2026-02-02T11:30:00Z' },
];

interface LeaseDetailScreenProps {
  leaseId?: string;
  lease?: Lease;
  onBack?: () => void;
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function LeaseDetailScreen({
  lease = MOCK_LEASE,
  onBack,
  onNavigate,
}: LeaseDetailScreenProps) {
  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← Leases</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>
          {lease.unitLabel} · {lease.buildingName}
        </Text>
        <Text style={s.headerSubtitle}>
          {new Date(lease.startsAt).toLocaleDateString()} –{' '}
          {new Date(lease.endsAt).toLocaleDateString()}
        </Text>
      </View>

      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          <Text style={styles.metricLabel}>Monthly rent</Text>
          <Text style={styles.metricValue}>
            {lease.rentAmount} {lease.currency}
          </Text>
          <Text style={[s.cardMeta, { marginTop: 8 }]}>
            As {lease.role}, paid to {lease.counterpartyName}
          </Text>
        </View>

        <Text style={styles.sectionTitle}>Payment history</Text>
        {MOCK_PAYMENTS.map((payment) => (
          <View key={payment.id} style={s.card}>
            <View style={s.cardHeader}>
              <Text style={s.cardTitle}>{payment.period}</Text>
              <Text style={s.cardMeta}>
                {payment.amount} {lease.currency}
              </Text>
            </View>
            <Text style={s.cardBody}>
              {payment.paidAt
                ? `Paid on ${new Date(payment.paidAt).toLocaleDateString()}`
                : 'Awaiting payment'}
            </Text>
          </View>
        ))}

        <Text style={styles.sectionTitle}>Documents</Text>
        <Pressable
          style={s.card}
          onPress={() =>
            onNavigate?.('DocumentDetail', {
              documentId: `lease-${lease.id}-contract`,
            })
          }
        >
          <Text style={s.cardTitle}>📄 Lease agreement</Text>
          <Text style={s.cardMeta}>Signed PDF — open to review or share.</Text>
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
