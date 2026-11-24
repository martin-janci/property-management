/** The list maps the server's `FaultSummary` and its real status enum. */
import { matchesFaultFilter, toUiFault } from './FaultsListScreen';

const summary = {
  id: 'f-1',
  building_id: 'b-1',
  unit_id: null,
  title: 'Leaking pipe',
  category: 'plumbing',
  priority: 'high',
  status: 'scheduled',
  created_at: '2026-11-20T10:00:00Z',
};

describe('toUiFault', () => {
  it('keeps the server status instead of collapsing it to "open"', () => {
    expect(toUiFault(summary).status).toBe('scheduled');
    expect(toUiFault({ ...summary, status: 'waiting_parts' }).status).toBe('waiting_parts');
  });

  it('falls back to created_at because the summary has no updated_at', () => {
    expect(toUiFault(summary).updatedAt).toBe('2026-11-20T10:00:00Z');
  });

  it('narrows unknown enum values to safe defaults', () => {
    const fault = toUiFault({ ...summary, status: 'bogus', priority: 'x', category: 'y' });
    expect(fault).toMatchObject({ status: 'new', priority: 'medium', category: 'other' });
  });
});

describe('matchesFaultFilter', () => {
  it('treats everything not resolved or closed as open', () => {
    for (const status of [
      'new',
      'triaged',
      'in_progress',
      'waiting_parts',
      'scheduled',
      'reopened',
    ] as const) {
      expect(matchesFaultFilter(status, 'open')).toBe(true);
      expect(matchesFaultFilter(status, 'resolved')).toBe(false);
    }
    for (const status of ['resolved', 'closed'] as const) {
      expect(matchesFaultFilter(status, 'open')).toBe(false);
      expect(matchesFaultFilter(status, 'resolved')).toBe(true);
      expect(matchesFaultFilter(status, 'all')).toBe(true);
    }
  });
});
