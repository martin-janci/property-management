/**
 * ReportFaultScreen — category-chip i18n regression test
 * (code-review-mobile-rn-report-fault-category-chips-english).
 *
 * The 7 category chips ("Plumbing", "Electrical", "Structural", "HVAC",
 * "Elevator", "Security", "Other") were rendered from a hardcoded English
 * `getCategoryLabel` map even though react-i18next was already wired for the
 * rest of the form (labels, priorities, validation). A locale switch left the
 * chips stuck in English while everything around them translated. This suite
 * locks the contract: every category chip label is now sourced from an i18n
 * key via `t()`, not a literal.
 *
 * Two layers of assertion (mirrors FaultsListScreen.i18n.test.tsx):
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`
 *     (the repo-wide convention). We assert the screen renders the i18n KEYS
 *     for every chip and, crucially, that the OLD hardcoded English literals
 *     are gone. If any label were still hardcoded, the "not present"
 *     assertions would fail.
 *  2. Resource layer — the locale JSON files actually carry translated,
 *     locale-distinct values for those keys, proving the keys resolve to real
 *     translations rather than dangling references.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { ReportFaultScreen } from './ReportFaultScreen';

// --- Mocks (mirror ReportFaultScreen.test.tsx — the screen's hooks/native deps
//     must resolve for the render layer to mount). ---

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useTenantId: jest.fn(),
  apiRequest: jest.fn(),
}));

jest.mock('../../hooks', () => ({
  useOfflineSupport: jest.fn(),
}));

jest.mock('./faultAttachments', () => ({
  uploadFaultPhotos: jest.fn(),
}));

jest.mock('../../utils', () => ({
  generateIdempotencyKey: () => 'idem-test-key',
  compressImagesIfNeeded: jest.fn(),
  bytesToMb: (bytes: number) => String(bytes),
}));

jest.mock('expo-image-picker', () => ({
  requestCameraPermissionsAsync: jest.fn(),
  requestMediaLibraryPermissionsAsync: jest.fn(),
  launchCameraAsync: jest.fn(),
  launchImageLibraryAsync: jest.fn(),
}));
jest.mock('expo-location', () => ({
  requestForegroundPermissionsAsync: jest.fn(),
  getCurrentPositionAsync: jest.fn(),
  reverseGeocodeAsync: jest.fn(),
}));

const useApiMock = jest.requireMock('../../hooks/useApi');
const mockUseApiQuery = useApiMock.useApiQuery as jest.Mock;
const mockUseTenantId = useApiMock.useTenantId as jest.Mock;
const mockUseOfflineSupport = jest.requireMock('../../hooks').useOfflineSupport as jest.Mock;

// The i18n keys the 7 category chips now render through.
const CATEGORY_KEYS = [
  'categoryPlumbing',
  'categoryElectrical',
  'categoryStructural',
  'categoryHvac',
  'categoryElevator',
  'categorySecurity',
  'categoryOther',
] as const;

// The old hardcoded English literals that must no longer be rendered.
const HARDCODED = ['Plumbing', 'Electrical', 'Structural', 'HVAC', 'Elevator', 'Security', 'Other'];

async function renderScreen() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return await render(
    <QueryClientProvider client={queryClient}>
      <ReportFaultScreen />
    </QueryClientProvider>
  );
}

describe('ReportFaultScreen category-chip i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseTenantId.mockReturnValue({ tenantId: 'org-1', isLoading: false });
    mockUseApiQuery.mockReturnValue({ data: { buildings: [] }, isLoading: false, error: null });
    mockUseOfflineSupport.mockReturnValue({
      isConnected: true,
      isInternetReachable: true,
      addToQueue: jest.fn(),
    });
  });

  it('renders every category chip via its i18n key, not a hardcoded literal', async () => {
    await renderScreen();
    for (const key of CATEGORY_KEYS) {
      expect(screen.getByText(`faults.${key}`)).toBeTruthy();
    }
  });

  it('no longer renders any hardcoded English category literal', async () => {
    await renderScreen();
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });
});

describe('category keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.faults.categoryPlumbing).toBe('Plumbing');
    expect(en.faults.categoryElectrical).toBe('Electrical');
    expect(en.faults.categoryStructural).toBe('Structural');
    expect(en.faults.categoryHvac).toBe('HVAC');
    expect(en.faults.categoryElevator).toBe('Elevator');
    expect(en.faults.categorySecurity).toBe('Security');
    expect(en.faults.categoryOther).toBe('Other');
  });

  it('every locale defines all 7 category keys as non-empty strings', () => {
    for (const locale of [en, sk, cs, de, pl, hu]) {
      for (const key of CATEGORY_KEYS) {
        const value = (locale.faults as unknown as Record<string, string>)[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
    }
  });

  it('non-English locales translate the chips away from English', () => {
    // Plumbing and Other both translate in every supported locale — proving the
    // chips actually localise rather than echoing the English source.
    for (const locale of [sk, cs, de, pl, hu]) {
      const f = locale.faults as unknown as Record<string, string>;
      expect(f.categoryPlumbing).not.toBe(en.faults.categoryPlumbing);
      expect(f.categoryOther).not.toBe(en.faults.categoryOther);
    }
  });
});
