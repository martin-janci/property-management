/**
 * AuthContext — updateUser() user-shape normalisation regression test
 * (code-review-mobile-rn-session-user-unnormalized).
 *
 * The auth context's `updateUser()` merges a partial change into the cached
 * session user and persists it to SecureStore (`ppt_user`). It used to store
 * the merged object VERBATIM (`{ ...prev.user, ...changes }`), bypassing the
 * canonical shaping every other user-store site applies (login via
 * `buildSessionUser`, cold-start / biometric restore via `parseStoredUser` →
 * `normalizeUser`). That let a merged field — e.g. an arbitrary `role` or a junk
 * key — drift the in-memory and persisted user away from the shape a later
 * restore re-derives: the same blob would be re-shaped on the next cold start,
 * so the session silently changed under the user (and an un-collapsed role could
 * briefly leak into app state).
 *
 * This suite locks the contract: a value merged through `updateUser()` is
 * normalised through the same path as the restore (`role` collapsed via
 * `toAppRole`, junk keys dropped) before it reaches either the in-memory
 * context or the persisted `ppt_user` blob. It fails before the normalisation
 * is added (IG3): the verbatim merge stores `role: 'super_admin'` and the junk
 * key as-is.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import * as SecureStore from 'expo-secure-store';
import { Pressable, Text } from 'react-native';

import type { User } from './AuthContext';
import { AuthProvider, useAuth } from './AuthContext';

// Biometric probing in initialize() must resolve deterministically.
jest.mock('expo-local-authentication', () => ({
  hasHardwareAsync: jest.fn(async () => false),
  isEnrolledAsync: jest.fn(async () => false),
  authenticateAsync: jest.fn(async () => ({ success: false })),
}));

// Treat the stored access token as still valid so initialize() restores the
// session directly instead of attempting a network refresh.
jest.mock('../utils/jwt', () => ({
  isJwtExpired: () => false,
  decodeJwtPayload: () => ({}),
  extractTenantId: () => null,
}));

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const USER_KEY = 'ppt_user';

const getItemAsync = jest.mocked(SecureStore.getItemAsync);
const setItemAsync = jest.mocked(SecureStore.setItemAsync);

const storedUser = {
  id: 'u1',
  email: 'olivia@example.com',
  firstName: 'Olivia',
  lastName: 'Owner',
  role: 'owner',
};

// A change carrying an un-normalised role and a junk key. `updateUser` is typed
// `Partial<User>`, so the drift-shaped value is cast in — exactly the runtime
// shape that could arrive from a future caller echoing a server response.
const driftChange = {
  firstName: 'Olive',
  role: 'super_admin',
  bogus: 'should-be-dropped',
} as unknown as Partial<User>;

function Consumer() {
  const { user, updateUser } = useAuth();
  return (
    <>
      <Text testID="role">{user?.role ?? 'none'}</Text>
      <Text testID="first">{user?.firstName ?? 'none'}</Text>
      <Pressable
        testID="update"
        onPress={() => {
          void updateUser(driftChange);
        }}
      />
    </>
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
  getItemAsync.mockReset();
  setItemAsync.mockReset();
  setItemAsync.mockResolvedValue(undefined);
  getItemAsync.mockImplementation(async (key: string) => {
    if (key === ACCESS_TOKEN_KEY) return 'restored-access-token';
    if (key === USER_KEY) return JSON.stringify(storedUser);
    return null;
  });
});

describe('AuthContext → updateUser user-shape normalisation', () => {
  it('normalises the merged user in the in-memory context (role collapsed via toAppRole)', async () => {
    renderProvider();

    await waitFor(() => {
      expect(screen.getByTestId('role').props.children).toBe('owner');
    });

    await act(async () => {
      fireEvent.press(screen.getByTestId('update'));
    });

    await waitFor(() => {
      expect(screen.getByTestId('first').props.children).toBe('Olive');
    });
    // `super_admin` must collapse to the app role `admin`, never be stored raw.
    expect(screen.getByTestId('role').props.children).toBe('admin');
  });

  it('persists a normalised blob to SecureStore (role collapsed, junk key dropped)', async () => {
    renderProvider();

    await waitFor(() => {
      expect(screen.getByTestId('role').props.children).toBe('owner');
    });

    await act(async () => {
      fireEvent.press(screen.getByTestId('update'));
    });

    // The in-memory merge lands first; the SecureStore persist follows it.
    await waitFor(() => {
      expect(screen.getByTestId('first').props.children).toBe('Olive');
    });
    await waitFor(() => {
      expect(setItemAsync.mock.calls.some(([key]) => key === USER_KEY)).toBe(true);
    });

    const userWrite = [...setItemAsync.mock.calls].reverse().find(([key]) => key === USER_KEY);
    const persisted = JSON.parse(userWrite?.[1] as string);
    expect(persisted.role).toBe('admin');
    expect(persisted.firstName).toBe('Olive');
    expect(persisted).not.toHaveProperty('bogus');
  });
});
