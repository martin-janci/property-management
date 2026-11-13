/// <reference types="vitest/globals" />
/**
 * The /disputes list route must call the same hooks on every render (round-3
 * audit #1). It returned `<AuthRequiredGate/>` before `useState` /
 * `useDisputes`, so when the user's organization arrived after the first
 * render React threw "Rendered more hooks than during the previous render".
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { disputeRoutes } from './disputes';

const h = vi.hoisted(() => ({
  user: null as null | { id: string; organizationId?: string; role: string },
  useDisputes: vi.fn(),
}));

vi.mock('../lazyRoutes', () => ({
  DisputesPage: ({ total }: { total: number }) => <div>disputes list ({total})</div>,
  DisputeDetailPage: () => <div />,
  MediationWorkspacePage: () => <div />,
}));

vi.mock('@ppt/api-client', () => ({
  useDisputes: (org: string, query: unknown) => h.useDisputes(org, query),
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: h.user }),
}));

vi.mock('../../components', () => ({
  useToast: () => ({ showToast: vi.fn() }),
  AuthRequiredGate: () => <div>auth required</div>,
  // Route-level auth guard is transparent here — the guard itself is covered by
  // mgmt-routes-auth-guard.route.test.tsx; this suite exercises list behavior.
  ProtectedRoute: ({ children }: { children?: import('react').ReactNode }) => <>{children}</>,
}));

function tree() {
  return (
    <MemoryRouter initialEntries={['/disputes']}>
      <Routes>{disputeRoutes()}</Routes>
    </MemoryRouter>
  );
}

beforeEach(() => {
  h.useDisputes.mockReturnValue({ data: { data: [], total: 3 }, isLoading: false, error: null });
});

it('shows the gate without an organization, then the list once it arrives', () => {
  h.user = { id: 'u1', role: 'tenant' };
  const { rerender } = render(tree());
  expect(screen.getByText('auth required')).toBeTruthy();
  expect(h.useDisputes).toHaveBeenCalledWith('', expect.anything());

  h.user = { id: 'u1', organizationId: 'org-1', role: 'tenant' };
  rerender(tree());
  expect(screen.getByText('disputes list (3)')).toBeTruthy();
  expect(h.useDisputes).toHaveBeenLastCalledWith('org-1', expect.anything());
});
