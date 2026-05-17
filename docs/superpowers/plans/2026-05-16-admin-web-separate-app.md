# Separate super-admin web app (admin.rlt.sk) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the super-admin control plane from `ppt-web` `/admin/*` into its own Vite app served at `admin.rlt.sk`, deployed atomically alongside the existing blue/green stack.

**Architecture:** Three sequential PRs — (1) new `frontend/apps/admin-web` Vite SPA copying pages from `ppt-web/features/admin`; (2) deploy infra (Caddy site, ppt-deploy 5th container, CF DNS, `reserved_platform_hosts` seed); (3) remove `/admin/*` from `ppt-web`. Backend (`api-server` `/api/v1/admin/*`) unchanged. Same-origin proxy `admin.rlt.sk/api/* → api-server`. Sessions and cookies scoped to `admin.rlt.sk` only.

**Tech Stack:** React 19 + Vite + react-router-dom + TanStack Query + axios; `@ppt/admin-ui` + `@ppt/api-client` workspace packages; nginx:alpine static; Rust ppt-deploy (bollard, axum); Caddy 2 admin API; Cloudflare DNS.

**Spec:** `docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md`

**Branch convention:**
- PR-1 work happens on branch `feature/admin-web-app` (current branch).
- PR-2 work happens on `feature/admin-web-deploy` (branched from `main` after PR-1 merges, or from PR-1 head if pipelining).
- PR-3 work happens on `feature/admin-web-cutover` (branched from `main` after PR-2 verified).

---

# Phase 1 (PR-1): New `admin-web` Vite SPA

Produces a working, locally-runnable admin SPA in `frontend/apps/admin-web/`. Build is green; `pnpm -F @ppt/admin-web build` produces a static bundle. Old `/admin/*` in `ppt-web` is untouched.

## Task 1.1: Scaffold app directory with package.json and tsconfig

**Files:**
- Create: `frontend/apps/admin-web/package.json`
- Create: `frontend/apps/admin-web/tsconfig.json`
- Create: `frontend/apps/admin-web/tsconfig.node.json`
- Modify: `frontend/pnpm-workspace.yaml` (no change needed if it already globs `apps/*`)

- [ ] **Step 1: Verify workspace globs apps/***

Run: `grep -A3 packages frontend/pnpm-workspace.yaml`
Expected output should include a line like `- 'apps/*'`. If not, add it.

- [ ] **Step 2: Create `frontend/apps/admin-web/package.json`**

```json
{
  "name": "@ppt/admin-web",
  "private": true,
  "description": "Super-admin Control Plane (React SPA)",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "typecheck": "tsc --noEmit",
    "lint": "eslint . --ext ts,tsx",
    "test": "vitest",
    "test:run": "vitest run"
  },
  "dependencies": {
    "@ppt/admin-ui": "workspace:*",
    "@ppt/api-client": "workspace:*",
    "@ppt/shared": "workspace:*",
    "@ppt/ui-kit": "workspace:*",
    "@tanstack/react-query": "^5.17.0",
    "axios": "^1.6.0",
    "react": "^19.2.0",
    "react-dom": "^19.2.0",
    "react-router-dom": "^6.21.0"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.6.3",
    "@testing-library/react": "^16.1.0",
    "@testing-library/user-event": "^14.5.2",
    "@types/node": "^22.10.5",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.4",
    "eslint": "^9.17.0",
    "jsdom": "^25.0.1",
    "msw": "^2.7.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0",
    "vitest": "^2.1.8"
  }
}
```

Pin versions to whatever `frontend/apps/ppt-web/package.json` uses today — copy the exact `^x.y.z` strings to avoid drift.

- [ ] **Step 3: Create `frontend/apps/admin-web/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 4: Create `frontend/apps/admin-web/tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "strict": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 5: Install deps and verify workspace resolves**

Run: `cd frontend && pnpm install`
Expected: succeeds; lockfile gets a new `apps/admin-web` entry.

- [ ] **Step 6: Commit**

```bash
git add frontend/apps/admin-web/package.json frontend/apps/admin-web/tsconfig.json frontend/apps/admin-web/tsconfig.node.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml
git commit -m "feat(admin-web): scaffold package.json and tsconfig"
```

## Task 1.2: Vite config, index.html, and minimal entrypoint

**Files:**
- Create: `frontend/apps/admin-web/vite.config.ts`
- Create: `frontend/apps/admin-web/index.html`
- Create: `frontend/apps/admin-web/src/main.tsx`
- Create: `frontend/apps/admin-web/src/App.tsx`

- [ ] **Step 1: Create `vite.config.ts`**

```ts
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3100,
    // Vite 5+ host-header allowlist. admin lives under .rlt.sk:
    //   prod:    admin.rlt.sk
    //   staging: admin.staging.rlt.sk
    // The leading-dot form matches the apex and any subdomain.
    allowedHosts: ['.rlt.sk', 'localhost'],
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: true,
    outDir: 'dist',
  },
});
```

- [ ] **Step 2: Create `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="robots" content="noindex, nofollow" />
    <title>PPT Admin</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`noindex, nofollow` keeps admin out of search engines defense-in-depth.

- [ ] **Step 3: Create `src/main.tsx`**

```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';

import { App } from './App';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 30_000,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </QueryClientProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 4: Create minimal `src/App.tsx` placeholder**

```tsx
import { Route, Routes } from 'react-router-dom';

export function App() {
  return (
    <Routes>
      <Route path="*" element={<div>PPT Admin (scaffolding)</div>} />
    </Routes>
  );
}
```

- [ ] **Step 5: Verify build succeeds**

Run: `cd frontend && pnpm -F @ppt/admin-web build`
Expected: `dist/` produced with `index.html` and an `assets/` folder, no errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/apps/admin-web/vite.config.ts frontend/apps/admin-web/index.html frontend/apps/admin-web/src/main.tsx frontend/apps/admin-web/src/App.tsx
git commit -m "feat(admin-web): vite config + entrypoint placeholder"
```

## Task 1.3: API client with axios + 401 interceptor

**Files:**
- Create: `frontend/apps/admin-web/src/api/client.ts`
- Create: `frontend/apps/admin-web/src/api/client.test.ts`

- [ ] **Step 1: Write failing test for 401 interceptor**

```ts
// frontend/apps/admin-web/src/api/client.test.ts
import { describe, expect, it, vi } from 'vitest';

import { createApiClient } from './client';

describe('admin api client', () => {
  it('clears token and triggers onUnauthenticated when server returns 401', async () => {
    const onUnauthenticated = vi.fn();
    const tokenStore = {
      get: () => 'expired',
      set: vi.fn(),
      clear: vi.fn(),
    };
    const client = createApiClient({ baseURL: '/api', tokenStore, onUnauthenticated });

    // Mock fetch via axios adapter — simplest is to stub the underlying call.
    // For now, simulate the interceptor's effect directly by calling its rejection
    // handler with an axios-like error.
    const error = { response: { status: 401, data: { error: 'unauthenticated' } } };
    await client.handle401(error).catch(() => {});

    expect(tokenStore.clear).toHaveBeenCalled();
    expect(onUnauthenticated).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run src/api/client.test.ts`
Expected: FAIL — `createApiClient` not defined.

- [ ] **Step 3: Implement `client.ts`**

```ts
// frontend/apps/admin-web/src/api/client.ts
import axios, { AxiosError, AxiosInstance } from 'axios';

export interface TokenStore {
  get(): string | null;
  set(token: string): void;
  clear(): void;
}

export interface ApiClientOptions {
  baseURL: string;
  tokenStore: TokenStore;
  onUnauthenticated: () => void;
  onMfaRequired?: () => Promise<void>;
}

export interface AdminApiClient {
  axios: AxiosInstance;
  handle401(error: unknown): Promise<never>;
}

export function createApiClient(opts: ApiClientOptions): AdminApiClient {
  const instance = axios.create({
    baseURL: opts.baseURL,
    withCredentials: true,
  });

  instance.interceptors.request.use((config) => {
    const token = opts.tokenStore.get();
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  });

  const handle401 = async (error: unknown): Promise<never> => {
    const err = error as AxiosError<{ error?: string }>;
    if (err?.response?.status === 401 && err.response.data?.error === 'mfa_required' && opts.onMfaRequired) {
      await opts.onMfaRequired();
      // After MFA, the caller is expected to retry; we still reject so caller can decide.
    }
    opts.tokenStore.clear();
    opts.onUnauthenticated();
    throw err;
  };

  instance.interceptors.response.use(
    (resp) => resp,
    (error: AxiosError) => {
      if (error.response?.status === 401) {
        return handle401(error);
      }
      throw error;
    },
  );

  return { axios: instance, handle401 };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run src/api/client.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/apps/admin-web/src/api/
git commit -m "feat(admin-web): api client with 401 interceptor"
```

## Task 1.4: Token store (sessionStorage)

**Files:**
- Create: `frontend/apps/admin-web/src/auth/tokenStore.ts`
- Create: `frontend/apps/admin-web/src/auth/tokenStore.test.ts`

- [ ] **Step 1: Write failing test**

```ts
// frontend/apps/admin-web/src/auth/tokenStore.test.ts
import { afterEach, describe, expect, it } from 'vitest';

import { sessionTokenStore } from './tokenStore';

afterEach(() => sessionStorage.clear());

describe('sessionTokenStore', () => {
  it('returns null when no token stored', () => {
    expect(sessionTokenStore.get()).toBeNull();
  });
  it('stores and retrieves token', () => {
    sessionTokenStore.set('abc');
    expect(sessionTokenStore.get()).toBe('abc');
  });
  it('clears token', () => {
    sessionTokenStore.set('abc');
    sessionTokenStore.clear();
    expect(sessionTokenStore.get()).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run src/auth/tokenStore.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `tokenStore.ts`**

```ts
// frontend/apps/admin-web/src/auth/tokenStore.ts
import type { TokenStore } from '../api/client';

const STORAGE_KEY = 'ppt.admin.access_token';

export const sessionTokenStore: TokenStore = {
  get: () => sessionStorage.getItem(STORAGE_KEY),
  set: (token) => sessionStorage.setItem(STORAGE_KEY, token),
  clear: () => sessionStorage.removeItem(STORAGE_KEY),
};
```

`sessionStorage` (not `localStorage`) is intentional: closing the tab logs the admin out. Storage key is namespaced so the eventual ppt-web `localStorage` (different namespace and different origin anyway) can't collide.

- [ ] **Step 4: Configure vitest jsdom env**

Create `frontend/apps/admin-web/vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./vitest.setup.ts'],
  },
});
```

Create `frontend/apps/admin-web/vitest.setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
```

- [ ] **Step 5: Run tests pass**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run`
Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/apps/admin-web/src/auth/ frontend/apps/admin-web/vitest.config.ts frontend/apps/admin-web/vitest.setup.ts
git commit -m "feat(admin-web): sessionStorage-backed token store"
```

## Task 1.5: AdminAuthContext

**Files:**
- Create: `frontend/apps/admin-web/src/auth/AdminAuthContext.tsx`
- Create: `frontend/apps/admin-web/src/auth/AdminAuthContext.test.tsx`

- [ ] **Step 1: Write failing test**

```tsx
// frontend/apps/admin-web/src/auth/AdminAuthContext.test.tsx
import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { AdminAuthProvider, useAdminAuth } from './AdminAuthContext';

function Probe() {
  const auth = useAdminAuth();
  return (
    <div>
      <span data-testid="authed">{String(auth.isAuthenticated)}</span>
      <button onClick={() => auth.setToken('t')}>login</button>
      <button onClick={() => auth.logout()}>logout</button>
    </div>
  );
}

describe('AdminAuthContext', () => {
  it('starts unauthenticated', () => {
    render(
      <AdminAuthProvider>
        <Probe />
      </AdminAuthProvider>,
    );
    expect(screen.getByTestId('authed').textContent).toBe('false');
  });

  it('becomes authenticated after setToken', async () => {
    render(
      <AdminAuthProvider>
        <Probe />
      </AdminAuthProvider>,
    );
    await act(async () => screen.getByText('login').click());
    expect(screen.getByTestId('authed').textContent).toBe('true');
  });

  it('clears on logout', async () => {
    render(
      <AdminAuthProvider>
        <Probe />
      </AdminAuthProvider>,
    );
    await act(async () => screen.getByText('login').click());
    await act(async () => screen.getByText('logout').click());
    expect(screen.getByTestId('authed').textContent).toBe('false');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run src/auth/AdminAuthContext.test.tsx`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `AdminAuthContext.tsx`**

```tsx
// frontend/apps/admin-web/src/auth/AdminAuthContext.tsx
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
```

- [ ] **Step 4: Run tests**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run src/auth/AdminAuthContext.test.tsx`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add frontend/apps/admin-web/src/auth/AdminAuthContext.tsx frontend/apps/admin-web/src/auth/AdminAuthContext.test.tsx
git commit -m "feat(admin-web): AdminAuthContext (sessionStorage-backed)"
```

## Task 1.6: ProtectedRoute + LoginPage

**Files:**
- Create: `frontend/apps/admin-web/src/components/ProtectedRoute.tsx`
- Create: `frontend/apps/admin-web/src/pages/LoginPage.tsx`
- Create: `frontend/apps/admin-web/src/pages/LoginPage.test.tsx`

- [ ] **Step 1: Write failing test for LoginPage**

```tsx
// frontend/apps/admin-web/src/pages/LoginPage.test.tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { LoginPage } from './LoginPage';

describe('LoginPage', () => {
  it('calls login API and stores token on success', async () => {
    const login = vi.fn().mockResolvedValue({ access_token: 'tk' });
    const user = userEvent.setup();
    render(
      <MemoryRouter>
        <AdminAuthProvider>
          <LoginPage loginFn={login} />
        </AdminAuthProvider>
      </MemoryRouter>,
    );
    await user.type(screen.getByLabelText(/email/i), 'admin@example.com');
    await user.type(screen.getByLabelText(/password/i), 'secret');
    await user.click(screen.getByRole('button', { name: /sign in/i }));
    expect(login).toHaveBeenCalledWith({ email: 'admin@example.com', password: 'secret' });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run src/pages/LoginPage.test.tsx`
Expected: FAIL.

- [ ] **Step 3: Implement `ProtectedRoute.tsx`**

```tsx
// frontend/apps/admin-web/src/components/ProtectedRoute.tsx
import { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';

export function ProtectedRoute({ children }: { children: ReactNode }) {
  const auth = useAdminAuth();
  const location = useLocation();
  if (!auth.isAuthenticated) {
    return <Navigate to="/login" state={{ from: location.pathname }} replace />;
  }
  return <>{children}</>;
}
```

- [ ] **Step 4: Implement `LoginPage.tsx`**

```tsx
// frontend/apps/admin-web/src/pages/LoginPage.tsx
import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';

export interface LoginResponse {
  access_token: string;
}

export interface LoginPageProps {
  /** Override for tests; default issues real fetch against /api/v1/auth/login. */
  loginFn?: (creds: { email: string; password: string }) => Promise<LoginResponse>;
}

const defaultLoginFn = async (creds: { email: string; password: string }) => {
  const resp = await fetch('/api/v1/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(creds),
  });
  if (!resp.ok) throw new Error(`login failed: ${resp.status}`);
  return (await resp.json()) as LoginResponse;
};

export function LoginPage({ loginFn = defaultLoginFn }: LoginPageProps) {
  const auth = useAdminAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const { access_token } = await loginFn({ email, password });
      auth.setToken(access_token);
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'login failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={onSubmit} aria-label="admin login">
      <h1>PPT Admin</h1>
      <label>
        Email
        <input type="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
      </label>
      <label>
        Password
        <input type="password" required value={password} onChange={(e) => setPassword(e.target.value)} />
      </label>
      {error && <p role="alert">{error}</p>}
      <button type="submit" disabled={busy}>
        {busy ? 'Signing in…' : 'Sign in'}
      </button>
    </form>
  );
}
```

- [ ] **Step 5: Run tests**

Run: `cd frontend && pnpm -F @ppt/admin-web test:run`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add frontend/apps/admin-web/src/components/ProtectedRoute.tsx frontend/apps/admin-web/src/pages/LoginPage.tsx frontend/apps/admin-web/src/pages/LoginPage.test.tsx
git commit -m "feat(admin-web): ProtectedRoute + LoginPage"
```

## Task 1.7: Copy admin pages from ppt-web

**Files (copy):**
- `frontend/apps/ppt-web/src/features/admin/pages/agencies.tsx` → `frontend/apps/admin-web/src/pages/agencies.tsx`
- `frontend/apps/ppt-web/src/features/admin/pages/users.tsx` → `frontend/apps/admin-web/src/pages/users.tsx`
- `frontend/apps/ppt-web/src/features/admin/pages/audit.tsx` → `frontend/apps/admin-web/src/pages/audit.tsx`
- `frontend/apps/ppt-web/src/features/admin/pages/feature-flags.tsx` → `frontend/apps/admin-web/src/pages/feature-flags.tsx`
- `frontend/apps/ppt-web/src/features/admin/pages/platform.tsx` → `frontend/apps/admin-web/src/pages/platform.tsx`
- `frontend/apps/ppt-web/src/features/admin/usePrincipalCapabilities.ts` → `frontend/apps/admin-web/src/auth/usePrincipalCapabilities.ts`
- `frontend/apps/ppt-web/src/features/admin/ImpersonationWrapper.tsx` → `frontend/apps/admin-web/src/components/ImpersonationWrapper.tsx`

- [ ] **Step 1: Copy files verbatim (no edits yet)**

```bash
mkdir -p frontend/apps/admin-web/src/pages frontend/apps/admin-web/src/components frontend/apps/admin-web/src/auth
cp frontend/apps/ppt-web/src/features/admin/pages/agencies.tsx     frontend/apps/admin-web/src/pages/agencies.tsx
cp frontend/apps/ppt-web/src/features/admin/pages/users.tsx        frontend/apps/admin-web/src/pages/users.tsx
cp frontend/apps/ppt-web/src/features/admin/pages/audit.tsx        frontend/apps/admin-web/src/pages/audit.tsx
cp frontend/apps/ppt-web/src/features/admin/pages/feature-flags.tsx frontend/apps/admin-web/src/pages/feature-flags.tsx
cp frontend/apps/ppt-web/src/features/admin/pages/platform.tsx     frontend/apps/admin-web/src/pages/platform.tsx
cp frontend/apps/ppt-web/src/features/admin/usePrincipalCapabilities.ts frontend/apps/admin-web/src/auth/usePrincipalCapabilities.ts
cp frontend/apps/ppt-web/src/features/admin/ImpersonationWrapper.tsx   frontend/apps/admin-web/src/components/ImpersonationWrapper.tsx
```

- [ ] **Step 2: Audit imports — replace ppt-web-internal paths**

For each copied file, open it and replace any relative imports that pointed back into `ppt-web/src/...` with admin-web equivalents. Common pattern:

```diff
- import { something } from '../../../contexts/AuthContext';
+ import { something } from '../auth/AdminAuthContext';
```

Imports of `@ppt/admin-ui`, `@ppt/api-client`, `@ppt/shared`, `@ppt/ui-kit` stay verbatim — those are workspace packages.

- [ ] **Step 3: Typecheck**

Run: `cd frontend && pnpm -F @ppt/admin-web typecheck`
Fix any remaining import errors. Expected: no errors after fixups.

- [ ] **Step 4: Commit**

```bash
git add frontend/apps/admin-web/src/pages/ frontend/apps/admin-web/src/components/ImpersonationWrapper.tsx frontend/apps/admin-web/src/auth/usePrincipalCapabilities.ts
git commit -m "feat(admin-web): copy pages and capability hooks from ppt-web"
```

## Task 1.8: AdminLayout + final App router

**Files:**
- Create: `frontend/apps/admin-web/src/components/AdminLayout.tsx`
- Modify: `frontend/apps/admin-web/src/App.tsx`

- [ ] **Step 1: Create `AdminLayout.tsx`**

```tsx
// frontend/apps/admin-web/src/components/AdminLayout.tsx
import { ReactNode } from 'react';
import { Link, Outlet } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { usePrincipalCapabilities } from '../auth/usePrincipalCapabilities';

export function AdminLayout({ children }: { children?: ReactNode }) {
  const auth = useAdminAuth();
  const { capabilities } = usePrincipalCapabilities();
  const has = (cap: string) => capabilities.includes(cap);

  return (
    <div className="admin-shell">
      <aside>
        <h2>Admin</h2>
        <nav>
          <Link to="/">Dashboard</Link>
          {has('AgencyManage') && <Link to="/agencies">Agencies</Link>}
          {has('PrincipalKindEscalate') && <Link to="/users">Users</Link>}
          {has('AuditRead') && <Link to="/audit">Audit</Link>}
          {has('FeatureFlagsWrite') && <Link to="/feature-flags">Feature flags</Link>}
          {has('SiteSettingsWrite') && <Link to="/platform">Platform</Link>}
        </nav>
        <button onClick={auth.logout}>Sign out</button>
      </aside>
      <main>{children ?? <Outlet />}</main>
    </div>
  );
}
```

Capability strings here must match `frontend/packages/admin-ui/src/capabilities.ts`. Read that file and align names exactly.

- [ ] **Step 2: Replace `src/App.tsx`**

```tsx
// frontend/apps/admin-web/src/App.tsx
import { MfaChallengeProvider } from '@ppt/admin-ui';
import { Route, Routes } from 'react-router-dom';

import { AdminLayout } from './components/AdminLayout';
import { ImpersonationWrapper } from './components/ImpersonationWrapper';
import { ProtectedRoute } from './components/ProtectedRoute';
import { AdminAuthProvider } from './auth/AdminAuthContext';
import { Dashboard } from './pages/Dashboard';
import { LoginPage } from './pages/LoginPage';
import AgenciesPage from './pages/agencies';
import UsersPage from './pages/users';
import AuditPage from './pages/audit';
import FeatureFlagsPage from './pages/feature-flags';
import PlatformPage from './pages/platform';

export function App() {
  return (
    <AdminAuthProvider>
      <MfaChallengeProvider>
        <ImpersonationWrapper>
          <Routes>
            <Route path="/login" element={<LoginPage />} />
            <Route
              element={
                <ProtectedRoute>
                  <AdminLayout />
                </ProtectedRoute>
              }
            >
              <Route index element={<Dashboard />} />
              <Route path="agencies" element={<AgenciesPage />} />
              <Route path="users" element={<UsersPage />} />
              <Route path="audit" element={<AuditPage />} />
              <Route path="feature-flags" element={<FeatureFlagsPage />} />
              <Route path="platform" element={<PlatformPage />} />
            </Route>
          </Routes>
        </ImpersonationWrapper>
      </MfaChallengeProvider>
    </AdminAuthProvider>
  );
}
```

- [ ] **Step 3: Create `Dashboard.tsx`**

```tsx
// frontend/apps/admin-web/src/pages/Dashboard.tsx
import { usePrincipalCapabilities } from '../auth/usePrincipalCapabilities';

export function Dashboard() {
  const { capabilities, isPlatformPrincipal } = usePrincipalCapabilities();
  return (
    <section aria-label="dashboard">
      <h1>Admin dashboard</h1>
      <p>Platform principal: {String(isPlatformPrincipal)}</p>
      <p>Capabilities: {capabilities.length}</p>
      <ul>
        {capabilities.map((c) => (
          <li key={c}>{c}</li>
        ))}
      </ul>
    </section>
  );
}
```

- [ ] **Step 4: Typecheck + build**

Run: `cd frontend && pnpm -F @ppt/admin-web typecheck && pnpm -F @ppt/admin-web build`
Expected: no errors. `dist/` produced.

- [ ] **Step 5: Commit**

```bash
git add frontend/apps/admin-web/src/components/AdminLayout.tsx frontend/apps/admin-web/src/pages/Dashboard.tsx frontend/apps/admin-web/src/App.tsx
git commit -m "feat(admin-web): AdminLayout + final route tree"
```

## Task 1.9: Dockerfile + nginx config

**Files:**
- Create: `docker/frontend/admin-web.Dockerfile`
- Create: `docker/nginx/admin-web.nginx.conf.template`

Follow the existing `docker/frontend/ppt-web.Dockerfile` pattern. The template should proxy `/api/*` to api-server using the same `${BG_TARGET}` and `${BG_COLOR}` envsubst trick that `ppt-web` uses.

- [ ] **Step 1: Create `docker/frontend/admin-web.Dockerfile`**

```dockerfile
# Multi-stage Dockerfile for admin-web (React SPA with Vite)
# Produces a static build served by Nginx.

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM node:20-alpine AS deps
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy workspace manifests for every package admin-web depends on.
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
COPY frontend/packages/shared/package.json ./packages/shared/
COPY frontend/packages/ui-kit/package.json ./packages/ui-kit/
COPY frontend/packages/api-client/package.json ./packages/api-client/
COPY frontend/packages/admin-ui/package.json ./packages/admin-ui/
COPY frontend/apps/admin-web/package.json ./apps/admin-web/

RUN pnpm install

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:20-alpine AS builder
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate
COPY --from=deps /app/ ./
COPY frontend/ ./
RUN pnpm --filter @ppt/admin-web build

# =============================================================================
# Stage 3: Production - Nginx
# =============================================================================
FROM nginx:alpine AS production
RUN apk add --no-cache gettext

COPY docker/nginx/admin-web.nginx.conf.template /etc/nginx/conf.d/default.conf.template
COPY docker/nginx/render-template.sh /docker-entrypoint.d/10-render-template.sh
RUN chmod +x /docker-entrypoint.d/10-render-template.sh

COPY --from=builder /app/apps/admin-web/dist /usr/share/nginx/html

RUN addgroup -g 1001 -S ppt && \
    adduser -S -D -H -u 1001 -h /var/cache/nginx -s /sbin/nologin -G ppt -g ppt ppt && \
    chown -R ppt:ppt /var/cache/nginx /var/run /run /var/log/nginx /usr/share/nginx/html /etc/nginx/conf.d

USER ppt
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
  CMD wget -qO- http://localhost:8080/health || exit 1
```

- [ ] **Step 2: Create `docker/nginx/admin-web.nginx.conf.template`**

```nginx
# Template — `${BG_TARGET}` and `${BG_COLOR}` are filled in at container
# startup by /docker-entrypoint.d/10-render-template.sh via `envsubst`.

server {
    listen 8080;
    listen [::]:8080;
    server_name _;

    root /usr/share/nginx/html;
    index index.html;

    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied expired no-cache no-store private auth;
    gzip_types
        text/plain text/css text/xml text/javascript
        application/javascript application/x-javascript application/json
        application/xml application/rss+xml application/atom+xml
        image/svg+xml;

    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Cache-Control "no-store" always;

    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable" always;
        add_header X-Frame-Options "DENY" always;
        add_header X-Content-Type-Options "nosniff" always;
        add_header X-XSS-Protection "1; mode=block" always;
        add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    }

    location /health {
        access_log off;
        return 200 "OK\n";
        add_header Content-Type text/plain;
    }

    # /api/* → api-server (same-origin shim; Caddy in front rewrites Host).
    location /api/ {
        proxy_pass http://${BG_TARGET}-api-${BG_COLOR}:8080;
        proxy_http_version 1.1;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 60s;
    }

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

Stronger CSP/cache headers than `ppt-web` (admin is sensitive). `X-Frame-Options: DENY` (not SAMEORIGIN) — admin must never be framed.

- [ ] **Step 3: Build the Docker image locally to verify**

```bash
cd /Users/martinjanci/projects/github.com/martin-janci/property-management/.claude/worktrees/stoic-sutherland-c5cc65
docker build -f docker/frontend/admin-web.Dockerfile -t admin-web:test .
```

Expected: image builds without errors.

- [ ] **Step 4: Smoke run**

```bash
docker run -d --name admin-web-test -p 8123:8080 -e BG_TARGET=staging -e BG_COLOR=blue admin-web:test
sleep 3
curl -s http://localhost:8123/health
# expected: "OK"
curl -s -o /dev/null -w '%{http_code}' http://localhost:8123/
# expected: 200
docker rm -f admin-web-test
```

- [ ] **Step 5: Commit**

```bash
git add docker/frontend/admin-web.Dockerfile docker/nginx/admin-web.nginx.conf.template
git commit -m "feat(admin-web): Dockerfile + nginx config template"
```

## Task 1.10: CI workflow for admin-web image

**Files:**
- Modify: `.github/workflows/docker-frontend.yml`

- [ ] **Step 1: Add admin-web matrix entry**

Edit `.github/workflows/docker-frontend.yml`. Locate the `matrix.include` block and append:

```yaml
          - target: ppt-admin-web
            dockerfile: docker/frontend/admin-web.Dockerfile
```

Also add to the `paths` trigger if `docker/frontend/admin-web.Dockerfile` or `docker/nginx/admin-web.nginx.conf.template` aren't covered by the existing globs — they are (`docker/frontend/**`, `docker/nginx/**`).

- [ ] **Step 2: Push branch to trigger workflow**

```bash
git push -u origin feature/admin-web-app
```

Watch the GitHub Actions run. Expected: 3 matrix jobs (ppt-web, ppt-reality-web, ppt-admin-web) all succeed; image `ghcr.io/martin-janci/ppt-admin-web:feature-admin-web-app` is pushed.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/docker-frontend.yml
git commit -m "ci(admin-web): add ppt-admin-web to docker-frontend matrix"
```

## Task 1.11: Open PR-1

- [ ] **Step 1: Push and open PR**

```bash
git push
gh pr create --title "feat(admin-web): scaffold separate super-admin app" --body "$(cat <<'EOF'
## Summary
Introduces `frontend/apps/admin-web/` — a standalone Vite SPA for the super-admin control plane, copied from `ppt-web/features/admin/pages/`. New Docker image `ghcr.io/martin-janci/ppt-admin-web` builds and is pushed by CI. No deploy plumbing in this PR; existing `ppt.rlt.sk/admin` continues to work unchanged.

Design: `docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md`

## Test plan
- [ ] `pnpm -F @ppt/admin-web build` clean
- [ ] `pnpm -F @ppt/admin-web test:run` all green
- [ ] CI matrix builds `ppt-admin-web` image and pushes to GHCR
- [ ] Local `docker run -p 8123:8080 -e BG_TARGET=staging -e BG_COLOR=blue ghcr.io/martin-janci/ppt-admin-web:<sha>` → `/health` 200, `/` 200
EOF
)"
```

Expected: PR opens, CI runs.

---

# Phase 2 (PR-2): Deploy infrastructure

Produces a publicly reachable `https://admin.rlt.sk/` and `https://admin.staging.rlt.sk/` deployed atomically with the rest of the blue/green stack. Existing four services unchanged.

**Pre-req:** PR-1 merged, `ghcr.io/martin-janci/ppt-admin-web:main` exists.

## Task 2.1: New branch from `main`

- [ ] **Step 1: Branch**

```bash
git fetch origin main
git checkout -b feature/admin-web-deploy origin/main
```

## Task 2.2: SQL migration — seed `admin.rlt.sk` into `reserved_platform_hosts`

**Files:**
- Create: `backend/crates/db/migrations/00147_reserved_admin_hosts.sql`

- [ ] **Step 1: Check next migration number**

Run: `ls backend/crates/db/migrations/ | sort | tail -5`
Confirm `00146_*` is the last applied; `00147` is free.

- [ ] **Step 2: Create migration file**

```sql
-- backend/crates/db/migrations/00147_reserved_admin_hosts.sql
--
-- Seed admin.rlt.sk and admin.staging.rlt.sk into reserved_platform_hosts so
-- the host_tenant middleware resolves them as TenantSource::PlatformHost.
-- The admin frontend (admin-web) and the api-server admin routes don't need
-- a tenant id, so PlatformHost is the correct resolution.
--
-- Idempotent: ON CONFLICT keeps re-applies safe.

INSERT INTO reserved_platform_hosts (host, reason) VALUES
  ('admin.rlt.sk',         'super-admin control plane (prod)'),
  ('admin.staging.rlt.sk', 'super-admin control plane (staging)')
ON CONFLICT (host) DO NOTHING;
```

- [ ] **Step 3: Apply against prod and staging DBs (manual run, idempotent)**

```bash
ssh hetzner 'PGPASSWORD="$(ssh hetzner 'sudo grep ^POSTGRES_PASSWORD= /srv/ppt/.env | cut -d= -f2-')" psql -h 127.0.0.1 -U ppt -d ppt_prod < /dev/stdin' < backend/crates/db/migrations/00147_reserved_admin_hosts.sql
ssh hetzner 'PGPASSWORD="$(ssh hetzner 'sudo grep ^POSTGRES_PASSWORD= /srv/ppt/.env | cut -d= -f2-')" psql -h 127.0.0.1 -U ppt -d ppt_staging < /dev/stdin' < backend/crates/db/migrations/00147_reserved_admin_hosts.sql
ssh hetzner 'PGPASSWORD="$(ssh hetzner 'sudo grep ^POSTGRES_PASSWORD= /srv/ppt/.env | cut -d= -f2-')" psql -h 127.0.0.1 -U ppt -d ppt_dev_template < /dev/stdin' < backend/crates/db/migrations/00147_reserved_admin_hosts.sql
```

Expected: `INSERT 0 2` for the first run, `INSERT 0 0` afterwards (idempotent).

- [ ] **Step 4: Verify**

```bash
ssh hetzner "PGPASSWORD="$(ssh hetzner 'sudo grep ^POSTGRES_PASSWORD= /srv/ppt/.env | cut -d= -f2-')" psql -h 127.0.0.1 -U ppt -d ppt_prod -c \"SELECT host FROM reserved_platform_hosts WHERE host LIKE 'admin%'\""
```

Expected: two rows.

- [ ] **Step 5: Commit migration**

```bash
git add backend/crates/db/migrations/00147_reserved_admin_hosts.sql
git commit -m "feat(db): reserve admin.rlt.sk hosts for super-admin"
```

## Task 2.3: Cloudflare DNS records

- [ ] **Step 1: Verify `admin.rlt.sk` resolves (already CNAME → rlt.sk in CF)**

```bash
dig +short admin.rlt.sk @1.1.1.1
```

Expected: Cloudflare anycast IPs (188.114.*). If not present, create:

```bash
# CF_DNS_TOKEN is stored on the VPS at /etc/ppt-deploy/secrets.env.
# NEVER paste the literal token into this file or any commit — GitHub
# push protection (correctly) rejects it.
CF_TOKEN="$(ssh hetzner 'sudo cat /etc/ppt-deploy/secrets.env' | grep ^CF_DNS_TOKEN= | cut -d= -f2-)"
ZONE=3aabb12beb9e35b5dbcd2c3f1d15ec1e
curl -sX POST "https://api.cloudflare.com/client/v4/zones/$ZONE/dns_records" \
  -H "Authorization: Bearer $CF_TOKEN" -H "Content-Type: application/json" \
  -d '{"type":"A","name":"admin.rlt.sk","content":"178.105.92.238","proxied":true,"ttl":1}'
```

- [ ] **Step 2: Create `admin.staging.rlt.sk` if missing**

```bash
dig +short admin.staging.rlt.sk @1.1.1.1
```

If empty (reuse `CF_TOKEN` and `ZONE` from Step 1):

```bash
curl -sX POST "https://api.cloudflare.com/client/v4/zones/$ZONE/dns_records" \
  -H "Authorization: Bearer $CF_TOKEN" -H "Content-Type: application/json" \
  -d '{"type":"A","name":"admin.staging.rlt.sk","content":"178.105.92.238","proxied":true,"ttl":1}'
```

- [ ] **Step 3: Document in runbook**

Append to `docs/runbooks/dns.md` (create if missing):

```markdown
## admin.rlt.sk

- `admin.rlt.sk` — CNAME → rlt.sk, proxied. Resolves to the Hetzner VPS via CF.
- `admin.staging.rlt.sk` — A → 178.105.92.238, proxied.
- TLS: Cloudflare Universal SSL (`*.rlt.sk` wildcard) — no Advanced Certificate
  Manager required.
- Owners: super-admin control plane (see `docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md`).
```

```bash
git add docs/runbooks/dns.md
git commit -m "docs(runbooks): document admin.rlt.sk DNS"
```

## Task 2.4: ppt-deploy Rust patch — add admin-web to BlueGreenSpec

**Files (on the VPS at `/opt/ppt-deploy-build/`):**
- Modify: `servers/deploy-server/src/api/blue_green.rs`
- Modify: spec type definition (likely in `servers/deploy-server/src/api/blue_green.rs` or `crates/deploy-types/src/`)

The exact module structure must be confirmed by reading the source first.

- [ ] **Step 1: Read current `BlueGreenSpec`**

```bash
ssh hetzner "sudo grep -rn 'BlueGreenSpec\|admin_web' /opt/ppt-deploy-build/servers/deploy-server/src /opt/ppt-deploy-build/crates 2>/dev/null | head -20"
```

Locate the struct definition. Read it to understand field names for the existing four services (`api_image`, `reality_image`, `ppt_web_image`, `reality_web_image` or similar).

- [ ] **Step 2: Add `admin_web_image` field to the struct**

Edit the spec struct definition. Example diff (paths may differ):

```rust
pub struct BlueGreenSpec {
    pub api_image: String,
    pub reality_image: String,
    pub ppt_web_image: String,
    pub reality_web_image: String,
    pub admin_web_image: String, // NEW
    // ...
}
```

- [ ] **Step 3: Wire image resolution from registered tag**

Wherever the spec is built from a tag (e.g. `register_candidate`), add:

```rust
admin_web_image: format!("{prefix}/ppt-admin-web:{tag}", prefix = backend_image_prefix),
```

The `backend_image_prefix` field already exists in config (`ghcr.io/martin-janci`).

- [ ] **Step 4: Create 5th container in `run_blue_green_color`**

In the function that creates the four service containers, add a 5th:

```rust
let admin_web_name = format!("{target}-admin-web-{color}");
svc.docker
    .run_container(&ContainerSpec {
        name: admin_web_name.clone(),
        image: spec.admin_web_image.clone(),
        env: vec![
            format!("BG_TARGET={target}"),
            format!("BG_COLOR={color}"),
        ],
        network: target_network.clone(),
        // No host port publish — Caddy reaches it via container DNS.
        healthcheck: Some(HealthCheck {
            test: vec!["CMD-SHELL".into(), "wget -qO- http://localhost:8080/health || exit 1".into()],
            interval_s: 30,
            timeout_s: 10,
            retries: 3,
            start_period_s: 5,
        }),
        ..Default::default()
    })
    .await?;
```

Match the exact `ContainerSpec` shape used for the other four services — copy the `ppt-web` block and modify the name/image.

- [ ] **Step 5: Add health-probe target for admin-web**

In the post-create health loop, add `admin_web_name` to the list of containers waited on for `(healthy)` status.

- [ ] **Step 6: Register Caddy site `admin.{reality_apex}`**

In the function that calls `caddy_register_host` (or similar), add a new site:

```rust
let admin_apex = format!("admin.{}", target_cfg.reality_apex);
let admin_routes_json = json!([
  {
    "match": [{ "path": ["/api/*"] }],
    "handle": [{
      "handler": "subroute",
      "routes": [{
        "handle": [{
          "handler": "reverse_proxy",
          "upstreams": [{ "dial": format!("{target}-api-{color}:8080") }]
        }]
      }]
    }]
  },
  {
    "handle": [{
      "handler": "subroute",
      "routes": [{
        "handle": [{
          "handler": "reverse_proxy",
          "upstreams": [{ "dial": format!("{target}-admin-web-{color}:8080") }]
        }]
      }]
    }]
  }
]);
caddy_register_site(&admin_apex, &admin_routes_json).await?;
```

(The exact shape of `caddy_register_site` and the route JSON depends on existing helpers — match what is used for the other four hosts.)

- [ ] **Step 7: Unregister on rollback / teardown**

Mirror the registration in the teardown path: when a color is decommissioned, call `caddy_unregister_site(&admin_apex)` so blue/green flips clean up the previous color's admin route.

- [ ] **Step 8: Rebuild and deploy ppt-deploy**

```bash
ssh hetzner "
cd /opt/ppt-deploy-build && \
sudo -u ppt-deploy cargo build --release -p deploy-server && \
sudo cp target/release/ppt-deploy /usr/local/bin/ppt-deploy && \
sudo systemctl restart ppt-deploy"
```

Watch journal for clean startup:

```bash
ssh hetzner "sudo journalctl -u ppt-deploy --since '30 seconds ago' --no-pager | tail -10"
```

Expected: `ppt-deploy listening` with no errors.

- [ ] **Step 9: Commit the source change**

The ppt-deploy source lives in this repo under `backend/servers/deploy-server/` (mirroring `/opt/ppt-deploy-build/`). Apply the same edits there and commit:

```bash
git add backend/servers/deploy-server/
git commit -m "feat(ppt-deploy): add admin-web as 5th container in blue/green"
```

## Task 2.5: First atomic promote to staging

- [ ] **Step 1: Run `pmctl promote --target staging main`**

```bash
pmctl promote --target staging main 2>&1 | tail -30
```

Expected: completes successfully (or 504 timeout but completes in background — verify containers).

- [ ] **Step 2: Verify 5 staging containers running and healthy**

```bash
ssh hetzner "docker ps --filter name=staging- --format 'table {{.Names}}\t{{.Status}}'"
```

Expected: 5 rows (api, reality, ppt-web, reality-web, admin-web), all `(healthy)`.

- [ ] **Step 3: Verify admin.staging.rlt.sk reachable**

```bash
until [ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 https://admin.staging.rlt.sk/)" = "200" ]; do sleep 5; done
echo "admin.staging.rlt.sk OK"
curl -s -o /dev/null -w 'admin.staging api: %{http_code}\n' --max-time 8 https://admin.staging.rlt.sk/api/v1/admin/capabilities/registry
# Expected: 401 (unauthenticated) — proves tenant resolution + routing work
```

## Task 2.6: Promote to prod

- [ ] **Step 1: Promote**

```bash
pmctl promote --target prod main 2>&1 | tail -30
```

Watch for healthy:

```bash
until [ "$(ssh hetzner "docker ps --filter name=prod- --format '{{.Status}}' | grep -c healthy")" = "5" ]; do sleep 5; done
echo "all 5 prod containers healthy"
```

- [ ] **Step 2: Verify admin.rlt.sk**

```bash
until [ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 https://admin.rlt.sk/)" = "200" ]; do sleep 5; done
echo OK
curl -s -o /dev/null -w 'admin api: %{http_code}\n' --max-time 8 https://admin.rlt.sk/api/v1/admin/capabilities/registry
# Expected: 401
```

- [ ] **Step 3: Verify `ppt.rlt.sk/admin` still works (parallel operation)**

```bash
curl -s -o /dev/null -w '%{http_code}\n' --max-time 8 https://ppt.rlt.sk/admin
# Expected: 200 (legacy admin still functional)
```

- [ ] **Step 4: Verify other domains unchanged**

```bash
for u in https://ppt.rlt.sk/ https://www.rlt.sk/ https://rlt.sk/ https://api.rlt.sk/health https://reality.rlt.sk/; do
  printf '%-30s %s\n' "$u" "$(curl -s -o /dev/null -w '%{http_code}' --max-time 8 "$u")"
done
```

Expected: all 200.

## Task 2.7: Smoke E2E — login and grant capability

- [ ] **Step 1: Manual smoke**

In a browser, navigate to `https://admin.rlt.sk/`. Log in with a known super-admin account. Trigger MFA prompt, verify TOTP. Land on `/`. Navigate to `/audit` and confirm log entries render.

Document any UX hiccups (broken icons, missing translations) as follow-ups; do not block the PR unless something is broken (white screen, console errors).

- [ ] **Step 2: Rollback rehearsal**

```bash
pmctl rollback --target prod 2>&1 | tail
```

Verify:
- `prod-admin-web-*` flips from new color back to old.
- `admin.rlt.sk` still returns 200 (Caddy site recreated for the old color).
- `ppt.rlt.sk/admin` unaffected.

Then re-promote `main` to leave prod on the new release.

## Task 2.8: Open PR-2

- [ ] **Step 1: Push and open PR**

```bash
git push -u origin feature/admin-web-deploy
gh pr create --title "feat(admin-web): deploy admin.rlt.sk + ppt-deploy patch" --body "$(cat <<'EOF'
## Summary
- `00147_reserved_admin_hosts.sql` — adds admin.rlt.sk and admin.staging.rlt.sk to `reserved_platform_hosts`
- `ppt-deploy` Rust: 5th container (admin-web), Caddy site registration for `admin.{reality_apex}`
- Cloudflare DNS records for both hosts (already in place, documented in `docs/runbooks/dns.md`)

After this PR ships and `pmctl promote prod main` runs, both `https://ppt.rlt.sk/admin` (legacy) and `https://admin.rlt.sk/` (new) are reachable in parallel. PR-3 cuts over by removing `/admin/*` from ppt-web.

Design: `docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md`

## Test plan
- [ ] Staging: `pmctl promote --target staging main` → all 5 healthy → `admin.staging.rlt.sk` returns 200 → `/api/v1/admin/capabilities/registry` returns 401
- [ ] Prod: same flow → `admin.rlt.sk` returns 200 → manual login + MFA + audit page works
- [ ] `pmctl rollback prod` flips all 5 containers atomic; `admin.rlt.sk` still serves
- [ ] No regression on existing 4 hosts (ppt.rlt.sk, www.rlt.sk, rlt.sk, api.rlt.sk, api.ppt.rlt.sk origin)
EOF
)"
```

---

# Phase 3 (PR-3): Cutover — remove `/admin/*` from ppt-web

Smaller PR. After PR-2 has been live and verified (~1 week of soak), remove the legacy admin tree from `ppt-web` to fully realize the bundle-size and isolation wins.

## Task 3.1: New branch from `main`

- [ ] **Step 1: Branch**

```bash
git fetch origin main
git checkout -b feature/admin-web-cutover origin/main
```

## Task 3.2: Remove admin imports and route from `App.tsx`

**Files:**
- Modify: `frontend/apps/ppt-web/src/App.tsx`

- [ ] **Step 1: Read current `App.tsx`**

```bash
sed -n '1,40p' frontend/apps/ppt-web/src/App.tsx
```

- [ ] **Step 2: Remove admin imports**

Delete these lines (verify exact text first):

```diff
- import { MfaChallengeProvider } from '@ppt/admin-ui';
- import { AdminRouter, ImpersonationWrapper, usePrincipalCapabilities } from './features/admin';
```

`MfaChallengeProvider` import stays only if non-admin code in `ppt-web` still uses it for /settings/two-factor. Verify with:

```bash
grep -rn 'MfaChallengeProvider\|useMfaChallenge' frontend/apps/ppt-web/src
```

If no other consumer, remove from `App.tsx` and remove its `<MfaChallengeProvider>` wrapper.

- [ ] **Step 3: Remove `<Route path="/admin/*">` and `AdminRouterRoute` function**

Locate and delete:

```diff
- function AdminRouterRoute() {
-   const { capabilities, isPlatformPrincipal } = usePrincipalCapabilities();
-   return <AdminRouter capabilities={capabilities} isPlatformPrincipal={isPlatformPrincipal} />;
- }
```

And the route:

```diff
- <Route path="/admin/*" element={<AdminRouterRoute />} />
```

- [ ] **Step 4: Remove `<Link to="/admin">` in nav**

```diff
- {isPlatformPrincipal ? <Link to="/admin">{t('nav.admin')}</Link> : null}
```

The `isPlatformPrincipal` value may become unused — remove its `usePrincipalCapabilities()` call too. If `t('nav.admin')` key is no longer referenced, remove from i18n files (optional cleanup; doesn't break anything to leave).

- [ ] **Step 5: Typecheck**

Run: `cd frontend && pnpm -F @ppt/web typecheck`
Expected: no errors. Fix any leftover unused imports flagged by `noUnusedLocals`.

## Task 3.3: Delete `features/admin/` directory

- [ ] **Step 1: Delete**

```bash
rm -rf frontend/apps/ppt-web/src/features/admin/
```

- [ ] **Step 2: Verify build still works**

```bash
cd frontend && pnpm -F @ppt/web build
```

Expected: success.

- [ ] **Step 3: Run ppt-web tests**

```bash
cd frontend && pnpm -F @ppt/web test:run
```

Expected: all pass. Any test referencing `features/admin` should already be deleted by the rm; if a test in a sibling dir referenced it, fix or delete that test too.

## Task 3.4: Remove `@ppt/admin-ui` from ppt-web's dependencies

**Files:**
- Modify: `frontend/apps/ppt-web/package.json`

- [ ] **Step 1: Verify no remaining import**

```bash
grep -rn '@ppt/admin-ui' frontend/apps/ppt-web/src
```

Expected: zero matches.

- [ ] **Step 2: Remove from `dependencies`**

```diff
- "@ppt/admin-ui": "workspace:*",
```

- [ ] **Step 3: Update lockfile**

```bash
cd frontend && pnpm install
```

- [ ] **Step 4: Build once more**

```bash
cd frontend && pnpm -F @ppt/web build
```

Expected: success, and the new `dist/stats.html` is smaller (verify by file size).

## Task 3.5: Commit + open PR-3

- [ ] **Step 1: Commit**

```bash
git add frontend/apps/ppt-web/ frontend/pnpm-lock.yaml
git commit -m "refactor(ppt-web): remove /admin/* — moved to admin.rlt.sk"
```

- [ ] **Step 2: Open PR**

```bash
git push -u origin feature/admin-web-cutover
gh pr create --title "refactor(ppt-web): remove /admin/* — admin lives at admin.rlt.sk" --body "$(cat <<'EOF'
## Summary
PR-3 of 3 (cutover): removes the super-admin tree from ppt-web now that admin.rlt.sk has been live and verified. Deletes `frontend/apps/ppt-web/src/features/admin/`, the `/admin/*` route, the nav link, and the `@ppt/admin-ui` dependency from ppt-web.

After this lands, ppt-web bundle is smaller (no admin chunks), `ppt.rlt.sk/admin` returns 404, and `admin.rlt.sk` remains the only super-admin entrypoint.

Design: `docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md`

## Test plan
- [ ] `pnpm -F @ppt/web build` clean
- [ ] `pnpm -F @ppt/web test:run` all green
- [ ] After deploy: `https://ppt.rlt.sk/admin` returns 404 (SPA renders, then router 404s)
- [ ] `https://admin.rlt.sk/` continues to function
- [ ] Bundle size of `ppt-web/dist/` is smaller than pre-cutover (compare stats.html)
EOF
)"
```

## Task 3.6: Deploy PR-3

After merge:

- [ ] **Step 1: Promote**

```bash
pmctl promote --target staging main
# verify staging.ppt.rlt.sk/admin returns 404 SPA
pmctl promote --target prod main
```

- [ ] **Step 2: Verify**

```bash
curl -s -o /dev/null -w 'ppt.rlt.sk/admin: %{http_code}\n' https://ppt.rlt.sk/admin
# expected: 200 (SPA returns index, but router shows 404 page) — confirm in browser console no admin chunks loaded
curl -s -o /dev/null -w 'admin.rlt.sk: %{http_code}\n' https://admin.rlt.sk/
# expected: 200
```

---

# Self-review checklist

- ✅ **Spec coverage:**
  - Architecture (admin.rlt.sk → Caddy → admin-web + api-server proxy) — Tasks 1.2, 1.9, 2.4
  - Data flow (login, cookie scope, API proxy, error handling) — Tasks 1.3-1.6
  - Code organization (admin-web layout, copied pages) — Tasks 1.1-1.8
  - Deploy (DNS, Caddy, ppt-deploy patch, reserved_platform_hosts) — Tasks 2.2-2.4
  - Testing (unit, integration, E2E smoke) — Tasks 1.3-1.6 (unit), 2.5-2.7 (smoke)
  - Acceptance criteria 1-8 — covered by Tasks 2.5-2.8 and 3.6

- ✅ **Placeholder scan:** No "TBD" / "TODO"; every code step has actual code. Two acknowledged ambiguities (exact `BlueGreenSpec` struct path in 2.4, exact `caddy_register_site` helper shape) are flagged with "match what is used for the other four hosts" — concrete instruction the engineer can follow after reading the source.

- ✅ **Type consistency:** `TokenStore` interface declared in 1.3 used in 1.4 and 1.5. `LoginResponse`, `AdminAuthValue`, `AdminAuthProvider` consistent across 1.5-1.6. Container naming `{target}-admin-web-{color}` consistent across 2.4-2.6 and matches existing `{target}-api-{color}` etc.

# Execution handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-16-admin-web-separate-app.md`. Two execution options:**

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
