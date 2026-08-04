/// <reference types="vitest/globals" />
/**
 * Regression test for the ppt-web route-wrapper "X not found" fallbacks
 * (code-review-ppt-web-core-notfound-i18n-gap).
 *
 * The route wrappers under src/routes/groups/ used to render hardcoded English
 * "<Entity> not found" strings, so sk/cs/de/hu/pl users saw untranslated copy.
 * They now render via `t('errors.<entity>NotFound', '<English fallback>')`.
 *
 * This test pins that behaviour using the buildings detail wrapper as a
 * representative case: with the active language switched to Slovak, the
 * not-found fallback must render the *translated* Slovak string — proving the
 * copy is driven by i18n and not a hardcoded English literal.
 */
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import { MemoryRouter, Routes } from 'react-router-dom';
import sk from '../../../messages/sk.json';
import { buildingRoutes } from './buildings';

// Force the :buildingId param to be missing so the wrapper hits its
// not-found fallback branch, while keeping the rest of react-router real.
vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useParams: () => ({}) };
});

// Keep the auth gate and lazy pages out of the way — we only care about the
// not-found branch, which renders before any inner page mounts.
vi.mock('../../components', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('../lazyRoutes', () => ({
  BookFacilityPage: () => <div />,
  BuildingDetailPage: () => <div data-testid="building-detail-page" />,
  BuildingsPage: () => <div />,
  CreateFacilityPage: () => <div />,
  EditFacilityPage: () => <div />,
  FacilitiesPage: () => <div />,
  MyBookingsPage: () => <div />,
  MyUnitPage: () => <div />,
  PendingBookingsPage: () => <div />,
}));

function renderBuildingDetail() {
  return render(
    <MemoryRouter initialEntries={['/buildings/does-not-matter']}>
      <Routes>{buildingRoutes()}</Routes>
    </MemoryRouter>
  );
}

describe('route-wrapper "not found" fallbacks are i18n-driven', () => {
  afterEach(async () => {
    await i18n.changeLanguage('en');
  });

  it('renders the English fallback via the i18n key (not the inner page)', () => {
    renderBuildingDetail();
    expect(screen.queryByTestId('building-detail-page')).not.toBeInTheDocument();
    expect(screen.getByText('Building not found')).toBeInTheDocument();
  });

  it('renders the translated Slovak string when the language is Slovak', async () => {
    // The Slovak bundle is not loaded by the shared test setup (English-only),
    // so register it here before switching languages.
    i18n.addResourceBundle('sk', 'translation', sk, true, true);
    await i18n.changeLanguage('sk');

    renderBuildingDetail();

    const expected = sk.errors.buildingNotFound;
    // Guard: the assertion is only meaningful if the translation actually differs
    // from the English literal that used to be hardcoded here.
    expect(expected).not.toBe('Building not found');
    expect(screen.getByText(expected)).toBeInTheDocument();
    expect(screen.queryByText('Building not found')).not.toBeInTheDocument();
  });
});
