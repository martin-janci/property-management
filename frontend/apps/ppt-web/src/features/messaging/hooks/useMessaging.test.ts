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
import {
  mapApiMessageToUi,
  mapApiThreadDetailToUi,
  mapApiThreadToUi,
  toStartThreadRequest,
} from './useMessaging';

function participant(firstName: string, lastName: string, id = 'p1'): ParticipantInfo {
  return { id, firstName, lastName, email: `${id}@example.com` };
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

function thread(others: ParticipantInfo[]): ThreadWithPreview {
  return {
    id: 't1',
    organizationId: 'o1',
    participantIds: [...others.map((p) => p.id), 'me'],
    participants: others,
    lastMessage: {
      id: 'm1',
      content: 'hi',
      senderId: others[0].id,
      isFromMe: false,
      createdAt: '2026-06-06T00:00:00Z',
    },
    unreadCount: 0,
    createdAt: '2026-06-06T00:00:00Z',
    updatedAt: '2026-06-06T00:00:00Z',
  };
}

function threadDetail(others: ParticipantInfo[]): ThreadDetailResponse {
  return {
    thread: {
      id: 't1',
      organizationId: 'o1',
      participantIds: [...others.map((p) => p.id), 'me'],
      lastMessageAt: '2026-06-06T00:00:00Z',
      createdAt: '2026-06-06T00:00:00Z',
      updatedAt: '2026-06-06T00:00:00Z',
    },
    participants: others,
    messages: [message(others[0])],
    messageCount: 1,
  };
}

describe('messaging name-join mappers (#1071)', () => {
  it('drops the trailing space when lastName is empty', () => {
    expect(mapApiMessageToUi(message(participant('Jane', ''))).senderName).toBe('Jane');

    const ui = mapApiThreadToUi(thread([participant('Jane', '')]));
    expect(ui.lastMessageSenderName).toBe('Jane');
    expect(ui.participants[0].userName).toBe('Jane');

    expect(
      mapApiThreadDetailToUi(threadDetail([participant('Jane', '')])).participants[0].userName
    ).toBe('Jane');
  });

  it('joins a real first and last name with a single space', () => {
    expect(mapApiMessageToUi(message(participant('Jane', 'Doe'))).senderName).toBe('Jane Doe');

    const ui = mapApiThreadToUi(thread([participant('Jane', 'Doe')]));
    expect(ui.lastMessageSenderName).toBe('Jane Doe');
    expect(ui.participants[0].userName).toBe('Jane Doe');
  });
});

describe('group conversation participant mapping ([BIT-206])', () => {
  const alice = participant('Alice', 'Adams', 'p1');
  const bob = participant('Bob', 'Brown', 'p2');

  it('renders every other participant in the list preview, not just one', () => {
    const ui = mapApiThreadToUi(thread([alice, bob]));
    expect(ui.participants.map((p) => p.userName)).toEqual(['Alice Adams', 'Bob Brown']);
    expect(ui.participants.map((p) => p.userId)).toEqual(['p1', 'p2']);
  });

  it('attributes the list preview to the actual last-message sender', () => {
    // last message is from `bob` (the second participant), not the first.
    const t = thread([alice, bob]);
    t.lastMessage = { ...t.lastMessage!, senderId: 'p2' };
    const ui = mapApiThreadToUi(t);
    expect(ui.lastMessageSenderName).toBe('Bob Brown');
  });

  it('renders every other participant in the thread detail', () => {
    const ui = mapApiThreadDetailToUi(threadDetail([alice, bob]));
    expect(ui.participants.map((p) => p.userName)).toEqual(['Alice Adams', 'Bob Brown']);
    expect(ui.participantCount).toBe(3); // alice + bob + me
  });
});

describe('toStartThreadRequest ([BIT-206])', () => {
  it('sends the full recipient set as snake_case recipient_ids', () => {
    const req = toStartThreadRequest({
      recipientIds: ['p1', 'p2', 'p3'],
      initialMessage: 'hi all',
    });
    expect(req).toEqual({ recipient_ids: ['p1', 'p2', 'p3'], initial_message: 'hi all' });
  });

  it('keeps a single recipient as a one-element array', () => {
    const req = toStartThreadRequest({ recipientIds: ['p1'], initialMessage: 'hi' });
    expect(req.recipient_ids).toEqual(['p1']);
  });
});
