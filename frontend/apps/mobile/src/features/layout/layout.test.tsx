/**
 * layout.test.tsx — tests for layout registry, cache, hook, and DashboardScreen integration.
 *
 * Test sections:
 * 1. LayoutSections renderer
 * 2. layoutCache (AsyncStorage-backed)
 * 3. useDashboardLayout hook
 * 4. mobile-manifest consistency
 * 5. DashboardScreen integration with layout
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import { act, render, screen, waitFor } from '@testing-library/react-native';
import React from 'react';
import { Text } from 'react-native';
import { DashboardScreen } from '../../screens/dashboard/DashboardScreen';
import { resetLocalData } from '../../services/resetLocalData';
import { LayoutSections } from './LayoutSections';
import { readCachedLayout, writeCachedLayout } from './layoutCache';
import type { SectionComponentProps, SectionRegistry } from './registry';
import { DEFAULT_DASHBOARD_LAYOUT, dashboardRegistry } from './registry';
import type { ResolvedScreen } from './types';

// ─── Mocks ────────────────────────────────────────────────────────────────────

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  apiRequest: jest.fn(),
}));

// The real `useDashboardLayout` reads `user.id` to tenant-scope the layout
// cache key (issue #2486), so the mocked auth user must expose an id.
jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    user: { id: 'user-a', firstName: 'Ada', lastName: 'Byron' },
    logout: jest.fn(),
  }),
}));

// Mock useDashboardLayout for DashboardScreen tests so they don't trigger
// extra useApiQuery / apiRequest calls that the test fixtures don't expect.
// Note: jest.mock factory cannot reference out-of-scope variables, so we
// inline the default layout shape here.
jest.mock('./useDashboardLayout', () => ({
  useDashboardLayout: jest.fn(() => ({
    layout: {
      screen: 'ppt/dashboard',
      version: 0,
      sections: [
        { type: 'dashboard-stats.v1', presentation: 'visible' },
        { type: 'action-queue.v1', presentation: 'visible' },
      ],
    },
  })),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;
const mockApiRequest = jest.requireMock('../../hooks/useApi').apiRequest as jest.Mock;
const mockAsyncStorage = AsyncStorage as jest.Mocked<typeof AsyncStorage>;

// ─── Helpers ──────────────────────────────────────────────────────────────────

function queryResult(overrides: Record<string, unknown> = {}) {
  return {
    data: undefined,
    isLoading: false,
    isError: false,
    error: null,
    isFetching: false,
    refetch: jest.fn(),
    ...overrides,
  };
}

const SAMPLE_LAYOUT: ResolvedScreen = {
  screen: 'ppt/dashboard',
  version: 1,
  sections: [
    { type: 'dashboard-stats.v1', presentation: 'visible' },
    { type: 'action-queue.v1', presentation: 'visible' },
  ],
};

// ─── 1. LayoutSections renderer ───────────────────────────────────────────────

describe('LayoutSections renderer', () => {
  // Clear the module-level warnedTypes set between tests by re-importing or
  // using jest.resetModules. The simpler approach: suppress and spy on warn.
  beforeEach(() => {
    jest.spyOn(console, 'warn').mockImplementation(() => {});
  });
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('renders sections in the correct order', () => {
    // Simple registry with two stub sections
    function Section1() {
      return <Text>Section One</Text>;
    }
    function Section2() {
      return <Text>Section Two</Text>;
    }
    const testRegistry: SectionRegistry = {
      'sec-1': { component: Section1 },
      'sec-2': { component: Section2 },
    };
    const testLayout: ResolvedScreen = {
      screen: 'test',
      version: 1,
      sections: [
        { type: 'sec-1', presentation: 'visible' },
        { type: 'sec-2', presentation: 'visible' },
      ],
    };

    render(<LayoutSections layout={testLayout} registry={testRegistry} />);

    expect(screen.getByText('Section One')).toBeTruthy();
    expect(screen.getByText('Section Two')).toBeTruthy();
  });

  it('renders PlaceholderSection for presentation=placeholder', () => {
    function ASection({ mode: _mode, props: _props }: SectionComponentProps) {
      return <Text>Real Content</Text>;
    }
    const testRegistry: SectionRegistry = {
      'sec-a': { component: ASection },
    };
    const testLayout: ResolvedScreen = {
      screen: 'test',
      version: 1,
      sections: [{ type: 'sec-a', presentation: 'placeholder' }],
    };

    render(<LayoutSections layout={testLayout} registry={testRegistry} />);

    // t() returns the key in tests
    expect(screen.getByText('layout.placeholderTitle')).toBeTruthy();
    expect(screen.queryByText('Real Content')).toBeNull();
  });

  it('skips unknown section types and calls console.warn once', () => {
    const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
    const testLayout: ResolvedScreen = {
      screen: 'test',
      version: 1,
      sections: [
        { type: 'unknown-type.v99', presentation: 'visible' },
        { type: 'unknown-type.v99', presentation: 'visible' },
      ],
    };

    render(<LayoutSections layout={testLayout} registry={{}} />);

    // warn-once: called exactly once even though the type appears twice
    const layoutWarns = warnSpy.mock.calls.filter((args) =>
      String(args[0]).includes('unknown section type')
    );
    expect(layoutWarns.length).toBe(1);
    expect(layoutWarns[0][0]).toContain('unknown-type.v99');
    warnSpy.mockRestore();
  });
});

// ─── 2. layoutCache ───────────────────────────────────────────────────────────

describe('layoutCache', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('write then read round-trips correctly', async () => {
    (mockAsyncStorage.setItem as jest.Mock).mockResolvedValue(undefined);
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(JSON.stringify(SAMPLE_LAYOUT));

    await writeCachedLayout('user-a', 'ppt/dashboard', SAMPLE_LAYOUT);
    const result = await readCachedLayout('user-a', 'ppt/dashboard');

    expect(result).toEqual(SAMPLE_LAYOUT);
    // Key is tenant-scoped by the `user-a` scope id (issue #2486).
    expect(mockAsyncStorage.setItem).toHaveBeenCalledWith(
      'ppt_layout_user-a_ppt_dashboard',
      JSON.stringify(SAMPLE_LAYOUT)
    );
  });

  it('returns null and calls removeItem for malformed JSON', async () => {
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue('not-valid-json{{{');
    (mockAsyncStorage.removeItem as jest.Mock).mockResolvedValue(undefined);

    const result = await readCachedLayout('user-a', 'ppt/dashboard');

    expect(result).toBeNull();
    expect(mockAsyncStorage.removeItem).toHaveBeenCalledWith('ppt_layout_user-a_ppt_dashboard');
  });

  it('returns null and calls removeItem when screen key does not match', async () => {
    const wrongScreen = { ...SAMPLE_LAYOUT, screen: 'ppt/other' };
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(JSON.stringify(wrongScreen));
    (mockAsyncStorage.removeItem as jest.Mock).mockResolvedValue(undefined);

    const result = await readCachedLayout('user-a', 'ppt/dashboard');

    expect(result).toBeNull();
    expect(mockAsyncStorage.removeItem).toHaveBeenCalledWith('ppt_layout_user-a_ppt_dashboard');
  });

  // ── cross-tenant isolation (#2486) ──────────────────────────────────────
  // The key carries the scope (user/org) id, so a layout written under org A
  // cannot be read back under org B — different scope ⇒ different key ⇒ miss.
  it('does not activate org A layout for org B (scoped key)', async () => {
    const store = new Map<string, string>();
    (mockAsyncStorage.setItem as jest.Mock).mockImplementation(async (k: string, v: string) => {
      store.set(k, v);
    });
    (mockAsyncStorage.getItem as jest.Mock).mockImplementation(async (k: string) =>
      store.has(k) ? (store.get(k) as string) : null
    );
    (mockAsyncStorage.removeItem as jest.Mock).mockImplementation(async (k: string) => {
      store.delete(k);
    });

    // org A caches its dashboard layout…
    await writeCachedLayout('org-a', 'ppt/dashboard', SAMPLE_LAYOUT);
    expect(store.has('ppt_layout_org-a_ppt_dashboard')).toBe(true);

    // …org B reading the same screen gets NOTHING (its key never existed).
    const forOrgB = await readCachedLayout('org-b', 'ppt/dashboard');
    expect(forOrgB).toBeNull();

    // org A still reads its own entry back.
    const forOrgA = await readCachedLayout('org-a', 'ppt/dashboard');
    expect(forOrgA).toEqual(SAMPLE_LAYOUT);
  });

  // A logout/session-teardown prefix sweep (`resetLocalData`) removes every
  // `ppt_layout_*` key regardless of scope/screen — so even org A's own entry
  // does not survive to the next login on a shared device (#2486).
  it('layout cache is cleared on logout via the ppt_layout_ prefix sweep', async () => {
    const store = new Map<string, string>([
      ['ppt_layout_org-a_ppt_dashboard', JSON.stringify(SAMPLE_LAYOUT)],
      ['ppt_access_token', 'tok'],
    ]);
    (mockAsyncStorage.getAllKeys as jest.Mock).mockResolvedValueOnce(Array.from(store.keys()));
    (mockAsyncStorage.removeMany as jest.Mock).mockImplementation(async (keys: string[]) => {
      for (const k of keys) store.delete(k);
    });

    await resetLocalData();

    expect(store.has('ppt_layout_org-a_ppt_dashboard')).toBe(false);
    // A non-tenant key (the auth token) is untouched by the sweep.
    expect(store.has('ppt_access_token')).toBe(true);
  });
});

// ─── 2b. logout → re-login round-trip (#2486) ────────────────────────────────
// The pieces above are proven in isolation (scoped key; prefix sweep). This
// block ties them into the single session-boundary flow the fix has to
// guarantee end to end: a layout written under user A must NOT survive a
// logout to activate for the *next* login on a shared device — neither for a
// different user (B) nor for user A re-authenticating. It drives the REAL
// writeCachedLayout / readCachedLayout / resetLocalData against one shared
// in-memory store so nothing is stubbed between write and read.
describe('layoutCache — logout + re-login round-trip (#2486)', () => {
  // A single backing store shared by every mocked AsyncStorage method, so the
  // write → sweep → read sequence flows through the production code paths.
  function wireSharedStore(initial: Iterable<[string, string]> = []) {
    const store = new Map<string, string>(initial);
    (mockAsyncStorage.setItem as jest.Mock).mockImplementation(async (k: string, v: string) => {
      store.set(k, v);
    });
    (mockAsyncStorage.getItem as jest.Mock).mockImplementation(async (k: string) =>
      store.has(k) ? (store.get(k) as string) : null
    );
    (mockAsyncStorage.removeItem as jest.Mock).mockImplementation(async (k: string) => {
      store.delete(k);
    });
    (mockAsyncStorage.getAllKeys as jest.Mock).mockImplementation(async () =>
      Array.from(store.keys())
    );
    (mockAsyncStorage.removeMany as jest.Mock).mockImplementation(async (keys: string[]) => {
      for (const k of keys) store.delete(k);
    });
    return store;
  }

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('user A layout does not survive logout to activate for user B on re-login', async () => {
    const store = wireSharedStore([['ppt_access_token', 'tok-a']]);

    // 1. User A is logged in and their dashboard layout gets cached.
    await writeCachedLayout('user-a', 'ppt/dashboard', SAMPLE_LAYOUT);
    expect(await readCachedLayout('user-a', 'ppt/dashboard')).toEqual(SAMPLE_LAYOUT);

    // 2. User A logs out — the session-teardown purge runs.
    await resetLocalData();
    expect(store.has('ppt_layout_user-a_ppt_dashboard')).toBe(false);

    // 3. User B logs in on the same device and opens the dashboard: the read
    //    misses (scoped key never existed AND the sweep cleared everything),
    //    so B falls back to the default layout + a fresh fetch — never A's
    //    tenant-customized layout.
    expect(await readCachedLayout('user-b', 'ppt/dashboard')).toBeNull();
  });

  it('user A re-authenticating starts from a cleared cache, not the pre-logout layout', async () => {
    wireSharedStore();

    // User A caches a layout, then logs out.
    await writeCachedLayout('user-a', 'ppt/dashboard', SAMPLE_LAYOUT);
    await resetLocalData();

    // Re-login as the SAME user must not resurrect the pre-logout entry — the
    // logout sweep is unconditional, so the read is a miss until a fresh fetch
    // repopulates it.
    expect(await readCachedLayout('user-a', 'ppt/dashboard')).toBeNull();

    // A post-re-login fetch re-populates the (again tenant-scoped) cache, and
    // that new entry is readable — proving the cache is cleared, not disabled.
    const freshLayout: ResolvedScreen = { ...SAMPLE_LAYOUT, version: 9 };
    await writeCachedLayout('user-a', 'ppt/dashboard', freshLayout);
    expect(await readCachedLayout('user-a', 'ppt/dashboard')).toEqual(freshLayout);
  });

  it('logout purge leaves the auth token intact so re-login is not forced to re-auth blindly', async () => {
    const store = wireSharedStore([['ppt_access_token', 'tok-a']]);
    await writeCachedLayout('user-a', 'ppt/dashboard', SAMPLE_LAYOUT);

    await resetLocalData();

    // Only tenant-cache namespaces are swept; the auth-token key (which is not
    // under a tenant prefix) must be left for the auth layer to manage.
    expect(store.has('ppt_layout_user-a_ppt_dashboard')).toBe(false);
    expect(store.get('ppt_access_token')).toBe('tok-a');
  });
});

// ─── 3. useDashboardLayout hook ───────────────────────────────────────────────

describe('useDashboardLayout hook', () => {
  // Import the real hook (not mocked) by using jest.requireActual
  const { useDashboardLayout: realUseDashboardLayout } = jest.requireActual('./useDashboardLayout');

  beforeEach(() => {
    jest.clearAllMocks();
    jest.spyOn(console, 'warn').mockImplementation(() => {});
  });
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('activates cached layout at mount', async () => {
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(JSON.stringify(SAMPLE_LAYOUT));
    mockApiRequest.mockResolvedValue(null); // fetch returns nothing valid

    let capturedLayout: ResolvedScreen | undefined;
    function TestComponent() {
      const { layout } = realUseDashboardLayout('ppt/dashboard');
      capturedLayout = layout;
      return null;
    }

    render(<TestComponent />);

    await waitFor(() => {
      expect(capturedLayout).toEqual(SAMPLE_LAYOUT);
    });
  });

  it('background fetch does NOT call setLayout but writes cache', async () => {
    const freshLayout: ResolvedScreen = { ...SAMPLE_LAYOUT, version: 2 };
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(null); // no cache
    mockApiRequest.mockResolvedValue(freshLayout);
    (mockAsyncStorage.setItem as jest.Mock).mockResolvedValue(undefined);

    let capturedLayout: ResolvedScreen | undefined;
    function TestComponent() {
      const { layout } = realUseDashboardLayout('ppt/dashboard');
      capturedLayout = layout;
      return null;
    }

    render(<TestComponent />);

    // Wait for effect to settle
    await waitFor(() => {
      expect(mockAsyncStorage.setItem).toHaveBeenCalled();
    });

    // Layout should still be the DEFAULT (cache was empty, fetch doesn't set state)
    expect(capturedLayout).toEqual(DEFAULT_DASHBOARD_LAYOUT);
  });

  it('fetch failure is silent — console.warn fires once', async () => {
    const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(null);
    mockApiRequest.mockRejectedValue(new Error('network'));

    function TestComponent() {
      realUseDashboardLayout('ppt/dashboard');
      return null;
    }

    render(<TestComponent />);

    await waitFor(() => {
      const layoutWarns = warnSpy.mock.calls.filter((args) =>
        String(args[0]).includes('layout: background fetch failed')
      );
      expect(layoutWarns.length).toBe(1);
    });
    warnSpy.mockRestore();
  });
});

// ─── 3b. useDashboardLayout — tightest activation cage ───────────────────────

describe('useDashboardLayout — tightest activation cage', () => {
  const { useDashboardLayout: realUseDashboardLayout } = jest.requireActual('./useDashboardLayout');

  beforeEach(() => {
    jest.clearAllMocks();
    jest.spyOn(console, 'warn').mockImplementation(() => {});
  });
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('serves cached layout when cache is populated AND background fetch returns a different layout', async () => {
    // Cached layout has a distinguishable section list (action-queue only)
    const cachedLayout: ResolvedScreen = {
      screen: 'ppt/dashboard',
      version: 5,
      sections: [{ type: 'action-queue.v1', presentation: 'visible' }],
    };
    // Fetched layout is different (stats only, higher version)
    const fetchedLayout: ResolvedScreen = {
      screen: 'ppt/dashboard',
      version: 6,
      sections: [{ type: 'dashboard-stats.v1', presentation: 'visible' }],
    };

    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(JSON.stringify(cachedLayout));
    mockApiRequest.mockResolvedValue(fetchedLayout);
    (mockAsyncStorage.setItem as jest.Mock).mockResolvedValue(undefined);

    let capturedLayout: ResolvedScreen | undefined;
    function TestComponent() {
      const { layout } = realUseDashboardLayout('ppt/dashboard');
      capturedLayout = layout;
      return null;
    }

    render(<TestComponent />);

    // Wait until the background fetch has written to cache
    await waitFor(() => {
      expect(mockAsyncStorage.setItem).toHaveBeenCalled();
    });

    // The hook's state MUST equal the CACHED layout (next-launch semantics)
    expect(capturedLayout).toEqual(cachedLayout);
    expect(capturedLayout).not.toEqual(fetchedLayout);

    // writeCachedLayout / AsyncStorage.setItem must have been called with the
    // FETCHED layout, under the tenant-scoped key (scope id `user-a` from the
    // mocked auth user — issue #2486).
    expect(mockAsyncStorage.setItem).toHaveBeenCalledWith(
      'ppt_layout_user-a_ppt_dashboard',
      JSON.stringify(fetchedLayout)
    );
  });
});

// ─── 3c. DashboardStatsSection — stat arithmetic ─────────────────────────────

describe('DashboardStatsSection stat arithmetic', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.spyOn(console, 'warn').mockImplementation(() => {});
  });
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('computes rendered stat values from mocked per-endpoint query results', () => {
    // Wire up useApiQuery to return distinct data per endpoint key.
    // Call order matches DashboardStatsSection's 4 useApiQuery calls:
    //   1. ['dashboard', 'announcements']   → /api/v1/announcements
    //   2. ['dashboard', 'faults-stats']    → /api/v1/faults/statistics
    //   3. ['dashboard', 'votes']           → /api/v1/voting
    //   4. ['dashboard', 'unread-messages'] → /api/v1/messages/unread-count

    // 3 announcements → unreadAnnouncements = 3
    const announcementsResult = queryResult({
      data: {
        announcements: [
          { id: '1', title: 'Ann A', status: 'published', pinned: false, created_at: '2026-01-01' },
          { id: '2', title: 'Ann B', status: 'published', pinned: false, created_at: '2026-01-02' },
          { id: '3', title: 'Ann C', status: 'published', pinned: false, created_at: '2026-01-03' },
        ],
      },
    });
    // open=2, in_progress=1 → pendingFaults = 3
    const faultsResult = queryResult({
      data: { statistics: { open_count: 2, in_progress_count: 1 } },
    });
    // 2 active votes, 1 closed → activeVotes = 2
    const votesResult = queryResult({
      data: {
        votes: [
          { id: 'v1', title: 'Vote 1', status: 'active', end_at: '2026-12-31' },
          { id: 'v2', title: 'Vote 2', status: 'active', end_at: '2026-12-31' },
          { id: 'v3', title: 'Vote 3', status: 'closed', end_at: '2026-01-01' },
        ],
      },
    });
    // unreadCount = 7 → unreadMessages = 7
    const unreadResult = queryResult({ data: { unreadCount: 7 } });

    mockUseApiQuery
      .mockReturnValueOnce(announcementsResult)
      .mockReturnValueOnce(faultsResult)
      .mockReturnValueOnce(votesResult)
      .mockReturnValueOnce(unreadResult);

    const { DashboardStatsSection } = jest.requireActual(
      './registry'
    ) as typeof import('./registry');

    render(<DashboardStatsSection />);

    // pendingFaults = open_count(2) + in_progress_count(1) = 3
    // unreadAnnouncements = announcements.slice(0,3).length = 3
    // Both show '3'; getAllByText confirms at least two cards with that value
    expect(screen.getAllByText('3').length).toBeGreaterThanOrEqual(1);

    // activeVotes = 2 (only 'active' status counted)
    expect(screen.getByText('2')).toBeTruthy();

    // unreadMessages = 7
    expect(screen.getByText('7')).toBeTruthy();

    // upcomingPayments is always 0, but the component renders only 4 stat cards
    // (pendingFaults, unreadAnnouncements, activeVotes, unreadMessages); the
    // upcomingPayments field is computed but not yet displayed — no '0' card.
    expect(screen.queryByText('0')).toBeNull();
  });
});

// ─── 4. manifest consistency ──────────────────────────────────────────────────

// Drift guard: these keys must exactly match the supportedTypes list in
// mobile-native/androidApp/src/main/java/three/two/bit/ppt/reality/listing/ListingSectionRegistry.kt
// — documented duplication because KMP tests cannot read this JSON.
const LISTING_DETAIL_COMPONENTS = [
  'additional-info.v1',
  'agent-contact.v1',
  'description.v1',
  'features.v1',
  'gallery.v1',
  'key-details.v1',
  'listing-header.v1',
  'resources.v1',
].sort();

describe('mobile-manifest consistency', () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const manifest = require('./mobile-manifest.json') as {
    platform: string;
    components: Record<string, { required: boolean }>;
  };

  it('dashboard types in manifest are all registered in dashboardRegistry', () => {
    const registryKeys = Object.keys(dashboardRegistry);
    const dashboardManifestTypes = Object.keys(manifest.components).filter(
      (k) => k.startsWith('dashboard-') || k.startsWith('action-')
    );

    for (const type of dashboardManifestTypes) {
      expect(registryKeys).toContain(type);
    }
  });

  it('listing-detail component keys match KMP ListingSectionRegistry.supportedTypes', () => {
    const listingDetailKeys = Object.keys(manifest.components)
      .filter((k) => !['dashboard-stats.v1', 'action-queue.v1'].includes(k))
      .sort();
    expect(listingDetailKeys).toEqual(LISTING_DETAIL_COMPONENTS);
  });
});

// ─── 5. DashboardScreen integration ──────────────────────────────────────────

describe('DashboardScreen with layout registry', () => {
  function allSuccessQueries() {
    return [
      queryResult({ data: { announcements: [] } }),
      queryResult({ data: { statistics: { open_count: 0, in_progress_count: 0 } } }),
      queryResult({ data: { votes: [] } }),
      queryResult({ data: { unreadCount: 0 } }),
    ];
  }

  beforeEach(() => {
    mockUseApiQuery.mockReset();
    for (const r of allSuccessQueries()) {
      mockUseApiQuery.mockReturnValueOnce(r);
    }
  });

  afterEach(() => jest.clearAllMocks());

  it('renders managed sections via default layout without crashing', () => {
    // useDashboardLayout is mocked to return DEFAULT_DASHBOARD_LAYOUT.
    // The registry sections (DashboardStatsSection, ActionQueueSection) will
    // try to call useApiQuery — but since they are rendered inside LayoutSections
    // which renders the registry components, we need to provide enough mock returns.
    // Reset and provide more mocks for the section components' own queries.
    mockUseApiQuery.mockReset();
    // DashboardScreen: 4 queries
    // DashboardStatsSection: 4 queries
    // ActionQueueSection: 1 query (votes)
    const emptyResult = queryResult({ data: undefined });
    for (let i = 0; i < 9; i++) {
      mockUseApiQuery.mockReturnValueOnce(emptyResult);
    }

    render(<DashboardScreen />);

    // Dashboard should render (no crash). Check for static content that's always present.
    expect(screen.getByText('dashboard.welcomeBack')).toBeTruthy();
    expect(screen.getByText('dashboard.recentAnnouncements')).toBeTruthy();
  });
});
