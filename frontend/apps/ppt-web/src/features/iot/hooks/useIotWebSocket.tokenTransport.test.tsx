/// <reference types="vitest/globals" />
/**
 * Token-transport (credential-hygiene) regression tests for
 * {@link useIotWebSocket}.
 *
 * Bug (code-review-ppt-web-core-ws-jwt-in-query-param): the IoT sensor
 * WebSocket opened `new WebSocket('…/iot/sensors/ws?token=<jwt>&…')`, placing
 * the bearer JWT in the URL query string. A query-string credential leaks into
 * api-server access logs, reverse-proxy logs, and the browser's own history —
 * a single captured line is enough for session takeover. (The shared
 * notification socket in `lib/websocket.ts` had already moved off this transport.)
 *
 * Fix: carry the JWT in the `Sec-WebSocket-Protocol` handshake header as a
 * `bearer.<jwt>` subprotocol (the WebSocket constructor's second argument), via
 * the shared {@link buildWsSubprotocols} helper — the same transport the
 * notification socket uses. Only the non-secret `organization_id` stays in the
 * query string. The api-server (`routes/iot/realtime.rs`) reads the subprotocol
 * first, keeping `?token=` only as a fallback for not-yet-migrated clients.
 *
 * The load-bearing assertions below are: (1) the JWT never appears in the
 * connection URL, and (2) it IS offered as the `bearer.<jwt>` subprotocol.
 * Against the pre-fix hook both fail — the URL carried the token and no
 * subprotocol list was passed at all.
 */

import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WS_BEARER_SUBPROTOCOL_PREFIX, WS_PROTOCOL_VERSION } from '../../../lib/websocket';
import { useIotWebSocket } from './useIotWebSocket';

const TEST_JWT = 'header.payload.signature-jwt-value';

vi.mock('@ppt/api-client', () => ({
  getToken: () => TEST_JWT,
}));

/** Fake WebSocket that records the URL and the subprotocol argument. */
class RecordingWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  static lastUrl: string | null = null;
  static lastProtocols: string | string[] | undefined;

  readyState = RecordingWebSocket.CONNECTING;
  onopen: ((ev: unknown) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: string }) => void) | null = null;

  constructor(
    public url: string,
    protocols?: string | string[]
  ) {
    RecordingWebSocket.lastUrl = url;
    RecordingWebSocket.lastProtocols = protocols;
  }

  send(): void {}
  close(): void {
    this.readyState = RecordingWebSocket.CLOSED;
  }
}

let originalWebSocket: typeof globalThis.WebSocket;

beforeEach(() => {
  originalWebSocket = globalThis.WebSocket;
  // biome-ignore lint/suspicious/noExplicitAny: test double for the global WebSocket
  globalThis.WebSocket = RecordingWebSocket as any;
  RecordingWebSocket.lastUrl = null;
  RecordingWebSocket.lastProtocols = undefined;
});

afterEach(() => {
  globalThis.WebSocket = originalWebSocket;
  vi.restoreAllMocks();
});

describe('useIotWebSocket — JWT transport hygiene', () => {
  function openSocket(): void {
    renderHook(() => useIotWebSocket('sensor-1', 'org-42'));
    act(() => {});
  }

  it('never puts the JWT in the connection URL query string', () => {
    openSocket();

    const url = RecordingWebSocket.lastUrl;
    expect(url).toBeTruthy();
    // The bearer credential must not appear anywhere in the URL...
    expect(url).not.toContain(TEST_JWT);
    // ...and specifically not as a `token=` query param (the leaking transport).
    expect(url).not.toMatch(/[?&]token=/);
  });

  it('keeps only the non-secret organization_id in the query string', () => {
    openSocket();

    const url = RecordingWebSocket.lastUrl ?? '';
    expect(url).toContain('/api/v1/iot/sensors/ws?');
    // The base is empty in tests (`__API_BASE_URL__` unset), so the URL is
    // relative — resolve it against a dummy origin to inspect the query string.
    const parsed = new URL(url, 'ws://example.test');
    expect(parsed.searchParams.get('organization_id')).toBe('org-42');
    // organization_id is the ONLY query parameter — no token rider.
    expect([...parsed.searchParams.keys()]).toEqual(['organization_id']);
  });

  it('carries the JWT as the bearer.<jwt> Sec-WebSocket-Protocol subprotocol', () => {
    openSocket();

    const protocols = RecordingWebSocket.lastProtocols;
    expect(Array.isArray(protocols)).toBe(true);
    const list = protocols as string[];
    // The bearer token subprotocol must carry the exact JWT...
    expect(list).toContain(`${WS_BEARER_SUBPROTOCOL_PREFIX}${TEST_JWT}`);
    // ...alongside the protocol-version subprotocol the server echoes.
    expect(list).toContain(WS_PROTOCOL_VERSION);
  });
});
