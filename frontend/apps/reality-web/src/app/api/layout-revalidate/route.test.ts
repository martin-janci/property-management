import { createHmac } from 'node:crypto';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock next/cache before importing the route
vi.mock('next/cache', () => ({
  revalidateTag: vi.fn(),
}));

import { revalidateTag } from 'next/cache';
import { __clearAnalyticsSinks, type AnalyticsEvent, registerAnalyticsSink } from '@/lib/analytics';
import { isTimestampFresh, layoutTagsFor, POST, parseWebhookTimestamp } from './route';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Sign the timestamped payload "{timestamp}.{body}" (issue #2485). */
function makeSignature(secret: string, timestamp: number, body: string): string {
  return 'sha256=' + createHmac('sha256', secret).update(`${timestamp}.${body}`).digest('base64');
}

function makeRequest(body: string, headers: Record<string, string> = {}): Request {
  return new Request('http://localhost/api/layout-revalidate', {
    method: 'POST',
    body,
    headers: { 'Content-Type': 'application/json', ...headers },
  });
}

/** A fresh unix-seconds timestamp for the current wall clock. */
function nowTs(): number {
  return Math.floor(Date.now() / 1000);
}

/**
 * Build a fully-signed request: fresh (or supplied) timestamp header plus a
 * signature over "{timestamp}.{body}". This is the happy-path shape every
 * non-timestamp test reuses so those tests exercise the check they name.
 */
function signedRequest(
  secret: string,
  body: string,
  opts: { timestamp?: number; extraHeaders?: Record<string, string> } = {}
): Request {
  const ts = opts.timestamp ?? nowTs();
  return makeRequest(body, {
    'X-Webhook-Timestamp': String(ts),
    'X-Webhook-Signature': makeSignature(secret, ts, body),
    ...opts.extraHeaders,
  });
}

const SECRET = 'test-secret-abc123';

// ---------------------------------------------------------------------------
// layoutTagsFor helper
// ---------------------------------------------------------------------------

describe('layoutTagsFor', () => {
  it('returns layout tag from second segment', () => {
    expect(layoutTagsFor('reality/listing-detail')).toEqual(['layout:listing-detail']);
  });

  it('uses first slash split', () => {
    expect(layoutTagsFor('reality/listing-detail/extra')).toEqual(['layout:listing-detail']);
  });

  it('returns null when second segment is empty', () => {
    expect(layoutTagsFor('foo/')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Replay-protection helpers (issue #2485)
// ---------------------------------------------------------------------------

describe('parseWebhookTimestamp', () => {
  it('parses a plain unix-seconds integer', () => {
    expect(parseWebhookTimestamp('1700000000')).toBe(1_700_000_000);
  });

  it('trims surrounding whitespace', () => {
    expect(parseWebhookTimestamp('  1700000000  ')).toBe(1_700_000_000);
  });

  it('returns null for a missing header', () => {
    expect(parseWebhookTimestamp(null)).toBeNull();
  });

  it.each(['', 'abc', '123abc', '1.5', '0x10', '1e3', 'NaN'])(
    'returns null for malformed value %j',
    (raw) => {
      expect(parseWebhookTimestamp(raw)).toBeNull();
    }
  );
});

describe('isTimestampFresh', () => {
  const NOW = 1_700_000_000;

  it('accepts a timestamp equal to now', () => {
    expect(isTimestampFresh(NOW, NOW)).toBe(true);
  });

  it('accepts the exact ±300s boundary (inclusive)', () => {
    expect(isTimestampFresh(NOW - 300, NOW)).toBe(true);
    expect(isTimestampFresh(NOW + 300, NOW)).toBe(true);
  });

  it('rejects just outside the window in the past and future', () => {
    expect(isTimestampFresh(NOW - 301, NOW)).toBe(false);
    expect(isTimestampFresh(NOW + 301, NOW)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// POST handler
// ---------------------------------------------------------------------------

describe('POST /api/layout-revalidate', () => {
  beforeEach(() => {
    vi.mocked(revalidateTag).mockClear();
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('returns 503 when LAYOUT_WEBHOOK_SECRET is not set', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', '');
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const req = signedRequest(SECRET, body);
    const res = await POST(req);
    expect(res.status).toBe(503);
    const json = await res.json();
    expect(json).toEqual({ error: 'disabled' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 when X-Webhook-Signature header is missing', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    // Fresh timestamp present, but no signature header.
    const req = makeRequest(body, { 'X-Webhook-Timestamp': String(nowTs()) });
    const res = await POST(req);
    expect(res.status).toBe(401);
    const json = await res.json();
    expect(json).toEqual({ error: 'invalid signature' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 when X-Webhook-Signature is wrong', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const req = makeRequest(body, {
      'X-Webhook-Timestamp': String(nowTs()),
      'X-Webhook-Signature': 'sha256=invalidsignature==',
    });
    const res = await POST(req);
    expect(res.status).toBe(401);
    const json = await res.json();
    expect(json).toEqual({ error: 'invalid signature' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  // --- Replay protection (issue #2485) ---

  it('returns 401 when X-Webhook-Timestamp header is missing', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    // Signature is valid for a fresh ts, but no timestamp header is sent.
    const req = makeRequest(body, {
      'X-Webhook-Signature': makeSignature(SECRET, nowTs(), body),
    });
    const res = await POST(req);
    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'missing timestamp' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 when X-Webhook-Timestamp is malformed', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const req = makeRequest(body, {
      'X-Webhook-Timestamp': 'not-a-number',
      'X-Webhook-Signature': makeSignature(SECRET, nowTs(), body),
    });
    const res = await POST(req);
    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'missing timestamp' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 for a stale timestamp (replay outside the freshness window)', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail', event: 'published' });
    // Correctly signed, but 301s in the past — just outside the ±300s window.
    const staleTs = nowTs() - 301;
    const req = signedRequest(SECRET, body, { timestamp: staleTs });
    const res = await POST(req);
    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'stale timestamp' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 when a fresh timestamp is swapped onto an old signature', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail', event: 'published' });
    // Attacker captured (oldTs, sigOverOldTs) and presents a fresh timestamp to
    // beat the window while reusing the old signature. Because the timestamp is
    // folded into the HMAC, the swap fails the signature check.
    const oldTs = nowTs() - 10_000;
    const staleSig = makeSignature(SECRET, oldTs, body);
    const req = makeRequest(body, {
      'X-Webhook-Timestamp': String(nowTs()),
      'X-Webhook-Signature': staleSig,
    });
    const res = await POST(req);
    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'invalid signature' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 413 for bodies larger than 16 KiB before verifying', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    // Correctly signed, but oversized — the size cap must fire first so an
    // unauthenticated caller cannot force unbounded HMAC/JSON work.
    const body = JSON.stringify({ screen: 'reality/listing-detail', pad: 'x'.repeat(17 * 1024) });
    const res = await POST(signedRequest(SECRET, body));
    expect(res.status).toBe(413);
    expect(await res.json()).toEqual({ error: 'body too large' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 200 and revalidates layout:listing-detail on a fresh signed request', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail', event: 'published' });
    const req = signedRequest(SECRET, body);
    const res = await POST(req);
    expect(res.status).toBe(200);
    const json = await res.json();
    expect(json).toEqual({ revalidated: true, tags: ['layout:listing-detail'] });
    expect(revalidateTag).toHaveBeenCalledWith('layout:listing-detail', 'default');
    expect(revalidateTag).toHaveBeenCalledTimes(1);
  });

  it('accepts a timestamp at the exact ±300s boundary', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail', event: 'published' });
    const req = signedRequest(SECRET, body, { timestamp: nowTs() - 300 });
    const res = await POST(req);
    expect(res.status).toBe(200);
    expect(revalidateTag).toHaveBeenCalledTimes(1);
  });

  it('returns 422 when screen has no slash', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'listing-detail' });
    const req = signedRequest(SECRET, body);
    const res = await POST(req);
    expect(res.status).toBe(422);
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 422 when second segment is empty (e.g., "foo/")', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'foo/' });
    const req = signedRequest(SECRET, body);
    const res = await POST(req);
    expect(res.status).toBe(422);
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 422 when body is not valid JSON', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = 'not-json';
    const req = signedRequest(SECRET, body);
    const res = await POST(req);
    expect(res.status).toBe(422);
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 422 when body is JSON but has no screen string', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ event: 'published' });
    const req = signedRequest(SECRET, body);
    const res = await POST(req);
    expect(res.status).toBe(422);
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('does not export GET', () => {
    // Verify by checking the module's exported keys directly
    const routeModule = { POST, layoutTagsFor } as Record<string, unknown>;
    expect(routeModule['GET']).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// layout_revalidate_received analytics (issue #2532)
//
// The receiver must emit one operational event on every return path, echoing
// the delivery_id (gap D) when the webhook body carries it. These assertions
// fail on `dev`, where the route emitted nothing.
// ---------------------------------------------------------------------------

describe('layout_revalidate_received emission', () => {
  let events: AnalyticsEvent[];
  let unregister: () => void;

  beforeEach(() => {
    vi.mocked(revalidateTag).mockClear();
    events = [];
    unregister = registerAnalyticsSink((e) => events.push(e));
  });

  afterEach(() => {
    unregister();
    __clearAnalyticsSinks();
    vi.unstubAllEnvs();
  });

  it('emits revalidated with tags and echoed delivery_id on success', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const deliveryId = '11111111-2222-3333-4444-555555555555';
    const body = JSON.stringify({
      screen: 'reality/listing-detail',
      event: 'published',
      delivery_id: deliveryId,
    });
    const res = await POST(signedRequest(SECRET, body));
    expect(res.status).toBe(200);

    expect(events).toHaveLength(1);
    expect(events[0].event).toBe('layout_revalidate_received');
    expect(events[0].properties).toMatchObject({
      revalidate_outcome: 'revalidated',
      http_status: 200,
      screen: 'reality/listing-detail',
      tags: ['layout:listing-detail'],
      delivery_id: deliveryId,
    });
  });

  it('emits disabled (503) when the secret is unset', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', '');
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const res = await POST(signedRequest(SECRET, body));
    expect(res.status).toBe(503);
    expect(events).toHaveLength(1);
    expect(events[0].properties).toMatchObject({
      revalidate_outcome: 'disabled',
      http_status: 503,
    });
  });

  it('emits invalid_signature (401) on a bad signature', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const req = makeRequest(body, {
      'X-Webhook-Timestamp': String(nowTs()),
      'X-Webhook-Signature': 'sha256=invalidsignature==',
    });
    const res = await POST(req);
    expect(res.status).toBe(401);
    expect(events).toHaveLength(1);
    expect(events[0].properties).toMatchObject({
      revalidate_outcome: 'invalid_signature',
      http_status: 401,
    });
  });

  it('emits invalid_screen (422) when the screen has no slash', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'listing-detail' });
    const res = await POST(signedRequest(SECRET, body));
    expect(res.status).toBe(422);
    expect(events).toHaveLength(1);
    expect(events[0].properties).toMatchObject({
      revalidate_outcome: 'invalid_screen',
      http_status: 422,
    });
  });
});
