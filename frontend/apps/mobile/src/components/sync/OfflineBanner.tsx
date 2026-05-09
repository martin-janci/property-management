import { useEffect, useRef } from 'react';
/**
 * OfflineBanner - Connectivity status banner (Epic 123 - Story 123.1)
 *
 * Displays a persistent banner when the device is offline,
 * showing the number of pending actions waiting to sync.
 */
import { useTranslation } from 'react-i18next';
import { Animated, StyleSheet, Text, View } from 'react-native';
import { colors } from '../../screens/shared/screenStyles';

export interface OfflineBannerProps {
  /** Whether the device is currently online */
  isConnected: boolean;
  /** Number of pending actions in the offline queue */
  queuedActionsCount: number;
  /** Whether sync is currently in progress */
  isSyncing?: boolean;
}

export function OfflineBanner({ isConnected, queuedActionsCount, isSyncing }: OfflineBannerProps) {
  const { t } = useTranslation();
  const slideAnim = useRef(new Animated.Value(-60)).current;

  useEffect(() => {
    Animated.timing(slideAnim, {
      toValue: isConnected && !isSyncing ? -60 : 0,
      duration: 300,
      useNativeDriver: true,
    }).start();
  }, [isConnected, isSyncing, slideAnim]);

  // Don't render anything if connected and not syncing and no queued actions
  if (isConnected && !isSyncing && queuedActionsCount === 0) {
    return null;
  }

  const getBannerStyle = () => {
    if (isSyncing) return styles.syncingBanner;
    if (!isConnected) return styles.offlineBanner;
    return styles.pendingBanner;
  };

  const getMessage = () => {
    if (isSyncing) {
      return t('offline.syncing', { count: queuedActionsCount });
    }
    if (!isConnected) {
      if (queuedActionsCount > 0) {
        return t('offline.offlineWithPending', { count: queuedActionsCount });
      }
      return t('offline.title');
    }
    if (queuedActionsCount > 0) {
      return t('offline.pending', { count: queuedActionsCount });
    }
    return '';
  };

  const getIcon = () => {
    if (isSyncing) return '↻';
    if (!isConnected) return '📵';
    if (queuedActionsCount > 0) return '⏳';
    return '';
  };

  return (
    <Animated.View
      style={[styles.banner, getBannerStyle(), { transform: [{ translateY: slideAnim }] }]}
    >
      <View style={styles.content}>
        <Text style={styles.icon}>{getIcon()}</Text>
        <Text style={styles.text}>{getMessage()}</Text>
        {queuedActionsCount > 0 && (
          <View style={styles.badge}>
            <Text style={styles.badgeText}>{queuedActionsCount}</Text>
          </View>
        )}
      </View>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  banner: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    paddingTop: 44, // Safe area
    paddingBottom: 8,
    paddingHorizontal: 16,
    zIndex: 1000,
  },
  offlineBanner: {
    backgroundColor: colors.dangerBg,
  },
  syncingBanner: {
    backgroundColor: colors.accentSoft,
  },
  pendingBanner: {
    backgroundColor: colors.warningBg,
  },
  content: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
  },
  icon: {
    fontSize: 16,
    marginRight: 8,
  },
  text: {
    fontSize: 14,
    fontWeight: '500',
    color: colors.text,
  },
  badge: {
    backgroundColor: colors.dangerDark,
    borderRadius: 10,
    minWidth: 20,
    height: 20,
    alignItems: 'center',
    justifyContent: 'center',
    marginLeft: 8,
    paddingHorizontal: 6,
  },
  badgeText: {
    color: colors.white,
    fontSize: 12,
    fontWeight: 'bold',
  },
});
