/**
 * EmptyState — inline empty placeholder for mobile screens.
 */

import type { ReactNode } from 'react';
import { StyleSheet, Text, View } from 'react-native';

export interface EmptyStateProps {
  icon?: string;
  title: string;
  description?: string;
  children?: ReactNode;
}

export function EmptyState({ icon = '📭', title, description, children }: EmptyStateProps) {
  return (
    <View style={styles.container} accessibilityRole="summary">
      <Text style={styles.icon}>{icon}</Text>
      <Text style={styles.title}>{title}</Text>
      {description && <Text style={styles.description}>{description}</Text>}
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
    gap: 8,
  },
  icon: { fontSize: 48, marginBottom: 8 },
  title: { fontSize: 18, fontWeight: '600', color: '#1f2937', textAlign: 'center' },
  description: {
    fontSize: 14,
    color: '#6b7280',
    textAlign: 'center',
    maxWidth: 320,
  },
});
