/**
 * LeasesScreen (UC-34.1).
 *
 * Lists active and historical leases for the user (as tenant or landlord).
 */

import { useCallback, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type LeaseStatus = 'active' | 'pending' | 'expired' | 'terminated';
export type LeaseRole = 'tenant' | 'landlord';

export interface Lease {
  id: string;
  unitLabel: string;
  buildingName: string;
  role: LeaseRole;
  rentAmount: number;
  currency: string;
  startsAt: string;
  endsAt: string;
  status: LeaseStatus;
  counterpartyName: string;
}

const MOCK_LEASES: Lease[] = [
  {
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
  },
  {
    id: 'l2',
    unitLabel: 'Garage 12',
    buildingName: 'Lipová Residence',
    role: 'landlord',
    rentAmount: 80,
    currency: 'EUR',
    startsAt: '2025-01-01T00:00:00Z',
    endsAt: '2025-12-31T00:00:00Z',
    status: 'expired',
    counterpartyName: 'Peter Kovac',
  },
];

function statusColor(status: LeaseStatus): { background: string; color: string } {
  switch (status) {
    case 'active':
      return { background: colors.successBg, color: colors.successDark };
    case 'pending':
      return { background: colors.warningBg, color: colors.warningDark };
    case 'expired':
      return { background: colors.border, color: colors.textSecondary };
    case 'terminated':
      return { background: colors.dangerBg, color: colors.danger };
  }
}

interface LeasesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function LeasesScreen({ onNavigate }: LeasesScreenProps) {
  const [refreshing, setRefreshing] = useState(false);

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 500));
    setRefreshing(false);
  }, []);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Leases</Text>
        <Text style={s.headerSubtitle}>Your active and past rental agreements.</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {MOCK_LEASES.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📑</Text>
            <Text style={s.emptyTitle}>No leases</Text>
            <Text style={s.emptyText}>You haven't been added to any rental agreement yet.</Text>
          </View>
        ) : (
          MOCK_LEASES.map((lease) => {
            const sc = statusColor(lease.status);
            return (
              <Pressable
                key={lease.id}
                style={s.card}
                onPress={() => onNavigate?.('LeaseDetail', { leaseId: lease.id })}
              >
                <View style={s.cardHeader}>
                  <Text style={s.cardTitle}>
                    {lease.unitLabel} · {lease.buildingName}
                  </Text>
                  <View style={[s.badge, { backgroundColor: sc.background }]}>
                    <Text style={[s.badgeText, { color: sc.color }]}>{lease.status}</Text>
                  </View>
                </View>
                <Text style={s.cardBody}>
                  As {lease.role}: {lease.counterpartyName}
                </Text>
                <Text style={[s.cardBody, { fontWeight: '600' }]}>
                  {lease.rentAmount} {lease.currency} / month
                </Text>
                <Text style={[s.cardMeta, { marginTop: 8 }]}>
                  {new Date(lease.startsAt).toLocaleDateString()} –{' '}
                  {new Date(lease.endsAt).toLocaleDateString()}
                </Text>
              </Pressable>
            );
          })
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

// Reserved for future overrides.
export const LEASES_SCREEN_STYLES = StyleSheet.create({});
