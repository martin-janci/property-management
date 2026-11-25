/**
 * The fault detail reads the server's `{ fault, timeline, attachments }`
 * envelope (snake_case, `FaultWithDetails` flattened into `fault`).
 */
import { FAULT_STATUS_LABEL_KEY, parseFaultDetail, parseFaultTimeline } from './FaultDetailScreen';

const envelope = {
  fault: {
    id: 'f-1',
    title: 'Leaking pipe',
    description: 'Water under the sink',
    location_description: 'Kitchen',
    category: 'plumbing',
    priority: 'high',
    status: 'waiting_parts',
    created_at: '2026-11-20T10:00:00Z',
    reporter_name: 'Jana Nova',
    assigned_to_name: null,
    building_name: 'Block A',
    unit_designation: '3B',
    resolution_notes: null,
  },
  timeline: [
    {
      id: 't-1',
      action: 'triaged',
      note: 'Plumber booked',
      user_name: 'Manager',
      is_internal: false,
      created_at: '2026-11-20T11:00:00Z',
    },
    {
      id: 't-2',
      action: 'comment',
      note: 'internal only',
      user_name: 'Manager',
      is_internal: true,
      created_at: '2026-11-20T12:00:00Z',
    },
    { action: 'comment' },
    null,
  ],
  attachments: [],
};

describe('parseFaultDetail', () => {
  it('reads the fault out of the envelope', () => {
    expect(parseFaultDetail(envelope)).toEqual({
      id: 'f-1',
      title: 'Leaking pipe',
      description: 'Water under the sink',
      location: 'Kitchen',
      category: 'plumbing',
      priority: 'high',
      status: 'waiting_parts',
      createdAt: '2026-11-20T10:00:00Z',
      reporterName: 'Jana Nova',
      assignedToName: '',
      buildingName: 'Block A',
      unitDesignation: '3B',
      resolutionNotes: '',
    });
  });

  it('returns null for a missing or malformed envelope', () => {
    expect(parseFaultDetail(undefined)).toBeNull();
    expect(parseFaultDetail({})).toBeNull();
    expect(parseFaultDetail({ fault: { id: 'f-1' } })).toBeNull();
    // A bare fault (no envelope) is not what the server sends.
    expect(parseFaultDetail(envelope.fault)).toBeNull();
  });
});

describe('parseFaultTimeline', () => {
  it('keeps public rows and drops internal or malformed ones', () => {
    expect(parseFaultTimeline(envelope)).toEqual([
      {
        id: 't-1',
        action: 'triaged',
        note: 'Plumber booked',
        userName: 'Manager',
        createdAt: '2026-11-20T11:00:00Z',
      },
    ]);
    expect(parseFaultTimeline({ fault: envelope.fault })).toEqual([]);
  });
});

describe('FAULT_STATUS_LABEL_KEY', () => {
  it('has a label for every server fault_status value', () => {
    for (const status of [
      'new',
      'triaged',
      'in_progress',
      'waiting_parts',
      'scheduled',
      'resolved',
      'closed',
      'reopened',
    ]) {
      expect(FAULT_STATUS_LABEL_KEY[status]).toMatch(/^faults\.status/);
    }
  });
});
