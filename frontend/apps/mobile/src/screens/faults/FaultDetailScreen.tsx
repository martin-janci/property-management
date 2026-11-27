/**
 * FaultDetailScreen (UC-03).
 *
 * Full view of one fault report. Reached from `FaultsListScreen` when a card
 * is tapped — `onNavigate('FaultDetail', { faultId })`. Before this screen
 * existed that name had no case in the `App.tsx` switch, so the tap fell
 * through to the Dashboard.
 *
 * Loads `GET /api/v1/faults/{id}`, which returns
 * `{ fault: FaultWithDetails, timeline: FaultTimelineEntryWithUser[], attachments }`
 * (snake_case; the fault row is flattened into `fault`). The payload is read
 * defensively: a missing or malformed `fault` renders the error state.
 */

import { useTranslation } from 'react-i18next';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface FaultDetail {
  id: string;
  title: string;
  description: string;
  location: string;
  category: string;
  priority: string;
  status: string;
  createdAt: string;
  reporterName: string;
  assignedToName: string;
  buildingName: string;
  unitDesignation: string;
  resolutionNotes: string;
}

export interface FaultTimelineItem {
  id: string;
  action: string;
  note: string;
  userName: string;
  createdAt: string;
}

const str = (value: unknown): string => (typeof value === 'string' ? value : '');

/** Read the fault out of the `{ fault, timeline }` envelope. */
export function parseFaultDetail(data: unknown): FaultDetail | null {
  const fault = (data as { fault?: unknown } | null)?.fault;
  if (typeof fault !== 'object' || fault === null) return null;
  const f = fault as Record<string, unknown>;
  if (!str(f.id) || !str(f.title)) return null;
  return {
    id: str(f.id),
    title: str(f.title),
    description: str(f.description),
    location: str(f.location_description),
    category: str(f.category),
    priority: str(f.priority),
    status: str(f.status),
    createdAt: str(f.created_at),
    reporterName: str(f.reporter_name),
    assignedToName: str(f.assigned_to_name),
    buildingName: str(f.building_name),
    unitDesignation: str(f.unit_designation),
    resolutionNotes: str(f.resolution_notes),
  };
}

/** Read the timeline, dropping malformed rows and manager-internal notes. */
export function parseFaultTimeline(data: unknown): FaultTimelineItem[] {
  const timeline = (data as { timeline?: unknown } | null)?.timeline;
  if (!Array.isArray(timeline)) return [];
  const items: FaultTimelineItem[] = [];
  for (const row of timeline) {
    if (typeof row !== 'object' || row === null) continue;
    const r = row as Record<string, unknown>;
    if (!str(r.id) || r.is_internal === true) continue;
    items.push({
      id: str(r.id),
      action: str(r.action),
      note: str(r.note),
      userName: str(r.user_name),
      createdAt: str(r.created_at),
    });
  }
  return items;
}

/** i18n key for a server `fault_status` value; unknown values show as-is. */
export const FAULT_STATUS_LABEL_KEY: Record<string, string> = {
  new: 'faults.statusNew',
  triaged: 'faults.statusTriaged',
  in_progress: 'faults.statusInProgress',
  waiting_parts: 'faults.statusWaitingParts',
  scheduled: 'faults.statusScheduled',
  resolved: 'faults.statusResolved',
  closed: 'faults.statusClosed',
  reopened: 'faults.statusReopened',
};

const PRIORITY_LABEL_KEY: Record<string, string> = {
  low: 'faults.priorityLow',
  medium: 'faults.priorityMedium',
  high: 'faults.priorityHigh',
  urgent: 'faults.priorityUrgent',
};

/**
 * i18n key for a server `fault.category` enum value. The server's `heating`
 * value maps to the `categoryHvac` catalog key (HVAC groups heating + cooling),
 * so a plain `faults.category${value}` interpolation won't do — an explicit
 * map is required. Unknown/future category values are not listed here and fall
 * back to the raw string via `defaultValue` at the call site.
 */
export const FAULT_CATEGORY_LABEL_KEY: Record<string, string> = {
  plumbing: 'faults.categoryPlumbing',
  electrical: 'faults.categoryElectrical',
  structural: 'faults.categoryStructural',
  heating: 'faults.categoryHvac',
  elevator: 'faults.categoryElevator',
  security: 'faults.categorySecurity',
  other: 'faults.categoryOther',
};

interface FaultDetailScreenProps {
  faultId?: string;
  onBack?: () => void;
}

export function FaultDetailScreen({ faultId, onBack }: FaultDetailScreenProps) {
  const { t } = useTranslation();

  const { data, isLoading, error } = useApiQuery<unknown>(
    ['faults', 'detail', faultId ?? ''],
    `/api/v1/faults/${faultId}`,
    { staleTime: 30_000, enabled: !!faultId }
  );

  const back = (
    <Pressable style={styles.backButton} onPress={onBack} disabled={!onBack}>
      <Text style={styles.backButtonText}>{t('faults.backToList')}</Text>
    </Pressable>
  );

  const fault = parseFaultDetail(data);

  if (!faultId || error || (!isLoading && !fault)) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <Text style={s.emptyIcon}>⚠️</Text>
          <Text style={s.emptyTitle}>{t('faults.detailLoadError')}</Text>
        </View>
      </View>
    );
  }

  if (isLoading || !fault) {
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

  const timeline = parseFaultTimeline(data);
  const statusKey = FAULT_STATUS_LABEL_KEY[fault.status];
  const priorityKey = PRIORITY_LABEL_KEY[fault.priority];
  // Route the category enum through i18n (localized sk/cs/de/en), falling back
  // to the raw server string for any unknown/future value instead of leaving a
  // dangling catalog key on screen.
  const categoryKey = FAULT_CATEGORY_LABEL_KEY[fault.category] ?? fault.category;
  const place = [fault.buildingName, fault.unitDesignation, fault.location]
    .filter(Boolean)
    .join(' · ');

  const rows: Array<[string, string]> = [
    [t('faults.categoryLabel'), t(categoryKey, { defaultValue: fault.category })],
    [t('faults.priority'), priorityKey ? t(priorityKey) : fault.priority],
    [t('faults.location'), place],
    [t('faults.reportedBy'), fault.reporterName],
    [t('faults.assignee'), fault.assignedToName],
  ];

  return (
    <View style={s.container}>
      {back}
      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          <Text style={s.cardTitle}>{fault.title}</Text>
          <View style={s.cardFooter}>
            <Text style={s.cardMeta}>{fault.createdAt ? formatDate(fault.createdAt) : ''}</Text>
            <View style={s.badge}>
              <Text style={s.badgeText}>{statusKey ? t(statusKey) : fault.status}</Text>
            </View>
          </View>
          {fault.description ? <Text style={s.cardBody}>{fault.description}</Text> : null}
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

        {fault.resolutionNotes ? (
          <View style={s.card}>
            <Text style={styles.sectionTitle}>{t('faults.resolution')}</Text>
            <Text style={s.cardBody}>{fault.resolutionNotes}</Text>
          </View>
        ) : null}

        <View style={s.card}>
          <Text style={styles.sectionTitle}>{t('faults.timeline')}</Text>
          {timeline.length === 0 ? (
            <Text style={s.cardMeta}>{t('faults.noTimeline')}</Text>
          ) : (
            timeline.map((item) => (
              <View key={item.id} style={styles.timelineItem}>
                <Text style={styles.rowValue}>
                  {FAULT_STATUS_LABEL_KEY[item.action]
                    ? t(FAULT_STATUS_LABEL_KEY[item.action])
                    : item.action.replace(/_/g, ' ')}
                </Text>
                {item.note ? <Text style={s.cardBody}>{item.note}</Text> : null}
                <Text style={s.cardMeta}>
                  {[item.userName, item.createdAt ? formatDate(item.createdAt) : '']
                    .filter(Boolean)
                    .join(' · ')}
                </Text>
              </View>
            ))
          )}
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
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 6,
    gap: 12,
  },
  rowLabel: {
    fontSize: 14,
    color: colors.textMuted,
  },
  rowValue: {
    fontSize: 14,
    color: colors.text,
    fontWeight: '500',
    flexShrink: 1,
    textAlign: 'right',
  },
  sectionTitle: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 8,
  },
  timelineItem: {
    paddingVertical: 8,
    borderTopWidth: 1,
    borderTopColor: colors.border,
  },
});
