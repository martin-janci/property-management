/**
 * QueryErrorBanner — inline, retryable error banner.
 *
 * Extracted from `DashboardScreen` (#2282/#2304) so the "surface a distinct,
 * retryable banner above whatever partial data loaded" contract can be reused
 * across screens instead of copy-pasting the JSX + styles (#2323).
 *
 * Unlike the full-screen `ErrorState`, this renders as a compact strip meant
 * to sit ABOVE partial content: on a screen that fans out several independent
 * queries, the ones that succeeded still render beneath the banner. The banner
 * shows a caller-provided, already-localized `message` (never a raw
 * backend/error string — that would leak internals) and a retry button wired
 * to `onRetry` (typically the screen's existing pull-to-refresh `onRefresh`).
 */

import { useTranslation } from 'react-i18next';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { colors } from '../screens/shared/screenStyles';

export interface QueryErrorBannerProps {
  /** Localized, user-facing message. Do NOT pass a raw `error.message`. */
  message: string;
  /** Called when the user taps retry — usually the screen's `onRefresh`. */
  onRetry: () => void;
}

export function QueryErrorBanner({ message, onRetry }: QueryErrorBannerProps) {
  const { t } = useTranslation();
  return (
    <View style={styles.banner} accessibilityRole="alert">
      <Text style={styles.icon}>⚠️</Text>
      <Text style={styles.text}>{message}</Text>
      <Pressable style={styles.button} onPress={onRetry} accessibilityRole="button">
        <Text style={styles.buttonText}>{t('common.retry')}</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  banner: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
    marginHorizontal: 16,
    marginTop: 16,
    padding: 12,
    borderRadius: 12,
    backgroundColor: colors.dangerBg,
    borderWidth: 1,
    borderColor: colors.danger,
  },
  icon: { fontSize: 20 },
  text: { flex: 1, fontSize: 13, color: colors.dangerDark },
  button: {
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 8,
    backgroundColor: colors.danger,
  },
  buttonText: { color: colors.white, fontSize: 13, fontWeight: '600' },
});
