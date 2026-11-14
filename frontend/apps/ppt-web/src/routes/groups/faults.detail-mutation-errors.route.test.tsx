/// <reference types="vitest/globals" />
/**
 * Fault-detail action mutations surface failures to the user
 * (code-review-ppt-web-core-fault-detail-mutations-swallow-errors).
 *
 * The detail route (`FaultDetailPageRoute`) wires the page's triage / resolve /
 * confirm / reopen / add-comment / delete-attachment callbacks straight to the
 * TanStack mutation `.mutate(...)`. Those calls used to pass NO options object,
 * so a rejected mutation had no `onError` handler and the failure was swallowed
 * — the manager saw nothing and could believe the action had succeeded. Only
 * the photo-upload and withdraw callbacks surfaced their errors.
 *
 * The fix threads an `onError` toast (consistent with the sibling photo-upload
 * / withdraw handlers) through every detail-page action mutation. This test
 * drives each action to failure and asserts an error toast is raised.
 *
 * IG3: on the pre-fix tree the mutations are invoked WITHOUT an `onError`
 * option, so the fake `mutate` below never runs the callback and `showToast`
 * is never called — each case fails. With the fix the option is present and
 * the error toast fires.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

// A manager drives all detail-page actions.
const authState = vi.hoisted(() => ({ role: 'manager' as string | undefined }));
// Captured across renders so the assertions can inspect the raised toasts.
const toastState = vi.hoisted(() => ({ showToast: vi.fn() }));
// The fault status gates which action buttons the page renders.
const faultState = vi.hoisted(() => ({ status: 'triaged' as string }));

function auth(): AuthContextValue {
  return {
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'mgr-1', email: 'm@x.z', role: authState.role } as AuthContextValue['user'],
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

/**
 * A mutation whose `.mutate(vars, opts?)` always fails by invoking the
 * caller-supplied `onError`. On the pre-fix tree the route passes no `opts`,
 * so `onError` is undefined and nothing happens — exactly the swallowed-error
 * bug this test pins.
 */
const mutationError = new Error('boom');
function failingMutation() {
  return {
    mutate: vi.fn(
      (_vars: unknown, opts?: { onError?: (e: unknown) => void; onSuccess?: () => void }) => {
        opts?.onError?.(mutationError);
      }
    ),
    isPending: false,
  };
}

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => auth() };
});
vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => auth() };
});
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: toastState.showToast }) };
});

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  getToken: () => 'test-token',
  useFault: () => ({
    data: {
      fault: {
        id: 'f-1',
        organization_id: 'o-1',
        building_id: 'b-1',
        unit_id: null,
        reporter_id: 'rep-1',
        title: 'Dripping tap in unit 3B',
        description: 'Water everywhere',
        location_description: null,
        category: 'plumbing',
        priority: 'medium',
        status: faultState.status,
        created_at: '2026-06-01T00:00:00Z',
        updated_at: '2026-06-01T00:00:00Z',
        reporter_name: 'Rita Reporter',
        reporter_email: 'rita@x.z',
        building_name: 'Maple Court',
        building_address: '1 Maple St',
        attachment_count: 1,
        comment_count: 0,
      },
      timeline: [],
      attachments: [
        {
          id: 'att-1',
          fault_id: 'f-1',
          filename: 'leak.jpg',
          original_filename: 'leak.jpg',
          content_type: 'image/jpeg',
          size_bytes: 1024,
          storage_url: 'https://example.test/leak.jpg',
          thumbnail_url: null,
          description: null,
          created_at: '2026-06-01T00:00:00Z',
        },
      ],
    },
    isLoading: false,
    error: null,
  }),
  useTriageFault: () => failingMutation(),
  useResolveFault: () => failingMutation(),
  useConfirmFault: () => failingMutation(),
  useReopenFault: () => failingMutation(),
  useAddComment: () => failingMutation(),
  useAddAttachment: () => failingMutation(),
  useDeleteAttachment: () => failingMutation(),
  useWithdrawFault: () => failingMutation(),
  useBuildings: () => ({ data: undefined }),
}));

import { faultRoutes } from './faults';

function renderDetail() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/faults/f-1']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{faultRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

function lastErrorToast() {
  return toastState.showToast.mock.calls
    .map((c) => c[0] as { type?: string })
    .filter((t) => t?.type === 'error')
    .at(-1);
}

describe('fault-detail action mutations surface errors', () => {
  beforeEach(() => {
    authState.role = 'manager';
    toastState.showToast.mockClear();
    faultState.status = 'triaged';
  });

  it('surfaces an error toast when adding a comment fails', async () => {
    renderDetail();
    const textarea = await screen.findByPlaceholderText('Write a comment...');
    fireEvent.change(textarea, { target: { value: 'Please hurry' } });
    fireEvent.click(screen.getByRole('button', { name: 'Add Comment' }));

    expect(lastErrorToast()).toMatchObject({ type: 'error', error: mutationError });
  });

  it('surfaces an error toast when resolving fails', async () => {
    faultState.status = 'triaged';
    renderDetail();
    // Open the resolve dialog, then submit it.
    const openBtn = await screen.findByRole('button', { name: 'Resolve' });
    fireEvent.click(openBtn);
    const resolveButtons = screen.getAllByRole('button', { name: 'Resolve' });
    fireEvent.click(resolveButtons[resolveButtons.length - 1]);

    expect(lastErrorToast()).toMatchObject({ type: 'error', error: mutationError });
  });

  it('surfaces an error toast when reopening fails', async () => {
    faultState.status = 'closed';
    renderDetail();
    const openBtn = await screen.findByRole('button', { name: 'Reopen' });
    fireEvent.click(openBtn);
    const reason = screen.getByPlaceholderText('Why does this need to be reopened?');
    fireEvent.change(reason, { target: { value: 'Still broken' } });
    const reopenButtons = screen.getAllByRole('button', { name: 'Reopen' });
    fireEvent.click(reopenButtons[reopenButtons.length - 1]);

    expect(lastErrorToast()).toMatchObject({ type: 'error', error: mutationError });
  });

  it('surfaces an error toast when deleting an attachment fails', async () => {
    renderDetail();
    // The per-attachment delete control is icon-only; its accessible label is
    // the `title` ("Delete").
    const delBtn = await screen.findByTitle('Delete');
    fireEvent.click(delBtn);

    expect(lastErrorToast()).toMatchObject({ type: 'error', error: mutationError });
  });
});
