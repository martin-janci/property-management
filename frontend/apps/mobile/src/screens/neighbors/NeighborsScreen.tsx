/**
 * NeighborsScreen (UC-06).
 *
 * Lists fellow residents who have opted into the directory. Mock data is
 * used until `@ppt/api-client/neighbors` is wired in.
 */

import { useCallback, useMemo, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface Neighbor {
  id: string;
  name: string;
  apartment: string;
  floor: number;
  initials: string;
  phone?: string;
  email?: string;
}

const MOCK_NEIGHBORS: Neighbor[] = [
  { id: 'n1', name: 'Anna Novak', apartment: '3B', floor: 3, initials: 'AN', phone: '+421 905 123 456' },
  { id: 'n2', name: 'Peter Kovac', apartment: '2A', floor: 2, initials: 'PK', email: 'peter@example.com' },
  { id: 'n3', name: 'Eva Horvath', apartment: '5C', floor: 5, initials: 'EH' },
  { id: 'n4', name: 'Martin Toth', apartment: '1A', floor: 1, initials: 'MT', phone: '+421 905 555 444' },
];

interface NeighborsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function NeighborsScreen({ onNavigate }: NeighborsScreenProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [search, setSearch] = useState('');

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 600));
    setRefreshing(false);
  }, []);

  const filtered = useMemo(
    () =>
      MOCK_NEIGHBORS.filter((n) =>
        search.trim() === ''
          ? true
          : `${n.name} ${n.apartment}`.toLowerCase().includes(search.toLowerCase())
      ),
    [search]
  );

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Neighbours</Text>
        <Text style={s.headerSubtitle}>{MOCK_NEIGHBORS.length} residents in your building</Text>
      </View>

      <View style={s.searchContainer}>
        <TextInput
          style={s.searchInput}
          placeholder="Search by name or apartment…"
          value={search}
          onChangeText={setSearch}
        />
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>🏘️</Text>
            <Text style={s.emptyTitle}>No matches</Text>
            <Text style={s.emptyText}>Try a different name or apartment number.</Text>
          </View>
        ) : (
          filtered.map((neighbor) => (
            <Pressable
              key={neighbor.id}
              style={[s.card, styles.row]}
              onPress={() => onNavigate?.('NeighborDetail', { neighborId: neighbor.id })}
            >
              <View style={styles.avatar}>
                <Text style={styles.avatarText}>{neighbor.initials}</Text>
              </View>
              <View style={styles.info}>
                <Text style={s.cardTitle}>{neighbor.name}</Text>
                <Text style={s.cardMeta}>
                  Apt {neighbor.apartment} · Floor {neighbor.floor}
                </Text>
                {neighbor.phone && <Text style={styles.contact}>📞 {neighbor.phone}</Text>}
                {neighbor.email && <Text style={styles.contact}>✉️ {neighbor.email}</Text>}
              </View>
            </Pressable>
          ))
        )}

        <Pressable style={s.primaryButton} onPress={() => onNavigate?.('InviteNeighbor')}>
          <Text style={s.primaryButtonText}>Invite a neighbour</Text>
        </Pressable>

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  row: { flexDirection: 'row', alignItems: 'center', gap: 16 },
  avatar: {
    width: 56, height: 56, borderRadius: 28,
    backgroundColor: colors.accent, alignItems: 'center', justifyContent: 'center',
  },
  avatarText: { color: '#fff', fontSize: 18, fontWeight: '600' },
  info: { flex: 1 },
  contact: { fontSize: 13, color: colors.textMuted, marginTop: 4 },
});
