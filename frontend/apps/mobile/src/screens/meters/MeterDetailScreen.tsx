/**
 * MeterDetailScreen (UC-11.4 / 11.6).
 *
 * Single meter view: history of readings, consumption summary and a CTA to
 * submit a new reading.
 *
 * Wired to `GET /api/v1/meters/{id}` on the api-server, which returns a
 * `MeterResponse { meter, recent_readings }` (see
 * `db::models::meter::{Meter, MeterReading, MeterResponse}`). Those structs
 * serialize with the default (snake_case) serde naming, so the wire format is
 * snake_case and `Decimal` fields arrive as JSON strings. The response is
 * consumed without a generated client, so it is validated defensively — a
 * malformed shape degrades to an empty history rather than crashing.
 */

import type { Meter, MeterReading, MeterResponse } from '@ppt/api-client';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { resolveLocale } from '../../i18n/format';
import { warnIfListDegraded, warnIfParseFailed } from '../shared/parserWarnings';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface MeterReadingRecord {
  id: string;
  value: number;
  unit: string;
  takenAt: string;
}

/**
 * `Meter` from `GET /api/v1/meters/{id}`, typed against the generated
 * `@ppt/api-client` payload so backend field renames surface at compile time.
 * Relaxed to `Partial` (only `id` required) because the shape is validated
 * defensively at runtime.
 */
export type ApiMeter = Partial<Meter> & Pick<Meter, 'id'>;

/** `MeterReading` fields consumed from the `recent_readings` list. */
export type ApiMeterReading = Pick<MeterReading, 'id' | 'reading' | 'reading_date'> &
  Partial<Pick<MeterReading, 'created_at'>>;

/** Validated subset of the generated `MeterResponse` envelope. */
export interface ApiMeterResponse {
  meter: ApiMeter;
  recent_readings: ApiMeterReading[];
}

/** Parse a `Decimal`-as-string (or number) into a finite number, or null. */
function toNumber(value: string | number | null | undefined): number | null {
  if (value == null) return null;
  const n = typeof value === 'number' ? value : Number.parseFloat(value);
  return Number.isFinite(n) ? n : null;
}

function isApiMeterReading(value: unknown): value is ApiMeterReading {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === 'string' &&
    (typeof v.reading === 'string' || typeof v.reading === 'number') &&
    typeof v.reading_date === 'string'
  );
}

/** Narrow the raw `GET /api/v1/meters/{id}` body into a validated response.
 *  Tolerates any garbage top-level shape by returning null so the screen can
 *  fall back to its empty state instead of dereferencing `undefined.meter`. */
export function parseMeterResponse(data: unknown): ApiMeterResponse | null {
  const parsed = parseMeterResponseInner(data);
  warnIfParseFailed('parseMeterResponse', data, parsed);
  return parsed;
}

function parseMeterResponseInner(data: unknown): ApiMeterResponse | null {
  if (typeof data !== 'object' || data === null) return null;
  const v = data as Record<string, unknown>;
  const meter = v['meter' satisfies keyof MeterResponse];
  if (typeof meter !== 'object' || meter === null) return null;
  const m = meter as Record<string, unknown>;
  if (typeof m.id !== 'string') return null;
  const rawReadings = v['recent_readings' satisfies keyof MeterResponse];
  const candidates: unknown[] | null = Array.isArray(rawReadings) ? rawReadings : null;
  const readings = (candidates ?? []).filter(isApiMeterReading);
  warnIfListDegraded(
    'parseMeterResponse.recent_readings',
    rawReadings,
    candidates,
    readings.length
  );
  return { meter: meter as ApiMeter, recent_readings: readings };
}

/** Map api-server readings onto the UI record shape, newest first. Exported so
 *  the mapping can be unit-tested without rendering the screen. */
export function toUiReadings(response: ApiMeterResponse): MeterReadingRecord[] {
  const unit = response.meter.unit_of_measure?.trim() || '';
  return response.recent_readings
    .map((r) => ({
      id: r.id,
      value: toNumber(r.reading) ?? 0,
      unit,
      takenAt: r.reading_date,
    }))
    .sort((a, b) => new Date(b.takenAt).getTime() - new Date(a.takenAt).getTime());
}

interface MeterDetailScreenProps {
  meterId?: string;
  meterLabel?: string;
  onBack?: () => void;
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MeterDetailScreen({
  meterId,
  meterLabel,
  onBack,
  onNavigate,
}: MeterDetailScreenProps) {
  const { t } = useTranslation();
  const { data, isLoading, error, refetch, isFetching } = useApiQuery<unknown>(
    ['meters', 'detail', meterId],
    `/api/v1/meters/${meterId ?? ''}`,
    { staleTime: 30_000, enabled: !!meterId }
  );

  const response = parseMeterResponse(data);
  const readings = response ? toUiReadings(response) : [];
  const label = response?.meter.meter_number?.trim() || meterLabel || t('meters.meterFallback');

  const lastTwo = readings.slice(0, 2);
  const monthDelta = lastTwo.length === 2 ? Math.max(0, lastTwo[0].value - lastTwo[1].value) : null;

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← {t('meters.backToMeters')}</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>{label}</Text>
        {meterId && <Text style={s.headerSubtitle}>ID: {meterId}</Text>}
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading || !meterId ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('meters.detailLoadError')}</Text>
          </View>
        ) : (
          <>
            {readings.length > 0 && (
              <View style={s.card}>
                <Text style={styles.metricLabel}>{t('meters.latestReading')}</Text>
                <Text style={styles.metricValue}>
                  {readings[0].value} {readings[0].unit}
                </Text>
                <Text style={s.cardMeta}>
                  {new Date(readings[0].takenAt).toLocaleDateString(resolveLocale())}
                </Text>
                {monthDelta != null && (
                  <Text style={[s.cardBody, { marginTop: 8 }]}>
                    {t('meters.usedSincePrevious', {
                      amount: monthDelta.toFixed(2),
                      unit: readings[0].unit,
                    })}
                  </Text>
                )}
              </View>
            )}

            <Text style={styles.sectionTitle}>{t('meters.readingHistory')}</Text>
            {readings.length === 0 ? (
              <View style={s.emptyState}>
                <Text style={s.emptyIcon}>📏</Text>
                <Text style={s.emptyTitle}>{t('meters.noReadingsTitle')}</Text>
                <Text style={s.emptyText}>{t('meters.noReadingsText')}</Text>
              </View>
            ) : (
              readings.map((r) => (
                <View key={r.id} style={s.card}>
                  <View style={s.cardHeader}>
                    <Text style={s.cardTitle}>
                      {r.value} {r.unit}
                    </Text>
                    <Text style={s.cardMeta}>
                      {new Date(r.takenAt).toLocaleDateString(resolveLocale())}
                    </Text>
                  </View>
                </View>
              ))
            )}

            <Pressable
              style={s.primaryButton}
              onPress={() => onNavigate?.('MeterReading', { meterId })}
            >
              <Text style={s.primaryButtonText}>{t('meters.submitNewReading')}</Text>
            </Pressable>
          </>
        )}

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
