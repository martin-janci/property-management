/**
 * Contract tests for the signer-facing `/sign` transport (Epic 84, Story 84.2).
 *
 * The backend `ErrorResponse` (see `backend/crates/common/src/errors.rs`)
 * serializes its discriminant in the JSON `code` field:
 *   { "code": "SIGNING_LINK_EXPIRED", "message": "...", "timestamp": "..." }
 *
 * `signRequest` must read that `code` field so the signer page can branch its
 * UI on `SIGNING_LINK_EXPIRED` / `INVALID_SIGNING_LINK` / `ALREADY_SIGNED` /
 * `LINK_ALREADY_USED` / `REQUEST_NOT_OPEN`. A previous revision read `body.error`
 * (a field the backend never sends), so `SignError.code` always fell back to
 * `HTTP_<status>` and every error-state branch was dead. These tests pin the
 * real wire contract so that regression can't return silently.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { getSignContext, SignError, submitSignature } from './api';

const TOKEN = 'test-token';

/** Build an error Response whose JSON body mimics the backend `ErrorResponse`. */
function errorJson(status: number, body: unknown): Response {
  return {
    ok: false,
    status,
    json: async () => body,
  } as Response;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('signRequest error mapping — reads backend `code` field', () => {
  it('410 SIGNING_LINK_EXPIRED body → SignError.code === "SIGNING_LINK_EXPIRED"', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      errorJson(410, {
        code: 'SIGNING_LINK_EXPIRED',
        message: 'This signing link has expired',
        timestamp: '2026-07-15T00:00:00Z',
      })
    );

    await expect(getSignContext(TOKEN)).rejects.toMatchObject({
      name: 'SignError',
      code: 'SIGNING_LINK_EXPIRED',
      status: 410,
      message: 'This signing link has expired',
    });
  });

  it('propagates each signer error discriminant from the `code` field', async () => {
    const cases: Array<{ status: number; code: string }> = [
      { status: 401, code: 'INVALID_SIGNING_LINK' },
      { status: 409, code: 'ALREADY_SIGNED' },
      { status: 409, code: 'LINK_ALREADY_USED' },
      { status: 409, code: 'REQUEST_NOT_OPEN' },
    ];

    for (const { status, code } of cases) {
      vi.spyOn(globalThis, 'fetch').mockResolvedValue(
        errorJson(status, { code, message: `err ${code}`, timestamp: 'x' })
      );
      const err = await submitSignature(TOKEN).catch((e) => e);
      expect(err).toBeInstanceOf(SignError);
      expect(err.code).toBe(code);
      expect(err.status).toBe(status);
    }
  });

  it('falls back to HTTP_<status> only when the body carries no code', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(errorJson(500, {}));
    const err = await getSignContext(TOKEN).catch((e) => e);
    expect(err).toBeInstanceOf(SignError);
    expect(err.code).toBe('HTTP_500');
  });
});
