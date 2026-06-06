// Wiring contract for `useDeepLinkRouting` — the deep-link glue extracted
// from `App.tsx` (refactor-mobile-app-tsx-churn). Behaviour-preserving:
// these assertions pin exactly what the two inlined `App.tsx` effects did so
// future regressions in the extracted hook are caught.
//
//  1. registers a handler with `deepLinkManager` and kicks off `initialize()`
//  2. a dispatched parsed link is mapped via `resolveDeepLinkTarget` and
//     forwarded to `onNavigate` (screen + params)
//  3. the auth gate (`setAuthenticated`) is kept in sync with `isAuthenticated`
//  4. the registered handler is unsubscribed on cleanup
//
// NOTE: we intentionally do NOT import `@testing-library/react-native`. The
// repo currently has a `react-test-renderer` 19.2.0-vs-19.2.6 peer-dep
// mismatch that fails every test importing that package (see the comment in
// `usePushNotifications.test.ts`). Instead we mock React's `useEffect` to run
// effects synchronously and collect their cleanups — enough to drive this
// hook's wiring without mounting the RN tree.

import { deepLinkManager, type ParsedDeepLink } from '../qrcode';
import { useDeepLinkRouting } from './useDeepLinkRouting';

const mockUnsubscribe = jest.fn();
const mockDeepLink = {
  registeredHandler: null as ((link: ParsedDeepLink) => void) | null,
};

jest.mock('../qrcode', () => ({
  deepLinkManager: {
    addHandler: jest.fn((handler: (link: ParsedDeepLink) => void) => {
      mockDeepLink.registeredHandler = handler;
      return mockUnsubscribe;
    }),
    initialize: jest.fn().mockResolvedValue(undefined),
    setAuthenticated: jest.fn(),
  },
}));

// Run every `useEffect` synchronously and collect cleanups so the test can
// simulate unmount. Effects run in declaration order, matching React.
type EffectCleanup = () => void;
const mockCleanups: EffectCleanup[] = [];
jest.mock('react', () => ({
  useEffect: (effect: () => undefined | EffectCleanup) => {
    const cleanup = effect();
    if (typeof cleanup === 'function') {
      mockCleanups.push(cleanup);
    }
  },
}));

const manager = deepLinkManager as unknown as {
  addHandler: jest.Mock;
  initialize: jest.Mock;
  setAuthenticated: jest.Mock;
};

beforeEach(() => {
  jest.clearAllMocks();
  mockDeepLink.registeredHandler = null;
  mockCleanups.length = 0;
});

describe('useDeepLinkRouting', () => {
  it('registers a handler and initializes the manager', () => {
    const onNavigate = jest.fn();
    useDeepLinkRouting(onNavigate, false);

    expect(manager.addHandler).toHaveBeenCalledTimes(1);
    expect(manager.initialize).toHaveBeenCalledTimes(1);
  });

  it('routes a dispatched link through resolveDeepLinkTarget to onNavigate', () => {
    const onNavigate = jest.fn();
    useDeepLinkRouting(onNavigate, true);

    // A Documents link carrying an id resolves to the DocumentDetail screen.
    mockDeepLink.registeredHandler?.({
      success: true,
      screen: 'Documents',
      params: { id: 'doc-7' },
    } as ParsedDeepLink);

    expect(onNavigate).toHaveBeenCalledWith('DocumentDetail', { documentId: 'doc-7' });
  });

  it('ignores links that resolve to no target', () => {
    const onNavigate = jest.fn();
    useDeepLinkRouting(onNavigate, true);

    mockDeepLink.registeredHandler?.({ success: false } as ParsedDeepLink);

    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('keeps the auth gate in sync with isAuthenticated', () => {
    const onNavigate = jest.fn();

    useDeepLinkRouting(onNavigate, false);
    expect(manager.setAuthenticated).toHaveBeenLastCalledWith(false);

    useDeepLinkRouting(onNavigate, true);
    expect(manager.setAuthenticated).toHaveBeenLastCalledWith(true);
  });

  it('unsubscribes the handler on cleanup', () => {
    const onNavigate = jest.fn();
    useDeepLinkRouting(onNavigate, false);

    // Simulate unmount: run the effect cleanups React would invoke.
    for (const cleanup of mockCleanups) {
      cleanup();
    }

    expect(mockUnsubscribe).toHaveBeenCalledTimes(1);
  });
});
