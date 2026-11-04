/**
 * Contract tests for the hand-rolled leases / violations API client.
 *
 * Pins the request wiring (URL, HTTP method, JSON body) of every action the
 * ppt-web leases routes call against the live backend router
 * (`backend/servers/api-server/src/routes/leases.rs`, `violations.rs`,
 * `organizations/members.rs`), and that every call carries the bearer token —
 * the module previously used a bare `fetch` with no Authorization header, so
 * every lease / violation request was rejected with 401 by `AuthUser`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import {
  addViolationEvidence,
  createLeaseTemplate,
  createViolationAppeal,
  getLease,
  initiateScreening,
  listOrganizationMembers,
  listViolationEvidence,
  recordLeasePayment,
  reviewApplication,
  updateLeaseTemplate,
  updateViolation,
} from './api';

const ID = '11111111-1111-1111-1111-111111111111';
const ORG = '22222222-2222-2222-2222-222222222222';

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

function lastCall(): {
  url: string;
  method: string;
  body: unknown;
  headers: Record<string, string>;
} {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const init = (call[1] ?? {}) as RequestInit;
  return {
    url: String(call[0]),
    method: (init.method ?? 'GET').toUpperCase(),
    body: typeof init.body === 'string' ? JSON.parse(init.body) : undefined,
    headers: (init.headers ?? {}) as Record<string, string>,
  };
}

describe('leases api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({}));
    setTokenProvider(() => 'test-token');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    clearTokenProvider();
  });

  it('sends the bearer token on lease requests', async () => {
    await getLease(ID);
    const { url, headers } = lastCall();
    expect(url).toBe(`/api/v1/leases/${ID}`);
    expect(headers.Authorization).toBe('Bearer test-token');
  });

  it('recordLeasePayment → POST /api/v1/leases/{payment_id}/payments', async () => {
    const body = {
      paid_amount: 850,
      payment_method: 'bank_transfer',
      payment_reference: 'VS123',
      paid_at: '2026-11-01T00:00:00Z',
    };
    await recordLeasePayment(ID, body);
    const call = lastCall();
    expect(call.method).toBe('POST');
    expect(call.url).toBe(`/api/v1/leases/${ID}/payments`);
    expect(call.body).toEqual(body);
  });

  it('createLeaseTemplate → POST /api/v1/leases/templates', async () => {
    const body = { organization_id: ORG, name: 'Standard', content_html: '<p>x</p>' };
    await createLeaseTemplate(body);
    const call = lastCall();
    expect(call.method).toBe('POST');
    expect(call.url).toBe('/api/v1/leases/templates');
    expect(call.body).toEqual(body);
  });

  it('updateLeaseTemplate → PUT /api/v1/leases/templates/{id}', async () => {
    await updateLeaseTemplate(ID, { organization_id: ORG, is_active: false });
    const call = lastCall();
    expect(call.method).toBe('PUT');
    expect(call.url).toBe(`/api/v1/leases/templates/${ID}`);
    expect(call.body).toEqual({ organization_id: ORG, is_active: false });
  });

  it('reviewApplication → POST /api/v1/leases/applications/{id}/review', async () => {
    await reviewApplication(ID, { status: 'approved', decision_notes: 'ok' });
    const call = lastCall();
    expect(call.method).toBe('POST');
    expect(call.url).toBe(`/api/v1/leases/applications/${ID}/review`);
    expect(call.body).toEqual({ status: 'approved', decision_notes: 'ok' });
  });

  it('initiateScreening → POST /api/v1/leases/applications/{id}/screening', async () => {
    await initiateScreening(ID, { organization_id: ORG, screening_types: ['credit_check'] });
    const call = lastCall();
    expect(call.method).toBe('POST');
    expect(call.url).toBe(`/api/v1/leases/applications/${ID}/screening`);
    expect(call.body).toEqual({ organization_id: ORG, screening_types: ['credit_check'] });
  });

  it('listOrganizationMembers → GET /api/v1/organizations/{id}/members', async () => {
    await listOrganizationMembers(ORG);
    const call = lastCall();
    expect(call.method).toBe('GET');
    expect(call.url).toBe(`/api/v1/organizations/${ORG}/members`);
  });

  it('updateViolation → PUT /api/v1/violations/{id}', async () => {
    await updateViolation(ID, { status: 'resolved', resolution_notes: 'fixed' });
    const call = lastCall();
    expect(call.method).toBe('PUT');
    expect(call.url).toBe(`/api/v1/violations/${ID}`);
    expect(call.body).toEqual({ status: 'resolved', resolution_notes: 'fixed' });
  });

  it('createViolationAppeal → POST /api/v1/violations/{id}/appeals', async () => {
    await createViolationAppeal(ID, { reason: 'Not me' });
    const call = lastCall();
    expect(call.method).toBe('POST');
    expect(call.url).toBe(`/api/v1/violations/${ID}/appeals`);
    expect(call.body).toEqual({ reason: 'Not me' });
  });

  it('listViolationEvidence → GET /api/v1/violations/{id}/evidence', async () => {
    await listViolationEvidence(ID);
    const call = lastCall();
    expect(call.method).toBe('GET');
    expect(call.url).toBe(`/api/v1/violations/${ID}/evidence`);
  });

  it('addViolationEvidence → POST /api/v1/violations/{id}/evidence', async () => {
    const body = { file_name: 'a.jpg', file_type: 'image/jpeg', storage_path: 'org/a.jpg' };
    await addViolationEvidence(ID, body);
    const call = lastCall();
    expect(call.method).toBe('POST');
    expect(call.url).toBe(`/api/v1/violations/${ID}/evidence`);
    expect(call.body).toEqual(body);
  });

  it('surfaces the server error message on non-2xx', async () => {
    vi.mocked(fetch).mockResolvedValueOnce({
      ok: false,
      status: 403,
      json: async () => ({ code: 'FORBIDDEN', message: 'Only managers can review' }),
    } as Response);
    await expect(reviewApplication(ID, { status: 'rejected' })).rejects.toThrow(
      'Only managers can review'
    );
  });
});
