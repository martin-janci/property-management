/**
 * ProfileScreen — save-persistence regression test.
 *
 * Guards the code-review finding (code-review-mobile-rn-profile-local-only-save):
 * the profile "Save changes" button used to write only to local SecureStore and
 * never call the api-server, so edits silently failed to persist server-side.
 * This suite asserts that a save:
 *   1. issues the `PATCH /api/v1/auth/me` mutation with the combined
 *      `displayName` (first + last), and
 *   2. on success updates the cached session user via `AuthContext.updateUser`
 *      (the single source of truth that keeps context + SecureStore in sync),
 *   3. surfaces a server failure instead of showing the "updated" notice.
 *
 * The `useApiMutation` mock is a synchronous, state-driven stand-in: `mutate`
 * flips real React state (so `isPending` re-renders) and invokes the caller's
 * `onSuccess` / `onError` — the exact contract the screen relies on, without the
 * flakiness of a real async TanStack mutation under jest-expo.
 */

import { fireEvent, render, waitFor } from '@testing-library/react-native';
import { Alert } from 'react-native';
import { ProfileScreen } from './ProfileScreen';

const mockMutationFn = jest.fn();
// Toggled per test to make the next `mutate` succeed or fail.
let mockSaveShouldFail = false;
const SAVE_ERROR = 'Service unavailable';

jest.mock('../../hooks/useApi', () => {
  const React = require('react');
  return {
    useApiMutation: () => {
      const [state, setState] = React.useState({ isPending: false });
      const mutate = (
        vars: unknown,
        opts?: { onSuccess?: () => void | Promise<void>; onError?: (e: Error) => void }
      ) => {
        mockMutationFn(vars);
        if (mockSaveShouldFail) {
          const err = new Error(SAVE_ERROR);
          setState({ isPending: false });
          opts?.onError?.(err);
        } else {
          setState({ isPending: false });
          void opts?.onSuccess?.();
        }
      };
      return { mutate, ...state };
    },
  };
});

const mockUpdateUser = jest.fn();
jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    user: {
      id: 'u-1',
      email: 'resident@example.com',
      firstName: 'Ada',
      lastName: 'Lovelace',
      role: 'resident',
    },
    logout: jest.fn(),
    updateUser: mockUpdateUser,
    biometricAvailable: false,
    biometricEnabled: false,
    enableBiometric: jest.fn(),
    disableBiometric: jest.fn(),
  }),
}));

describe('ProfileScreen — save outcomes', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockSaveShouldFail = false;
    jest.spyOn(Alert, 'alert').mockImplementation(() => {});
  });

  it('persists the edited name to PATCH /api/v1/auth/me and updates the cached user', async () => {
    const { getByDisplayValue, getByText, queryByText } = await render(<ProfileScreen />);

    await fireEvent.changeText(getByDisplayValue('Ada'), 'Grace');
    await fireEvent.changeText(getByDisplayValue('Lovelace'), 'Hopper');
    await fireEvent.press(getByText('common.saveChanges'));

    // The edit was actually sent to the server (not a local-only no-op).
    expect(mockMutationFn).toHaveBeenCalledTimes(1);
    expect(mockMutationFn).toHaveBeenCalledWith({ displayName: 'Grace Hopper' });

    // On success the cached session user is updated through the context.
    await waitFor(() =>
      expect(mockUpdateUser).toHaveBeenCalledWith({ firstName: 'Grace', lastName: 'Hopper' })
    );

    // The success notice is shown and no error alert was raised.
    await waitFor(() => expect(queryByText('auth.profileSaved')).toBeTruthy());
    expect(Alert.alert).not.toHaveBeenCalled();
  });

  it('surfaces a server failure and does not mark the profile as saved', async () => {
    mockSaveShouldFail = true;
    const { getByDisplayValue, getByText, queryByText } = await render(<ProfileScreen />);

    await fireEvent.changeText(getByDisplayValue('Ada'), 'Grace');
    await fireEvent.press(getByText('common.saveChanges'));

    expect(mockMutationFn).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(Alert.alert).toHaveBeenCalledTimes(1));

    // Failure must NOT update the cached user or show the success notice.
    expect(mockUpdateUser).not.toHaveBeenCalled();
    expect(queryByText('auth.profileSaved')).toBeNull();
  });

  it('blocks a save with a blank name and never hits the network', async () => {
    const { getByDisplayValue, getByText } = await render(<ProfileScreen />);

    await fireEvent.changeText(getByDisplayValue('Ada'), '   ');
    await fireEvent.press(getByText('common.saveChanges'));

    expect(mockMutationFn).not.toHaveBeenCalled();
    expect(Alert.alert).toHaveBeenCalledTimes(1);
  });
});
