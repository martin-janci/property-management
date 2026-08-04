/**
 * MetersScreen (UC-11.1).
 *
 * Lists meters assigned to the signed-in resident's unit with their last
 * reading. The existing `MeterReadingScreen` handles the submission flow; this
 * screen wraps that flow with a list view.
 *
 * Wired to `GET /api/v1/meters/units/{unit_id}` on the api-server, which
 * returns `Vec<Meter>` (see `db::models::meter::Meter` — default snake_case
 * serde, `Decimal` fields as JSON strings). The unit id comes from the
 * authenticated user (`useAuth().user.unitId`); the query stays disabled until
 * it is known. The response is consumed without a generated client, so it is
 * validated defensively — malformed rows are dropped rather than crashing the
 * list.
 */

import type { Meter as ApiMeterPayload } from '@ppt/api-client';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useAuth } from '../../contexts/AuthContext';
import { useApiQuery } from '../../hooks/useApi';
import { warnIfListDegraded } from '../shared/parserWarnings';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type MeterCommodity = 'water_cold' | 'water_hot' | 'water' | 'electricity' | 'gas' | 'heat';

export interface Meter {
  id: string;
  label: string;
  commodity: MeterCommodity;
  unit: string;
  lastReading: number | null;
  lastReadingAt: string | null;
}

/**
 * `Meter` row from `GET /api/v1/meters/units/{unit_id}`, typed against the
 * generated `@ppt/api-client` payload so a regenerated client surfaces backend
 * field renames at compile time. Relaxed to `Partial` (only `id` is required)
 * because rows are validated defensively at runtime.
 */
export type ApiMeter = Partial<ApiMeterPayload> & Pick<ApiMeterPayload, 'id'>;

interface ApiMetersEnvelope {
  meters: ApiMeter[];
}

function isApiMeter(value: unknown): value is ApiMeter {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.id === 'string';
}

/** Normalise the `/api/v1/meters/units/{unit_id}` response into a validated
 *  list. The unit route returns a bare `Vec<Meter>`; a `{ meters: [...] }`
 *  envelope is also accepted defensively. Any other shape yields `[]`. */
export function parseMeters(data: unknown): ApiMeter[] {
  const candidates: unknown[] | null = Array.isArray(data)
    ? data
    : typeof data === 'object' && data !== null && Array.isArray((data as ApiMetersEnvelope).meters)
      ? (data as ApiMetersEnvelope).meters
      : null;
  const meters = (candidates ?? []).filter(isApiMeter);
  warnIfListDegraded('parseMeters', data, candidates, meters.length);
  return meters;
}

/** Map the api-server `meter_type` string onto the UI commodity enum. */
export function toUiCommodity(meterType: string | null | undefined): MeterCommodity {
  switch (meterType) {
    case 'cold_water':
      return 'water_cold';
    case 'hot_water':
      return 'water_hot';
    case 'water':
      return 'water';
    case 'gas':
      return 'gas';
    case 'heat':
    case 'solar':
      return 'heat';
    default:
      return 'electricity';
  }
}

/** Map an api-server meter onto the UI shape. Exported for unit testing. */
export function toUiMeter(m: ApiMeter): Meter {
  const rawReading = m.current_reading;
  const reading =
    rawReading == null
      ? null
      : typeof rawReading === 'number'
        ? rawReading
        : Number.parseFloat(rawReading);
  return {
    id: m.id,
    label: m.meter_number?.trim() || m.location?.trim() || 'Meter',
    commodity: toUiCommodity(m.meter_type),
    unit: m.unit_of_measure?.trim() || '',
    lastReading: reading != null && Number.isFinite(reading) ? reading : null,
    lastReadingAt: m.last_reading_date ?? null,
  };
}

function commodityIcon(commodity: MeterCommodity): string {
  switch (commodity) {
    case 'water_cold':
    case 'water':
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

interface MetersScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MetersScreen({ onNavigate }: MetersScreenProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const unitId = user?.unitId;

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<unknown>(
    ['meters', 'unit', unitId],
    `/api/v1/meters/units/${unitId ?? ''}`,
    { staleTime: 30_000, enabled: !!unitId }
  );

  const meters: Meter[] = parseMeters(data).map(toUiMeter);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Meters</Text>
        <Text style={s.headerSubtitle}>Meters registered to your unit.</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {!unitId ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>🏠</Text>
            <Text style={s.emptyTitle}>{t('meters.noUnitTitle')}</Text>
            <Text style={s.emptyText}>{t('meters.noUnitText')}</Text>
          </View>
        ) : isLoading ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('meters.loadError')}</Text>
          </View>
        ) : meters.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📏</Text>
            <Text style={s.emptyTitle}>{t('meters.emptyTitle')}</Text>
            <Text style={s.emptyText}>{t('meters.emptyText')}</Text>
          </View>
        ) : (
          meters.map((meter) => (
            <Pressable
              key={meter.id}
              style={s.card}
              onPress={() => onNavigate?.('MeterDetail', { meterId: meter.id })}
            >
              <View style={s.cardHeader}>
                <Text style={styles.icon}>{commodityIcon(meter.commodity)}</Text>
                <Text style={[s.cardTitle, { marginLeft: 8 }]}>{meter.label}</Text>
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
          ))
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  icon: { fontSize: 20 },
  linkButton: { paddingVertical: 4 },
  linkButtonText: { color: colors.accent, fontSize: 14, fontWeight: '500' },
});
