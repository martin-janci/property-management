/// <reference types="vitest/globals" />
/**
 * Unit tests for the realtime WebSocket contract helpers (#775, #819).
 *
 * These cover the two ends of the client/server contract that were broken:
 *
 * 1. URL builder — the client must connect to the canonical mounted path
 *    `/api/v1/users/me/notifications/ws` with the JWT as a `?token=` query
 *    param (the api-server reads `WsQuery.token`; browsers cannot set a WS
 *    `Authorization` header). A bare `/ws` 404s on upgrade.
 * 2. Message parser — the server emits the canonical `{ event, payload }`
 *    envelope (`WsEvent` in `ws_notifications.rs`). The client must map
 *    `event` -> internal `type` so subscribers fire on the real event names
 *    (`notification.created`, `preference.updated`).
 *
 * A drift in either direction means realtime cannot connect or silently drops
 * every event, so the round-trip is asserted explicitly.
 */

import { describe, expect, it } from 'vitest';
import { buildWebSocketUrl, parseServerMessage, WS_NOTIFICATIONS_PATH } from './websocket';

describe('buildWebSocketUrl', () => {
  it('appends the canonical handler path to a bare origin', () => {
    const url = buildWebSocketUrl('ws://localhost:8080', 'jwt-abc');
    expect(url).toBe(`ws://localhost:8080${WS_NOTIFICATIONS_PATH}?token=jwt-abc`);
  });

  it('attaches the JWT as the token query param', () => {
    const url = new URL(buildWebSocketUrl('wss://app.example.com', 'jwt-xyz'));
    expect(url.searchParams.get('token')).toBe('jwt-xyz');
    expect(url.pathname).toBe(WS_NOTIFICATIONS_PATH);
  });

  it('corrects a legacy /ws path on the base to the canonical path', () => {
    const url = new URL(buildWebSocketUrl('ws://localhost:8080/ws', 'tok'));
    expect(url.pathname).toBe(WS_NOTIFICATIONS_PATH);
  });

  it('preserves the wss scheme for production hosts', () => {
    const url = buildWebSocketUrl('wss://app.example.com', 'tok');
    expect(url.startsWith('wss://app.example.com')).toBe(true);
  });

  it('url-encodes a token containing reserved characters', () => {
    const token = 'a.b+c/d=';
    const url = new URL(buildWebSocketUrl('ws://localhost:8080', token));
    // round-trips back to the exact token despite encoding on the wire
    expect(url.searchParams.get('token')).toBe(token);
  });
});

describe('parseServerMessage', () => {
  it('maps the canonical {event, payload} envelope onto the internal type', () => {
    const msg = parseServerMessage({
      event: 'notification.created',
      payload: { notification_id: '00000000-0000-0000-0000-000000000001', title: 'Hi' },
    });
    expect(msg).not.toBeNull();
    expect(msg?.type).toBe('notification.created');
    expect(msg?.payload).toEqual({
      notification_id: '00000000-0000-0000-0000-000000000001',
      title: 'Hi',
    });
    expect(typeof msg?.timestamp).toBe('string');
  });

  it('handles the preference.updated event', () => {
    const msg = parseServerMessage({ event: 'preference.updated', payload: { channel: 'email' } });
    expect(msg?.type).toBe('preference.updated');
    expect(msg?.payload).toEqual({ channel: 'email' });
  });

  it('defaults a missing payload to null', () => {
    const msg = parseServerMessage({ event: 'preference.updated' });
    expect(msg?.type).toBe('preference.updated');
    expect(msg?.payload).toBeNull();
  });

  it('returns null for a frame without an event field', () => {
    expect(parseServerMessage({ type: 'pong' })).toBeNull();
    expect(parseServerMessage({ foo: 'bar' })).toBeNull();
  });

  it('returns null for non-object input', () => {
    expect(parseServerMessage(null)).toBeNull();
    expect(parseServerMessage('string')).toBeNull();
    expect(parseServerMessage(42)).toBeNull();
  });
});
