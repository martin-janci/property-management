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
import { colors, screenStyles as s } from '../shared/screenStyles';
import {
  canReport,
  formatMonth,
  latestSubmissionByMonth,
  parseReportCount,
  recordedByMonth,
  type SubmissionRow,
  visibleMonths,
} from './helpers';

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

  const recorded = recordedByMonth(monthsQuery.data);
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
    const count = parseReportCount(draft);
    if (count === null) {
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
