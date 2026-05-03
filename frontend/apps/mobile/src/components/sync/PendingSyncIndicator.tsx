/**
 * PendingSyncIndicator - "Will sync when online" indicator (Epic 123 - Story 123.1)
 *
 * Shows inline indicator for items that are pending sync.
 */
import { useTranslation } from 'react-i18next';
import { StyleSheet, Text, View } from 'react-native';
import { colors } from '../../screens/shared/screenStyles';

export type SyncStatus = 'pending' | 'syncing' | 'synced' | 'failed';

export interface PendingSyncIndicatorProps {
  /** Current sync status of the item */
  status: SyncStatus;
  /** Optional timestamp of when the item was created */
  createdAt?: Date;
  /** Whether to show the full message or just an icon */
  compact?: boolean;
  /** Callback when retry is tapped (for failed items) */
  onRetry?: () => void;
}

export function PendingSyncIndicator({
  status,
  createdAt,
  compact = false,
  onRetry,
}: PendingSyncIndicatorProps) {
  const { t } = useTranslation();

  const getStatusConfig = () => {
    switch (status) {
      case 'pending':
        return {
          icon: '⏳',
          text: t('sync.willSyncWhenOnline'),
          color: colors.warningDark,
          bgColor: colors.warningBg,
        };
      case 'syncing':
        return {
          icon: '↻',
          text: t('sync.syncing'),
          color: colors.accent,
          bgColor: colors.accentSoft,
        };
      case 'synced':
        return {
          icon: '✓',
          text: t('sync.synced'),
          color: colors.success,
          bgColor: colors.successBg,
        };
      case 'failed':
        return {
          icon: '⚠',
          text: t('sync.failed'),
          color: colors.dangerDark,
          bgColor: colors.dangerBg,
        };
    }
  };

  const config = getStatusConfig();

  if (compact) {
    return (
      <View style={[styles.compactContainer, { backgroundColor: config.bgColor }]}>
        <Text style={[styles.icon, { color: config.color }]}>{config.icon}</Text>
      </View>
    );
  }

  return (
    <View style={[styles.container, { backgroundColor: config.bgColor }]}>
      <Text style={[styles.icon, { color: config.color }]}>{config.icon}</Text>
      <View style={styles.textContainer}>
        <Text style={[styles.statusText, { color: config.color }]}>{config.text}</Text>
        {createdAt && status === 'pending' && (
          <Text style={styles.timeText}>
            {t('sync.savedLocally', {
              time: formatRelativeTime(createdAt),
            })}
          </Text>
        )}
      </View>
      {status === 'failed' && onRetry && (
        <Text style={styles.retryButton} onPress={onRetry}>
          {t('sync.retry')}
        </Text>
      )}
    </View>
  );
}

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins}m ago`;

  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;

  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 8,
    paddingHorizontal: 12,
    borderRadius: 8,
    marginVertical: 4,
  },
  compactContainer: {
    width: 24,
    height: 24,
    borderRadius: 12,
    alignItems: 'center',
    justifyContent: 'center',
  },
  icon: {
    fontSize: 16,
    marginRight: 8,
  },
  textContainer: {
    flex: 1,
  },
  statusText: {
    fontSize: 14,
    fontWeight: '500',
  },
  timeText: {
    fontSize: 12,
    color: colors.textMuted,
    marginTop: 2,
  },
  retryButton: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.accent,
    paddingHorizontal: 8,
  },
});
