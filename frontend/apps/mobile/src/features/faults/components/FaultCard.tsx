/**
 * FaultCard component for mobile - displays a fault summary card.
 * Epic 4: Fault Reporting & Resolution (UC-03)
 */

import { useTranslation } from 'react-i18next';
import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { formatDate } from '../../../i18n/format';
import { colors } from '../../../screens/shared/screenStyles';

export type FaultStatus =
  | 'new'
  | 'triaged'
  | 'in_progress'
  | 'waiting_parts'
  | 'scheduled'
  | 'resolved'
  | 'closed'
  | 'reopened';

export type FaultPriority = 'low' | 'medium' | 'high' | 'urgent';

export type FaultCategory =
  | 'plumbing'
  | 'electrical'
  | 'heating'
  | 'structural'
  | 'exterior'
  | 'elevator'
  | 'common_area'
  | 'security'
  | 'cleaning'
  | 'other';

export interface FaultSummary {
  id: string;
  buildingId: string;
  unitId?: string;
  title: string;
  category: FaultCategory;
  priority: FaultPriority;
  status: FaultStatus;
  createdAt: string;
}

interface FaultCardProps {
  fault: FaultSummary;
  onPress?: () => void;
}

const statusColors: Record<FaultStatus, { bg: string; text: string }> = {
  new: { bg: colors.dangerBg, text: colors.statusNewInk },
  triaged: { bg: colors.statusTriagedBg, text: colors.statusTriagedInk },
  in_progress: { bg: colors.warningBg, text: colors.statusInProgressInk },
  waiting_parts: { bg: colors.statusWaitPartsBg, text: colors.statusWaitPartsInk },
  scheduled: { bg: colors.statusScheduledBg, text: colors.statusScheduledInk },
  resolved: { bg: colors.successBg, text: colors.successDark },
  closed: { bg: colors.surfaceMuted, text: colors.textSecondary },
  reopened: { bg: colors.dangerBg, text: colors.statusNewInk },
};

const priorityColors: Record<FaultPriority, string> = {
  low: colors.textMuted,
  medium: colors.accent,
  high: colors.priorityHighInk,
  urgent: colors.dangerDark,
};

export function FaultCard({ fault, onPress }: FaultCardProps) {
  const { t } = useTranslation();
  const statusColor = statusColors[fault.status];

  return (
    <TouchableOpacity style={styles.card} onPress={onPress} activeOpacity={0.7}>
      <View style={styles.header}>
        <Text style={styles.title} numberOfLines={2}>
          {fault.title}
        </Text>
        {fault.priority === 'urgent' && <Text style={styles.urgentIcon}>⚠️</Text>}
      </View>

      <View style={styles.badges}>
        <View style={[styles.badge, { backgroundColor: statusColor.bg }]}>
          <Text style={[styles.badgeText, { color: statusColor.text }]}>
            {t(`faults.status.${fault.status}`)}
          </Text>
        </View>
        <Text style={[styles.priority, { color: priorityColors[fault.priority] }]}>
          {t(`faults.priority.${fault.priority}`)}
        </Text>
        <Text style={styles.category}>{t(`faults.category.${fault.category}`)}</Text>
      </View>

      <Text style={styles.date}>
        {t('faults.reported')}: {formatDate(fault.createdAt)}
      </Text>
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: colors.white,
    borderRadius: 12,
    padding: 16,
    marginHorizontal: 16,
    marginVertical: 8,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    justifyContent: 'space-between',
  },
  title: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    flex: 1,
    marginRight: 8,
  },
  urgentIcon: {
    fontSize: 16,
  },
  badges: {
    flexDirection: 'row',
    alignItems: 'center',
    flexWrap: 'wrap',
    marginTop: 8,
    gap: 8,
  },
  badge: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 4,
  },
  badgeText: {
    fontSize: 12,
    fontWeight: '500',
  },
  priority: {
    fontSize: 12,
    fontWeight: '500',
  },
  category: {
    fontSize: 12,
    color: colors.textMuted,
  },
  date: {
    fontSize: 12,
    color: colors.textSubtle,
    marginTop: 8,
  },
});
