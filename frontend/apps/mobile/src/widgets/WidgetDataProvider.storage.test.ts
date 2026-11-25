/**
 * Regression tests for corrupt/absent widget storage (code-review finding:
 * unguarded `JSON.parse` in the widget storage read path).
 *
 * `getAllWidgetConfigs()` and the private `getAllCachedData()` (reached via
 * `getCachedWidgetData` / `cacheWidgetData`) both `JSON.parse` a value read
 * from AsyncStorage. If the stored blob is corrupt — truncated by a killed
 * write, mangled by a bad migration, or otherwise non-JSON — an unguarded
 * parse throws a `SyntaxError` that crashes the widget instead of degrading to
 * an empty state. These tests pin the guarded behaviour: a corrupt value must
 * be treated as "no data", never thrown.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import { WIDGET_CONFIG_KEY, WIDGET_DATA_KEY } from '../services/localCacheKeys';
import { WidgetDataProvider } from './WidgetDataProvider';

jest.mock('@react-native-async-storage/async-storage', () => {
  const store = new Map<string, string>();
  return {
    __store: store,
    getItem: jest.fn(async (k: string) => store.get(k) ?? null),
    setItem: jest.fn(async (k: string, v: string) => {
      store.set(k, v);
    }),
    removeItem: jest.fn(async (k: string) => {
      store.delete(k);
    }),
  };
});

const store = (AsyncStorage as unknown as { __store: Map<string, string> }).__store;

beforeEach(() => {
  store.clear();
  jest.clearAllMocks();
});

describe('WidgetDataProvider corrupt storage handling', () => {
  it('getAllWidgetConfigs returns [] when the stored config blob is corrupt', async () => {
    store.set(WIDGET_CONFIG_KEY, '{not valid json');
    const provider = new WidgetDataProvider('https://example.test');

    await expect(provider.getAllWidgetConfigs()).resolves.toEqual([]);
  });

  it('getAllWidgetConfigs returns [] when nothing is stored', async () => {
    const provider = new WidgetDataProvider('https://example.test');

    await expect(provider.getAllWidgetConfigs()).resolves.toEqual([]);
  });

  it('getCachedWidgetData returns null when the cached-data blob is corrupt', async () => {
    store.set(WIDGET_DATA_KEY, '}{broken');
    const provider = new WidgetDataProvider('https://example.test');

    await expect(provider.getCachedWidgetData('widget-1')).resolves.toBeNull();
  });

  it('cacheWidgetData recovers from a corrupt cached-data blob instead of throwing', async () => {
    store.set(WIDGET_DATA_KEY, 'not json at all');
    const provider = new WidgetDataProvider('https://example.test');

    await expect(
      provider.cacheWidgetData('widget-1', {
        type: 'quick_actions',
        actions: [],
      })
    ).resolves.toBeUndefined();

    // The corrupt blob is replaced with a well-formed one holding the new entry.
    const cached = await provider.getCachedWidgetData('widget-1');
    expect(cached).toEqual({ type: 'quick_actions', actions: [] });
  });

  it('round-trips valid config data unchanged', async () => {
    const provider = new WidgetDataProvider('https://example.test');
    await provider.saveWidgetConfig({
      id: 'w-1',
      type: 'notifications_count',
      size: 'small',
      refreshInterval: 15,
    });

    const configs = await provider.getAllWidgetConfigs();
    expect(configs).toEqual([
      { id: 'w-1', type: 'notifications_count', size: 'small', refreshInterval: 15 },
    ]);
  });
});
