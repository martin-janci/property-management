/**
 * PersonMonthsScreen (UC-10).
 *
 * Read-only view of the person-months recorded for the signed-in resident's
 * unit(s), used for cost allocation. Editing stays with the building manager
 * (the api-server only lets manager roles write person-months), so the screen
 * has no edit or bulk-entry controls; residents are told to contact their
 * manager if a value is wrong.
 *
 * Data:
 * - `GET /api/v1/users/me/units` (camelCase `MyUnit[]`) resolves the caller's
 *   own units, which supplies the building and unit ids.
 * - `GET /api/v1/buildings/{building_id}/units/{unit_id}/person-months?year=`
 *   (snake_case `PersonMonthResponse[]`) returns the recorded months.
 */

import type { MyUnit } from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

/** One `PersonMonthResponse` row from the api-server. */
export interface PersonMonthRow {
  id: string;
  unit_id: string;
  year: number;
  month: number;
  count: number;
  source: string;
  notes: string | null;
}

/** Validate the raw response and sort newest month first. Exported for tests. */
export function parsePersonMonths(data: unknown): PersonMonthRow[] {
  if (!Array.isArray(data)) return [];
  return data
    .filter(
      (r): r is PersonMonthRow =>
        !!r &&
        typeof r === 'object' &&
        typeof (r as PersonMonthRow).id === 'string' &&
        typeof (r as PersonMonthRow).month === 'number' &&
        typeof (r as PersonMonthRow).year === 'number' &&
        typeof (r as PersonMonthRow).count === 'number'
    )
    .sort((a, b) => b.year - a.year || b.month - a.month);
}

function formatMonth(year: number, month: number): string {
  return formatDate(new Date(year, month - 1, 1), { month: 'long', year: 'numeric' });
}

interface PersonMonthsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function PersonMonthsScreen(_props: PersonMonthsScreenProps) {
  const { t } = useTranslation();
  const [year, setYear] = useState(() => new Date().getFullYear());
  const [selectedUnitId, setSelectedUnitId] = useState<string | null>(null);

  const unitsQuery = useApiQuery<MyUnit[]>(['my-units', 'list'], '/api/v1/users/me/units', {
    staleTime: 60_000,
  });
  const units = Array.isArray(unitsQuery.data) ? unitsQuery.data : [];
  const current = units.find((u) => u.unit.id === selectedUnitId) ?? units[0];
  const buildingId = current?.unit.buildingId;
  const unitId = current?.unit.id;

  const monthsQuery = useApiQuery<unknown>(
    ['person-months', buildingId, unitId, year],
    `/api/v1/buildings/${buildingId ?? ''}/units/${unitId ?? ''}/person-months?year=${year}`,
    { staleTime: 30_000, enabled: !!buildingId && !!unitId }
  );
  const rows = parsePersonMonths(monthsQuery.data);

  const { refetch: refetchUnits } = unitsQuery;
  const { refetch: refetchMonths } = monthsQuery;
  const onRefresh = useCallback(async () => {
    await Promise.all([refetchUnits(), refetchMonths()]);
  }, [refetchUnits, refetchMonths]);

  const loading = unitsQuery.isLoading || (!!unitId && monthsQuery.isLoading);
  const failed = unitsQuery.error || monthsQuery.error;

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>{t('personMonths.title')}</Text>
        <Text style={s.headerSubtitle}>{t('personMonths.subtitle')}</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={
          <RefreshControl
            refreshing={unitsQuery.isFetching || monthsQuery.isFetching}
            onRefresh={onRefresh}
          />
        }
      >
        {units.length > 1 && (
          <View style={styles.chipRow}>
            {units.map((u) => {
              const active = u.unit.id === unitId;
              return (
                <Pressable
                  key={u.unit.id}
                  style={[s.filterChip, active && s.filterChipActive]}
                  onPress={() => setSelectedUnitId(u.unit.id)}
                >
                  <Text style={[s.filterText, active && s.filterTextActive]}>
                    {u.unit.designation}
                  </Text>
                </Pressable>
              );
            })}
          </View>
        )}

        <View style={styles.yearRow}>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('personMonths.previousYear')}
            onPress={() => setYear((y) => y - 1)}
            style={styles.yearButton}
          >
            <Text style={styles.yearButtonText}>‹</Text>
          </Pressable>
          <Text style={styles.yearText}>{year}</Text>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('personMonths.nextYear')}
            onPress={() => setYear((y) => y + 1)}
            style={styles.yearButton}
          >
            <Text style={styles.yearButtonText}>›</Text>
          </Pressable>
        </View>

        {loading ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : failed ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('personMonths.loadError')}</Text>
          </View>
        ) : !current ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>🏠</Text>
            <Text style={s.emptyTitle}>{t('personMonths.noUnit')}</Text>
          </View>
        ) : rows.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>👥</Text>
            <Text style={s.emptyTitle}>{t('personMonths.empty')}</Text>
          </View>
        ) : (
          rows.map((row) => (
            <View key={row.id} style={s.card}>
              <View style={s.cardHeader}>
                <Text style={s.cardTitle}>{formatMonth(row.year, row.month)}</Text>
              </View>
              <Text style={styles.value}>
                {t('personMonths.personCount', { count: row.count })}
              </Text>
              {row.notes ? <Text style={s.cardBody}>{row.notes}</Text> : null}
            </View>
          ))
        )}

        {current && <Text style={styles.hint}>{t('personMonths.contactManager')}</Text>}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  chipRow: { flexDirection: 'row', flexWrap: 'wrap', gap: 8, marginBottom: 12 },
  yearRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 16,
    marginBottom: 12,
  },
  yearButton: { paddingHorizontal: 12, paddingVertical: 4 },
  yearButtonText: { fontSize: 22, color: colors.accent },
  yearText: { fontSize: 17, fontWeight: '600', color: colors.text },
  value: { fontSize: 15, color: colors.textSecondary, marginTop: 4 },
  hint: { fontSize: 13, color: colors.textMuted, textAlign: 'center', marginTop: 8 },
});
