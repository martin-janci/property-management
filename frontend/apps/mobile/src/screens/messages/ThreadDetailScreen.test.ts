import {
  formatMessageTime,
  parseThreadDetail,
  threadTitle,
  toUiMessages,
} from './ThreadDetailScreen';

let warnSpy: jest.SpyInstance;
beforeEach(() => {
  warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
});
afterEach(() => {
  warnSpy.mockRestore();
});

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

  it('emits a dev-only warning for an unrecognized non-null payload', () => {
    parseThreadDetail('nope');
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('parseThreadDetail'));
  });

  it('emits a dev-only warning when every message row is dropped', () => {
    parseThreadDetail({ messages: [{ id: 'no-content' }] });
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('dropped all 1'));
  });

  it('stays silent for a null payload and an empty thread', () => {
    parseThreadDetail(null);
    parseThreadDetail({ participants: [], messages: [] });
    expect(warnSpy).not.toHaveBeenCalled();
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

  it('keeps a message with a malformed createdAt and sinks it to the end', () => {
    // `isApiMessage` only validates id + content, so a non-date createdAt
    // survives parsing. It must not break the sort (NaN comparator) — the
    // anomalous row sorts last while the valid rows keep chronological order.
    const detail = parseThreadDetail({
      messages: [
        { id: 'bad', content: 'no timestamp', createdAt: 'x' },
        { id: 'm2', content: 'Reply', createdAt: '2026-04-22T09:15:00Z' },
        { id: 'm1', content: 'Question', createdAt: '2026-04-22T08:30:00Z' },
      ],
    });
    const msgs = toUiMessages(detail, undefined);
    expect(msgs.map((m) => m.id)).toEqual(['m1', 'm2', 'bad']);
  });
});

describe('formatMessageTime', () => {
  it('formats a valid ISO timestamp to an hour:minute label', () => {
    // Non-empty and free of the "Invalid Date" sentinel.
    const label = formatMessageTime('2026-04-22T08:30:00Z');
    expect(label).not.toBe('');
    expect(label).not.toMatch(/Invalid Date/);
  });

  it.each([
    ['a non-date string', 'x'],
    ['an empty string', ''],
    ['garbage', 'not-a-date'],
  ])('returns an empty string rather than "Invalid Date" for %s', (_label, input) => {
    expect(formatMessageTime(input)).toBe('');
  });
});
