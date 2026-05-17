import { clearTokenProvider, setTokenProvider } from '@ppt/api-client';
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

import { sessionTokenStore } from './tokenStore';

interface AdminAuthValue {
  isAuthenticated: boolean;
  token: string | null;
  setToken: (token: string) => void;
  logout: () => void;
}

const AdminAuthContext = createContext<AdminAuthValue | null>(null);

export function AdminAuthProvider({ children }: { children: ReactNode }) {
  const [token, setTokenState] = useState<string | null>(() => sessionTokenStore.get());

  const setToken = useCallback((next: string) => {
    sessionTokenStore.set(next);
    setTokenState(next);
  }, []);

  const logout = useCallback(() => {
    sessionTokenStore.clear();
    setTokenState(null);
  }, []);

  // Register the sessionStorage token with @ppt/api-client so admin
  // page hooks (useAgencies, suspendAgency, etc.) automatically attach
  // `Authorization: Bearer <token>` on every request. Without this the
  // admin pages would call api-server unauthenticated and get 401 even
  // after a successful login.
  //
  // Reads the token via `sessionTokenStore.get()` (not via the local
  // `token` state) so the api-client always sees the latest value even
  // when callers fire requests between a `setToken` call and React's
  // next render.
  useEffect(() => {
    setTokenProvider(() => sessionTokenStore.get());
    return () => {
      clearTokenProvider();
    };
  }, []);

  const value = useMemo<AdminAuthValue>(
    () => ({ isAuthenticated: token !== null, token, setToken, logout }),
    [token, setToken, logout]
  );

  return <AdminAuthContext.Provider value={value}>{children}</AdminAuthContext.Provider>;
}

export function useAdminAuth(): AdminAuthValue {
  const ctx = useContext(AdminAuthContext);
  if (!ctx) {
    throw new Error('useAdminAuth must be used inside <AdminAuthProvider>');
  }
  return ctx;
}
