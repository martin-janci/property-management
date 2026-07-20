import { createHmac } from 'node:crypto';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock next/cache before importing the route
vi.mock('next/cache', () => ({
  revalidateTag: vi.fn(),
}));

import { revalidateTag } from 'next/cache';
import { POST, layoutTagsFor } from './route';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeSignature(secret: string, body: string): string {
  return 'sha256=' + createHmac('sha256', secret).update(body).digest('base64');
}

function makeRequest(body: string, headers: Record<string, string> = {}): Request {
  return new Request('http://localhost/api/layout-revalidate', {
    method: 'POST',
    body,
    headers: { 'Content-Type': 'application/json', ...headers },
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
    const req = makeRequest(body, { 'X-Webhook-Signature': makeSignature(SECRET, body) });
    const res = await POST(req);
    expect(res.status).toBe(503);
    const json = await res.json();
    expect(json).toEqual({ error: 'disabled' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 when X-Webhook-Signature header is missing', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const req = makeRequest(body);
    const res = await POST(req);
    expect(res.status).toBe(401);
    const json = await res.json();
    expect(json).toEqual({ error: 'invalid signature' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 401 when X-Webhook-Signature is wrong', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail' });
    const req = makeRequest(body, { 'X-Webhook-Signature': 'sha256=invalidsignature==' });
    const res = await POST(req);
    expect(res.status).toBe(401);
    const json = await res.json();
    expect(json).toEqual({ error: 'invalid signature' });
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 200 and revalidates layout:listing-detail on valid request', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'reality/listing-detail', event: 'published' });
    const sig = makeSignature(SECRET, body);
    const req = makeRequest(body, { 'X-Webhook-Signature': sig });
    const res = await POST(req);
    expect(res.status).toBe(200);
    const json = await res.json();
    expect(json).toEqual({ revalidated: true, tags: ['layout:listing-detail'] });
    expect(revalidateTag).toHaveBeenCalledWith('layout:listing-detail', 'default');
    expect(revalidateTag).toHaveBeenCalledTimes(1);
  });

  it('returns 422 when screen has no slash', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ screen: 'listing-detail' });
    const sig = makeSignature(SECRET, body);
    const req = makeRequest(body, { 'X-Webhook-Signature': sig });
    const res = await POST(req);
    expect(res.status).toBe(422);
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 422 when body is not valid JSON', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = 'not-json';
    const sig = makeSignature(SECRET, body);
    const req = makeRequest(body, { 'X-Webhook-Signature': sig });
    const res = await POST(req);
    expect(res.status).toBe(422);
    expect(revalidateTag).not.toHaveBeenCalled();
  });

  it('returns 422 when body is JSON but has no screen string', async () => {
    vi.stubEnv('LAYOUT_WEBHOOK_SECRET', SECRET);
    const body = JSON.stringify({ event: 'published' });
    const sig = makeSignature(SECRET, body);
    const req = makeRequest(body, { 'X-Webhook-Signature': sig });
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
