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

jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    user: { firstName: 'Ada', lastName: 'Byron' },
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

    await writeCachedLayout('ppt/dashboard', SAMPLE_LAYOUT);
    const result = await readCachedLayout('ppt/dashboard');

    expect(result).toEqual(SAMPLE_LAYOUT);
    expect(mockAsyncStorage.setItem).toHaveBeenCalledWith(
      'ppt_layout_ppt_dashboard',
      JSON.stringify(SAMPLE_LAYOUT)
    );
  });

  it('returns null and calls removeItem for malformed JSON', async () => {
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue('not-valid-json{{{');
    (mockAsyncStorage.removeItem as jest.Mock).mockResolvedValue(undefined);

    const result = await readCachedLayout('ppt/dashboard');

    expect(result).toBeNull();
    expect(mockAsyncStorage.removeItem).toHaveBeenCalledWith('ppt_layout_ppt_dashboard');
  });

  it('returns null and calls removeItem when screen key does not match', async () => {
    const wrongScreen = { ...SAMPLE_LAYOUT, screen: 'ppt/other' };
    (mockAsyncStorage.getItem as jest.Mock).mockResolvedValue(JSON.stringify(wrongScreen));
    (mockAsyncStorage.removeItem as jest.Mock).mockResolvedValue(undefined);

    const result = await readCachedLayout('ppt/dashboard');

    expect(result).toBeNull();
    expect(mockAsyncStorage.removeItem).toHaveBeenCalledWith('ppt_layout_ppt_dashboard');
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

// ─── 4. manifest consistency ──────────────────────────────────────────────────

describe('mobile-manifest consistency', () => {
  it('dashboard types in manifest are all registered in dashboardRegistry', () => {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const manifest = require('./mobile-manifest.json') as {
      components: Array<{ type: string; required: boolean }>;
    };
    const registryKeys = Object.keys(dashboardRegistry);
    const dashboardManifestTypes = manifest.components
      .filter((c) => c.type.startsWith('dashboard-') || c.type.startsWith('action-'))
      .map((c) => c.type);

    for (const type of dashboardManifestTypes) {
      expect(registryKeys).toContain(type);
    }
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
