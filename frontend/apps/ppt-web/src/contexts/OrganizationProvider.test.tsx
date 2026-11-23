/**
 * OrganizationProvider tests — lock in the memoized context value.
 *
 * Regression guard for the "unmemoized context value" finding: the provider
 * must hand consumers a referentially *stable* value while the organization
 * identity is unchanged, even though `AuthProvider` produces a brand-new
 * `user` object on every render. Without the `useMemo` (deps:
 * organizationId, organizationName) the provider would allocate a fresh
 * object literal each render and force every `useOrganization()` consumer to
 * re-render. The stable-identity assertions below fail on the unmemoized
 * version and pass on the memoized one.
 */
/// <reference types="vitest/globals" />

import type { AuthUser } from '@ppt/api-client';
import { render } from '@testing-library/react';
import { useContext } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { OrganizationContext, type OrganizationContextValue } from '../hooks/useOrganization';

// Drive the provider via a mutable mock `useAuth`, so each render can supply a
// fresh `user` object (mirroring AuthProvider's per-render allocation) while
// keeping the organization identity under test's control.
let mockAuth: { user: AuthUser | null } = { user: null };
vi.mock('./AuthContext', () => ({
  useAuth: () => mockAuth,
}));

// Imported after the mock is registered.
import { OrganizationProvider } from './OrganizationProvider';

function userWithOrg(organizationId: string, organizationName?: string): AuthUser {
  // A new object each call — the provider must not key its memo off `user`'s
  // reference, only off the org identity primitives.
  return {
    id: 'user-1',
    email: 'u@example.com',
    organizationId,
    organizationName,
  };
}

/** Captures the context value reference seen on each render. */
function Capture({ sink }: { sink: Array<OrganizationContextValue | null> }) {
  sink.push(useContext(OrganizationContext));
  return null;
}

function tree(sink: Array<OrganizationContextValue | null>) {
  return (
    <OrganizationProvider>
      <Capture sink={sink} />
    </OrganizationProvider>
  );
}

beforeEach(() => {
  mockAuth = { user: null };
});

describe('OrganizationProvider', () => {
  it('keeps a stable context value reference across re-renders when the org identity is unchanged', () => {
    const sink: Array<OrganizationContextValue | null> = [];
    mockAuth = { user: userWithOrg('org-1', 'Acme') };

    const { rerender } = render(tree(sink));
    // New `user` object, identical org identity — simulates an AuthProvider
    // re-render that did not change the organization.
    mockAuth = { user: userWithOrg('org-1', 'Acme') };
    rerender(tree(sink));

    expect(sink.length).toBe(2);
    expect(sink[0]).toEqual({ organizationId: 'org-1', organizationName: 'Acme' });
    // The load-bearing assertion: identical reference => consumers skip re-render.
    expect(sink[1]).toBe(sink[0]);
  });

  it('produces a new context value reference when the org identity changes', () => {
    const sink: Array<OrganizationContextValue | null> = [];
    mockAuth = { user: userWithOrg('org-1', 'Acme') };

    const { rerender } = render(tree(sink));
    mockAuth = { user: userWithOrg('org-2', 'Globex') };
    rerender(tree(sink));

    expect(sink[1]).not.toBe(sink[0]);
    expect(sink[1]).toEqual({ organizationId: 'org-2', organizationName: 'Globex' });
  });

  it('exposes null when there is no authenticated organization', () => {
    const sink: Array<OrganizationContextValue | null> = [];
    mockAuth = { user: null };

    render(tree(sink));

    expect(sink[0]).toBeNull();
  });
});
