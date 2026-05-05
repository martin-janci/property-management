/**
 * SyncStatusBadge - Badge showing pending count (Epic 123 - Story 123.1)
 *
 * Small badge that can be attached to navigation items to show
 * the number of pending sync items.
 */
import { StyleSheet, Text, View } from 'react-native';
import { colors } from '../../screens/shared/screenStyles';

export interface SyncStatusBadgeProps {
  /** Number of pending items */
  count: number;
  /** Badge size variant */
  size?: 'small' | 'medium' | 'large';
  /** Whether sync is currently in progress */
  isSyncing?: boolean;
}

export function SyncStatusBadge({
  count,
  size = 'medium',
  isSyncing = false,
}: SyncStatusBadgeProps) {
  if (count === 0 && !isSyncing) {
    return null;
  }

  const sizeStyles = {
    small: {
      minWidth: 16,
      height: 16,
      borderRadius: 8,
      fontSize: 10,
      paddingHorizontal: 4,
    },
    medium: {
      minWidth: 20,
      height: 20,
      borderRadius: 10,
      fontSize: 12,
      paddingHorizontal: 6,
    },
    large: {
      minWidth: 24,
      height: 24,
      borderRadius: 12,
      fontSize: 14,
      paddingHorizontal: 8,
    },
  };

  const currentSize = sizeStyles[size];

  return (
    <View
      style={[
        styles.badge,
        {
          minWidth: currentSize.minWidth,
          height: currentSize.height,
          borderRadius: currentSize.borderRadius,
          paddingHorizontal: currentSize.paddingHorizontal,
        },
        isSyncing && styles.syncingBadge,
      ]}
    >
      {isSyncing ? (
        <Text style={[styles.syncIcon, { fontSize: currentSize.fontSize }]}>↻</Text>
      ) : (
        <Text style={[styles.text, { fontSize: currentSize.fontSize }]}>
          {count > 99 ? '99+' : count}
        </Text>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  badge: {
    backgroundColor: colors.dangerDark,
    alignItems: 'center',
    justifyContent: 'center',
  },
  syncingBadge: {
    backgroundColor: colors.accent,
  },
  text: {
    color: colors.white,
    fontWeight: 'bold',
  },
  syncIcon: {
    color: colors.white,
  },
});
