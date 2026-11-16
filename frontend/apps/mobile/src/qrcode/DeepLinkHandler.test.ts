// Regression for the code-review finding "DeepLinkManager.initialize() discards
// the Linking.addEventListener subscription": the discarded subscription leaks
// and keeps firing, so a second initialize() would stack a second 'url'
// listener and double-dispatch every incoming deep link. These tests pin that
// initialize() is idempotent (never stacks listeners) and that cleanup()
// removes the listener.
import { Linking } from 'react-native';
import { DeepLinkManager } from './DeepLinkHandler';

// `expo-constants` (pulled in transitively via universalLinks.ts) has no
// config in the test env; the placeholder host fallback is enough here.
jest.mock('expo-constants', () => ({ expoConfig: { extra: {} } }));

type UrlListener = (event: { url: string }) => void;
interface Subscription {
  remove: jest.Mock;
}

// Model react-native's `Linking` like the real emitter: `addEventListener`
// registers a listener and returns a subscription whose `.remove()` truly
// deregisters it. `emitUrl` then fans an event out to exactly the listeners
// still active — so a leaked subscription is observable as a double dispatch.
// State is per-test (cleared in beforeEach) rather than read back off the
// spy's global `mock.results`, which accumulates across tests.
const active = new Set<UrlListener>();
const subscriptions: Subscription[] = [];

function emitUrl(url: string): void {
  for (const listener of [...active]) {
    listener({ url });
  }
}

let addEventListenerSpy: jest.SpyInstance;

beforeEach(() => {
  active.clear();
  subscriptions.length = 0;

  jest.spyOn(Linking, 'getInitialURL').mockResolvedValue(null as unknown as string);

  addEventListenerSpy = jest
    .spyOn(Linking, 'addEventListener')
    .mockImplementation((type: string, listener: UrlListener) => {
      if (type !== 'url') {
        throw new Error(`unexpected event type: ${type}`);
      }
      active.add(listener);
      const subscription: Subscription = {
        remove: jest.fn(() => {
          active.delete(listener);
        }),
      };
      subscriptions.push(subscription);
      return subscription as unknown as ReturnType<typeof Linking.addEventListener>;
    });
});

afterEach(() => {
  jest.restoreAllMocks();
});

describe('DeepLinkManager listener lifecycle', () => {
  it('registers exactly one "url" listener on initialize()', async () => {
    const manager = new DeepLinkManager();
    await manager.initialize();

    expect(addEventListenerSpy).toHaveBeenCalled();
    expect(subscriptions).toHaveLength(1);
    expect(active.size).toBe(1);
  });

  it('is idempotent: re-initialize() does not stack listeners or double-fire', async () => {
    const manager = new DeepLinkManager();
    manager.setAuthenticated(true);

    const handler = jest.fn();
    manager.addHandler(handler);

    await manager.initialize();
    await manager.initialize();
    await manager.initialize();

    // Three inits create three subscriptions, but only one listener stays
    // active — each new listener replaces (and removes) the previous one.
    expect(subscriptions).toHaveLength(3);
    expect(active.size).toBe(1);

    // The subscriptions from the first two inits must have been removed; the
    // latest one is still live.
    expect(subscriptions[0].remove).toHaveBeenCalledTimes(1);
    expect(subscriptions[1].remove).toHaveBeenCalledTimes(1);
    expect(subscriptions[2].remove).not.toHaveBeenCalled();

    // A single incoming URL dispatches to the handler exactly once (no
    // double-fire from a leaked listener).
    emitUrl('ppt://dashboard');
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('cleanup() removes the listener and is safe before initialize()', async () => {
    const manager = new DeepLinkManager();

    // Safe to call before initialize().
    expect(() => manager.cleanup()).not.toThrow();

    manager.setAuthenticated(true);
    const handler = jest.fn();
    manager.addHandler(handler);

    await manager.initialize();
    expect(active.size).toBe(1);

    manager.cleanup();
    expect(active.size).toBe(0);

    // No listener is active, so an emitted URL reaches no handler.
    emitUrl('ppt://dashboard');
    expect(handler).not.toHaveBeenCalled();

    // Idempotent: a second cleanup() is a no-op.
    expect(() => manager.cleanup()).not.toThrow();
    expect(active.size).toBe(0);
  });
});

// Security regression (shared device): a deep-link intent queued while logged
// out must not survive a logout, and must never be dispatched across a user
// boundary. `ppt://faults/<id>` is an auth-required route, so an incoming URL
// while unauthenticated is queued as `pendingLink`.
describe('DeepLinkManager pending-link user isolation', () => {
  const AUTH_REQUIRED_URL = 'ppt://faults/999';

  it('clears a queued pending link on logout', async () => {
    const manager = new DeepLinkManager();
    await manager.initialize();

    // Unauthenticated: the auth-required link is queued, not dispatched.
    emitUrl(AUTH_REQUIRED_URL);
    expect(manager.getPendingLink()).not.toBeNull();

    // Logout must drop it so the next user can't inherit it.
    manager.setAuthenticated(false);
    expect(manager.getPendingLink()).toBeNull();
  });

  it('does not dispatch a link reserved for one user to a different user', async () => {
    const manager = new DeepLinkManager();
    const handler = jest.fn();
    manager.addHandler(handler);
    await manager.initialize();

    manager.setAuthenticated(true, 'user-A'); // A signs in
    manager.setAuthenticated(false); // A signs out (reservation stays with A)

    emitUrl(AUTH_REQUIRED_URL); // queued, reserved for A
    expect(manager.getPendingLink()).not.toBeNull();

    manager.setAuthenticated(true, 'user-B'); // B signs in on the shared device
    expect(handler).not.toHaveBeenCalled(); // A's link is NOT leaked to B
    expect(manager.getPendingLink()).toBeNull(); // and is not left dangling
  });

  it('dispatches a reserved link back to the same user who returns', async () => {
    const manager = new DeepLinkManager();
    const handler = jest.fn();
    manager.addHandler(handler);
    await manager.initialize();

    manager.setAuthenticated(true, 'user-A');
    manager.setAuthenticated(false);

    emitUrl(AUTH_REQUIRED_URL); // reserved for A
    manager.setAuthenticated(true, 'user-A'); // A returns
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('dispatches an unreserved link (fresh device) to the first user to authenticate', async () => {
    const manager = new DeepLinkManager();
    const handler = jest.fn();
    manager.addHandler(handler);
    await manager.initialize(); // never authenticated → no prior user

    emitUrl(AUTH_REQUIRED_URL); // reserved for null (claimable)
    manager.setAuthenticated(true, 'user-A');
    expect(handler).toHaveBeenCalledTimes(1);
  });
});
