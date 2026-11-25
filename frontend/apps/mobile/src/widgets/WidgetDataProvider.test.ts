/**
 * Regression tests for WidgetDataProvider's HTTP error handling.
 *
 * The private `apiRequest` helper used to swallow every failure: on a non-OK
 * response or a thrown fetch, it logged only under `__DEV__` and then returned
 * `{} as T`. That silently defeated the cache-fallback path in
 * `updateAllWidgets` — a failed fetch looked like a *successful* fetch of empty
 * data, so the provider cached and surfaced zeros instead of the last-known
 * good values, and callers had no way to distinguish an outage from genuinely
 * empty data. See code-review finding
 * `code-review-mobile-rn-widget-apirequest-swallows-errors`.
 *
 * These tests pin that the error is now surfaced (propagated) so the existing
 * fallback-to-cache logic actually runs.
 *
 * `@react-native-async-storage/async-storage` is mocked globally in
 * `src/test/setup.ts`; `fetch` is stubbed here.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import { WIDGET_CONFIG_KEY, WIDGET_DATA_KEY } from '../services/localCacheKeys';
import type { NotificationsWidgetData, WidgetConfig } from './types';
import { WidgetDataProvider } from './WidgetDataProvider';

const mockedGetItem = AsyncStorage.getItem as jest.Mock;
const mockedSetItem = AsyncStorage.setItem as jest.Mock;

/** Build a minimal `Response`-like object for the fetch mock. */
function fakeResponse(opts: { status?: number; ok?: boolean; json?: () => Promise<unknown> }) {
  const status = opts.status ?? 200;
  return {
    ok: opts.ok ?? (status >= 200 && status < 300),
    status,
    json: opts.json ?? (async () => ({})),
  };
}

const NOTIFICATIONS_CONFIG: WidgetConfig = {
  id: 'widget-notifications-1',
  type: 'notifications_count',
  size: 'small',
  refreshInterval: 15,
};

describe('WidgetDataProvider error handling', () => {
  let provider: WidgetDataProvider;
  let fetchMock: jest.Mock;
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    jest.clearAllMocks();
    provider = new WidgetDataProvider('https://api.mocked');
    originalFetch = globalThis.fetch;
    fetchMock = jest.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('propagates a non-OK API response instead of swallowing it', async () => {
    fetchMock.mockResolvedValueOnce(fakeResponse({ status: 503, ok: false }));

    await expect(provider.fetchWidgetData(NOTIFICATIONS_CONFIG)).rejects.toThrow(
      /API request failed: 503/
    );
  });

  it('propagates a network/fetch rejection instead of swallowing it', async () => {
    fetchMock.mockRejectedValueOnce(new Error('network down'));

    await expect(provider.fetchWidgetData(NOTIFICATIONS_CONFIG)).rejects.toThrow('network down');
  });

  it('falls back to cached data when a widget fetch fails', async () => {
    const cachedData: NotificationsWidgetData = {
      type: 'notifications_count',
      unreadCount: 7,
      categories: { announcements: 3, faults: 1, votes: 2, documents: 1, messages: 0 },
    };

    // getAllWidgetConfigs() reads the config key; getCachedWidgetData() reads
    // the data key. Route each AsyncStorage read to the right fixture.
    mockedGetItem.mockImplementation(async (key: string) => {
      if (key === WIDGET_CONFIG_KEY) {
        return JSON.stringify([NOTIFICATIONS_CONFIG]);
      }
      if (key === WIDGET_DATA_KEY) {
        return JSON.stringify({
          [NOTIFICATIONS_CONFIG.id]: {
            data: cachedData,
            cachedAt: new Date().toISOString(),
          },
        });
      }
      return null;
    });
    mockedSetItem.mockResolvedValue(undefined);

    fetchMock.mockResolvedValueOnce(fakeResponse({ status: 500, ok: false }));

    const results = await provider.updateAllWidgets();

    // Before the fix, apiRequest returned `{}` so the fetch "succeeded" with
    // zeroed data — the provider surfaced unreadCount: 0, not the cache.
    expect(results.get(NOTIFICATIONS_CONFIG.id)).toEqual(cachedData);
  });
});
