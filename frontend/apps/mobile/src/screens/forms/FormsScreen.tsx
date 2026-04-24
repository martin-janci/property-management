/**
 * FormsScreen (UC-09).
 *
 * Lists forms residents need to complete (consent, declarations, surveys).
 */

import { useCallback, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type FormStatus = 'pending' | 'in_progress' | 'completed';

export interface ResidentForm {
  id: string;
  title: string;
  description: string;
  dueAt?: string;
  status: FormStatus;
  required: boolean;
}

const MOCK_FORMS: ResidentForm[] = [
  {
    id: 'f1',
    title: 'Annual fire-safety declaration',
    description: 'Confirm that all fire-safety equipment in your unit is functional.',
    dueAt: '2026-04-30T23:59:00Z',
    status: 'pending',
    required: true,
  },
  {
    id: 'f2',
    title: 'Pet registration',
    description: 'Optional: register pets living in your unit.',
    status: 'in_progress',
    required: false,
  },
  {
    id: 'f3',
    title: 'Energy-efficiency survey',
    description: 'Help the building plan upgrades by sharing your usage habits.',
    dueAt: '2026-05-10T23:59:00Z',
    status: 'pending',
    required: false,
  },
  {
    id: 'f4',
    title: 'GDPR consent — communications',
    description: 'Choose which channels we may use to contact you.',
    status: 'completed',
    required: true,
  },
];

function statusBadgeStyle(status: FormStatus) {
  switch (status) {
    case 'pending':
      return { background: '#fee2e2', color: colors.danger };
    case 'in_progress':
      return { background: '#fef3c7', color: '#b45309' };
    case 'completed':
      return { background: '#d1fae5', color: '#047857' };
  }
}

interface FormsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function FormsScreen({ onNavigate }: FormsScreenProps) {
  const [refreshing, setRefreshing] = useState(false);

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 500));
    setRefreshing(false);
  }, []);

  const pending = MOCK_FORMS.filter((f) => f.status !== 'completed').length;

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Forms</Text>
        {pending > 0 && (
          <Text style={s.headerSubtitle}>
            {pending} form{pending === 1 ? '' : 's'} need{pending === 1 ? 's' : ''} attention
          </Text>
        )}
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {MOCK_FORMS.map((form) => {
          const style = statusBadgeStyle(form.status);
          return (
            <Pressable
              key={form.id}
              style={s.card}
              onPress={() => onNavigate?.('FormFill', { formId: form.id })}
            >
              <View style={s.cardHeader}>
                <Text style={s.cardTitle} numberOfLines={2}>
                  {form.title}
                </Text>
                <View style={[s.badge, { backgroundColor: style.background }]}>
                  <Text style={[s.badgeText, { color: style.color }]}>
                    {form.status.replace('_', ' ')}
                  </Text>
                </View>
              </View>
              <Text style={s.cardBody}>{form.description}</Text>
              <View style={s.cardFooter}>
                {form.required && (
                  <View style={[s.badge, { backgroundColor: '#fef2f2' }]}>
                    <Text style={[s.badgeText, { color: colors.danger }]}>Required</Text>
                  </View>
                )}
                {form.dueAt && (
                  <Text style={[s.cardMeta, { marginLeft: 'auto' }]}>
                    Due {new Date(form.dueAt).toLocaleDateString()}
                  </Text>
                )}
              </View>
            </Pressable>
          );
        })}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

// Reserved for future use.
export const FORMS_SCREEN_STYLES = StyleSheet.create({});
