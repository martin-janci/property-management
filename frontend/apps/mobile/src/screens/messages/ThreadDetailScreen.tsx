/**
 * ThreadDetailScreen (UC-05.4 / 05.5).
 *
 * Conversation view with a compose input.
 *
 * Wired to `GET /api/v1/messages/threads/{id}` (returns a
 * `ThreadDetailResponse { thread, participants, messages, messageCount }`) and
 * `POST /api/v1/messages/threads/{id}/messages` for sending. The messaging
 * response structs use `#[serde(rename_all = "camelCase")]`, so the wire format
 * is camelCase (see `MessageWithSender`, `ParticipantInfo`). `fromMe` is derived
 * by comparing each message's `sender.id` to the authenticated user id
 * (`useAuth().user.id`). The response is consumed without a generated client,
 * so it is validated defensively.
 */

import type { MessageWithSender, ParticipantInfo, ThreadDetailResponse } from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { QueryErrorBanner } from '../../components/QueryErrorBanner';
import { useAuth } from '../../contexts/AuthContext';
import { useApiMutation, useApiQuery } from '../../hooks/useApi';
import { warnIfListDegraded, warnIfParseFailed } from '../shared/parserWarnings';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface Message {
  id: string;
  body: string;
  sentAt: string;
  fromMe: boolean;
  authorName: string;
}

/**
 * `ParticipantInfo` (camelCase wire format), typed against the generated
 * `@ppt/api-client` payload so backend field renames surface at compile time.
 * Relaxed to `Partial` (only `id` required) because the shape is validated
 * defensively at runtime.
 */
type ApiParticipant = Partial<ParticipantInfo> & Pick<ParticipantInfo, 'id'>;

/** `MessageWithSender` fields consumed by this screen (camelCase wire format). */
type ApiMessage = Pick<MessageWithSender, 'id' | 'content' | 'createdAt'> & {
  sender?: ApiParticipant;
};

/** Validated subset of the generated `ThreadDetailResponse` envelope. */
interface ApiThreadDetail {
  participants?: ApiParticipant[];
  messages?: ApiMessage[];
}

function participantName(p: ApiParticipant | undefined): string {
  if (!p) return '';
  const full = `${p.firstName ?? ''} ${p.lastName ?? ''}`.trim();
  return full || p.email || '';
}

/** Derive the conversation title from the thread's other participants. */
export function threadTitle(detail: ApiThreadDetail | null, fallback: string): string {
  const people = detail?.participants ?? [];
  if (people.length === 0) return fallback;
  const [first, ...rest] = people;
  const name = participantName(first) || fallback;
  return rest.length > 0 ? `${name} +${rest.length}` : name;
}

function isApiMessage(value: unknown): value is ApiMessage {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.id === 'string' && typeof v.content === 'string';
}

/** Validate the raw `GET .../threads/{id}` body, or null on a bad shape. */
export function parseThreadDetail(data: unknown): ApiThreadDetail | null {
  const parsed = parseThreadDetailInner(data);
  warnIfParseFailed('parseThreadDetail', data, parsed);
  return parsed;
}

function parseThreadDetailInner(data: unknown): ApiThreadDetail | null {
  if (typeof data !== 'object' || data === null) return null;
  const v = data as Record<string, unknown>;
  const rawMessages = v['messages' satisfies keyof ThreadDetailResponse];
  const candidates: unknown[] | null = Array.isArray(rawMessages) ? rawMessages : null;
  const messages = (candidates ?? []).filter(isApiMessage);
  warnIfListDegraded('parseThreadDetail.messages', rawMessages, candidates, messages.length);
  const rawParticipants = v['participants' satisfies keyof ThreadDetailResponse];
  return {
    participants: Array.isArray(rawParticipants) ? (rawParticipants as ApiParticipant[]) : [],
    messages,
  };
}

/** Map api-server messages onto UI bubbles, oldest first. `currentUserId`
 *  drives the `fromMe` flag. Exported for unit testing. */
export function toUiMessages(detail: ApiThreadDetail | null, currentUserId?: string): Message[] {
  return (detail?.messages ?? [])
    .map((m) => {
      const fromMe = !!currentUserId && m.sender?.id === currentUserId;
      return {
        id: m.id,
        body: m.content,
        sentAt: m.createdAt,
        fromMe,
        authorName: fromMe ? 'You' : participantName(m.sender) || 'Participant',
      };
    })
    .sort((a, b) => new Date(a.sentAt).getTime() - new Date(b.sentAt).getTime());
}

interface ThreadDetailScreenProps {
  threadId?: string;
  participantName?: string;
  onBack?: () => void;
}

export function ThreadDetailScreen({
  threadId,
  participantName: participantNameProp = 'Conversation',
  onBack,
}: ThreadDetailScreenProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [draft, setDraft] = useState('');

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<unknown>(
    ['messages', 'thread', threadId],
    `/api/v1/messages/threads/${threadId ?? ''}`,
    { staleTime: 15_000, enabled: !!threadId }
  );

  const sendMessage = useApiMutation<unknown, { content: string }>(
    `/api/v1/messages/threads/${threadId ?? ''}/messages`,
    'POST'
  );

  const detail = parseThreadDetail(data);
  const messages = toUiMessages(detail, user?.id);
  const title = threadTitle(detail, participantNameProp);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const handleSend = () => {
    const trimmed = draft.trim();
    if (!trimmed || !threadId || sendMessage.isPending) return;
    sendMessage.mutate(
      { content: trimmed },
      {
        onSuccess: () => {
          setDraft('');
          refetch();
        },
        onError: () => {
          // Surface the failure inline (see `sendMessage.isError` banner
          // below) and deliberately DO NOT clear `draft` — the user's message
          // stays in the composer so they can retry without retyping.
        },
      }
    );
  };

  return (
    <KeyboardAvoidingView
      style={s.container}
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
    >
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← Messages</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>{title}</Text>
      </View>

      <ScrollView style={s.scrollView} contentContainerStyle={styles.threadContent}>
        {isLoading || !threadId ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('messages.threadLoadError')}</Text>
            <Pressable style={s.primaryButton} onPress={onRefresh}>
              <Text style={s.primaryButtonText}>{t('common.tryAgain')}</Text>
            </Pressable>
          </View>
        ) : messages.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>💬</Text>
            <Text style={s.emptyTitle}>{t('messages.threadEmptyTitle')}</Text>
            <Text style={s.emptyText}>{t('messages.threadEmptyText')}</Text>
          </View>
        ) : (
          messages.map((message) => (
            <View
              key={message.id}
              style={[styles.bubble, message.fromMe ? styles.bubbleMine : styles.bubbleTheirs]}
            >
              {!message.fromMe && <Text style={styles.author}>{message.authorName}</Text>}
              <Text style={[styles.body, message.fromMe && styles.bodyMine]}>{message.body}</Text>
              <Text style={[styles.time, message.fromMe && styles.timeMine]}>
                {new Date(message.sentAt).toLocaleTimeString('en-US', {
                  hour: '2-digit',
                  minute: '2-digit',
                })}
              </Text>
            </View>
          ))
        )}
        {isFetching && !isLoading ? <Text style={styles.syncing}>Syncing…</Text> : null}
        <View style={s.bottomSpacer} />
      </ScrollView>

      {sendMessage.isError ? (
        // Surface the failed send inline, above the composer, and keep the
        // drafted text intact (handleSend only clears `draft` on success) so
        // the user can retry without retyping. A localized, non-leaking
        // message per the QueryErrorBanner contract — never the raw
        // backend/error string.
        <QueryErrorBanner
          message="Couldn't send your message. Please try again."
          onRetry={handleSend}
        />
      ) : null}

      <View style={styles.composer}>
        <TextInput
          style={styles.composerInput}
          value={draft}
          onChangeText={setDraft}
          placeholder="Write a message…"
          multiline
        />
        <Pressable
          onPress={handleSend}
          style={[
            styles.sendButton,
            (!draft.trim() || sendMessage.isPending) && styles.sendButtonDisabled,
          ]}
          disabled={!draft.trim() || sendMessage.isPending}
        >
          <Text style={styles.sendButtonText}>{sendMessage.isPending ? '…' : 'Send'}</Text>
        </Pressable>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  threadContent: { paddingBottom: 24 },
  syncing: { textAlign: 'center', color: colors.textMuted, fontSize: 12, marginTop: 8 },
  bubble: {
    padding: 12,
    borderRadius: 16,
    marginBottom: 10,
    maxWidth: '85%',
  },
  bubbleMine: { backgroundColor: colors.accent, alignSelf: 'flex-end' },
  bubbleTheirs: { backgroundColor: colors.surface, alignSelf: 'flex-start' },
  author: {
    fontSize: 12,
    fontWeight: '600',
    color: colors.textMuted,
    marginBottom: 4,
  },
  body: { fontSize: 15, color: colors.text },
  bodyMine: { color: colors.surface },
  time: {
    fontSize: 11,
    color: colors.textSubtle,
    marginTop: 4,
    alignSelf: 'flex-end',
  },
  timeMine: { color: colors.textOnAccentSoft },
  composer: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    gap: 8,
    padding: 12,
    backgroundColor: colors.surface,
    borderTopWidth: 1,
    borderTopColor: colors.border,
  },
  composerInput: {
    flex: 1,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    padding: 12,
    fontSize: 15,
    maxHeight: 120,
  },
  sendButton: {
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderRadius: 8,
    backgroundColor: colors.accent,
  },
  sendButtonDisabled: { backgroundColor: colors.accentDisabled },
  sendButtonText: { color: colors.surface, fontWeight: '600' },
});
