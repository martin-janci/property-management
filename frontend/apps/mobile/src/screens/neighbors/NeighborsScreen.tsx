/**
 * NeighborsScreen (UC-06).
 *
 * Lists fellow residents who have opted into the directory.
 * Wired to backend GET /api/v1/buildings/{building_id}/neighbors
 * with privacy-aware filtering (Epic 6, Story 6.6).
 *
 * Building context: fetches the first building the user belongs to
 * from GET /api/v1/buildings; consistent with other mobile screens.
 */

import { useCallback, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { colors, screenStyles as s } from '../shared/screenStyles';

// ---------------------------------------------------------------------------
// Types (mirrors backend NeighborView / PrivacySettings)
// ---------------------------------------------------------------------------

interface NeighborView {
  user_id: string;
  display_name: string;
  unit_label: string;
  is_visible: boolean;
  email: string | null;
  phone: string | null;
  resident_type: string;
}

interface NeighborsResponse {
  neighbors: NeighborView[];
  count: number;
  total: number;
}

export interface BuildingItem {
  id: string;
  name?: string | null;
}

/** Response shape of `GET /api/v1/buildings` (`BuildingsListResponse`). */
export interface BuildingsPaginatedResponse {
  buildings: BuildingItem[];
  total: number;
}

/**
 * Resolve the first building from the `GET /api/v1/buildings` response.
 *
 * The backend returns `{ buildings, total }` — NOT the legacy `{ items }`
 * shape the screen used to read, which always yielded `undefined` and broke
 * the neighbours lookup. Returns id/name (name coerced from a nullable
 * string to `undefined`) or both `undefined` when the list is empty/absent.
 */
export function selectBuilding(data: BuildingsPaginatedResponse | undefined): {
  buildingId: string | undefined;
  buildingName: string | undefined;
} {
  const first = data?.buildings?.[0];
  return {
    buildingId: first?.id,
    buildingName: first?.name ?? undefined,
  };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface NeighborsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function NeighborsScreen(_props: NeighborsScreenProps) {
  const [search, setSearch] = useState('');
  const [refreshKey, setRefreshKey] = useState(0);

  // Fetch the first building the user belongs to
  const buildingsQuery = useApiQuery<BuildingsPaginatedResponse>(
    ['buildings', 'list'],
    '/api/v1/buildings'
  );

  const { buildingId, buildingName } = selectBuilding(buildingsQuery.data);

  // Fetch neighbors once we have a building id
  const neighborsQuery = useApiQuery<NeighborsResponse>(
    ['neighbors', 'list', buildingId ?? '', refreshKey],
    `/api/v1/buildings/${buildingId}/neighbors`,
    { enabled: !!buildingId }
  );

  const isLoading = buildingsQuery.isLoading || neighborsQuery.isLoading;
  const error = buildingsQuery.error ?? neighborsQuery.error;

  const onRefresh = useCallback(() => {
    setRefreshKey((k) => k + 1);
  }, []);

  // Only show visible neighbors (privacy-aware filtering)
  const visibleNeighbors = useMemo(
    () => (neighborsQuery.data?.neighbors ?? []).filter((n) => n.is_visible),
    [neighborsQuery.data]
  );

  const filtered = useMemo(
    () =>
      visibleNeighbors.filter((n) =>
        search.trim() === ''
          ? true
          : `${n.display_name} ${n.unit_label}`.toLowerCase().includes(search.toLowerCase())
      ),
    [visibleNeighbors, search]
  );

  const initials = (name: string) =>
    name
      .split(' ')
      .map((w) => w[0] ?? '')
      .join('')
      .toUpperCase()
      .slice(0, 2);

  if (isLoading) {
    return (
      <View style={[s.container, styles.center]}>
        <ActivityIndicator size="large" color={colors.accent} />
        <Text style={[s.cardMeta, { marginTop: 12 }]}>Loading neighbours…</Text>
      </View>
    );
  }

  if (error) {
    return (
      <View style={[s.container, styles.center]}>
        <Text style={s.emptyIcon}>⚠️</Text>
        <Text style={s.emptyTitle}>Could not load neighbours</Text>
        <Text style={s.emptyText}>{error instanceof Error ? error.message : 'Unknown error'}</Text>
        <Pressable style={[s.primaryButton, { marginTop: 16 }]} onPress={onRefresh}>
          <Text style={s.primaryButtonText}>Retry</Text>
        </Pressable>
      </View>
    );
  }

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Neighbours</Text>
        <Text style={s.headerSubtitle}>
          {buildingName
            ? `${visibleNeighbors.length} residents in ${buildingName}`
            : `${visibleNeighbors.length} residents in your building`}
        </Text>
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
        refreshControl={
          <RefreshControl
            refreshing={neighborsQuery.isFetching && !isLoading}
            onRefresh={onRefresh}
          />
        }
      >
        {filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>🏘️</Text>
            <Text style={s.emptyTitle}>{search.trim() ? 'No matches' : 'No neighbours yet'}</Text>
            <Text style={s.emptyText}>
              {search.trim()
                ? 'Try a different name or apartment number.'
                : 'Residents who opt into the directory will appear here.'}
            </Text>
          </View>
        ) : (
          // The list row shows the full neighbour record (name, unit, phone,
          // email). There is no NeighborDetail screen, so the row is a static
          // card rather than a dead navigation target.
          filtered.map((neighbor) => (
            <View key={neighbor.user_id} style={[s.card, styles.row]}>
              <View style={styles.avatar}>
                <Text style={styles.avatarText}>{initials(neighbor.display_name)}</Text>
              </View>
              <View style={styles.info}>
                <Text style={s.cardTitle}>{neighbor.display_name}</Text>
                <Text style={s.cardMeta}>
                  {neighbor.unit_label}
                  {neighbor.resident_type === 'owner' ? ' · Owner' : ''}
                </Text>
                {neighbor.phone && <Text style={styles.contact}>📞 {neighbor.phone}</Text>}
                {neighbor.email && <Text style={styles.contact}>✉️ {neighbor.email}</Text>}
              </View>
            </View>
          ))
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  center: { alignItems: 'center', justifyContent: 'center' },
  row: { flexDirection: 'row', alignItems: 'center', gap: 16 },
  avatar: {
    width: 56,
    height: 56,
    borderRadius: 28,
    backgroundColor: colors.accent,
    alignItems: 'center',
    justifyContent: 'center',
  },
  avatarText: { color: colors.surface, fontSize: 18, fontWeight: '600' },
  info: { flex: 1 },
  contact: { fontSize: 13, color: colors.textMuted, marginTop: 4 },
});
