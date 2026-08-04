/**
 * BuildingsScreen (UC-15).
 *
 * Lists buildings in the user's organization (often one, but can be many for
 * managers and multi-property residents) and exposes building-level shortcuts.
 *
 * Wired to `GET /api/v1/buildings?organization_id=<tenant>` on the api-server.
 * That route runs under `RlsConnection` and requires both an Authorization
 * header and the `organization_id` query param to equal the authenticated
 * tenant — `useApiQuery` sends the bearer token + `X-Tenant-ID`, and the org
 * id comes from `useTenantId()` (the JWT `tenant_id` claim).
 */

import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery, useTenantId } from '../../hooks/useApi';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface Building {
  id: string;
  name: string;
  address: string;
  unitCount: number;
  floors: number;
  status: string;
}

/** Subset of `BuildingSummary` from `GET /api/v1/buildings`. */
export interface ApiBuildingSummary {
  id: string;
  name?: string | null;
  street: string;
  city: string;
  postal_code: string;
  total_floors: number;
  status: string;
  unit_count?: number | null;
}

interface ApiBuildingsListResponse {
  buildings: ApiBuildingSummary[];
  total?: number;
}

/** Narrow an unknown value to a well-formed `ApiBuildingSummary`.
 *
 *  The list endpoint is consumed without a generated client, so the response
 *  is `unknown` at runtime. A blind cast would crash `.map(toUiBuilding)` the
 *  moment the backend returned an unexpected shape. Require the string fields
 *  the UI dereferences and drop anything malformed. */
function isApiBuildingSummary(value: unknown): value is ApiBuildingSummary {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === 'string' &&
    typeof v.street === 'string' &&
    typeof v.city === 'string' &&
    typeof v.status === 'string'
  );
}

/** Normalise the `/api/v1/buildings` response into a validated summary list.
 *  Accepts the `{ buildings: [...] }` envelope and tolerates any other/garbage
 *  shape by returning an empty list rather than throwing. Malformed rows are
 *  dropped so one bad entry can't take down the whole screen. */
export function parseBuildingSummaries(data: unknown): ApiBuildingSummary[] {
  const candidates: unknown[] =
    typeof data === 'object' &&
    data !== null &&
    Array.isArray((data as { buildings?: unknown }).buildings)
      ? (data as { buildings: unknown[] }).buildings
      : Array.isArray(data)
        ? data
        : [];
  return candidates.filter(isApiBuildingSummary);
}

/** Map an api-server building summary onto the UI shape used by the screen.
 *  Exported so the mapping (name fallback, address composition, numeric
 *  `?? 0` defaults) can be unit-tested without rendering the screen. */
export function toUiBuilding(b: ApiBuildingSummary): Building {
  const name = b.name?.trim() || b.street;
  const address = [b.street, b.postal_code, b.city].filter(Boolean).join(', ');
  return {
    id: b.id,
    name,
    address,
    unitCount: b.unit_count ?? 0,
    floors: b.total_floors ?? 0,
    status: b.status,
  };
}

interface BuildingsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function BuildingsScreen({ onNavigate }: BuildingsScreenProps) {
  const { t } = useTranslation();
  const { tenantId } = useTenantId();

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiBuildingsListResponse>(
    ['buildings', 'list', tenantId],
    `/api/v1/buildings?organization_id=${tenantId ?? ''}`,
    { staleTime: 30_000, enabled: !!tenantId }
  );

  const buildings: Building[] = parseBuildingSummaries(data).map(toUiBuilding);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Buildings</Text>
        <Text style={s.headerSubtitle}>Properties associated with your account.</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading || !tenantId ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('buildings.loadError')}</Text>
          </View>
        ) : buildings.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>🏢</Text>
            <Text style={s.emptyTitle}>{t('buildings.emptyTitle')}</Text>
            <Text style={s.emptyText}>{t('buildings.emptyText')}</Text>
          </View>
        ) : (
          buildings.map((building) => (
            <Pressable
              key={building.id}
              style={s.card}
              onPress={() => onNavigate?.('BuildingDetail', { buildingId: building.id })}
            >
              <View style={s.cardHeader}>
                <Text style={s.cardTitle}>{building.name}</Text>
                <View style={s.badge}>
                  <Text style={s.badgeText}>{building.status}</Text>
                </View>
              </View>
              <Text style={styles.address}>📍 {building.address}</Text>
              <View style={styles.statsRow}>
                <Stat label="Units" value={String(building.unitCount)} />
                <Stat label="Floors" value={String(building.floors)} />
              </View>
            </Pressable>
          ))
        )}

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
