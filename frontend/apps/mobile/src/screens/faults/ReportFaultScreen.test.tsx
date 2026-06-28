/**
 * ReportFaultScreen — unit tests.
 *
 * Originally a regression guard for the bug where `handleSubmit` faked the
 * network call (code-review-mobile-rn-report-fault-fake-submit). Extended for
 * Story 4.1 (gap #970.6): offline fault queue + image-compression. The screen
 * now creates the fault via `apiRequest` directly so it can fall back to the
 * persisted offline queue, carrying a Story 2B.7 idempotency key so retries /
 * replays never duplicate.
 *
 * Covers:
 *   - Renders the building picker from the buildings query.
 *   - The buildings query carries the required `organization_id` and is gated
 *     on the tenant id.
 *   - Online submit fires `POST /api/v1/faults` with the form payload + an
 *     idempotency key; success Alert follows a 2xx.
 *   - Validation blocks submit (no building selected) — no request, no enqueue.
 *   - Offline submit enqueues the create instead of firing a request.
 *   - A failed request falls back to the queue, reusing the SAME idempotency
 *     key it attempted with (no duplicate across the retry).
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';
import { ReportFaultScreen } from './ReportFaultScreen';

// --- Mocks ---

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useTenantId: jest.fn(),
  apiRequest: jest.fn(),
}));

jest.mock('../../hooks', () => ({
  useOfflineSupport: jest.fn(),
}));

const FIXED_KEY = 'idem-test-key';
jest.mock('../../utils', () => ({
  generateIdempotencyKey: () => FIXED_KEY,
  compressImagesIfNeeded: jest.fn(async (uris: string[]) => ({
    uris,
    originalBytes: 0,
    finalBytes: 0,
    compressed: false,
    stillOverLimit: false,
  })),
  bytesToMb: (bytes: number) => String(bytes),
}));

// Expo native modules touched by the screen — stub so the import graph resolves.
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
const mockApiRequest = useApiMock.apiRequest as jest.Mock;
const mockUseOfflineSupport = jest.requireMock('../../hooks').useOfflineSupport as jest.Mock;

// --- Helpers ---

const TENANT_ID = 'org-7f3c';

const BUILDINGS = [
  { id: 'b-1', name: 'Lipová Residence', street: 'Lipová 5', city: 'Bratislava' },
  { id: 'b-2', name: 'Cottage 12', street: 'Záhradná 12', city: 'Pezinok' },
];

const EXPECTED_PAYLOAD = {
  building_id: 'b-1',
  title: 'Leaking pipe',
  description: 'Water is leaking from the ceiling in the hallway near unit 3.',
  category: 'plumbing',
  priority: 'medium',
  location_description: '3rd floor',
  idempotency_key: FIXED_KEY,
};

let addToQueue: jest.Mock;

function setConnectivity(isConnected: boolean) {
  mockUseOfflineSupport.mockReturnValue({
    isConnected,
    isInternetReachable: isConnected,
    addToQueue,
  });
}

function renderScreen(props: React.ComponentProps<typeof ReportFaultScreen> = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ReportFaultScreen {...props} />
    </QueryClientProvider>
  );
}

/** Fill all required fields with valid values, selecting the first building. */
function fillValidForm() {
  fireEvent.press(screen.getByText('Lipová Residence'));
  fireEvent.changeText(screen.getByPlaceholderText('faults.titlePlaceholder'), 'Leaking pipe');
  fireEvent.changeText(
    screen.getByPlaceholderText('faults.descriptionPlaceholder'),
    'Water is leaking from the ceiling in the hallway near unit 3.'
  );
  fireEvent.changeText(screen.getByPlaceholderText('faults.locationPlaceholder'), '3rd floor');
  // Category grid uses the hardcoded English labels.
  fireEvent.press(screen.getByText('Plumbing'));
}

// --- Tests ---

describe('ReportFaultScreen', () => {
  let alertSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    addToQueue = jest.fn().mockResolvedValue(undefined);
    mockApiRequest.mockResolvedValue({ id: 'fault-1', message: 'ok' });
    mockUseTenantId.mockReturnValue({ tenantId: TENANT_ID, isLoading: false });
    mockUseApiQuery.mockReturnValue({
      data: { buildings: BUILDINGS },
      isLoading: false,
      error: null,
    });
    setConnectivity(true);
    alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
  });

  afterEach(() => {
    alertSpy.mockRestore();
  });

  it('renders the building picker from the buildings query', () => {
    renderScreen();
    expect(screen.getByText('Lipová Residence')).toBeTruthy();
    expect(screen.getByText('Cottage 12')).toBeTruthy();
  });

  it('fires POST /api/v1/faults with the form payload (incl. idempotency key) when online', async () => {
    renderScreen();
    fillValidForm();
    fireEvent.press(screen.getByText('faults.submitButton'));

    await waitFor(() => expect(mockApiRequest).toHaveBeenCalledTimes(1));
    expect(mockApiRequest).toHaveBeenCalledWith('/api/v1/faults', {
      method: 'POST',
      body: EXPECTED_PAYLOAD,
    });
    expect(addToQueue).not.toHaveBeenCalled();
  });

  it('shows the success Alert after a 2xx and does not enqueue', async () => {
    const onSuccess = jest.fn();
    renderScreen({ onSuccess });
    fillValidForm();
    fireEvent.press(screen.getByText('faults.submitButton'));

    await waitFor(() =>
      expect(alertSpy).toHaveBeenCalledWith(
        'common.done',
        'faults.successSubmit',
        expect.any(Array)
      )
    );
    expect(addToQueue).not.toHaveBeenCalled();
  });

  it('queries buildings with the required organization_id and keys the cache on it', () => {
    renderScreen();

    const buildingsCall = mockUseApiQuery.mock.calls.find(
      ([key]) => Array.isArray(key) && key[0] === 'buildings'
    );
    expect(buildingsCall).toBeDefined();

    const [queryKey, path] = buildingsCall as [readonly unknown[], string];
    expect(path).toBe(`/api/v1/buildings?organization_id=${TENANT_ID}`);
    expect(queryKey).toContain(TENANT_ID);
  });

  it('holds the buildings query disabled until the tenant id resolves', () => {
    mockUseTenantId.mockReturnValue({ tenantId: null, isLoading: true });
    renderScreen();

    const buildingsCall = mockUseApiQuery.mock.calls.find(
      ([key]) => Array.isArray(key) && key[0] === 'buildings'
    );
    expect(buildingsCall).toBeDefined();
    const [, , options] = buildingsCall as [readonly unknown[], string, { enabled?: boolean }];
    expect(options.enabled).toBe(false);
    expect(screen.getByText('faults.loadingBuildings')).toBeTruthy();
  });

  it('does not submit or enqueue when no building is selected', () => {
    renderScreen();
    fireEvent.changeText(screen.getByPlaceholderText('faults.titlePlaceholder'), 'Leaking pipe');
    fireEvent.changeText(
      screen.getByPlaceholderText('faults.descriptionPlaceholder'),
      'Water is leaking from the ceiling in the hallway near unit 3.'
    );
    fireEvent.changeText(screen.getByPlaceholderText('faults.locationPlaceholder'), '3rd floor');
    fireEvent.press(screen.getByText('Plumbing'));

    fireEvent.press(screen.getByText('faults.submitButton'));
    expect(mockApiRequest).not.toHaveBeenCalled();
    expect(addToQueue).not.toHaveBeenCalled();
    expect(screen.getByText('faults.selectBuilding')).toBeTruthy();
  });

  it('enqueues the create instead of firing a request when offline', async () => {
    setConnectivity(false);
    renderScreen();
    fillValidForm();
    // Offline the button label switches to the "save & sync later" copy.
    fireEvent.press(screen.getByText('faults.submitButtonOffline'));

    await waitFor(() => expect(addToQueue).toHaveBeenCalledTimes(1));
    expect(addToQueue).toHaveBeenCalledWith({
      type: 'CREATE',
      endpoint: '/api/v1/faults',
      method: 'POST',
      body: EXPECTED_PAYLOAD,
    });
    expect(mockApiRequest).not.toHaveBeenCalled();
    expect(alertSpy).toHaveBeenCalledWith(
      'faults.queuedTitle',
      'faults.queuedMessage',
      expect.any(Array)
    );
  });

  it('falls back to the queue on request failure, reusing the attempted idempotency key', async () => {
    mockApiRequest.mockRejectedValueOnce(new Error('network down'));
    renderScreen();
    fillValidForm();
    fireEvent.press(screen.getByText('faults.submitButton'));

    await waitFor(() => expect(addToQueue).toHaveBeenCalledTimes(1));

    const attemptedKey = mockApiRequest.mock.calls[0][1].body.idempotency_key;
    const queuedKey = addToQueue.mock.calls[0][0].body.idempotency_key;
    // Same key on the attempt and the queued replay → the idempotent backend
    // returns the existing fault rather than inserting a duplicate.
    expect(queuedKey).toBe(attemptedKey);
    expect(alertSpy).toHaveBeenCalledWith(
      'faults.queuedTitle',
      'faults.queuedOnErrorMessage',
      expect.any(Array)
    );
  });
});
