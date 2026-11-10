/// <reference types="vitest/globals" />
/**
 * FacilitiesPage fetch-error regression (code-review finding
 * code-review-ppt-web-ui-facilities-fetch-no-error-ui).
 *
 * The page's `listFacilities` call swallowed its rejection in a bare
 * `catch` that only `console.error`'d, leaving `facilities` as `[]`. The
 * `FacilityList` then rendered its "No facilities found." empty state, so a
 * genuine fetch failure (500, network drop, auth loss) was indistinguishable
 * from a building that legitimately has no facilities — the outage was hidden
 * from the user.
 *
 * The fix flags the error and renders a dedicated error state (title +
 * description + retry) instead of the empty placeholder. These tests pin that:
 *
 *  - on an API rejection the error title is shown and the empty-state copy is
 *    NOT (fails on `main`, where the empty state leaks through);
 *  - the retry button re-issues the fetch and recovers once it succeeds.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import en from '../../../../messages/en.json';
import '../../../i18n';
import { FacilitiesPage } from './FacilitiesPage';

// ─── network boundary: @ppt/api-client ──────────────────────────────────────
const listFacilities = vi.fn();

vi.mock('@ppt/api-client', () => ({
  listFacilities: (...args: unknown[]) => listFacilities(...args),
}));

function renderPage() {
  return render(
    <MemoryRouter initialEntries={['/buildings/b-1/facilities']}>
      <Routes>
        <Route path="/buildings/:buildingId/facilities" element={<FacilitiesPage />} />
      </Routes>
    </MemoryRouter>
  );
}

const ERR = en.facilities.errors;
const EMPTY = 'No facilities found.';

describe('FacilitiesPage fetch error state', () => {
  beforeEach(() => {
    listFacilities.mockReset();
  });

  it('shows a distinct error state (not the empty placeholder) when the fetch fails', async () => {
    listFacilities.mockRejectedValueOnce(new Error('boom: GET /facilities 500'));

    renderPage();

    expect(await screen.findByText(ERR.fetchFailedTitle)).toBeInTheDocument();
    expect(screen.getByText(ERR.fetchFailedDescription)).toBeInTheDocument();
    // The empty-state copy must NOT be shown for a failed fetch.
    expect(screen.queryByText(EMPTY)).not.toBeInTheDocument();
  });

  it('shows the empty placeholder (not the error state) when the fetch succeeds with no items', async () => {
    listFacilities.mockResolvedValueOnce({ items: [], total: 0 });

    renderPage();

    expect(await screen.findByText(EMPTY)).toBeInTheDocument();
    expect(screen.queryByText(ERR.fetchFailedTitle)).not.toBeInTheDocument();
  });

  it('retries the fetch when the retry button is clicked', async () => {
    listFacilities
      .mockRejectedValueOnce(new Error('transient'))
      .mockResolvedValueOnce({ items: [], total: 0 });

    renderPage();

    const retry = await screen.findByRole('button', { name: ERR.retry });
    fireEvent.click(retry);

    await waitFor(() => expect(screen.getByText(EMPTY)).toBeInTheDocument());
    expect(screen.queryByText(ERR.fetchFailedTitle)).not.toBeInTheDocument();
    expect(listFacilities).toHaveBeenCalledTimes(2);
  });
});
