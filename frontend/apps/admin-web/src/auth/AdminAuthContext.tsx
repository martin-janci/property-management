import { createContext, ReactNode, useCallback, useContext, useMemo, useState } from 'react';

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

  const value = useMemo<AdminAuthValue>(
    () => ({ isAuthenticated: token !== null, token, setToken, logout }),
    [token, setToken, logout],
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
