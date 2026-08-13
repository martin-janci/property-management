/**
 * PersonMonthsScreen (UC-10).
 *
 * Lets a resident view and edit the number of person-months declared per
 * unit per month, used for cost-allocation.
 *
 * NOTE: still backed by mock data. The read endpoint
 * (`GET /api/v1/buildings/{building_id}/units/{unit_id}/person-months?year=…`)
 * needs both a building id and a unit id plus a year, and its `PersonMonth`
 * rows carry no per-entry draft/submitted/confirmed status — the field this
 * screen's editing flow depends on. Wiring it cleanly needs a resident-scoped
 * "my person-months" endpoint (or a status column) on the backend, so it is
 * intentionally left on mock data (tracked as a backend follow-up). Sibling
 * list/detail screens (Meters, Leases, Forms, Threads) are already wired.
 */

import { useCallback, useState } from 'react';
import {
  Alert,
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

interface PersonMonthEntry {
  id: string;
  month: string; // ISO YYYY-MM
  declaredPersons: number;
  status: 'draft' | 'submitted' | 'confirmed';
}

const MOCK_ENTRIES: PersonMonthEntry[] = [
  { id: 'pm-2026-04', month: '2026-04', declaredPersons: 3, status: 'draft' },
  { id: 'pm-2026-03', month: '2026-03', declaredPersons: 3, status: 'confirmed' },
  { id: 'pm-2026-02', month: '2026-02', declaredPersons: 2, status: 'confirmed' },
  { id: 'pm-2026-01', month: '2026-01', declaredPersons: 2, status: 'confirmed' },
];

function formatMonth(month: string): string {
  const [year, m] = month.split('-').map(Number);
  return formatDate(new Date(year, (m ?? 1) - 1, 1), {
    month: 'long',
    year: 'numeric',
  });
}

interface PersonMonthsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function PersonMonthsScreen({ onNavigate }: PersonMonthsScreenProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [entries, setEntries] = useState<PersonMonthEntry[]>(MOCK_ENTRIES);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draftValue, setDraftValue] = useState('');

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 500));
    setRefreshing(false);
  }, []);

  const startEdit = (entry: PersonMonthEntry) => {
    if (entry.status === 'confirmed') return;
    setEditingId(entry.id);
    setDraftValue(String(entry.declaredPersons));
  };

  const saveEdit = (entry: PersonMonthEntry) => {
    const next = Number(draftValue);
    if (!Number.isInteger(next) || next < 0 || next > 20) {
      Alert.alert('Invalid value', 'Enter a whole number between 0 and 20.');
      return;
    }
    setEntries((prev) =>
      prev.map((e) =>
        e.id === entry.id ? { ...e, declaredPersons: next, status: 'submitted' } : e
      )
    );
    setEditingId(null);
    setDraftValue('');
  };

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Person-months</Text>
        <Text style={s.headerSubtitle}>
          Declare how many people lived in your unit each month for cost allocation.
        </Text>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {entries.map((entry) => {
          const isEditing = entry.id === editingId;
          return (
            <View key={entry.id} style={s.card}>
              <View style={s.cardHeader}>
                <Text style={s.cardTitle}>{formatMonth(entry.month)}</Text>
                <View
                  style={[
                    s.badge,
                    entry.status === 'confirmed' && styles.badgeConfirmed,
                    entry.status === 'submitted' && styles.badgeSubmitted,
                    entry.status === 'draft' && styles.badgeDraft,
                  ]}
                >
                  <Text
                    style={[
                      s.badgeText,
                      entry.status === 'confirmed' && styles.badgeConfirmedText,
                      entry.status === 'submitted' && styles.badgeSubmittedText,
                      entry.status === 'draft' && styles.badgeDraftText,
                    ]}
                  >
                    {entry.status}
                  </Text>
                </View>
              </View>

              {isEditing ? (
                <View style={styles.editRow}>
                  <TextInput
                    style={styles.editInput}
                    value={draftValue}
                    onChangeText={setDraftValue}
                    keyboardType="number-pad"
                  />
                  <Pressable
                    style={[s.primaryButton, styles.saveButton]}
                    onPress={() => saveEdit(entry)}
                  >
                    <Text style={s.primaryButtonText}>Save</Text>
                  </Pressable>
                  <Pressable style={styles.cancelButton} onPress={() => setEditingId(null)}>
                    <Text style={styles.cancelButtonText}>Cancel</Text>
                  </Pressable>
                </View>
              ) : (
                <View style={styles.viewRow}>
                  <Text style={styles.value}>
                    {entry.declaredPersons} person{entry.declaredPersons === 1 ? '' : 's'}
                  </Text>
                  {entry.status !== 'confirmed' && (
                    <Pressable onPress={() => startEdit(entry)} style={styles.editLink}>
                      <Text style={styles.editLinkText}>Edit</Text>
                    </Pressable>
                  )}
                </View>
              )}
            </View>
          );
        })}

        <Pressable style={s.primaryButton} onPress={() => onNavigate?.('PersonMonthsBulk')}>
          <Text style={s.primaryButtonText}>Bulk entry…</Text>
        </Pressable>

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  badgeDraft: { backgroundColor: colors.warningBg },
  badgeDraftText: { color: colors.warningDark },
  badgeSubmitted: { backgroundColor: colors.accentSoft },
  badgeSubmittedText: { color: colors.accentDark },
  badgeConfirmed: { backgroundColor: colors.successBg },
  badgeConfirmedText: { color: colors.successDark },
  viewRow: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  value: { fontSize: 18, fontWeight: '600', color: colors.text },
  editLink: { padding: 8 },
  editLinkText: { color: colors.accent, fontWeight: '500' },
  editRow: { flexDirection: 'row', alignItems: 'center', gap: 8, flexWrap: 'wrap' },
  editInput: {
    flex: 1,
    minWidth: 80,
    padding: 12,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    fontSize: 16,
  },
  saveButton: { marginTop: 0, paddingHorizontal: 16, paddingVertical: 12 },
  cancelButton: { padding: 12 },
  cancelButtonText: { color: colors.textMuted, fontWeight: '500' },
});
