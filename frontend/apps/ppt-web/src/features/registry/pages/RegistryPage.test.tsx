/// <reference types="vitest/globals" />
/**
 * RegistryPage auth-context regression tests.
 *
 * History: RegistryPage (and RegistryRulesPage) built the registry API client
 * at *module scope* with only a `baseUrl` and no `accessToken`/`tenantId`:
 *
 *     const registryApi = createRegistryApi({ baseUrl: API_BASE_URL });
 *
 * Every registry request therefore shipped with no `Authorization: Bearer`
 * header (401 / non-functional), and because the singleton was created once at
 * import time it never picked up a token nor rotated when the token refreshed.
 *
 * These tests lock in the canonical `useBuildings.ts` pattern: the client is
 * (re)built from `getToken()` inside a `useMemo`, so it carries the current
 * access token and is rebuilt when the token changes. They fail on the
 * pre-fix version because `createRegistryApi` was only ever called with
 * `{ baseUrl }` — never with an `accessToken`.
 */
import { render } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const { createRegistryApi, getToken } = vi.hoisted(() => ({
  createRegistryApi: vi.fn(() => ({})),
  getToken: vi.fn<[], string | null>(() => 'tok-initial'),
}));

const stubHooks = {
  usePetsList: () => ({ data: undefined, isLoading: false, error: null }),
  useVehiclesList: () => ({ data: undefined, isLoading: false, error: null }),
  useDeletePet: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useReviewPet: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useDeleteVehicle: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useReviewVehicle: () => ({ mutateAsync: vi.fn(), isPending: false }),
};

vi.mock('@ppt/api-client', () => ({
  createRegistryApi,
  createRegistryHooks: vi.fn(() => stubHooks),
  getToken,
}));

// Imported after the mock so the (pre-fix) module-scope client build is
// recorded against the mocked `createRegistryApi`.
import { RegistryPage } from './RegistryPage';

afterEach(() => {
  vi.clearAllMocks();
  getToken.mockReturnValue('tok-initial');
});

describe('RegistryPage auth context', () => {
  it('builds the registry API client with the current access token', () => {
    getToken.mockReturnValue('tok-initial');
    render(<RegistryPage />);

    // Every build must carry the Bearer token — none may be token-less.
    expect(createRegistryApi).toHaveBeenCalledWith(
      expect.objectContaining({ accessToken: 'tok-initial' })
    );
    expect(createRegistryApi).not.toHaveBeenCalledWith(
      expect.objectContaining({ accessToken: undefined })
    );
  });

  it('rebuilds the client when the token rotates', () => {
    getToken.mockReturnValue('tok-old');
    const { rerender } = render(<RegistryPage />);

    getToken.mockReturnValue('tok-new');
    rerender(<RegistryPage />);

    expect(createRegistryApi).toHaveBeenCalledWith(
      expect.objectContaining({ accessToken: 'tok-new' })
    );
  });
});
