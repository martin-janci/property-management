/// <reference types="vitest/globals" />
/**
 * VotingPage building scope regression test.
 *
 * The page used to always list votes for `buildings[0]` of the organization.
 * It now honours `?building=`, offers a picker, and defaults residents to the
 * building they actually live in.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { buildVotingBuildingOptions } from '../hooks';
import { VotingPage } from './VotingPage';

const h = vi.hoisted(() => ({
  buildings: [] as Array<{ id: string; name: string }>,
  myUnits: [] as Array<{ building: { id: string; name: string | null } }>,
  votesQueries: [] as Array<Record<string, unknown>>,
}));

vi.mock('@ppt/api-client', () => ({
  useBuildings: () => ({ data: { items: h.buildings }, isLoading: false }),
  useMyUnits: () => ({ data: h.myUnits, isLoading: false }),
  useVotes: (query: Record<string, unknown>) => {
    h.votesQueries.push(query);
    return { data: [], isLoading: false, error: null };
  },
}));

let lastSearch = '';
function LocationSpy() {
  lastSearch = useLocation().search;
  return null;
}

function renderAt(url: string) {
  return render(
    <MemoryRouter initialEntries={[url]}>
      <Routes>
        <Route
          path="/voting"
          element={
            <>
              <VotingPage onOpenVote={() => {}} onCreateVote={() => {}} />
              <LocationSpy />
            </>
          }
        />
      </Routes>
    </MemoryRouter>
  );
}

const lastBuildingQueried = () => h.votesQueries[h.votesQueries.length - 1]?.building_id;

beforeEach(() => {
  h.buildings = [
    { id: 'b-1', name: 'Hlavná 1' },
    { id: 'b-2', name: 'Dlhá 2' },
  ];
  h.myUnits = [];
  h.votesQueries = [];
  lastSearch = '';
});

describe('VotingPage building selection', () => {
  it('defaults to the first org building when nothing is chosen', () => {
    renderAt('/voting');
    expect(lastBuildingQueried()).toBe('b-1');
    expect(screen.getByLabelText('Building')).toHaveValue('b-1');
  });

  it('uses the building from ?building=', () => {
    renderAt('/voting?building=b-2');
    expect(lastBuildingQueried()).toBe('b-2');
    expect(screen.getByLabelText('Building')).toHaveValue('b-2');
  });

  it('ignores an unknown ?building= value', () => {
    renderAt('/voting?building=nope');
    expect(lastBuildingQueried()).toBe('b-1');
  });

  it('writes the selection to the URL and refetches for that building', () => {
    renderAt('/voting');
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b-2' } });
    expect(lastSearch).toBe('?building=b-2');
    expect(lastBuildingQueried()).toBe('b-2');
  });

  it('defaults a resident to the building they live in, marked as theirs', () => {
    h.myUnits = [{ building: { id: 'b-2', name: 'Dlhá 2' } }];
    renderAt('/voting');
    expect(lastBuildingQueried()).toBe('b-2');
    expect(screen.getByRole('option', { name: 'Dlhá 2 (my building)' })).toBeInTheDocument();
  });
});

describe('buildVotingBuildingOptions', () => {
  it('lists own buildings first, dedupes, and keeps own buildings the org list lacks', () => {
    const opts = buildVotingBuildingOptions(
      [
        { id: 'a', name: 'A' },
        { id: 'b', name: 'B' },
      ],
      [
        { id: 'b', name: 'B' },
        { id: 'c', name: null, street: 'Krátka 3', city: 'Košice' },
      ]
    );
    expect(opts).toEqual([
      { id: 'b', label: 'B', isMine: true },
      { id: 'c', label: 'Krátka 3, Košice', isMine: true },
      { id: 'a', label: 'A', isMine: false },
    ]);
  });

  it('labels by address when the building has no name', () => {
    const [opt] = buildVotingBuildingOptions(
      [{ id: 'x', name: '', address: { street: 'Nová 5', city: 'Žilina' } }],
      []
    );
    expect(opt.label).toBe('Nová 5, Žilina');
  });
});
