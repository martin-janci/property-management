/**
 * Contract tests for the notification-preferences API client (Epic 8A, Story 8A.1).
 *
 * Pins the request wiring (URL + HTTP method) of the hand-rolled client
 * against the backend contract so the two can't silently drift apart.
 *
 * Also verifies the 409 → ConfirmationRequiredError promotion so that the
 * optimistic-rollback path in the UI hooks is exercised.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  ConfirmationRequiredError,
  getNotificationPreferences,
  updateNotificationPreference,
} from './api';

/** Minimal ok JSON response stub. */
function okJson(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

/** Read the (url, method) pair the client passed to the last fetch call. */
function lastCall(): { url: string; method: string } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const url = String(call[0]);
  const method = ((call[1] as RequestInit | undefined)?.method ?? 'GET').toUpperCase();
  return { url, method };
}

describe('notification-preferences api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ preferences: [], allDisabledWarning: null })
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('getNotificationPreferences → GET /api/v1/users/me/notification-preferences', async () => {
    await getNotificationPreferences();
    expect(lastCall()).toEqual({
      url: '/api/v1/users/me/notification-preferences',
      method: 'GET',
    });
  });

  it('updateNotificationPreference → PATCH /api/v1/users/me/notification-preferences/{channel}', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ preference: { channel: 'email', enabled: false, updatedAt: '2026-01-01T00:00:00Z' } })
    );
    await updateNotificationPreference('email', { enabled: false });
    expect(lastCall()).toEqual({
      url: '/api/v1/users/me/notification-preferences/email',
      method: 'PATCH',
    });
  });

  it('updateNotificationPreference sends the request body as JSON', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ preference: { channel: 'push', enabled: true, updatedAt: '2026-01-01T00:00:00Z' } })
    );
    await updateNotificationPreference('push', { enabled: true });
    const calls = vi.mocked(fetch).mock.calls;
    const init = calls[calls.length - 1][1] as RequestInit;
    expect(JSON.parse(init.body as string)).toEqual({ enabled: true });
  });

  it('updateNotificationPreference promotes 409 to ConfirmationRequiredError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson(
        { error: { message: 'Disabling this channel would silence all notifications.' } },
        409
      )
    );
    await expect(
      updateNotificationPreference('in_app', { enabled: false })
    ).rejects.toBeInstanceOf(ConfirmationRequiredError);
  });

  it('ConfirmationRequiredError carries the channel that triggered it', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ error: { message: 'Requires confirmation' } }, 409)
    );
    try {
      await updateNotificationPreference('push', { enabled: false });
      expect.fail('should have thrown');
    } catch (err) {
      expect(err).toBeInstanceOf(ConfirmationRequiredError);
      expect((err as ConfirmationRequiredError).channel).toBe('push');
    }
  });

  it('updateNotificationPreference with confirmDisableAll=true sends flag in body', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ preference: { channel: 'push', enabled: false, updatedAt: '2026-01-01T00:00:00Z' } })
    );
    await updateNotificationPreference('push', { enabled: false, confirmDisableAll: true });
    const calls = vi.mocked(fetch).mock.calls;
    const init = calls[calls.length - 1][1] as RequestInit;
    expect(JSON.parse(init.body as string)).toEqual({
      enabled: false,
      confirmDisableAll: true,
    });
  });
});
