'use client';

import { useQueryClient } from '@tanstack/react-query';
import * as LocalAuthentication from 'expo-local-authentication';
import * as SecureStore from 'expo-secure-store';
import type { ReactNode } from 'react';
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { resetLocalData } from '../services/resetLocalData';

// Token storage keys
const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';
const BIOMETRIC_ENABLED_KEY = 'ppt_biometric_enabled';

export interface User {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  role: 'owner' | 'tenant' | 'resident' | 'manager' | 'admin';
  buildingId?: string;
  unitId?: string;
  avatarUrl?: string;
}

export interface AuthState {
  isLoading: boolean;
  isAuthenticated: boolean;
  user: User | null;
  accessToken: string | null;
  biometricEnabled: boolean;
  biometricAvailable: boolean;
}

export interface AuthContextValue extends AuthState {
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshToken: () => Promise<void>;
  enableBiometric: () => Promise<boolean>;
  disableBiometric: () => Promise<void>;
  authenticateWithBiometric: () => Promise<boolean>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

interface AuthProviderProps {
  children: ReactNode;
  apiBaseUrl: string;
}

export function AuthProvider({ children, apiBaseUrl }: AuthProviderProps) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  const [state, setState] = useState<AuthState>({
    isLoading: true,
    isAuthenticated: false,
    user: null,
    accessToken: null,
    biometricEnabled: false,
    biometricAvailable: false,
  });

  // Check biometric availability and load stored auth on mount
  useEffect(() => {
    async function initialize() {
      try {
        // Check biometric availability
        const compatible = await LocalAuthentication.hasHardwareAsync();
        const enrolled = await LocalAuthentication.isEnrolledAsync();
        const biometricAvailable = compatible && enrolled;

        // Load stored tokens
        const accessToken = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
        const userJson = await SecureStore.getItemAsync(USER_KEY);
        const biometricEnabled = await SecureStore.getItemAsync(BIOMETRIC_ENABLED_KEY);

        if (accessToken && userJson) {
          const user = JSON.parse(userJson) as User;
          setState({
            isLoading: false,
            isAuthenticated: true,
            user,
            accessToken,
            biometricEnabled: biometricEnabled === 'true',
            biometricAvailable,
          });
        } else {
          setState((prev) => ({
            ...prev,
            isLoading: false,
            biometricAvailable,
          }));
        }
      } catch (error) {
        console.error('Failed to initialize auth:', error);
        setState((prev) => ({
          ...prev,
          isLoading: false,
        }));
      }
    }

    initialize();
  }, []);

  const login = useCallback(
    async (email: string, password: string) => {
      setState((prev) => ({ ...prev, isLoading: true }));

      try {
        const response = await fetch(`${apiBaseUrl}/api/v1/auth/login`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ email, password }),
        });

        if (!response.ok) {
          const error = await response.json();
          throw new Error(error.message || 'Login failed');
        }

        const data = await response.json();
        // The api-server returns snake_case fields; map them to the camelCase
        // names we use throughout the app state.
        const accessToken = data.access_token;
        const refreshToken = data.refresh_token;
        const user = data.user;

        // Store tokens securely
        await SecureStore.setItemAsync(ACCESS_TOKEN_KEY, accessToken);
        await SecureStore.setItemAsync(REFRESH_TOKEN_KEY, refreshToken);
        await SecureStore.setItemAsync(USER_KEY, JSON.stringify(user));

        // A login always begins a fresh session, so no previous org's cached
        // data may survive it (issue #2361). Removing only the ['auth',
        // 'tenant-id'] key fixed tenant-id resolution (issue #2329) but left
        // every other cached query from the prior org in place — the mobile
        // read-query keys (['buildings','list'], ['documents','list',…],
        // ['faults','list'], etc.) are NOT tenant-scoped, so after a login that
        // follows a session which wasn't explicitly logged out, TanStack Query
        // would serve user A's data to user B until a background refetch (or
        // never, if offline). Clear the whole cache to mirror logout().
        //
        // queryClient.clear() only wipes the in-memory cache; the AsyncStorage
        // caches (offline queue/responses, home-screen widget data) outlive the
        // process, so purge those first or a prior org's data survives the
        // login (issue #2399, follow-up to #2361).
        await resetLocalData();
        queryClient.clear();

        setState((prev) => ({
          ...prev,
          isLoading: false,
          isAuthenticated: true,
          user,
          accessToken,
        }));
      } catch (error) {
        setState((prev) => ({ ...prev, isLoading: false }));
        throw error;
      }
    },
    [apiBaseUrl, queryClient]
  );

  const logout = useCallback(async () => {
    try {
      // Clear stored tokens
      await SecureStore.deleteItemAsync(ACCESS_TOKEN_KEY);
      await SecureStore.deleteItemAsync(REFRESH_TOKEN_KEY);
      await SecureStore.deleteItemAsync(USER_KEY);

      // Wipe the whole query cache on logout (issue #2329). `useTenantId`
      // caches the tenant id forever (`staleTime: Infinity`) and nothing else
      // invalidated it, so after logout + login as a user in a *different* org
      // its consumers kept serving the previous org's tenant id until an app
      // restart — producing persistent cross-tenant 403s / wrong cache keys.
      // Clearing everything also ensures no other org's data survives logout.
      //
      // Purge the AsyncStorage-backed caches too (offline queue/responses,
      // widget data) — they persist across restarts, so clearing only the
      // in-memory query cache would leave a prior org's data (and a prior
      // user's queued writes) behind (issue #2399, follow-up to #2361).
      await resetLocalData();
      queryClient.clear();

      setState((prev) => ({
        ...prev,
        isAuthenticated: false,
        user: null,
        accessToken: null,
      }));
    } catch (error) {
      console.error('Failed to logout:', error);
      throw error;
    }
  }, [queryClient]);

  const refreshToken = useCallback(async () => {
    try {
      const storedRefreshToken = await SecureStore.getItemAsync(REFRESH_TOKEN_KEY);

      if (!storedRefreshToken) {
        throw new Error('No refresh token available');
      }

      const response = await fetch(`${apiBaseUrl}/api/v1/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ refresh_token: storedRefreshToken }),
      });

      if (!response.ok) {
        throw new Error('Token refresh failed');
      }

      const data = await response.json();
      const accessToken = data.access_token;
      const newRefreshToken = data.refresh_token;

      // Store new tokens
      await SecureStore.setItemAsync(ACCESS_TOKEN_KEY, accessToken);
      await SecureStore.setItemAsync(REFRESH_TOKEN_KEY, newRefreshToken);

      setState((prev) => ({
        ...prev,
        accessToken,
      }));
    } catch (error) {
      // If refresh fails, log out
      await logout();
      throw error;
    }
  }, [apiBaseUrl, logout]);

  const enableBiometric = useCallback(async (): Promise<boolean> => {
    if (!state.biometricAvailable) {
      return false;
    }

    try {
      const result = await LocalAuthentication.authenticateAsync({
        promptMessage: t('auth.biometric.enablePrompt'),
        cancelLabel: t('auth.biometric.cancel'),
        fallbackLabel: t('auth.biometric.usePasscode'),
      });

      if (result.success) {
        await SecureStore.setItemAsync(BIOMETRIC_ENABLED_KEY, 'true');
        setState((prev) => ({ ...prev, biometricEnabled: true }));
        return true;
      }

      return false;
    } catch (error) {
      console.error('Failed to enable biometric:', error);
      return false;
    }
  }, [state.biometricAvailable, t]);

  const disableBiometric = useCallback(async () => {
    await SecureStore.deleteItemAsync(BIOMETRIC_ENABLED_KEY);
    setState((prev) => ({ ...prev, biometricEnabled: false }));
  }, []);

  const authenticateWithBiometric = useCallback(async (): Promise<boolean> => {
    if (!state.biometricEnabled || !state.biometricAvailable) {
      return false;
    }

    try {
      const result = await LocalAuthentication.authenticateAsync({
        promptMessage: t('auth.biometric.authenticatePrompt'),
        cancelLabel: t('auth.biometric.cancel'),
        fallbackLabel: t('auth.biometric.usePassword'),
      });

      if (result.success) {
        // Biometric unlock restores the SAME stored user's session, so unlike
        // login()/logout() it deliberately does NOT call resetLocalData() /
        // queryClient.clear() — the existing cache still belongs to this user
        // and re-fetching it on every unlock would be wasteful and drop
        // offline data (issue #2399).
        // Check if we have stored credentials
        const accessToken = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
        const userJson = await SecureStore.getItemAsync(USER_KEY);

        if (accessToken && userJson) {
          const user = JSON.parse(userJson) as User;
          setState((prev) => ({
            ...prev,
            isAuthenticated: true,
            user,
            accessToken,
          }));
          return true;
        }
      }

      return false;
    } catch (error) {
      console.error('Biometric authentication failed:', error);
      return false;
    }
  }, [state.biometricEnabled, state.biometricAvailable, t]);

  const value = useMemo<AuthContextValue>(
    () => ({
      ...state,
      login,
      logout,
      refreshToken,
      enableBiometric,
      disableBiometric,
      authenticateWithBiometric,
    }),
    [
      state,
      login,
      logout,
      refreshToken,
      enableBiometric,
      disableBiometric,
      authenticateWithBiometric,
    ]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
