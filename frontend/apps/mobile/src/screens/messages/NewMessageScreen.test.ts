/** Pure helpers behind the new-message form (`POST /messages/threads`). */
import { buildStartThreadBody, selectableRecipients, startedThreadId } from './NewMessageScreen';

const neighbor = (user_id: string, is_visible = true) => ({
  user_id,
  display_name: `User ${user_id}`,
  unit_label: '1A',
  is_visible,
  email: null,
  phone: null,
});

describe('selectableRecipients', () => {
  it('keeps visible neighbours and drops the current user', () => {
    const list = [neighbor('me'), neighbor('a'), neighbor('hidden', false)] as never[];
    expect(selectableRecipients(list, 'me').map((n) => n.user_id)).toEqual(['a']);
    expect(selectableRecipients(undefined, 'me')).toEqual([]);
  });
});

describe('buildStartThreadBody', () => {
  it('sends the camelCase fields StartThreadRequest expects', () => {
    expect(buildStartThreadBody('a', '  Hello  ')).toEqual({
      recipientId: 'a',
      initialMessage: 'Hello',
    });
  });

  it('is null until a recipient and a non-blank message are present', () => {
    expect(buildStartThreadBody(undefined, 'Hello')).toBeNull();
    expect(buildStartThreadBody('a', '   ')).toBeNull();
  });
});

describe('startedThreadId', () => {
  it('reads thread.id from the response envelope', () => {
    expect(startedThreadId({ thread: { id: 't-1' }, messages: [] })).toBe('t-1');
    expect(startedThreadId({})).toBeUndefined();
    expect(startedThreadId(null)).toBeUndefined();
  });
});
