/**
 * BuildingsScreen (UC-15).
 *
 * Lists buildings the user belongs to (often one, but can be many for
 * managers and multi-property residents) and exposes building-level shortcuts.
 */

import { useCallback, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface Building {
  id: string;
  name: string;
  address: string;
  unitCount: number;
  floors: number;
  yearBuilt?: number;
  managerName?: string;
  role: 'resident' | 'owner' | 'manager';
}

const MOCK_BUILDINGS: Building[] = [
  {
    id: 'b1',
    name: 'Lipová Residence',
    address: 'Lipová 5, Bratislava',
    unitCount: 36,
    floors: 7,
    yearBuilt: 2008,
    managerName: 'Building Management s.r.o.',
    role: 'resident',
  },
  {
    id: 'b2',
    name: 'Cottage 12',
    address: 'Záhradná 12, Pezinok',
    unitCount: 1,
    floors: 2,
    yearBuilt: 1998,
    role: 'owner',
  },
];

interface BuildingsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function BuildingsScreen({ onNavigate }: BuildingsScreenProps) {
  const [refreshing, setRefreshing] = useState(false);

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 500));
    setRefreshing(false);
  }, []);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Buildings</Text>
        <Text style={s.headerSubtitle}>Properties associated with your account.</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {MOCK_BUILDINGS.map((building) => (
          <Pressable
            key={building.id}
            style={s.card}
            onPress={() => onNavigate?.('BuildingDetail', { buildingId: building.id })}
          >
            <View style={s.cardHeader}>
              <Text style={s.cardTitle}>{building.name}</Text>
              <View style={s.badge}>
                <Text style={s.badgeText}>{building.role}</Text>
              </View>
            </View>
            <Text style={styles.address}>📍 {building.address}</Text>
            <View style={styles.statsRow}>
              <Stat label="Units" value={String(building.unitCount)} />
              <Stat label="Floors" value={String(building.floors)} />
              {building.yearBuilt && <Stat label="Built" value={String(building.yearBuilt)} />}
            </View>
            {building.managerName && (
              <Text style={[s.cardMeta, { marginTop: 12 }]}>
                Managed by {building.managerName}
              </Text>
            )}
          </Pressable>
        ))}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <View style={styles.stat}>
      <Text style={styles.statValue}>{value}</Text>
      <Text style={styles.statLabel}>{label}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  address: { fontSize: 14, color: colors.textMuted, marginBottom: 12 },
  statsRow: { flexDirection: 'row', gap: 16 },
  stat: { alignItems: 'flex-start' },
  statValue: { fontSize: 18, fontWeight: '700', color: colors.text },
  statLabel: {
    fontSize: 11,
    color: colors.textMuted,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
});
