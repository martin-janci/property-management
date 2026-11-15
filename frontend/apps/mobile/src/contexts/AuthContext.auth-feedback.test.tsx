/**
 * AuthContext — FeedbackManager auth-token wiring regression test
 * (code-review-mobile-rn-feedback-manager-no-auth-token).
 *
 * The global `feedbackManager` singleton exposes `setAuthToken()` (the only
 * thing that makes `apiRequest` attach `Authorization: Bearer <token>` to
 * `POST /api/v1/feedback`). But nothing ever called it: the singleton is
 * constructed at module load with a null token and no consumer fed it the
 * signed-in user's access token — so every feedback submission went out
 * unauthenticated (dropped as 401 and queued forever, or persisted without
 * user attribution).
 *
 * This suite locks the contract: AuthProvider must propagate the restored
 * session's `accessToken` into the singleton, and must clear it to `null` on
 * logout (shared-device regression — the next user must not inherit the
 * previous user's bearer). It fails before the wiring effect is added (IG3).
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import * as SecureStore from 'expo-secure-store';
import { Pressable } from 'react-native';

import { feedbackManager } from '../onboarding/FeedbackManager';
import { AuthProvider, useAuth } from './AuthContext';

// Biometric probing in initialize() must resolve deterministically.
jest.mock('expo-local-authentication', () => ({
  hasHardwareAsync: jest.fn(async () => false),
  isEnrolledAsync: jest.fn(async () => false),
  authenticateAsync: jest.fn(async () => ({ success: false })),
}));

// Treat the stored access token as still valid so initialize() restores the
// session directly instead of attempting a network refresh.
jest.mock('../utils/jwt', () => ({ isJwtExpired: () => false }));

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const USER_KEY = 'ppt_user';

const getItemAsync = jest.mocked(SecureStore.getItemAsync);
const setAuthTokenSpy = jest.spyOn(feedbackManager, 'setAuthToken');

const storedUser = {
  id: 'u1',
  email: 'owner@example.com',
  firstName: 'Olivia',
  lastName: 'Owner',
  role: 'owner',
};

function Consumer() {
  const { logout } = useAuth();
  return (
    <Pressable
      testID="logout"
      accessibilityLabel="logout"
      onPress={() => {
        void logout();
      }}
    />
  );
}

function renderProvider() {
  const client = new QueryClient();
  return render(
    <QueryClientProvider client={client}>
      <AuthProvider apiBaseUrl="https://api.test.local">
        <Consumer />
      </AuthProvider>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  setAuthTokenSpy.mockClear();
  getItemAsync.mockReset();
  getItemAsync.mockImplementation(async (key: string) => {
    if (key === ACCESS_TOKEN_KEY) return 'restored-access-token';
    if (key === USER_KEY) return JSON.stringify(storedUser);
    return null;
  });
});

describe('AuthContext → FeedbackManager auth-token wiring', () => {
  it('propagates the restored session access token into the feedback singleton', async () => {
    renderProvider();

    await waitFor(() => {
      expect(setAuthTokenSpy).toHaveBeenCalledWith('restored-access-token');
    });
  });

  it('clears the feedback singleton bearer on logout (shared-device regression)', async () => {
    renderProvider();

    // Wait until the restored session has pushed its token into the singleton.
    await waitFor(() => {
      expect(setAuthTokenSpy).toHaveBeenCalledWith('restored-access-token');
    });

    setAuthTokenSpy.mockClear();
    await act(async () => {
      fireEvent.press(screen.getByTestId('logout'));
    });

    await waitFor(() => {
      expect(setAuthTokenSpy).toHaveBeenCalledWith(null);
    });
  });
});
