import { useEffect, useRef } from 'react';
/**
 * SyncProgressToast - Toast notification for sync progress (Epic 123 - Story 123.2)
 *
 * Shows a floating toast that displays sync progress and results.
 * Automatically dismisses after completion.
 */
import { useTranslation } from 'react-i18next';
import { Animated, StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { colors } from '../../screens/shared/screenStyles';

export interface SyncProgressToastProps {
  /** Whether the toast is visible */
  visible: boolean;
  /** Current sync progress (0-100) */
  progress: number;
  /** Total items to sync */
  total: number;
  /** Items synced so far */
  current: number;
  /** Number of failed items */
  failed: number;
  /** Whether sync is complete */
  isComplete: boolean;
  /** Callback when toast is dismissed */
  onDismiss: () => void;
  /** Callback to retry failed items */
  onRetry?: () => void;
}

export function SyncProgressToast({
  visible,
  progress,
  total,
  current,
  failed,
  isComplete,
  onDismiss,
  onRetry,
}: SyncProgressToastProps) {
  const { t } = useTranslation();
  const slideAnim = useRef(new Animated.Value(100)).current;
  const opacityAnim = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    if (visible) {
      Animated.parallel([
        Animated.timing(slideAnim, {
          toValue: 0,
          duration: 300,
          useNativeDriver: true,
        }),
        Animated.timing(opacityAnim, {
          toValue: 1,
          duration: 300,
          useNativeDriver: true,
        }),
      ]).start();
    } else {
      Animated.parallel([
        Animated.timing(slideAnim, {
          toValue: 100,
          duration: 300,
          useNativeDriver: true,
        }),
        Animated.timing(opacityAnim, {
          toValue: 0,
          duration: 300,
          useNativeDriver: true,
        }),
      ]).start();
    }
  }, [visible, slideAnim, opacityAnim]);

  // Auto-dismiss after 3 seconds when complete and no failures
  useEffect(() => {
    if (isComplete && failed === 0) {
      const timer = setTimeout(() => {
        onDismiss();
      }, 3000);
      return () => clearTimeout(timer);
    }
  }, [isComplete, failed, onDismiss]);

  if (!visible) return null;

  const getStatusIcon = () => {
    if (!isComplete) return '↻';
    if (failed > 0) return '⚠';
    return '✓';
  };

  const getStatusColor = () => {
    if (!isComplete) return colors.accent;
    if (failed > 0) return colors.dangerDark;
    return colors.success;
  };

  const getMessage = () => {
    if (!isComplete) {
      return t('sync.syncProgress', { current, total });
    }
    if (failed > 0) {
      return t('sync.syncFailed');
    }
    return t('sync.syncComplete');
  };

  return (
    <Animated.View
      style={[
        styles.container,
        {
          transform: [{ translateY: slideAnim }],
          opacity: opacityAnim,
        },
      ]}
    >
      <View style={styles.content}>
        <Text style={[styles.icon, { color: getStatusColor() }]}>{getStatusIcon()}</Text>
        <View style={styles.textContainer}>
          <Text style={styles.message}>{getMessage()}</Text>
          {!isComplete && (
            <View style={styles.progressBar}>
              <View style={[styles.progressFill, { width: `${progress}%` }]} />
            </View>
          )}
        </View>
        {isComplete && failed > 0 && onRetry && (
          <TouchableOpacity style={styles.retryButton} onPress={onRetry}>
            <Text style={styles.retryText}>{t('sync.retry')}</Text>
          </TouchableOpacity>
        )}
        <TouchableOpacity style={styles.dismissButton} onPress={onDismiss}>
          <Text style={styles.dismissText}>✕</Text>
        </TouchableOpacity>
      </View>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  container: {
    position: 'absolute',
    bottom: 100,
    left: 16,
    right: 16,
    backgroundColor: colors.text,
    borderRadius: 12,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 8,
  },
  content: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    paddingHorizontal: 16,
  },
  icon: {
    fontSize: 20,
    marginRight: 12,
  },
  textContainer: {
    flex: 1,
  },
  message: {
    color: colors.white,
    fontSize: 14,
    fontWeight: '500',
  },
  progressBar: {
    height: 4,
    backgroundColor: colors.textSecondary,
    borderRadius: 2,
    marginTop: 8,
    overflow: 'hidden',
  },
  progressFill: {
    height: '100%',
    backgroundColor: colors.accent,
    borderRadius: 2,
  },
  retryButton: {
    paddingHorizontal: 12,
    paddingVertical: 6,
    backgroundColor: colors.dangerDark,
    borderRadius: 6,
    marginLeft: 8,
  },
  retryText: {
    color: colors.white,
    fontSize: 12,
    fontWeight: '600',
  },
  dismissButton: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    marginLeft: 8,
  },
  dismissText: {
    color: colors.textSubtle,
    fontSize: 16,
  },
});
