/// <reference types="vitest/globals" />
/**
 * Unit tests for the messaging API → UI name-join mappers (#1071).
 *
 * The `users` table has a single `name` column, so the API maps it onto
 * `firstName` and leaves `lastName` empty. The mappers must not leave a
 * trailing space when joining first/last into a display name (e.g. `"Jane "`),
 * while still joining a real first+last pair with a single space.
 */

import type {
  MessageWithSender,
  ParticipantInfo,
  ThreadDetailResponse,
  ThreadWithPreview,
} from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { mapApiMessageToUi, mapApiThreadDetailToUi, mapApiThreadToUi } from './useMessaging';

function participant(firstName: string, lastName: string): ParticipantInfo {
  return { id: 'p1', firstName, lastName, email: 'p1@example.com' };
}

function message(sender: ParticipantInfo): MessageWithSender {
  return {
    id: 'm1',
    threadId: 't1',
    sender,
    content: 'hi',
    readAt: null,
    isDeleted: false,
    createdAt: '2026-06-06T00:00:00Z',
  };
}

function thread(other: ParticipantInfo): ThreadWithPreview {
  return {
    id: 't1',
    organizationId: 'o1',
    participantIds: ['p1', 'me'],
    otherParticipant: other,
    lastMessage: {
      id: 'm1',
      content: 'hi',
      senderId: 'p1',
      isFromMe: false,
      createdAt: '2026-06-06T00:00:00Z',
    },
    unreadCount: 0,
    createdAt: '2026-06-06T00:00:00Z',
    updatedAt: '2026-06-06T00:00:00Z',
  };
}

function threadDetail(other: ParticipantInfo): ThreadDetailResponse {
  return {
    thread: {
      id: 't1',
      organizationId: 'o1',
      participantIds: ['p1', 'me'],
      lastMessageAt: '2026-06-06T00:00:00Z',
      createdAt: '2026-06-06T00:00:00Z',
      updatedAt: '2026-06-06T00:00:00Z',
    },
    otherParticipant: other,
    messages: [message(other)],
    messageCount: 1,
  };
}

describe('messaging name-join mappers (#1071)', () => {
  it('drops the trailing space when lastName is empty', () => {
    expect(mapApiMessageToUi(message(participant('Jane', ''))).senderName).toBe('Jane');

    const ui = mapApiThreadToUi(thread(participant('Jane', '')));
    expect(ui.lastMessageSenderName).toBe('Jane');
    expect(ui.participants[0].userName).toBe('Jane');

    expect(
      mapApiThreadDetailToUi(threadDetail(participant('Jane', ''))).participants[0].userName
    ).toBe('Jane');
  });

  it('joins a real first and last name with a single space', () => {
    expect(mapApiMessageToUi(message(participant('Jane', 'Doe'))).senderName).toBe('Jane Doe');

    const ui = mapApiThreadToUi(thread(participant('Jane', 'Doe')));
    expect(ui.lastMessageSenderName).toBe('Jane Doe');
    expect(ui.participants[0].userName).toBe('Jane Doe');
  });
});
