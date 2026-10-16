/**
 * OutageDetailScreen (UC-12).
 *
 * Full view for a single utility outage. Reached from `OutagesScreen` when a
 * card is tapped — `onNavigate('OutageDetail', { outageId })`. Before this
 * screen existed the tap fell through the `App.tsx` switch default and bounced
 * the user back to the Dashboard with the `outageId` param discarded.
 *
 * Loads `GET /api/v1/outages/{id}` for the detail body. The id is optional at
 * the prop boundary (App hands it from untyped navigation params); a missing
 * id renders a back-only empty state rather than firing a `/outages/undefined`
 * request — the query is gated on `enabled: !!outageId`.
 */

import { useTranslation } from 'react-i18next';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate, formatTime } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

interface ApiOutageDetail {
  id: string;
  commodity: string;
  title: string;
  description?: string | null;
  status: string;
  severity?: string | null;
  starts_at: string;
  ends_at: string;
}

export const outageDetailKey = (id: string) => ['outages', 'detail', id] as const;

function formatRange(startsAt: string, endsAt: string): string {
  const start = new Date(startsAt);
  const end = new Date(endsAt);
  const sameDay = start.toDateString() === end.toDateString();
  const dateOpts: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric' };
  const timeOpts: Intl.DateTimeFormatOptions = { hour: '2-digit', minute: '2-digit' };
  if (sameDay) {
    return `${formatDate(start, dateOpts)} · ${formatTime(start, timeOpts)} – ${formatTime(
      end,
      timeOpts
    )}`;
  }
  return `${formatDate(start, dateOpts)} – ${formatDate(end, dateOpts)}`;
}

interface OutageDetailScreenProps {
  outageId?: string;
  onBack?: () => void;
}

export function OutageDetailScreen({ outageId, onBack }: OutageDetailScreenProps) {
  const { t } = useTranslation();

  const { data, isLoading, error } = useApiQuery<ApiOutageDetail>(
    outageDetailKey(outageId ?? ''),
    `/api/v1/outages/${outageId}`,
    { staleTime: 30_000, enabled: !!outageId }
  );

  const back = (
    <Pressable style={styles.backButton} onPress={onBack} disabled={!onBack}>
      <Text style={styles.backButtonText}>‹ Outages</Text>
    </Pressable>
  );

  // `outageId` is missing only if navigation was wired up wrong — render a
  // back-only empty state instead of fetching `/outages/undefined`.
  if (!outageId) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <Text style={s.emptyTitle}>{t('outages.emptyTitle')}</Text>
        </View>
      </View>
    );
  }

  if (isLoading) {
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

  if (error || !data) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <Text style={s.emptyIcon}>⚠️</Text>
          <Text style={s.emptyTitle}>{t('outages.loadError')}</Text>
        </View>
      </View>
    );
  }

  return (
    <View style={s.container}>
      {back}
      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          <Text style={s.cardTitle}>{data.title}</Text>
          <View style={s.cardFooter}>
            <Text style={s.cardMeta}>{formatRange(data.starts_at, data.ends_at)}</Text>
            <View style={s.badge}>
              <Text style={s.badgeText}>{data.status.replace('_', ' ')}</Text>
            </View>
          </View>
          {data.description ? <Text style={s.cardBody}>{data.description}</Text> : null}
        </View>
        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  backButton: {
    paddingHorizontal: 20,
    paddingTop: 56,
    paddingBottom: 12,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  backButtonText: {
    fontSize: 15,
    color: colors.accent,
    fontWeight: '500',
  },
});
