/// <reference types="vitest/globals" />
/**
 * Regression coverage: the core WebSocket client must NOT leak its connection
 * diagnostics to the browser console in a production build.
 *
 * `lib/websocket.ts` routes every diagnostic through the private `wsWarn` /
 * `wsError` helpers, which gate `console.warn` / `console.error` behind
 * `import.meta.env.DEV` (mirroring the retry logging in `lib/api.ts` and the
 * raw-detail gating in `lib/describeError`). This lets Vite dead-code-eliminate
 * the calls from the production bundle entirely — the WS internal state
 * (send failures, handler errors, reconnect give-up) surfaces only in dev.
 *
 * These helpers are module-private, so the gate is exercised through the public
 * `WebSocketService.send()` path: sending on a disconnected service hits the
 * `wsWarn('[WebSocket] Cannot send message: not connected')` branch without any
 * socket or network. Before the gating, that `console.warn` fired in every
 * build; this test pins it to dev-only so a future raw `console.*` regresses
 * the suite instead of shipping noise to end users.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { WebSocketService } from './websocket';

/** A fresh service is `disconnected`, so `send()` takes the wsWarn branch. */
function newDisconnectedService(): WebSocketService {
  return new WebSocketService({ getToken: () => null });
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
});

describe('WebSocket client console gating (production must stay quiet)', () => {
  it('does not write WS diagnostics to the console in a production build', () => {
    vi.stubEnv('DEV', false);
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    const service = newDisconnectedService();
    // Exercises the `not connected` diagnostic path (wsWarn).
    expect(service.send({ type: 'notification.created', payload: {} })).toBe(false);

    expect(warn).not.toHaveBeenCalled();
    expect(error).not.toHaveBeenCalled();
  });

  it('still logs WS diagnostics to the console in a development build', () => {
    vi.stubEnv('DEV', true);
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const service = newDisconnectedService();
    expect(service.send({ type: 'notification.created', payload: {} })).toBe(false);

    expect(warn).toHaveBeenCalledWith('[WebSocket] Cannot send message: not connected');
  });
});
