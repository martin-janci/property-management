/**
 * PersonMonthsScreen (UC-10).
 *
 * Shows the person-months recorded for the signed-in resident's unit(s), used
 * for cost allocation, and lets the resident report a different count for a
 * month. A report is a proposal: the building manager approves it (the count
 * is then recorded) or rejects it, so the resident sees each month's recorded
 * value next to the status of their latest report.
 *
 * Data:
 * - `GET /api/v1/users/me/units` (camelCase `MyUnit[]`) resolves the caller's
 *   own units, which supplies the building and unit ids.
 * - `GET /api/v1/buildings/{b}/units/{u}/person-months?year=` (snake_case
 *   `PersonMonthResponse[]`) returns the recorded months.
 * - `GET|POST /api/v1/buildings/{b}/units/{u}/person-months/submissions`
 *   lists the resident's reports for the year and creates a new one (a new
 *   report replaces a pending one for the same month).
 */

import type { MyUnit } from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useApiMutation, useApiQuery } from '../../hooks/useApi';
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

/** One resident submission (`PersonMonthSubmission`). */
export interface SubmissionRow {
  id: string;
  year: number;
  month: number;
  count: number;
  status: 'pending' | 'approved' | 'rejected';
  submitted_at: string;
  decision_note: string | null;
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

/** Latest submission per month (input is newest-first from the server). */
export function latestSubmissionByMonth(data: unknown): Map<number, SubmissionRow> {
  const out = new Map<number, SubmissionRow>();
  if (!Array.isArray(data)) return out;
  for (const r of data as SubmissionRow[]) {
    if (!r || typeof r.month !== 'number' || typeof r.status !== 'string') continue;
    const prev = out.get(r.month);
    if (!prev || r.submitted_at > prev.submitted_at) out.set(r.month, r);
  }
  return out;
}

/** Months to show for `year`: up to the current month, newest first. */
export function visibleMonths(year: number, today = new Date()): number[] {
  const thisYear = today.getFullYear();
  if (year > thisYear) return [];
  const last = year === thisYear ? today.getMonth() + 1 : 12;
  return Array.from({ length: last }, (_, i) => last - i);
}

/** The server accepts reports for the current month and the 24 before it. */
export function canReport(year: number, month: number, today = new Date()): boolean {
  const period = year * 12 + month;
  const now = today.getFullYear() * 12 + today.getMonth() + 1;
  return period <= now && period >= now - 24;
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
  const [editingMonth, setEditingMonth] = useState<number | null>(null);
  const [draft, setDraft] = useState('');
  const [formError, setFormError] = useState<string | null>(null);

  const unitsQuery = useApiQuery<MyUnit[]>(['my-units', 'list'], '/api/v1/users/me/units', {
    staleTime: 60_000,
  });
  const units = Array.isArray(unitsQuery.data) ? unitsQuery.data : [];
  const current = units.find((u) => u.unit.id === selectedUnitId) ?? units[0];
  const buildingId = current?.unit.buildingId;
  const unitId = current?.unit.id;
  const base = `/api/v1/buildings/${buildingId ?? ''}/units/${unitId ?? ''}/person-months`;
  const enabled = !!buildingId && !!unitId;

  const monthsQuery = useApiQuery<unknown>(
    ['person-months', buildingId, unitId, year],
    `${base}?year=${year}`,
    { staleTime: 30_000, enabled }
  );
  const submissionsQuery = useApiQuery<unknown>(
    ['person-months', 'submissions', buildingId, unitId, year],
    `${base}/submissions?year=${year}`,
    { staleTime: 30_000, enabled }
  );
  const submit = useApiMutation<SubmissionRow, { year: number; month: number; count: number }>(
    `${base}/submissions`,
    'POST'
  );

  const recorded = new Map(parsePersonMonths(monthsQuery.data).map((r) => [r.month, r]));
  const latest = latestSubmissionByMonth(submissionsQuery.data);
  const months = visibleMonths(year);

  const { refetch: refetchUnits } = unitsQuery;
  const { refetch: refetchMonths } = monthsQuery;
  const { refetch: refetchSubmissions } = submissionsQuery;
  const onRefresh = useCallback(async () => {
    await Promise.all([refetchUnits(), refetchMonths(), refetchSubmissions()]);
  }, [refetchUnits, refetchMonths, refetchSubmissions]);

  const startReport = (month: number) => {
    setEditingMonth(month);
    setDraft(String(recorded.get(month)?.count ?? ''));
    setFormError(null);
  };

  const sendReport = (month: number) => {
    const count = Number(draft);
    if (draft.trim() === '' || !Number.isInteger(count) || count < 0 || count > 50) {
      setFormError(t('personMonths.invalidMessage'));
      return;
    }
    submit.mutate(
      { year, month, count },
      {
        onSuccess: () => {
          setEditingMonth(null);
          setDraft('');
          void refetchSubmissions();
        },
        onError: () => setFormError(t('personMonths.submitFailed')),
      }
    );
  };

  const loading = unitsQuery.isLoading || (enabled && monthsQuery.isLoading);
  const failed = unitsQuery.error || monthsQuery.error;

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>{t('personMonths.title')}</Text>
        <Text style={s.headerSubtitle}>{t('personMonths.subtitle')}</Text>
      </View>

      <ScrollView
        style={s.scrollView}
        keyboardShouldPersistTaps="handled"
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
                  onPress={() => {
                    setSelectedUnitId(u.unit.id);
                    setEditingMonth(null);
                  }}
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
            onPress={() => {
              setYear((y) => y - 1);
              setEditingMonth(null);
            }}
            style={styles.yearButton}
          >
            <Text style={styles.yearButtonText}>‹</Text>
          </Pressable>
          <Text style={styles.yearText}>{year}</Text>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={t('personMonths.nextYear')}
            disabled={year >= new Date().getFullYear()}
            onPress={() => {
              setYear((y) => y + 1);
              setEditingMonth(null);
            }}
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
        ) : months.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>👥</Text>
            <Text style={s.emptyTitle}>{t('personMonths.empty')}</Text>
          </View>
        ) : (
          months.map((month) => {
            const row = recorded.get(month);
            const sub = latest.get(month);
            const isEditing = editingMonth === month;
            return (
              <View key={month} style={s.card}>
                <View style={s.cardHeader}>
                  <Text style={s.cardTitle}>{formatMonth(year, month)}</Text>
                  {sub && sub.status !== 'approved' && (
                    <View
                      style={[
                        s.badge,
                        sub.status === 'pending' ? styles.badgePending : styles.badgeRejected,
                      ]}
                    >
                      <Text
                        style={[
                          s.badgeText,
                          sub.status === 'pending'
                            ? styles.badgePendingText
                            : styles.badgeRejectedText,
                        ]}
                      >
                        {t(`personMonths.submissionStatus.${sub.status}`)}
                      </Text>
                    </View>
                  )}
                </View>
                <Text style={styles.value}>
                  {row
                    ? t('personMonths.personCount', { count: row.count })
                    : t('personMonths.notRecorded')}
                </Text>
                {sub?.status === 'pending' && (
                  <Text style={styles.subline}>
                    {t('personMonths.pendingCount', { count: sub.count })}
                  </Text>
                )}
                {sub?.status === 'rejected' && sub.decision_note ? (
                  <Text style={styles.subline}>{sub.decision_note}</Text>
                ) : null}

                {isEditing ? (
                  <View style={styles.editRow}>
                    <TextInput
                      style={styles.editInput}
                      value={draft}
                      onChangeText={setDraft}
                      keyboardType="number-pad"
                      accessibilityLabel={t('personMonths.reportLabel')}
                      maxLength={2}
                    />
                    <Pressable
                      style={[s.primaryButton, styles.sendButton]}
                      disabled={submit.isPending}
                      onPress={() => sendReport(month)}
                    >
                      <Text style={s.primaryButtonText}>{t('personMonths.send')}</Text>
                    </Pressable>
                    <Pressable onPress={() => setEditingMonth(null)} style={styles.linkButton}>
                      <Text style={styles.linkText}>{t('common.cancel')}</Text>
                    </Pressable>
                    {formError ? <Text style={styles.errorText}>{formError}</Text> : null}
                  </View>
                ) : (
                  canReport(year, month) && (
                    <Pressable onPress={() => startReport(month)} style={styles.linkButton}>
                      <Text style={styles.linkText}>{t('personMonths.report')}</Text>
                    </Pressable>
                  )
                )}
              </View>
            );
          })
        )}

        {current && <Text style={styles.hint}>{t('personMonths.approvalHint')}</Text>}

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
  subline: { fontSize: 13, color: colors.textMuted, marginTop: 2 },
  badgePending: { backgroundColor: colors.warningBg },
  badgePendingText: { color: colors.warningDark },
  badgeRejected: { backgroundColor: colors.dangerBg },
  badgeRejectedText: { color: colors.dangerDark },
  editRow: { flexDirection: 'row', alignItems: 'center', gap: 8, flexWrap: 'wrap', marginTop: 8 },
  editInput: {
    borderWidth: 1,
    borderColor: colors.borderStrong,
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 8,
    minWidth: 64,
    fontSize: 16,
    color: colors.text,
  },
  sendButton: { paddingVertical: 8, paddingHorizontal: 16, marginTop: 0 },
  linkButton: { paddingVertical: 8, alignSelf: 'flex-start' },
  linkText: { color: colors.accent, fontWeight: '500' },
  errorText: { color: colors.dangerDark, fontSize: 13, width: '100%' },
  hint: { fontSize: 13, color: colors.textMuted, textAlign: 'center', marginTop: 8 },
});
