/// <reference types="vitest/globals" />
/**
 * FacilitiesPage manager-gating tests (code-review finding
 * code-review-ppt-web-ui-facilities-ismanager-stub-true).
 *
 * Regression cover for the hardcoded-`isManager` bug: the page's local
 * `useIsManager()` hook was a placeholder that returned `true`
 * unconditionally, so the manager-only affordances (the "Create facility"
 * action surfaced through <FacilityList>, and the pending-approvals nav
 * link) rendered for every authenticated user regardless of role. These
 * tests pin that `isManager` is now derived from the real auth role via
 * `isManagerRole(user?.role)`:
 *
 *  - a non-manager role (resident) must NOT get the manager affordance
 *    (fails against the stub, which leaked `isManager=true` to everyone);
 *  - a manager-equivalent role still does.
 */
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { FacilitiesPage } from './FacilitiesPage';

// ── api-client ──
const mockListFacilities = vi.fn();
vi.mock('@ppt/api-client', () => ({
  listFacilities: (...args: unknown[]) => mockListFacilities(...args),
}));

// ── auth: role is driven per-test ──
let mockRole: string | undefined;
vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { role: mockRole } }),
}));

// ── capture the isManager prop handed to <FacilityList> ──
const capturedIsManager: Array<boolean | undefined> = [];
vi.mock('../components', () => ({
  FacilityList: ({ isManager }: { isManager?: boolean }) => {
    capturedIsManager.push(isManager);
    return <div data-testid="facility-list" />;
  },
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

beforeEach(() => {
  vi.clearAllMocks();
  capturedIsManager.length = 0;
  mockListFacilities.mockResolvedValue({ items: [], total: 0 });
});

describe('FacilitiesPage manager gating', () => {
  it('passes isManager=false for a non-manager role (resident)', async () => {
    mockRole = 'resident';
    renderPage();

    await waitFor(() => expect(capturedIsManager.length).toBeGreaterThan(0));
    expect(capturedIsManager.at(-1)).toBe(false);
    // The manager-only pending-approvals nav link must not render (asserted
    // by href so it doesn't depend on i18n being initialized in the test).
    const pendingLink = screen
      .queryAllByRole('link')
      .find((a) => a.getAttribute('href')?.includes('/facilities/pending'));
    expect(pendingLink).toBeUndefined();
  });

  it('passes isManager=true for a manager-equivalent role', async () => {
    mockRole = 'manager';
    renderPage();

    await waitFor(() => expect(capturedIsManager.length).toBeGreaterThan(0));
    expect(capturedIsManager.at(-1)).toBe(true);
  });
});
