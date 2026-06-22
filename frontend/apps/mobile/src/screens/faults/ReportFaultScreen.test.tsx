/**
 * ReportFaultScreen — unit tests
 * (code-review-mobile-rn-report-fault-fake-submit).
 *
 * Regression guard for the bug where `handleSubmit` faked the network call
 * with `setTimeout(1500)` and showed a success Alert without ever reaching the
 * backend. These tests assert that submitting actually fires the
 * `POST /api/v1/faults` mutation with the form payload, and that the success
 * Alert only follows a 2xx (mutation `onSuccess`).
 *
 * Covers:
 *   - Renders the building picker from the buildings query.
 *   - Submitting a valid form calls the create-fault mutation with the
 *     expected `{ building_id, title, description, category, priority }`.
 *   - Validation blocks submit (no building selected) — mutation NOT called.
 *   - mutation onSuccess fires the success Alert; onError fires the error Alert.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react-native';
import { Alert } from 'react-native';
import { ReportFaultScreen } from './ReportFaultScreen';

// ─── Mocks ───────────────────────────────────────────────────

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(),
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

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;
const mockUseApiMutation = jest.requireMock('../../hooks/useApi').useApiMutation as jest.Mock;

// ─── Helpers ───────────────────────────────────────────────

const BUILDINGS = [
  { id: 'b-1', name: 'Lipová Residence', street: 'Lipová 5', city: 'Bratislava' },
  { id: 'b-2', name: 'Cottage 12', street: 'Záhradná 12', city: 'Pezinok' },
];

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

// ─── Tests ─────────────────────────────────────────────────

describe('ReportFaultScreen', () => {
  let mutate: jest.Mock;
  let alertSpy: jest.SpyInstance;

  beforeEach(() => {
    jest.clearAllMocks();
    mutate = jest.fn();
    mockUseApiMutation.mockReturnValue({ mutate, isPending: false });
    mockUseApiQuery.mockReturnValue({
      data: { buildings: BUILDINGS },
      isLoading: false,
      error: null,
    });
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

  it('fires POST /api/v1/faults with the form payload on submit', () => {
    renderScreen();
    fillValidForm();
    fireEvent.press(screen.getByText('faults.submitButton'));

    expect(mutate).toHaveBeenCalledTimes(1);
    expect(mutate).toHaveBeenCalledWith(
      {
        building_id: 'b-1',
        title: 'Leaking pipe',
        description: 'Water is leaking from the ceiling in the hallway near unit 3.',
        category: 'plumbing',
        priority: 'medium',
        location_description: '3rd floor',
      },
      expect.any(Object)
    );
  });

  it('targets the /api/v1/faults endpoint with POST', () => {
    renderScreen();
    expect(mockUseApiMutation).toHaveBeenCalledWith('/api/v1/faults', 'POST');
  });

  it('does not call the mutation when no building is selected', () => {
    renderScreen();
    fireEvent.changeText(screen.getByPlaceholderText('faults.titlePlaceholder'), 'Leaking pipe');
    fireEvent.changeText(
      screen.getByPlaceholderText('faults.descriptionPlaceholder'),
      'Water is leaking from the ceiling in the hallway near unit 3.'
    );
    fireEvent.changeText(screen.getByPlaceholderText('faults.locationPlaceholder'), '3rd floor');
    fireEvent.press(screen.getByText('Plumbing'));

    fireEvent.press(screen.getByText('faults.submitButton'));
    expect(mutate).not.toHaveBeenCalled();
    expect(screen.getByText('faults.selectBuilding')).toBeTruthy();
  });

  it('shows the success Alert via the mutation onSuccess callback', () => {
    const onSuccess = jest.fn();
    renderScreen({ onSuccess });
    fillValidForm();
    fireEvent.press(screen.getByText('faults.submitButton'));

    const [, options] = mutate.mock.calls[0];
    options.onSuccess({ id: 'fault-1', message: 'ok' });

    expect(alertSpy).toHaveBeenCalledWith('common.done', 'faults.successSubmit', expect.any(Array));
  });

  it('shows an error Alert via the mutation onError callback', () => {
    renderScreen();
    fillValidForm();
    fireEvent.press(screen.getByText('faults.submitButton'));

    const [, options] = mutate.mock.calls[0];
    options.onError(new Error('Server exploded'));

    expect(alertSpy).toHaveBeenCalledWith('common.error', 'Server exploded');
  });
});
