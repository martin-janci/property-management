/**
 * ErrorState — inline error placeholder with optional retry callback.
 */

import type { ReactNode } from 'react';
import { Pressable, StyleSheet, Text, View } from 'react-native';

export interface ErrorStateProps {
  icon?: string;
  title?: string;
  description?: string;
  onRetry?: () => void;
  retryLabel?: string;
  children?: ReactNode;
}

export function ErrorState({
  icon = '⚠️',
  title = 'Something went wrong',
  description = 'Please try again. If the issue persists, contact support.',
  onRetry,
  retryLabel = 'Try again',
  children,
}: ErrorStateProps) {
  return (
    <View style={styles.container} accessibilityRole="alert">
      <Text style={styles.icon}>{icon}</Text>
      <Text style={styles.title}>{title}</Text>
      {description && <Text style={styles.description}>{description}</Text>}
      {onRetry && (
        <Pressable style={styles.button} onPress={onRetry}>
          <Text style={styles.buttonText}>{retryLabel}</Text>
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
  title: { fontSize: 18, fontWeight: '600', color: '#1f2937', textAlign: 'center' },
  description: {
    fontSize: 14, color: '#6b7280', textAlign: 'center', maxWidth: 320,
  },
  button: {
    marginTop: 8,
    backgroundColor: '#2563eb',
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 8,
  },
  buttonText: { color: '#fff', fontSize: 14, fontWeight: '600' },
});
