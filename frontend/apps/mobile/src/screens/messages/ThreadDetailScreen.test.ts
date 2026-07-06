import { parseThreadDetail, threadTitle, toUiMessages } from './ThreadDetailScreen';

const validDetail = {
  participants: [{ id: 'u-2', firstName: 'Building', lastName: 'Manager', email: 'm@x.sk' }],
  messages: [
    { id: 'm2', content: 'Reply', createdAt: '2026-04-22T09:15:00Z', sender: { id: 'u-2' } },
    { id: 'm1', content: 'Question', createdAt: '2026-04-22T08:30:00Z', sender: { id: 'u-1' } },
  ],
};

describe('parseThreadDetail', () => {
  it('parses a well-formed ThreadDetailResponse', () => {
    const parsed = parseThreadDetail(validDetail);
    expect(parsed?.messages).toHaveLength(2);
    expect(parsed?.participants).toHaveLength(1);
  });

  it.each([
    ['null', null],
    ['a string', 'nope'],
    ['a number', 42],
  ])('returns null for an unexpected shape: %s', (_label, input) => {
    expect(parseThreadDetail(input)).toBeNull();
  });

  it('drops malformed messages', () => {
    const parsed = parseThreadDetail({
      messages: [{ id: 'm1', content: 'ok', createdAt: 'x' }, null, { id: 'no-content' }],
    });
    expect(parsed?.messages).toHaveLength(1);
  });
});

describe('threadTitle', () => {
  it('uses the single participant name', () => {
    expect(threadTitle(parseThreadDetail(validDetail), 'fallback')).toBe('Building Manager');
  });

  it('appends +N for group threads', () => {
    const detail = parseThreadDetail({
      participants: [
        { id: 'a', firstName: 'Ann' },
        { id: 'b', firstName: 'Bob' },
      ],
      messages: [],
    });
    expect(threadTitle(detail, 'fallback')).toBe('Ann +1');
  });

  it('falls back when there are no participants', () => {
    expect(threadTitle(parseThreadDetail({ messages: [] }), 'Conversation')).toBe('Conversation');
  });
});

describe('toUiMessages', () => {
  it('derives fromMe from the current user id and sorts oldest first', () => {
    const msgs = toUiMessages(parseThreadDetail(validDetail), 'u-1');
    expect(msgs.map((m) => m.id)).toEqual(['m1', 'm2']);
    expect(msgs[0].fromMe).toBe(true);
    expect(msgs[0].authorName).toBe('You');
    expect(msgs[1].fromMe).toBe(false);
  });

  it('treats every message as not-from-me when the user id is unknown', () => {
    const msgs = toUiMessages(parseThreadDetail(validDetail), undefined);
    expect(msgs.every((m) => !m.fromMe)).toBe(true);
  });
});
