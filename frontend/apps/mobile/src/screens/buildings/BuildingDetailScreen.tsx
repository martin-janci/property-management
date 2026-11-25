/**
 * BuildingDetailScreen (UC-15).
 *
 * Read-only detail of one building, opened from the Buildings list
 * (`onNavigate('BuildingDetail', { buildingId })`). Before this screen existed
 * the tap fell through to the Dashboard.
 *
 * Wired to `GET /api/v1/buildings/{id}`, which returns a bare
 * `BuildingResponse` (snake_case, no envelope).
 */

import { useTranslation } from 'react-i18next';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface BuildingDetail {
  id: string;
  name: string;
  address: string;
  description: string;
  yearBuilt: number | null;
  floors: number | null;
  entrances: number | null;
  amenities: string[];
  status: string;
}

const str = (value: unknown): string => (typeof value === 'string' ? value : '');
const num = (value: unknown): number | null =>
  typeof value === 'number' && Number.isFinite(value) ? value : null;

/** Validate and map the bare `BuildingResponse`. `null` when it is not one. */
export function parseBuildingDetail(data: unknown): BuildingDetail | null {
  if (typeof data !== 'object' || data === null) return null;
  const b = data as Record<string, unknown>;
  if (!str(b.id) || !str(b.street)) return null;
  const cityLine = [str(b.postal_code), str(b.city)].filter(Boolean).join(' ');
  const address = [str(b.street), cityLine].filter(Boolean).join(', ');
  return {
    id: str(b.id),
    name: str(b.name) || str(b.street),
    address,
    description: str(b.description),
    yearBuilt: num(b.year_built),
    floors: num(b.total_floors),
    entrances: num(b.total_entrances),
    amenities: Array.isArray(b.amenities)
      ? b.amenities.filter((a): a is string => typeof a === 'string' && a !== '')
      : [],
    status: str(b.status),
  };
}

interface BuildingDetailScreenProps {
  buildingId?: string;
  onBack?: () => void;
}

export function BuildingDetailScreen({ buildingId, onBack }: BuildingDetailScreenProps) {
  const { t } = useTranslation();
  const { data, isLoading, error } = useApiQuery<unknown>(
    ['buildings', 'detail', buildingId],
    `/api/v1/buildings/${buildingId ?? ''}`,
    { staleTime: 60_000, enabled: !!buildingId }
  );

  const back = (
    <Pressable style={styles.backButton} onPress={onBack} disabled={!onBack}>
      <Text style={styles.backButtonText}>{t('buildings.backToList')}</Text>
    </Pressable>
  );

  const building = parseBuildingDetail(data);

  if (!buildingId || error || (!isLoading && !building)) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <Text style={s.emptyIcon}>⚠️</Text>
          <Text style={s.emptyTitle}>{t('buildings.detailLoadError')}</Text>
        </View>
      </View>
    );
  }

  if (isLoading || !building) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <ActivityIndicator size="large" color={colors.accent} />
          <Text style={s.emptyText}>{t('common.loading')}</Text>
        </View>
      </View>
    );
  }

  const rows: Array<[string, string]> = [
    [t('buildings.yearBuilt'), building.yearBuilt != null ? String(building.yearBuilt) : ''],
    [t('buildings.floors'), building.floors != null ? String(building.floors) : ''],
    [t('buildings.entrances'), building.entrances != null ? String(building.entrances) : ''],
  ];

  return (
    <View style={s.container}>
      {back}
      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          <View style={s.cardHeader}>
            <Text style={s.cardTitle}>{building.name}</Text>
            {building.status ? (
              <View style={s.badge}>
                <Text style={s.badgeText}>{building.status}</Text>
              </View>
            ) : null}
          </View>
          <Text style={styles.address}>📍 {building.address}</Text>
          {building.description ? <Text style={s.cardBody}>{building.description}</Text> : null}
        </View>

        <View style={s.card}>
          {rows
            .filter(([, value]) => value)
            .map(([label, value]) => (
              <View key={label} style={styles.row}>
                <Text style={styles.rowLabel}>{label}</Text>
                <Text style={styles.rowValue}>{value}</Text>
              </View>
            ))}
        </View>

        {building.amenities.length > 0 ? (
          <View style={s.card}>
            <Text style={styles.sectionTitle}>{t('buildings.amenities')}</Text>
            <Text style={s.cardBody}>{building.amenities.join(', ')}</Text>
          </View>
        ) : null}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  backButton: { paddingHorizontal: 16, paddingVertical: 12 },
  backButtonText: { color: colors.accent, fontSize: 16, fontWeight: '600' },
  address: { color: colors.textMuted, marginTop: 4, marginBottom: 8 },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 6,
  },
  rowLabel: { color: colors.textMuted, fontSize: 14 },
  rowValue: { color: colors.text, fontSize: 14, fontWeight: '500', flexShrink: 1 },
  sectionTitle: { color: colors.text, fontSize: 15, fontWeight: '600', marginBottom: 6 },
});
