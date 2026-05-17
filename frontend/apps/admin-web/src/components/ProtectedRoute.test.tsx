/**
 * ProtectedRoute — covers all 4 guard paths.
 */

import { CapabilityProvider } from '@ppt/admin-ui';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from '../auth/AdminAuthContext';
import { sessionTokenStore } from '../auth/tokenStore';
import { ProtectedRoute } from './ProtectedRoute';

// ---------------------------------------------------------------------------
// Module mocks
// ---------------------------------------------------------------------------

// usePrincipalCapabilities is consumed inside ProtectedRoute — mock the module
// so we can control isPlatformPrincipal / isLoading per test.

const mockUsePrincipalCapabilities = vi.fn(() => ({
  isPlatformPrincipal: false,
  capabilities: [] as string[],
  isLoading: false,
}));

vi.mock('../auth/usePrincipalCapabilities', () => ({
  usePrincipalCapabilities: () => mockUsePrincipalCapabilities(),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

interface RenderOptions {
  token?: string | null;
  capabilities?: string[];
  isPlatformPrincipal?: boolean;
  isLoading?: boolean;
  requiredCapability?: string | string[];
  initialEntries?: string[];
}

function renderProtected({
  token = null,
  capabilities = [],
  isPlatformPrincipal = false,
  isLoading = false,
  requiredCapability,
  initialEntries = ['/'],
}: RenderOptions = {}) {
  // Seed sessionStorage so AdminAuthProvider picks up the token
  sessionStorage.clear();
  if (token) sessionTokenStore.set(token);

  mockUsePrincipalCapabilities.mockReturnValue({
    isPlatformPrincipal,
    capabilities,
    isLoading,
  });

  const qc = makeQueryClient();

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={initialEntries}>
        <AdminAuthProvider>
          <CapabilityProvider
            value={{ isPlatformPrincipal, capabilities: capabilities as never[] }}
          >
            <Routes>
              <Route
                path="/"
                element={
                  <ProtectedRoute requiredCapability={requiredCapability}>
                    <span>Protected content</span>
                  </ProtectedRoute>
                }
              />
              <Route path="/login" element={<span>Login page</span>} />
            </Routes>
          </CapabilityProvider>
        </AdminAuthProvider>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('ProtectedRoute', () => {
  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // Path 1 — unauthenticated → redirect to /login
  it('redirects unauthenticated user to /login', () => {
    renderProtected({ token: null });
    expect(screen.getByText('Login page')).toBeDefined();
    expect(screen.queryByText('Protected content')).toBeNull();
  });

  // Path 2 — authenticated but not a platform principal → 403 section
  it('shows 403 section when user is not a platform principal', () => {
    renderProtected({ token: 'tok', isPlatformPrincipal: false, isLoading: false });
    expect(screen.getByRole('alert')).toBeDefined();
    expect(screen.getByText(/Not a platform principal/i)).toBeDefined();
    expect(screen.queryByText('Protected content')).toBeNull();
  });

  // Path 3 — platform principal but missing requiredCapability → ForbiddenPage
  it('shows ForbiddenPage when platform principal lacks required capability', () => {
    renderProtected({
      token: 'tok',
      isPlatformPrincipal: true,
      capabilities: ['agencies_read'],
      requiredCapability: 'tenant_purge',
    });
    // ForbiddenPage renders role="alert" section with "403 — Missing capability"
    expect(screen.getByText(/403.*Missing capability/i)).toBeDefined();
    expect(screen.queryByText('Protected content')).toBeNull();
  });

  // Path 4 — platform principal WITH required capability → renders children
  it('renders children when platform principal holds required capability', () => {
    renderProtected({
      token: 'tok',
      isPlatformPrincipal: true,
      capabilities: ['tenant_purge'],
      requiredCapability: 'tenant_purge',
    });
    expect(screen.getByText('Protected content')).toBeDefined();
  });

  // No requiredCapability → renders children for any platform principal
  it('renders children for platform principal when no requiredCapability set', () => {
    renderProtected({
      token: 'tok',
      isPlatformPrincipal: true,
      capabilities: [],
    });
    expect(screen.getByText('Protected content')).toBeDefined();
  });

  // isLoading → shows loading state
  it('shows loading indicator while capabilities are loading', () => {
    renderProtected({ token: 'tok', isPlatformPrincipal: false, isLoading: true });
    expect(screen.getByText(/Verifying access/i)).toBeDefined();
    expect(screen.queryByText('Protected content')).toBeNull();
  });

  // requiredCapability as array — any-of matching
  it('renders children when user holds at least one of the required capabilities (array)', () => {
    renderProtected({
      token: 'tok',
      isPlatformPrincipal: true,
      capabilities: ['agencies_read', 'audit_read'],
      requiredCapability: ['tenant_purge', 'audit_read'],
    });
    expect(screen.getByText('Protected content')).toBeDefined();
  });
});
