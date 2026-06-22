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

// Minimal React hook emulation so the test can drive the hook's wiring without
// mounting the RN tree.
//
//  - `useEffect` runs synchronously, but only re-runs when its dependency array
//    changes — emulated with a per-call-site dep cache keyed by declaration
//    order so the "stable, runs once" wiring effect ([]) is exercised
//    faithfully across re-renders, and cleanups are collected for unmount.
//  - `useRef` returns a persistent box per call site (also keyed by declaration
//    order), so the latest-callback ref survives re-renders.
//
// All shared state lives on a single `mock`-prefixed object so the
// `jest.mock('react', …)` factory may legally reference it.
type EffectCleanup = () => void;
const mockReactState = {
  cleanups: [] as EffectCleanup[],
  effectIndex: 0,
  effectDeps: [] as (readonly unknown[] | undefined)[],
  effectCleanupByIndex: [] as (EffectCleanup | undefined)[],
  refIndex: 0,
  refBoxes: [] as { current: unknown }[],
  depsChanged(prev: readonly unknown[] | undefined, next: readonly unknown[] | undefined): boolean {
    if (prev === undefined || next === undefined) {
      return true;
    }
    if (prev.length !== next.length) {
      return true;
    }
    return prev.some((value, i) => !Object.is(value, next[i]));
  },
  // Reset per-render call-site cursors. Call between simulated renders.
  startRender(): void {
    this.effectIndex = 0;
    this.refIndex = 0;
  },
  reset(): void {
    this.cleanups.length = 0;
    this.effectDeps.length = 0;
    this.effectCleanupByIndex.length = 0;
    this.refBoxes.length = 0;
    this.startRender();
  },
};
// Back-compat alias used by the unmount test.
const mockCleanups = mockReactState.cleanups;
const startRender = () => mockReactState.startRender();

jest.mock('react', () => ({
  useEffect: (effect: () => undefined | EffectCleanup, deps?: readonly unknown[]) => {
    const index = mockReactState.effectIndex++;
    if (mockReactState.depsChanged(mockReactState.effectDeps[index], deps)) {
      mockReactState.effectCleanupByIndex[index]?.();
      mockReactState.effectDeps[index] = deps;
      const cleanup = effect();
      mockReactState.effectCleanupByIndex[index] =
        typeof cleanup === 'function' ? cleanup : undefined;
      if (typeof cleanup === 'function') {
        mockReactState.cleanups.push(cleanup);
      }
    }
  },
  useRef: (initial: unknown) => {
    const index = mockReactState.refIndex++;
    if (mockReactState.refBoxes[index] === undefined) {
      mockReactState.refBoxes[index] = { current: initial };
    }
    return mockReactState.refBoxes[index];
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
  mockReactState.reset();
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

    startRender();
    useDeepLinkRouting(onNavigate, true);
    expect(manager.setAuthenticated).toHaveBeenLastCalledWith(true);
  });

  it('does not re-register or re-initialize when onNavigate identity changes', () => {
    // App passes a memoised callback, but a new identity on re-render must not
    // re-run the wiring effect (regression: duplicate handlers / re-init).
    const firstOnNavigate = jest.fn();
    useDeepLinkRouting(firstOnNavigate, false);

    startRender();
    const secondOnNavigate = jest.fn();
    useDeepLinkRouting(secondOnNavigate, false);

    expect(manager.addHandler).toHaveBeenCalledTimes(1);
    expect(manager.initialize).toHaveBeenCalledTimes(1);

    // A link dispatched after the re-render routes through the LATEST callback.
    mockDeepLink.registeredHandler?.({
      success: true,
      screen: 'Documents',
      params: { id: 'doc-9' },
    } as ParsedDeepLink);

    expect(firstOnNavigate).not.toHaveBeenCalled();
    expect(secondOnNavigate).toHaveBeenCalledWith('DocumentDetail', { documentId: 'doc-9' });
  });

  it('swallows a rejected initialize() so it is not an unhandled rejection', async () => {
    const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});
    manager.initialize.mockRejectedValueOnce(new Error('cold-start boom'));

    const onNavigate = jest.fn();
    expect(() => useDeepLinkRouting(onNavigate, false)).not.toThrow();

    // Let the rejected promise's `.catch` settle.
    await Promise.resolve();
    expect(warnSpy).toHaveBeenCalled();
    warnSpy.mockRestore();
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
