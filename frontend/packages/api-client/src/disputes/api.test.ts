import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../auth', () => ({ bearerAuthHeader: () => ({ Authorization: 'Bearer t' }) }));

import {
  assignMediator,
  completeActionItem,
  composeResolutionNotes,
  createActionItem,
  createDispute,
  escalateDispute,
  getDispute,
  getDisputeStatistics,
  getResolution,
  getTimeline,
  listDisputes,
  listEvidence,
  proposeResolution,
  resolveDispute,
  scheduleSession,
  sendActionReminder,
  updateDisputeStatus,
  voteOnResolution,
} from './api';

const row = (i: number) => ({
  id: `d${i}`,
  reference_number: `DSP-${i}`,
  category: 'noise',
  title: `Dispute ${i}`,
  status: 'filed',
  priority: 'high',
  filed_by_name: 'Jana',
  assigned_to_name: null,
  party_count: 2,
  created_at: '2026-11-01T00:00:00Z',
  updated_at: '2026-11-02T00:00:00Z',
});

const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});
afterEach(() => {
  vi.unstubAllGlobals();
});

function respond(body: unknown) {
  fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => body });
}

describe('listDisputes', () => {
  it('calls GET /api/v1/disputes with snake_case filters and limit+1', async () => {
    respond([]);
    await listDisputes('org-1', {
      status: 'filed',
      category: 'noise',
      priority: 'high',
      search: ' loud ',
      page: 2,
      limit: 10,
    });
    const url = new URL(fetchMock.mock.calls[0][0], 'http://x');
    expect(url.pathname).toBe('/api/v1/disputes');
    expect(Object.fromEntries(url.searchParams)).toEqual({
      organization_id: 'org-1',
      status: 'filed',
      category: 'noise',
      priority: 'high',
      search: 'loud',
      limit: '11',
      offset: '10',
    });
  });

  it('maps rows to camelCase and detects another page', async () => {
    respond([row(1), row(2), row(3)]);
    const res = await listDisputes('org-1', { page: 1, limit: 2 });
    expect(res.hasMore).toBe(true);
    expect(res.data).toHaveLength(2);
    expect(res.total).toBe(3);
    expect(res.data[0]).toEqual({
      id: 'd1',
      referenceNumber: 'DSP-1',
      category: 'noise',
      title: 'Dispute 1',
      status: 'filed',
      priority: 'high',
      filedByName: 'Jana',
      assignedToName: undefined,
      partyCount: 2,
      createdAt: '2026-11-01T00:00:00Z',
      updatedAt: '2026-11-02T00:00:00Z',
    });
  });

  it('reports an exact total on the last page', async () => {
    respond([row(1)]);
    const res = await listDisputes('org-1', { page: 3, limit: 10 });
    expect(res.hasMore).toBe(false);
    expect(res.total).toBe(21);
  });
});

const disputeWire = {
  id: 'd1',
  organization_id: 'org-1',
  building_id: 'b1',
  unit_id: null,
  reference_number: 'DSP-2026-00001',
  category: 'lease_terms',
  title: 'Deposit',
  description: 'Deposit not returned',
  desired_resolution: null,
  status: 'filed',
  priority: 'medium',
  filed_by: 'u1',
  assigned_to: null,
  resolved_at: null,
  resolution_notes: null,
  mediation_notes: null,
  created_at: '2026-11-01T00:00:00Z',
  updated_at: '2026-11-01T00:00:00Z',
};

/** The [url, init] of the n-th fetch call, with the body parsed. */
function call(n = 0) {
  const [url, init] = fetchMock.mock.calls[n] as [string, RequestInit | undefined];
  return {
    url,
    method: init?.method ?? 'GET',
    body: init?.body ? JSON.parse(String(init.body)) : undefined,
  };
}

describe('dispute detail', () => {
  it('getDispute calls GET /disputes/{id} and flattens dispute + parties', async () => {
    respond({
      dispute: disputeWire,
      parties: [
        {
          party: {
            id: 'p1',
            dispute_id: 'd1',
            user_id: 'u1',
            role: 'complainant',
            notified_at: null,
            responded_at: null,
            created_at: '2026-11-01T00:00:00Z',
          },
          user_name: 'Jana Nová',
          user_email: 'jana@example.com',
        },
      ],
      evidence_count: 2,
      activity_count: 3,
      session_count: 0,
      active_resolution: null,
      pending_actions: [],
    });
    const d = await getDispute('d1');
    expect(call()).toMatchObject({ url: '/api/v1/disputes/d1', method: 'GET' });
    expect(d).toMatchObject({
      id: 'd1',
      organizationId: 'org-1',
      buildingId: 'b1',
      unitId: undefined,
      referenceNumber: 'DSP-2026-00001',
      category: 'lease_terms',
      title: 'Deposit',
      status: 'filed',
      priority: 'medium',
      filedBy: 'u1',
      evidenceCount: 2,
      activityCount: 3,
      sessionCount: 0,
    });
    expect(d.parties).toEqual([
      {
        id: 'p1',
        userId: 'u1',
        role: 'complainant',
        userName: 'Jana Nová',
        userEmail: 'jana@example.com',
        notifiedAt: undefined,
        respondedAt: undefined,
        createdAt: '2026-11-01T00:00:00Z',
      },
    ]);
  });

  it('getTimeline reads /activities and maps activity_type → eventType', async () => {
    respond([
      {
        id: 'a1',
        dispute_id: 'd1',
        actor_id: 'u1',
        activity_type: 'dispute_filed',
        description: 'Filed',
        metadata: null,
        created_at: '2026-11-01T00:00:00Z',
      },
    ]);
    const events = await getTimeline('d1', { limit: 20 });
    expect(call().url).toBe('/api/v1/disputes/d1/activities?limit=20');
    expect(events).toEqual([
      {
        id: 'a1',
        disputeId: 'd1',
        eventType: 'dispute_filed',
        actorId: 'u1',
        description: 'Filed',
        metadata: undefined,
        createdAt: '2026-11-01T00:00:00Z',
      },
    ]);
  });

  it('listEvidence maps the snake_case evidence rows', async () => {
    respond([
      {
        id: 'e1',
        dispute_id: 'd1',
        uploaded_by: 'u1',
        filename: 'k/x.jpg',
        original_filename: 'x.jpg',
        content_type: 'image/jpeg',
        size_bytes: 10,
        storage_url: 'https://s3/x.jpg',
        description: null,
        created_at: '2026-11-01T00:00:00Z',
      },
    ]);
    const [e] = await listEvidence('d1');
    expect(e).toMatchObject({
      uploadedBy: 'u1',
      originalFilename: 'x.jpg',
      contentType: 'image/jpeg',
      sizeBytes: 10,
      storageUrl: 'https://s3/x.jpg',
      description: undefined,
    });
  });

  it('getDisputeStatistics reads /statistics and folds the count rows', async () => {
    respond({
      total_disputes: 5,
      by_status: [{ status: 'filed', count: 3 }],
      by_category: [{ category: 'noise', count: 5 }],
      by_priority: [{ priority: 'high', count: 1 }],
      avg_resolution_days: null,
      pending_actions: 2,
      overdue_actions: 1,
    });
    const s = await getDisputeStatistics('org-1');
    expect(call().url).toBe('/api/v1/disputes/statistics?organization_id=org-1');
    expect(s).toEqual({
      totalDisputes: 5,
      byStatus: { filed: 3 },
      byCategory: { noise: 5 },
      byPriority: { high: 1 },
      averageResolutionDays: undefined,
      pendingActions: 2,
      overdueActions: 1,
    });
  });
});

describe('dispute mutations', () => {
  it('createDispute POSTs /disputes with the snake_case FileDisputeRequest body', async () => {
    respond(disputeWire);
    const d = await createDispute('org-1', {
      category: 'noise',
      title: 'Loud',
      description: 'Very loud music every night',
      unitId: 'unit-1',
      buildingId: 'b1',
      respondentIds: ['u2'],
    });
    expect(call()).toEqual({
      url: '/api/v1/disputes',
      method: 'POST',
      body: {
        organization_id: 'org-1',
        category: 'noise',
        title: 'Loud',
        description: 'Very loud music every night',
        building_id: 'b1',
        unit_id: 'unit-1',
        respondent_ids: ['u2'],
      },
    });
    expect(d.id).toBe('d1');
    expect(d.referenceNumber).toBe('DSP-2026-00001');
  });

  it('createDispute always sends respondent_ids', async () => {
    respond(disputeWire);
    await createDispute('org-1', { category: 'other', title: 'T', description: 'D' });
    expect(call().body.respondent_ids).toEqual([]);
    expect(call().body).not.toHaveProperty('unit_id');
  });

  it('updateDisputeStatus PATCHes /disputes/{id}', async () => {
    respond({ ...disputeWire, status: 'under_review' });
    const d = await updateDisputeStatus('d1', { status: 'under_review', reason: '  ' });
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1',
      method: 'PATCH',
      body: { status: 'under_review' },
    });
    expect(d.status).toBe('under_review');
  });

  it('resolveDispute PATCHes /resolve with composed resolution_notes', async () => {
    respond({ ...disputeWire, status: 'resolved' });
    const data = {
      resolutionType: 'mutual_agreement' as const,
      resolutionDetails: ' Both agree to quiet hours. ',
      terms: 'Quiet after 22:00',
      requiresConfirmation: false,
    };
    await resolveDispute('d1', data);
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/resolve',
      method: 'PATCH',
      body: { resolution_notes: composeResolutionNotes(data) },
    });
    expect(composeResolutionNotes(data)).toBe(
      '[mutual_agreement] Both agree to quiet hours.\n\nQuiet after 22:00'
    );
  });

  it('escalateDispute POSTs /escalations with a severity', async () => {
    respond({
      id: 'x1',
      dispute_id: 'd1',
      action_item_id: null,
      escalated_by: 'u1',
      escalated_to: null,
      reason: 'No progress',
      severity: 'formal_notice',
      resolved: false,
      resolved_at: null,
      resolution_notes: null,
      created_at: '2026-11-01T00:00:00Z',
      updated_at: '2026-11-01T00:00:00Z',
    });
    const e = await escalateDispute('d1', { reason: 'No progress' });
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/escalations',
      method: 'POST',
      body: { reason: 'No progress', severity: 'formal_notice' },
    });
    expect(e).toMatchObject({ id: 'x1', severity: 'formal_notice', resolved: false });
  });

  it('assignMediator adds a mediator party', async () => {
    respond({
      id: 'p9',
      dispute_id: 'd1',
      user_id: 'u9',
      role: 'mediator',
      notified_at: null,
      responded_at: null,
      created_at: '2026-11-01T00:00:00Z',
    });
    const p = await assignMediator('d1', { mediatorId: 'u9' });
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/parties',
      method: 'POST',
      body: { user_id: 'u9', role: 'mediator' },
    });
    expect(p).toMatchObject({ id: 'p9', userId: 'u9', role: 'mediator' });
  });

  it('scheduleSession fills the flattened fields the backend requires', async () => {
    respond({});
    await scheduleSession('d1', { session_type: 'phone', scheduled_at: '2026-12-01T10:00:00Z' });
    expect(call().body).toEqual({
      session_type: 'phone',
      scheduled_at: '2026-12-01T10:00:00Z',
      dispute_id: 'd1',
      mediator_id: '00000000-0000-0000-0000-000000000000',
      attendee_party_ids: [],
    });
  });
});

describe('resolutions and action items', () => {
  const resolutionWire = {
    id: 'r1',
    dispute_id: 'd1',
    proposed_by: 'u1',
    resolution_text: 'Split',
    terms: [
      {
        id: 'term-1',
        description: 'Pay half',
        responsible_party_id: null,
        deadline: '2026-12-01T23:59:59Z',
        completed: false,
        completed_at: null,
      },
    ],
    status: 'proposed',
    proposed_at: '2026-11-01T00:00:00Z',
    accepted_at: null,
    implemented_at: null,
    created_at: '2026-11-01T00:00:00Z',
    updated_at: '2026-11-01T00:00:00Z',
  };

  it('proposeResolution sends terms with ids and end-of-day deadlines', async () => {
    respond(resolutionWire);
    const r = await proposeResolution('d1', {
      resolutionText: 'Split',
      terms: [{ description: 'Pay half', deadline: '2026-12-01' }],
    });
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/resolutions',
      method: 'POST',
      body: {
        dispute_id: 'd1',
        proposed_by: '00000000-0000-0000-0000-000000000000',
        resolution_text: 'Split',
        terms: [
          {
            id: 'term-1',
            description: 'Pay half',
            deadline: '2026-12-01T23:59:59Z',
            completed: false,
          },
        ],
      },
    });
    expect(r.terms[0]).toMatchObject({ id: 'term-1', deadline: '2026-12-01T23:59:59Z' });
  });

  it('getResolution maps votes and the acceptance rate', async () => {
    respond({
      resolution: resolutionWire,
      votes: [
        {
          id: 'v1',
          resolution_id: 'r1',
          party_id: 'p1',
          accepted: true,
          comments: null,
          voted_at: '2026-11-02T00:00:00Z',
        },
      ],
      acceptance_rate: 100,
    });
    const r = await getResolution('d1', 'r1');
    expect(call().url).toBe('/api/v1/disputes/d1/resolutions/r1');
    expect(r.acceptanceRate).toBe(100);
    expect(r.votes[0]).toEqual({
      id: 'v1',
      resolutionId: 'r1',
      partyId: 'p1',
      accepted: true,
      comments: undefined,
      votedAt: '2026-11-02T00:00:00Z',
    });
  });

  it('voteOnResolution POSTs /vote', async () => {
    respond({
      id: 'v1',
      resolution_id: 'r1',
      party_id: 'p1',
      accepted: false,
      comments: 'no',
      voted_at: 'x',
    });
    await voteOnResolution('d1', 'r1', { accepted: false, comments: 'no' });
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/resolutions/r1/vote',
      method: 'POST',
      body: { accepted: false, comments: 'no' },
    });
  });

  it('createActionItem sends dispute_id and a full due_date timestamp', async () => {
    respond({
      id: 'a1',
      dispute_id: 'd1',
      assigned_to: 'u2',
      title: 'Fix',
      description: 'Fix it',
      due_date: '2026-12-01T23:59:59Z',
      status: 'pending',
      created_at: 'x',
      updated_at: 'x',
    });
    const a = await createActionItem('d1', {
      title: 'Fix',
      description: 'Fix it',
      assignedTo: 'u2',
      dueDate: '2026-12-01',
    });
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/actions',
      method: 'POST',
      body: {
        dispute_id: 'd1',
        assigned_to: 'u2',
        title: 'Fix',
        description: 'Fix it',
        due_date: '2026-12-01T23:59:59Z',
      },
    });
    expect(a).toMatchObject({ id: 'a1', assignedTo: 'u2', status: 'pending' });
  });

  it('completeActionItem and sendActionReminder hit their POST routes', async () => {
    respond({ id: 'a1', dispute_id: 'd1', assigned_to: 'u2', status: 'completed' });
    await completeActionItem('d1', 'a1', ' done ');
    expect(call()).toEqual({
      url: '/api/v1/disputes/d1/actions/a1/complete',
      method: 'POST',
      body: { completion_notes: 'done' },
    });
    fetchMock.mockResolvedValue({ ok: true, status: 204, json: async () => ({}) });
    await sendActionReminder('d1', 'a1');
    expect(call(1)).toMatchObject({ url: '/api/v1/disputes/d1/actions/a1/remind', method: 'POST' });
  });
});
