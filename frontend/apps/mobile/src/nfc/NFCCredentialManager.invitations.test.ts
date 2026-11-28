/**
 * Characterization tests for `NFCCredentialManager`'s guest-access-invitation
 * surface — the one group of public methods the three sibling suites never
 * touch:
 *
 *  - `NFCCredentialManager.test.ts` — emergency revoke, access-log corruption
 *    guard, SecureStore chunking round-trip, cross-tenant purge.
 *  - `NFCCredentialManager.credentials.test.ts` — query / active filter / 24 h
 *    refresh threshold / usage counter / log cap / fetch fallback / migration.
 *  - `NFCCredentialManager.lifecycle.test.ts` — refreshCredential, manager-level
 *    server-clock expiry, syncServerClock guards, stored-format parsing.
 *
 * None of those exercise the guest-invitation request/response (write) paths:
 *
 *  - `createGuestInvitation` — POST `/api/v1/access/invitations` with the full
 *    invitation body, returning the server-created invitation.
 *  - `getGuestInvitations` — GET `/api/v1/access/invitations?building_id=…`,
 *    returning the building's invitations.
 *  - `cancelGuestInvitation` — POST `/api/v1/access/invitations/{id}/cancel`,
 *    resolving void.
 *
 * For each we pin: the HTTP verb, URL, auth header, and (for create) the
 * serialized request body; the happy-path return value; and the error path —
 * a non-2xx response surfaces as a thrown `API request failed: <status>`
 * (there is no cache fallback for these, unlike `fetchCredentials`).
 *
 * Every assertion pins existing behaviour; nothing in the manager changes.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';

import { NFCCredentialManager } from './NFCCredentialManager';
import type { GuestAccessInvitation } from './types';

// In-memory SecureStore backed by an isolated map, matching the shape the
// sibling suites use.
jest.mock('expo-secure-store', () => {
  const store = new Map<string, string>();
  return {
    __store: store,
    getItemAsync: jest.fn(async (k: string) => store.get(k) ?? null),
    setItemAsync: jest.fn(async (k: string, v: string) => {
      store.set(k, v);
    }),
    deleteItemAsync: jest.fn(async (k: string) => {
      store.delete(k);
    }),
  };
});

const secureStore = (SecureStore as unknown as { __store: Map<string, string> }).__store;

const asyncGet = AsyncStorage.getItem as jest.Mock;
const asyncSet = AsyncStorage.setItem as jest.Mock;
const asyncRemove = AsyncStorage.removeItem as jest.Mock;

const API_BASE = 'https://api.test';

function makeInvitation(overrides: Partial<GuestAccessInvitation> = {}): GuestAccessInvitation {
  return {
    id: 'inv-1',
    inviterId: 'user-1',
    guestName: 'Dana Guest',
    guestEmail: 'dana@example.com',
    buildingId: 'bld-1',
    accessPoints: ['ap-1'],
    validFrom: '2026-06-15T08:00:00.000Z',
    validUntil: '2026-06-16T08:00:00.000Z',
    maxEntries: 3,
    entryCount: 0,
    status: 'pending',
    claimLink: 'https://ppt.example/claim/inv-1',
    ...overrides,
  };
}

/** Capture-friendly fetch double returning `body` as a 200 JSON response. */
function okFetch(body: unknown): jest.Mock {
  return jest.fn(async () => ({
    ok: true,
    status: 200,
    json: async () => body,
  })) as unknown as jest.Mock;
}

/** A fetch double that rejects every request with the given HTTP status. */
function errorFetch(status: number): jest.Mock {
  return jest.fn(async () => ({
    ok: false,
    status,
    json: async () => ({}),
  })) as unknown as jest.Mock;
}

/** Read back the [url, init] a single-call fetch mock was invoked with. */
function lastRequest(fetchMock: jest.Mock): { url: string; init: RequestInit } {
  const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
  return { url, init };
}

let originalFetch: typeof globalThis.fetch;

beforeEach(() => {
  jest.clearAllMocks();
  secureStore.clear();
  asyncGet.mockResolvedValue(null);
  asyncSet.mockResolvedValue(undefined);
  asyncRemove.mockResolvedValue(undefined);
  originalFetch = globalThis.fetch;
});

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe('NFCCredentialManager.createGuestInvitation', () => {
  it('POSTs the full invitation body and returns the server-created invitation', async () => {
    const created = makeInvitation({ id: 'inv-created' });
    const fetchMock = okFetch({ invitation: created });
    globalThis.fetch = fetchMock;

    const manager = new NFCCredentialManager(API_BASE);
    manager.setAuthToken('tok-123');

    const result = await manager.createGuestInvitation(
      'bld-1',
      'Dana Guest',
      'dana@example.com',
      ['ap-1', 'ap-2'],
      '2026-06-15T08:00:00.000Z',
      '2026-06-16T08:00:00.000Z',
      5
    );

    expect(result).toEqual(created);

    const { url, init } = lastRequest(fetchMock);
    expect(url).toBe('https://api.test/api/v1/access/invitations');
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer tok-123');
    expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
    // The body carries every argument, including the optional maxEntries.
    expect(JSON.parse(init.body as string)).toEqual({
      buildingId: 'bld-1',
      guestName: 'Dana Guest',
      guestEmail: 'dana@example.com',
      accessPoints: ['ap-1', 'ap-2'],
      validFrom: '2026-06-15T08:00:00.000Z',
      validUntil: '2026-06-16T08:00:00.000Z',
      maxEntries: 5,
    });
  });

  it('omits maxEntries from the body when the caller does not supply it', async () => {
    const fetchMock = okFetch({ invitation: makeInvitation() });
    globalThis.fetch = fetchMock;

    const manager = new NFCCredentialManager(API_BASE);
    await manager.createGuestInvitation(
      'bld-1',
      'Dana Guest',
      'dana@example.com',
      ['ap-1'],
      '2026-06-15T08:00:00.000Z',
      '2026-06-16T08:00:00.000Z'
    );

    const body = JSON.parse(lastRequest(fetchMock).init.body as string);
    expect(body).not.toHaveProperty('maxEntries', expect.anything());
    expect(body.maxEntries).toBeUndefined();
  });

  it('propagates a server error instead of swallowing it (no cache fallback)', async () => {
    globalThis.fetch = errorFetch(422);

    const manager = new NFCCredentialManager(API_BASE);
    await expect(
      manager.createGuestInvitation(
        'bld-1',
        'Dana Guest',
        'dana@example.com',
        ['ap-1'],
        '2026-06-15T08:00:00.000Z',
        '2026-06-16T08:00:00.000Z'
      )
    ).rejects.toThrow('API request failed: 422');
  });
});

describe('NFCCredentialManager.getGuestInvitations', () => {
  it('GETs the building invitations and unwraps the invitation list', async () => {
    const invitations = [makeInvitation({ id: 'inv-1' }), makeInvitation({ id: 'inv-2' })];
    const fetchMock = okFetch({ invitations });
    globalThis.fetch = fetchMock;

    const manager = new NFCCredentialManager(API_BASE);
    const result = await manager.getGuestInvitations('bld-42');

    expect(result).toEqual(invitations);

    const { url, init } = lastRequest(fetchMock);
    expect(url).toBe('https://api.test/api/v1/access/invitations?building_id=bld-42');
    // GET is the default verb; no request body is sent.
    expect(init.method).toBe('GET');
    expect(init.body).toBeUndefined();
  });

  it('sends no Authorization header when no auth token is set', async () => {
    const fetchMock = okFetch({ invitations: [] });
    globalThis.fetch = fetchMock;

    const manager = new NFCCredentialManager(API_BASE);
    const result = await manager.getGuestInvitations('bld-1');

    expect(result).toEqual([]);
    const headers = lastRequest(fetchMock).init.headers as Record<string, string>;
    expect(headers.Authorization).toBeUndefined();
  });

  it('propagates a server error', async () => {
    globalThis.fetch = errorFetch(500);

    const manager = new NFCCredentialManager(API_BASE);
    await expect(manager.getGuestInvitations('bld-1')).rejects.toThrow('API request failed: 500');
  });
});

describe('NFCCredentialManager.cancelGuestInvitation', () => {
  it('POSTs to the invitation cancel endpoint and resolves void', async () => {
    const fetchMock = okFetch({});
    globalThis.fetch = fetchMock;

    const manager = new NFCCredentialManager(API_BASE);
    manager.setAuthToken('tok-xyz');

    await expect(manager.cancelGuestInvitation('inv-9')).resolves.toBeUndefined();

    const { url, init } = lastRequest(fetchMock);
    expect(url).toBe('https://api.test/api/v1/access/invitations/inv-9/cancel');
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>).Authorization).toBe('Bearer tok-xyz');
    // No request body for a cancel.
    expect(init.body).toBeUndefined();
  });

  it('propagates a server error so the caller can retry', async () => {
    globalThis.fetch = errorFetch(409);

    const manager = new NFCCredentialManager(API_BASE);
    await expect(manager.cancelGuestInvitation('inv-9')).rejects.toThrow('API request failed: 409');
  });
});
