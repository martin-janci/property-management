/**
 * ThreadDetailScreen (UC-05.4 / 05.5).
 *
 * Conversation view with a simple compose input. Mock data backs the
 * message list until the messaging API client is wired in.
 */

import { useState } from 'react';
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
import { colors, screenStyles as s } from '../shared/screenStyles';

interface Message {
  id: string;
  body: string;
  sentAt: string;
  fromMe: boolean;
  authorName: string;
}

const MOCK_MESSAGES: Message[] = [
  {
    id: 'm1',
    body: 'Hi! When can we expect the elevator repair to be finished?',
    sentAt: '2026-04-22T08:30:00Z',
    fromMe: true,
    authorName: 'You',
  },
  {
    id: 'm2',
    body: 'The technicians are scheduled for Wednesday morning. We expect it to be done by lunchtime.',
    sentAt: '2026-04-22T09:15:00Z',
    fromMe: false,
    authorName: 'Building manager',
  },
  {
    id: 'm3',
    body: 'Great, thank you for the update!',
    sentAt: '2026-04-22T09:18:00Z',
    fromMe: true,
    authorName: 'You',
  },
];

interface ThreadDetailScreenProps {
  threadId?: string;
  participantName?: string;
  onBack?: () => void;
}

export function ThreadDetailScreen({
  participantName = 'Building manager',
  onBack,
}: ThreadDetailScreenProps) {
  const [messages, setMessages] = useState<Message[]>(MOCK_MESSAGES);
  const [draft, setDraft] = useState('');

  const handleSend = () => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    setMessages((prev) => [
      ...prev,
      {
        id: `m${prev.length + 1}`,
        body: trimmed,
        sentAt: new Date().toISOString(),
        fromMe: true,
        authorName: 'You',
      },
    ]);
    setDraft('');
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
        <Text style={s.headerTitle}>{participantName}</Text>
      </View>

      <ScrollView style={s.scrollView} contentContainerStyle={styles.threadContent}>
        {messages.map((message) => (
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
        ))}
        <View style={s.bottomSpacer} />
      </ScrollView>

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
          style={[styles.sendButton, !draft.trim() && styles.sendButtonDisabled]}
          disabled={!draft.trim()}
        >
          <Text style={styles.sendButtonText}>Send</Text>
        </Pressable>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  threadContent: { paddingBottom: 24 },
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
  bodyMine: { color: '#fff' },
  time: {
    fontSize: 11,
    color: colors.textSubtle,
    marginTop: 4,
    alignSelf: 'flex-end',
  },
  timeMine: { color: 'rgba(255,255,255,0.85)' },
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
  sendButtonDisabled: { backgroundColor: '#93c5fd' },
  sendButtonText: { color: '#fff', fontWeight: '600' },
});
