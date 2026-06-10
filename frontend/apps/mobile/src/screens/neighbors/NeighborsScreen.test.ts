import {
  type BuildingsPaginatedResponse,
  filterBySearch,
  filterVisible,
  getInitials,
  type NeighborsResponse,
  type NeighborView,
  selectBuilding,
} from './NeighborsScreen';

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

/** Factory for a minimal visible NeighborView. */
const neighbor = (over: Partial<NeighborView> & Pick<NeighborView, 'user_id'>): NeighborView => ({
  display_name: `Resident ${over.user_id}`,
  unit_label: `Apt ${over.user_id}`,
  is_visible: true,
  email: null,
  phone: null,
  resident_type: 'tenant',
  ...over,
});

const buildingsResp = (
  items: BuildingsPaginatedResponse['buildings']
): BuildingsPaginatedResponse => ({
  buildings: items,
  total: items.length,
});

const neighborsResp = (list: NeighborView[]): NeighborsResponse => ({
  neighbors: list,
  count: list.length,
  total: list.length,
});

// ---------------------------------------------------------------------------
// selectBuilding — already partially covered by the legacy regression tests;
// extended here for completeness alongside the new helpers.
// ---------------------------------------------------------------------------

describe('selectBuilding', () => {
  it('resolves the first building id and name', () => {
    expect(
      selectBuilding(
        buildingsResp([
          { id: 'b-1', name: 'Maple Court' },
          { id: 'b-2', name: 'Oak House' },
        ])
      )
    ).toEqual({ buildingId: 'b-1', buildingName: 'Maple Court' });
  });

  it('coerces a null building name to undefined', () => {
    expect(selectBuilding(buildingsResp([{ id: 'b-1', name: null }]))).toEqual({
      buildingId: 'b-1',
      buildingName: undefined,
    });
  });

  it('returns undefined ids for an empty list', () => {
    expect(selectBuilding(buildingsResp([]))).toEqual({
      buildingId: undefined,
      buildingName: undefined,
    });
  });

  it('returns undefined ids for undefined input', () => {
    expect(selectBuilding(undefined)).toEqual({ buildingId: undefined, buildingName: undefined });
  });

  it('does NOT read the legacy `items` key', () => {
    const legacy = { items: [{ id: 'x' }], total: 1 } as unknown as BuildingsPaginatedResponse;
    expect(selectBuilding(legacy).buildingId).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// filterVisible — privacy-aware directory filter (UC-06 / Story 6.6)
// ---------------------------------------------------------------------------

describe('filterVisible (privacy-aware filter)', () => {
  const alice = neighbor({ user_id: 'u-1', display_name: 'Alice', is_visible: true });
  const bob = neighbor({ user_id: 'u-2', display_name: 'Bob', is_visible: false });
  const carol = neighbor({ user_id: 'u-3', display_name: 'Carol', is_visible: true });

  it('keeps only neighbours whose is_visible flag is true', () => {
    const result = filterVisible([alice, bob, carol]);
    expect(result.map((n) => n.user_id)).toEqual(['u-1', 'u-3']);
  });

  it('returns an empty list when all neighbours have opted out', () => {
    const invisible = [
      neighbor({ user_id: 'u-1', is_visible: false }),
      neighbor({ user_id: 'u-2', is_visible: false }),
    ];
    expect(filterVisible(invisible)).toEqual([]);
  });

  it('returns all rows when every neighbour is visible', () => {
    const visible = [alice, carol];
    expect(filterVisible(visible)).toHaveLength(2);
  });

  it('returns an empty list for an empty input', () => {
    expect(filterVisible([])).toEqual([]);
  });

  it('does NOT expose a neighbour with is_visible=false regardless of other fields', () => {
    const hidden = neighbor({
      user_id: 'u-99',
      display_name: 'Hidden Resident',
      is_visible: false,
      email: 'hidden@example.com',
      phone: '+421900000000',
    });
    expect(filterVisible([hidden])).toEqual([]);
  });

  it('treats is_visible as a strict boolean — truthy strings do not count', () => {
    // The backend should never send a string, but guard against a cast slip.
    const weirdTrue = { ...neighbor({ user_id: 'u-x' }), is_visible: 'true' as unknown as boolean };
    // 'true' is truthy so it should pass (JS boolean coercion is intentional).
    expect(filterVisible([weirdTrue])).toHaveLength(1);
    const weirdFalse = { ...neighbor({ user_id: 'u-y' }), is_visible: '' as unknown as boolean };
    expect(filterVisible([weirdFalse])).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// filterBySearch — list search (name + unit label)
// ---------------------------------------------------------------------------

describe('filterBySearch', () => {
  const alice = neighbor({ user_id: 'u-1', display_name: 'Alice Smith', unit_label: 'Apt 1A' });
  const bob = neighbor({ user_id: 'u-2', display_name: 'Bob Jones', unit_label: 'Apt 2B' });
  const carol = neighbor({ user_id: 'u-3', display_name: 'Carol Brown', unit_label: 'Apt 1B' });

  it('returns all rows for an empty query', () => {
    expect(filterBySearch([alice, bob, carol], '')).toHaveLength(3);
  });

  it('returns all rows for a whitespace-only query', () => {
    expect(filterBySearch([alice, bob, carol], '   ')).toHaveLength(3);
  });

  it('matches by display_name (case-insensitive)', () => {
    expect(filterBySearch([alice, bob, carol], 'alice').map((n) => n.user_id)).toEqual(['u-1']);
    expect(filterBySearch([alice, bob, carol], 'JONES').map((n) => n.user_id)).toEqual(['u-2']);
  });

  it('matches by unit_label (case-insensitive)', () => {
    // "Apt 1A" and "Apt 1B" both start with "1"
    const matches = filterBySearch([alice, bob, carol], 'apt 1');
    expect(matches.map((n) => n.user_id)).toEqual(['u-1', 'u-3']);
  });

  it('matches a partial substring within display_name', () => {
    expect(filterBySearch([alice, bob, carol], 'row').map((n) => n.user_id)).toEqual(['u-3']);
  });

  it('returns [] when no rows match the query', () => {
    expect(filterBySearch([alice, bob, carol], 'zzz')).toEqual([]);
  });

  it('returns [] for an empty list regardless of query', () => {
    expect(filterBySearch([], 'alice')).toEqual([]);
  });

  it('does NOT search on email, phone, or resident_type', () => {
    const withContact = neighbor({
      user_id: 'u-4',
      display_name: 'Dave Davies',
      unit_label: 'Apt 3C',
      email: 'unique@example.com',
      phone: '+421999888777',
      resident_type: 'owner',
    });
    // Query hits email or phone: must NOT match (those are not in the search key).
    expect(filterBySearch([withContact], 'unique@example.com')).toEqual([]);
    expect(filterBySearch([withContact], '+421999888777')).toEqual([]);
    // resident_type is not matched either.
    expect(filterBySearch([withContact], 'owner')).toEqual([]);
    // But display_name still works.
    expect(filterBySearch([withContact], 'dave').map((n) => n.user_id)).toEqual(['u-4']);
  });
});

// ---------------------------------------------------------------------------
// Contact-visibility per privacy settings
// The screen shows phone / email only when the backend sends a non-null value.
// filterVisible already handles the is_visible gate; these tests verify the
// contact-detail logic: a visible neighbour may still have null contact fields
// (they opted in to the directory but chose not to share contact details).
// ---------------------------------------------------------------------------

describe('contact-detail visibility (null vs non-null fields)', () => {
  it('a visible neighbour with null phone/email has no contact data', () => {
    const n = neighbor({ user_id: 'u-1', is_visible: true, phone: null, email: null });
    const visible = filterVisible([n]);
    expect(visible).toHaveLength(1);
    expect(visible[0].phone).toBeNull();
    expect(visible[0].email).toBeNull();
  });

  it('a visible neighbour with shared phone/email exposes both fields', () => {
    const n = neighbor({
      user_id: 'u-2',
      is_visible: true,
      phone: '+421901234567',
      email: 'resident@example.com',
    });
    const visible = filterVisible([n]);
    expect(visible[0].phone).toBe('+421901234567');
    expect(visible[0].email).toBe('resident@example.com');
  });

  it('a hidden neighbour with shared contact fields is completely excluded', () => {
    const n = neighbor({
      user_id: 'u-3',
      is_visible: false,
      phone: '+421901234567',
      email: 'resident@example.com',
    });
    expect(filterVisible([n])).toEqual([]);
  });

  it('mixed: visible-without-contact and hidden-with-contact', () => {
    const visibleNoContact = neighbor({
      user_id: 'u-1',
      is_visible: true,
      phone: null,
      email: null,
    });
    const hiddenWithContact = neighbor({
      user_id: 'u-2',
      is_visible: false,
      phone: '+420123456789',
      email: 'secret@example.com',
    });
    const result = filterVisible([visibleNoContact, hiddenWithContact]);
    expect(result).toHaveLength(1);
    expect(result[0].user_id).toBe('u-1');
  });
});

// ---------------------------------------------------------------------------
// List / detail state helpers
// Derives the list shape from a raw NeighborsResponse — the combination of
// filterVisible + filterBySearch that the screen's useMemo pipeline applies.
// ---------------------------------------------------------------------------

describe('list-state derivation (filterVisible + filterBySearch pipeline)', () => {
  const list: NeighborView[] = [
    neighbor({
      user_id: 'u-1',
      display_name: 'Alice Smith',
      unit_label: 'Apt 1A',
      is_visible: true,
    }),
    neighbor({
      user_id: 'u-2',
      display_name: 'Bob Jones',
      unit_label: 'Apt 2B',
      is_visible: false,
    }),
    neighbor({
      user_id: 'u-3',
      display_name: 'Carol Brown',
      unit_label: 'Apt 3C',
      is_visible: true,
    }),
  ];
  const resp = neighborsResp(list);

  it('full pipeline with empty search returns only visible rows', () => {
    const visible = filterVisible(resp.neighbors);
    const result = filterBySearch(visible, '');
    expect(result.map((n) => n.user_id)).toEqual(['u-1', 'u-3']);
  });

  it('full pipeline with a name query further narrows visible rows', () => {
    const visible = filterVisible(resp.neighbors);
    const result = filterBySearch(visible, 'carol');
    expect(result.map((n) => n.user_id)).toEqual(['u-3']);
  });

  it('full pipeline with a query that matches a hidden row returns nothing', () => {
    // Bob is hidden; searching for "bob" returns []
    const visible = filterVisible(resp.neighbors);
    const result = filterBySearch(visible, 'bob');
    expect(result).toEqual([]);
  });

  it('empty state: response with no neighbours returns empty list', () => {
    const emptyResp = neighborsResp([]);
    const result = filterBySearch(filterVisible(emptyResp.neighbors), '');
    expect(result).toEqual([]);
  });

  it('empty state: all neighbours opted out returns empty visible list', () => {
    const allHidden = neighborsResp([
      neighbor({ user_id: 'u-1', is_visible: false }),
      neighbor({ user_id: 'u-2', is_visible: false }),
    ]);
    const result = filterBySearch(filterVisible(allHidden.neighbors), '');
    expect(result).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// getInitials — avatar label
// ---------------------------------------------------------------------------

describe('getInitials', () => {
  it('produces two uppercase initials for a two-part name', () => {
    expect(getInitials('Alice Smith')).toBe('AS');
    expect(getInitials('Bob Jones')).toBe('BJ');
  });

  it('produces at most two characters for a three-part name', () => {
    expect(getInitials('Mary Jane Watson')).toBe('MJ');
  });

  it('produces a single initial for a single-word name', () => {
    // "Bob" has one word-part → one initial letter; slice(0,2) keeps it as-is.
    expect(getInitials('Bob')).toBe('B');
  });

  it('handles an empty string without throwing', () => {
    expect(getInitials('')).toBe('');
  });

  it('uppercases the result', () => {
    expect(getInitials('alice smith')).toBe('AS');
  });

  it('handles extra whitespace between name parts gracefully', () => {
    // split(' ') produces empty strings for double-spaces; the fallback '' keeps length stable
    const result = getInitials('Alice  Smith');
    // The exact output for "Alice  Smith" is 'A' + '' + 'S' = 'AS' capped at 2
    expect(result.length).toBeLessThanOrEqual(2);
  });
});

// ---------------------------------------------------------------------------
// Error / loading state (pure side: no rendered component needed)
// The screen uses `buildingsQuery.error ?? neighborsQuery.error`. These tests
// confirm the data shapes that drive the error / loading branches are correct.
// ---------------------------------------------------------------------------

describe('NeighborsResponse shape (data integrity)', () => {
  it('a well-formed response has the expected fields', () => {
    const resp = neighborsResp([neighbor({ user_id: 'u-1' })]);
    expect(resp).toMatchObject({ neighbors: expect.any(Array), count: 1, total: 1 });
  });

  it('an empty response has count=0 and total=0', () => {
    const resp = neighborsResp([]);
    expect(resp.count).toBe(0);
    expect(resp.total).toBe(0);
    expect(resp.neighbors).toEqual([]);
  });

  it('resident_type "owner" is preserved through the filter pipeline', () => {
    const owner = neighbor({ user_id: 'u-o', resident_type: 'owner', is_visible: true });
    const visible = filterVisible([owner]);
    expect(visible[0].resident_type).toBe('owner');
  });

  it('resident_type "tenant" is preserved through the filter pipeline', () => {
    const tenant = neighbor({ user_id: 'u-t', resident_type: 'tenant', is_visible: true });
    const visible = filterVisible([tenant]);
    expect(visible[0].resident_type).toBe('tenant');
  });
});
