/**
 * NewMessageScreen (UC-05).
 *
 * Starts a conversation: pick a neighbour from the building directory, write
 * the first message, send. The "+ New message" button on the Messages list
 * navigated here before this screen existed and fell through to the
 * Dashboard.
 *
 * Recipients come from `GET /api/v1/buildings/{id}/neighbors` (only residents
 * who opted into the directory, same source as the Neighbours screen). The
 * thread is created with `POST /api/v1/messages/threads`
 * (`StartThreadRequest`, camelCase: `recipientId`, `initialMessage`); the
 * server returns the existing 1:1 thread when one already exists.
 */

import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useAuth } from '../../contexts/AuthContext';
import { useApiMutation, useApiQuery, useTenantId } from '../../hooks/useApi';
import {
  type BuildingsPaginatedResponse,
  filterBySearch,
  filterVisible,
  type NeighborsResponse,
  type NeighborView,
  selectBuilding,
} from '../neighbors/NeighborsScreen';
import { colors, screenStyles as s } from '../shared/screenStyles';

/** Body of `POST /api/v1/messages/threads`. */
export interface StartThreadBody {
  recipientId: string;
  initialMessage: string;
}

/** Neighbours the user can write to: visible in the directory, not themself. */
export function selectableRecipients(
  neighbors: NeighborView[] | undefined,
  ownUserId: string | undefined
): NeighborView[] {
  return filterVisible(neighbors ?? []).filter((n) => n.user_id && n.user_id !== ownUserId);
}

/** Build the request body, or `null` while the form is incomplete. */
export function buildStartThreadBody(
  recipientId: string | undefined,
  message: string
): StartThreadBody | null {
  const text = message.trim();
  if (!recipientId || text === '') return null;
  return { recipientId, initialMessage: text };
}

/** Read the new thread's id out of the `{ thread, ... }` response. */
export function startedThreadId(data: unknown): string | undefined {
  const thread = (data as { thread?: { id?: unknown } } | null)?.thread;
  return typeof thread?.id === 'string' && thread.id ? thread.id : undefined;
}

interface NewMessageScreenProps {
  onCancel?: () => void;
  /** Called with the thread id once the first message is sent. */
  onSent?: (threadId: string | undefined) => void;
}

export function NewMessageScreen({ onCancel, onSent }: NewMessageScreenProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { tenantId } = useTenantId();
  const [search, setSearch] = useState('');
  const [recipientId, setRecipientId] = useState<string | undefined>(undefined);
  const [message, setMessage] = useState('');

  // Same keys + paths as NeighborsScreen so the two share a cache.
  const buildingsQuery = useApiQuery<BuildingsPaginatedResponse>(
    ['buildings', 'list', tenantId],
    `/api/v1/buildings?organization_id=${tenantId ?? ''}`,
    { enabled: !!tenantId }
  );
  const { buildingId } = selectBuilding(buildingsQuery.data);
  const neighborsQuery = useApiQuery<NeighborsResponse>(
    ['neighbors', 'list', buildingId ?? '', 0],
    `/api/v1/buildings/${buildingId}/neighbors`,
    { enabled: !!buildingId }
  );

  const startThread = useApiMutation<unknown, StartThreadBody>('/api/v1/messages/threads', 'POST');

  const recipients = useMemo(
    () => selectableRecipients(neighborsQuery.data?.neighbors, user?.id),
    [neighborsQuery.data, user?.id]
  );
  const filtered = useMemo(() => filterBySearch(recipients, search), [recipients, search]);

  const isLoading = buildingsQuery.isLoading || neighborsQuery.isLoading;
  const loadError = buildingsQuery.error ?? neighborsQuery.error;
  const body = buildStartThreadBody(recipientId, message);

  const send = () => {
    if (!body || startThread.isPending) return;
    startThread.mutate(body, { onSuccess: (data) => onSent?.(startedThreadId(data)) });
  };

  return (
    <View style={s.container}>
      <Pressable style={styles.backButton} onPress={onCancel} disabled={!onCancel}>
        <Text style={styles.backButtonText}>{t('messages.backToList')}</Text>
      </Pressable>

      <View style={styles.section}>
        <Text style={styles.label}>{t('messages.recipientLabel')}</Text>
        <TextInput
          style={s.searchInput}
          placeholder={t('neighbors.searchPlaceholder')}
          value={search}
          onChangeText={setSearch}
        />
      </View>

      <ScrollView style={s.scrollView} keyboardShouldPersistTaps="handled">
        {isLoading ? (
          <View style={s.emptyState}>
            <ActivityIndicator size="large" color={colors.accent} />
          </View>
        ) : loadError ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('neighbors.loadError')}</Text>
          </View>
        ) : filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>
              {recipients.length === 0 ? t('messages.noRecipients') : t('neighbors.noMatches')}
            </Text>
          </View>
        ) : (
          filtered.map((neighbor) => {
            const selected = neighbor.user_id === recipientId;
            return (
              <Pressable
                key={neighbor.user_id}
                style={[s.card, selected && styles.selectedCard]}
                onPress={() => setRecipientId(neighbor.user_id)}
                accessibilityRole="radio"
                accessibilityState={{ selected }}
              >
                <Text style={s.cardTitle}>{neighbor.display_name}</Text>
                {neighbor.unit_label ? <Text style={s.cardMeta}>{neighbor.unit_label}</Text> : null}
              </Pressable>
            );
          })
        )}
      </ScrollView>

      <View style={styles.section}>
        {startThread.error ? (
          <Text style={styles.errorText}>{t('messages.threadSendError')}</Text>
        ) : null}
        <TextInput
          style={[s.searchInput, styles.messageInput]}
          placeholder={t('messages.composerPlaceholder')}
          value={message}
          onChangeText={setMessage}
          multiline
        />
        <Pressable
          style={[s.primaryButton, (!body || startThread.isPending) && styles.disabled]}
          onPress={send}
          disabled={!body || startThread.isPending}
        >
          <Text style={s.primaryButtonText}>{t('messages.send')}</Text>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  backButton: { paddingHorizontal: 16, paddingVertical: 12 },
  backButtonText: { color: colors.accent, fontSize: 16, fontWeight: '600' },
  section: { paddingHorizontal: 16, paddingVertical: 8 },
  label: { color: colors.textMuted, fontSize: 13, marginBottom: 6 },
  selectedCard: { borderColor: colors.accent, borderWidth: 2 },
  messageInput: { minHeight: 72, textAlignVertical: 'top', marginBottom: 8 },
  errorText: { color: colors.danger, marginBottom: 8 },
  disabled: { opacity: 0.5 },
});
