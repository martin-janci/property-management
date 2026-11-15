/**
 * Regression guard: corrupt / legacy AsyncStorage payloads must not crash the
 * feedback store.
 *
 * `FeedbackManager.getDrafts()` and the private `getPendingFeedback()` (reached
 * through `getPendingFeedbackCount()` / `retryPendingFeedback()`) read JSON out
 * of AsyncStorage. A truncated write, a schema change, or hand-edited debug
 * data can leave a non-JSON string under `@ppt/feedback_drafts` /
 * `@ppt/pending_feedback`. An unguarded `JSON.parse` there throws a
 * `SyntaxError` that propagates up through every caller — saving a draft,
 * counting pending items, retrying submissions — effectively bricking the
 * feature until the app's storage is cleared.
 *
 * These tests assert the reads degrade to a sane empty default instead of
 * throwing. They fail before the try/catch guard is added (IG3).
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import { FeedbackManager } from './FeedbackManager';

const FEEDBACK_DRAFTS_KEY = '@ppt/feedback_drafts';
const PENDING_FEEDBACK_KEY = '@ppt/pending_feedback';

const getItem = jest.mocked(AsyncStorage.getItem);
const setItem = jest.mocked(AsyncStorage.setItem);
const removeItem = jest.mocked(AsyncStorage.removeItem);

function makeManager(): FeedbackManager {
  return new FeedbackManager('https://api.test.local', '1.0.0', '100');
}

beforeEach(() => {
  getItem.mockReset();
  setItem.mockReset();
  removeItem.mockReset();
  setItem.mockResolvedValue(undefined);
  removeItem.mockResolvedValue(undefined);
});

describe('FeedbackManager corrupt-storage resilience', () => {
  it('getDrafts returns [] when the stored drafts blob is not valid JSON', async () => {
    getItem.mockResolvedValue('{not json at all');

    const manager = makeManager();
    await expect(manager.getDrafts()).resolves.toEqual([]);
  });

  it('getPendingFeedbackCount returns 0 when the pending blob is corrupt', async () => {
    getItem.mockImplementation(async (key: string) =>
      key === PENDING_FEEDBACK_KEY ? '}{ broken' : null
    );

    const manager = makeManager();
    await expect(manager.getPendingFeedbackCount()).resolves.toBe(0);
  });

  it('saveDraft recovers from a corrupt existing draft list and persists a fresh one', async () => {
    getItem.mockResolvedValue('<<legacy binary>>');

    const manager = makeManager();
    await expect(
      manager.saveDraft({ type: 'bug', title: 'T', description: 'D' })
    ).resolves.toBeUndefined();

    // The write still happens, against a recovered empty base list.
    expect(setItem).toHaveBeenCalledWith(FEEDBACK_DRAFTS_KEY, expect.any(String));
    const written = JSON.parse(setItem.mock.calls[0][1]);
    expect(Array.isArray(written)).toBe(true);
    expect(written).toHaveLength(1);
    expect(written[0]).toMatchObject({ type: 'bug', title: 'T' });
  });

  it('still parses well-formed stored drafts', async () => {
    getItem.mockResolvedValue(JSON.stringify([{ title: 'kept', savedAt: 'x' }]));

    const manager = makeManager();
    await expect(manager.getDrafts()).resolves.toEqual([{ title: 'kept', savedAt: 'x' }]);
  });
});

describe('FeedbackManager auth-token header', () => {
  const fetchMock = jest.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({ id: 'fb-1' }),
    } as Response);
    globalThis.fetch = fetchMock as unknown as typeof fetch;
  });

  function headersOf(call: unknown): Record<string, string> {
    const init = (call as [string, { headers?: Record<string, string> }])[1];
    return init?.headers ?? {};
  }

  it('omits the Authorization header until a token is set', async () => {
    const manager = makeManager();

    await manager.submitFeedback({ type: 'bug', title: 'T', description: 'D' });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe('https://api.test.local/api/v1/feedback');
    expect(headersOf(fetchMock.mock.calls[0]).Authorization).toBeUndefined();
  });

  it('sends Bearer <token> once setAuthToken is called', async () => {
    const manager = makeManager();
    manager.setAuthToken('xyz');

    await manager.submitFeedback({ type: 'bug', title: 'T', description: 'D' });

    expect(headersOf(fetchMock.mock.calls[0]).Authorization).toBe('Bearer xyz');
  });

  it('drops the Authorization header again when the token is cleared', async () => {
    const manager = makeManager();
    manager.setAuthToken('xyz');
    manager.setAuthToken(null);

    await manager.submitFeedback({ type: 'bug', title: 'T', description: 'D' });

    expect(headersOf(fetchMock.mock.calls[0]).Authorization).toBeUndefined();
  });
});
