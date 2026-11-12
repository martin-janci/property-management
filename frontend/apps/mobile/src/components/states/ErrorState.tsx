/**
 * ErrorState — inline error placeholder with optional retry callback.
 */

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { colors } from '../../screens/shared/screenStyles';

export interface ErrorStateProps {
  icon?: string;
  /** Localized title. Falls back to the `common.errorTitle` i18n key. */
  title?: string;
  /** Localized description. Falls back to the `common.errorDescription` i18n key. */
  description?: string;
  onRetry?: () => void;
  /** Localized retry label. Falls back to the `common.tryAgain` i18n key. */
  retryLabel?: string;
  children?: ReactNode;
}

export function ErrorState({
  icon = '⚠️',
  title,
  description,
  onRetry,
  retryLabel,
  children,
}: ErrorStateProps) {
  const { t } = useTranslation();
  // Intrinsic copy flows through i18n (matches QueryErrorBanner); the
  // `defaultValue` keeps a graceful English fallback if a key is ever missing.
  const resolvedTitle = title ?? t('common.errorTitle', { defaultValue: 'Something went wrong' });
  const resolvedDescription =
    description ??
    t('common.errorDescription', {
      defaultValue: 'Please try again. If the issue persists, contact support.',
    });
  const resolvedRetryLabel = retryLabel ?? t('common.tryAgain', { defaultValue: 'Try again' });
  return (
    <View style={styles.container} accessibilityRole="alert">
      <Text style={styles.icon}>{icon}</Text>
      <Text style={styles.title}>{resolvedTitle}</Text>
      {resolvedDescription ? <Text style={styles.description}>{resolvedDescription}</Text> : null}
      {onRetry && (
        <Pressable style={styles.button} onPress={onRetry}>
          <Text style={styles.buttonText}>{resolvedRetryLabel}</Text>
        </Pressable>
      )}
      {children}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
    paddingHorizontal: 32,
    gap: 12,
  },
  icon: { fontSize: 48, marginBottom: 8 },
  title: { fontSize: 18, fontWeight: '600', color: colors.text, textAlign: 'center' },
  description: {
    fontSize: 14,
    color: colors.textMuted,
    textAlign: 'center',
    maxWidth: 320,
  },
  button: {
    marginTop: 8,
    backgroundColor: colors.accent,
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 8,
  },
  buttonText: { color: colors.white, fontSize: 14, fontWeight: '600' },
});
