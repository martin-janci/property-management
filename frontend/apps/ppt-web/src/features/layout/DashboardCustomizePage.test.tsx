/// <reference types="vitest/globals" />
/**
 * DashboardCustomizePage — TDD tests (Task 4).
 *
 * Five scenarios per the task brief:
 *  1. Renders editor rows from the envelope's published sections.
 *  2. Not-published envelope shows info panel, no editor.
 *  3. Toggle → Save calls saveTenantLayoutOverride('ppt/dashboard', patch).
 *  4. 422 errors render verbatim.
 *  5. ManagerDashboardPage shows Customize link for org_admin, hides for resident.
 */
import type { TenantLayoutEnvelope } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ManagerDashboardPage } from '../dashboard/pages/ManagerDashboardPage';
import { DashboardCustomizePage } from './DashboardCustomizePage';

// ---------------------------------------------------------------------------
// Shared mutable state — mutated per test, read by the mocks
// ---------------------------------------------------------------------------

let mockEnvelope: TenantLayoutEnvelope = {
  override: null,
  rails: { hideable: ['hero'], mode_editable: [], reorderable: false, prop_whitelist: {} },
  published: {
    sections: [
      { type: 'hero', visible: true },
      { type: 'stats', visible: true },
    ],
  },
  manifest: null,
};

// mutateAsync is replaced per test
let mockSaveImpl: ReturnType<typeof vi.fn> = vi.fn().mockResolvedValue({});

let mockAuthRole = 'org_admin';

// ---------------------------------------------------------------------------
// Module mocks
// ---------------------------------------------------------------------------

vi.mock('@ppt/api-client', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    useTenantLayout: vi.fn(() => ({
      data: mockEnvelope,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
    })),
    useSaveTenantLayoutOverride: vi.fn(() => ({
      mutateAsync: (...args: unknown[]) => mockSaveImpl(...args),
      isPending: false,
    })),
    useResolvedLayout: vi.fn(() => ({ data: null, isLoading: false, isError: false })),
    TenantLayoutError: class TenantLayoutError extends Error {
      status: number;
      errors: string[];
      constructor(status: number, errors: string[]) {
        super(`TenantLayoutError: HTTP ${status}`);
        this.name = 'TenantLayoutError';
        this.status = status;
        this.errors = errors;
      }
    },
  };
});

vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    useTranslation: () => ({ t: (key: string) => key }),
  };
});

vi.mock('../../contexts/AuthContext', () => ({
  useAuth: vi.fn(() => ({ user: { role: mockAuthRole }, isAuthenticated: true })),
}));

vi.mock('../../../contexts/AuthContext', () => ({
  useAuth: vi.fn(() => ({ user: { role: mockAuthRole }, isAuthenticated: true })),
}));

// Stub useActionQueue for ManagerDashboardPage
vi.mock('../dashboard/hooks/useActionQueue', () => ({
  useActionQueue: vi.fn(() => ({
    items: [],
    total: 0,
    stats: { total: 0, urgent: 0, high: 0, medium: 0, low: 0 },
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
    dismissItem: vi.fn(),
    handleAction: vi.fn(),
    isExecuting: false,
  })),
}));

// Stub ToastProvider. `showToast` is a single stable spy (not a fresh vi.fn()
// per call) so tests can assert on what was toasted.
const mockShowToast = vi.fn();
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    useToast: vi.fn(() => ({ showToast: mockShowToast })),
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

afterEach(() => {
  vi.clearAllMocks();
  mockSaveImpl = vi.fn().mockResolvedValue({});
  mockAuthRole = 'org_admin';
});

function renderPage() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <DashboardCustomizePage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

function renderManagerDashboard() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <ManagerDashboardPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Test 1: renders editor rows from the envelope's published sections
// ---------------------------------------------------------------------------
describe('Test 1: renders editor rows from published sections', () => {
  it('shows a row for each published section type', async () => {
    mockEnvelope = {
      override: null,
      rails: { hideable: ['hero'], mode_editable: [], reorderable: false, prop_whitelist: {} },
      published: {
        sections: [
          { type: 'hero', visible: true },
          { type: 'stats', visible: true },
        ],
      },
      manifest: null,
    };
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('hero')).toBeInTheDocument();
      expect(screen.getByText('stats')).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Test 2: not-published envelope shows info panel and no editor
// ---------------------------------------------------------------------------
describe('Test 2: not-published envelope shows info panel, no editor', () => {
  it('renders the not-published info panel when published is null', async () => {
    mockEnvelope = {
      override: null,
      rails: { hideable: [], mode_editable: [], reorderable: false, prop_whitelist: {} },
      published: null,
      manifest: null,
    };
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('layout.customize.notPublished')).toBeInTheDocument();
    });
    expect(screen.queryByText('hero')).toBeNull();
  });

  it('renders read-only rows when rails is empty object without crashing', async () => {
    mockEnvelope = {
      override: null,
      rails: Object.create(null),
      published: {
        sections: [
          { type: 'hero', visible: true },
          { type: 'stats', visible: true },
        ],
      },
      manifest: null,
    };
    renderPage();
    await waitFor(() => {
      expect(screen.getByText('hero')).toBeInTheDocument();
      expect(screen.getByText('stats')).toBeInTheDocument();
    });
  });
});

// ---------------------------------------------------------------------------
// Test 3: toggle → Save calls saveTenantLayoutOverride
// ---------------------------------------------------------------------------
describe('Test 3: toggle → Save calls mutation', () => {
  it('calls mutateAsync with patched override when Save is clicked after toggling', async () => {
    const user = userEvent.setup();
    mockSaveImpl = vi.fn().mockResolvedValue({});
    mockEnvelope = {
      override: null,
      rails: { hideable: ['hero'], mode_editable: [], reorderable: false, prop_whitelist: {} },
      published: { sections: [{ type: 'hero', visible: true }] },
      manifest: null,
    };
    renderPage();

    await waitFor(() => expect(screen.getByText('hero')).toBeInTheDocument());

    const toggle = screen.getByRole('button', { name: /layout\.customize\.(hide|show)/ });
    await user.click(toggle);

    const saveBtn = screen.getByRole('button', { name: 'layout.customize.save' });
    expect(saveBtn).not.toBeDisabled();
    await user.click(saveBtn);

    await waitFor(() => {
      expect(mockSaveImpl).toHaveBeenCalledOnce();
    });
    const [calledWith] = mockSaveImpl.mock.calls[0] as [unknown];
    expect(calledWith).toMatchObject({ sections: { hero: { visible: false } } });
  });
});

// ---------------------------------------------------------------------------
// Test 4: 422 errors render verbatim in role=alert list
// ---------------------------------------------------------------------------
describe('Test 4: 422 errors render verbatim', () => {
  it('shows error list when mutateAsync rejects with TenantLayoutError', async () => {
    const user = userEvent.setup();
    const { TenantLayoutError } = await import('@ppt/api-client');
    mockSaveImpl = vi
      .fn()
      .mockRejectedValue(new TenantLayoutError(422, ['Field X is invalid', 'Field Y missing']));
    mockEnvelope = {
      override: null,
      rails: { hideable: ['hero'], mode_editable: [], reorderable: false, prop_whitelist: {} },
      published: { sections: [{ type: 'hero', visible: true }] },
      manifest: null,
    };
    renderPage();

    await waitFor(() => expect(screen.getByText('hero')).toBeInTheDocument());

    const toggle = screen.getByRole('button', { name: /layout\.customize\.(hide|show)/ });
    await user.click(toggle);
    const saveBtn = screen.getByRole('button', { name: 'layout.customize.save' });
    await user.click(saveBtn);

    await waitFor(() => {
      const alert = document.querySelector('[role="alert"]');
      expect(alert).toBeInTheDocument();
      expect(alert?.textContent).toContain('Field X is invalid');
      expect(alert?.textContent).toContain('Field Y missing');
    });
  });
});

// ---------------------------------------------------------------------------
// Test 4b: a 422 carrying an EMPTY errors array must fall back to the generic
// save-error toast. Rendering `[]` into the alert list would otherwise leave
// the user with an empty <ul role="alert"> and no indication the save failed.
// ---------------------------------------------------------------------------
describe('Test 4b: 422 with empty errors shows generic toast', () => {
  it('shows the generic save-error toast instead of an empty list', async () => {
    const user = userEvent.setup();
    const { TenantLayoutError } = await import('@ppt/api-client');
    mockSaveImpl = vi.fn().mockRejectedValue(new TenantLayoutError(422, []));
    mockEnvelope = {
      override: null,
      rails: { hideable: ['hero'], mode_editable: [], reorderable: false, prop_whitelist: {} },
      published: { sections: [{ type: 'hero', visible: true }] },
      manifest: null,
    };
    renderPage();

    await waitFor(() => expect(screen.getByText('hero')).toBeInTheDocument());

    await user.click(screen.getByRole('button', { name: /layout\.customize\.(hide|show)/ }));
    await user.click(screen.getByRole('button', { name: 'layout.customize.save' }));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', title: 'layout.customize.saveError' })
      );
    });
    // No empty validation-error list rendered
    expect(document.querySelector('ul[role="alert"]')).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Test 6: an edit made while a save is in flight must not be discarded.
//
// Regression for the tautological "changed since sent" check: the previous
// implementation compared the closure-captured override against a snapshot
// taken from the same render (always equal), so it cleared dirty on success
// even when the user had edited during the in-flight save — silently losing
// that edit. The fix reads the CURRENT override via a ref and re-marks the
// form dirty when it diverges from the sent payload.
// ---------------------------------------------------------------------------
describe('Test 6: edit during in-flight save is preserved', () => {
  it('keeps the form dirty when a change lands while the save is pending', async () => {
    const user = userEvent.setup();

    // Deferred save — stays pending until we resolve it, letting us inject an
    // edit between save-start and save-success.
    let resolveSave: (value: unknown) => void = () => {};
    mockSaveImpl = vi.fn(
      () =>
        new Promise((res) => {
          resolveSave = res;
        })
    );

    mockEnvelope = {
      override: null,
      rails: {
        hideable: ['hero', 'stats'],
        mode_editable: [],
        reorderable: false,
        prop_whitelist: {},
      },
      published: {
        sections: [
          { type: 'hero', visible: true },
          { type: 'stats', visible: true },
        ],
      },
      manifest: null,
    };
    renderPage();

    await waitFor(() => expect(screen.getByText('hero')).toBeInTheDocument());

    // Edit #1: toggle hero → form dirty, Save enabled.
    const firstToggle = screen.getAllByRole('button', {
      name: /layout\.customize\.(hide|show)/,
    })[0];
    await user.click(firstToggle);

    const saveBtn = screen.getByRole('button', { name: 'layout.customize.save' });
    expect(saveBtn).not.toBeDisabled();

    // Start the save — mutateAsync is now pending.
    await user.click(saveBtn);
    await waitFor(() => expect(mockSaveImpl).toHaveBeenCalledOnce());

    // The save captured only the hero patch.
    const [sentPayload] = mockSaveImpl.mock.calls[0] as [{ sections?: Record<string, unknown> }];
    expect(sentPayload.sections).toMatchObject({ hero: { visible: false } });
    expect(sentPayload.sections).not.toHaveProperty('stats');

    // Edit #2 (concurrent): toggle stats WHILE the save is in flight.
    const statsToggle = screen.getAllByRole('button', {
      name: /layout\.customize\.(hide|show)/,
    })[1];
    await user.click(statsToggle);

    // Now let the in-flight save resolve.
    await act(async () => {
      resolveSave({});
    });

    // The later edit must survive: the form stays dirty (Save re-enabled) so
    // the newer stats change is not silently discarded.
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'layout.customize.save' })).not.toBeDisabled();
    });

    // A follow-up save must send BOTH edits, proving the stats toggle was kept.
    resolveSave = () => {};
    mockSaveImpl.mockResolvedValueOnce({});
    await user.click(screen.getByRole('button', { name: 'layout.customize.save' }));
    await waitFor(() => expect(mockSaveImpl).toHaveBeenCalledTimes(2));
    const [secondPayload] = mockSaveImpl.mock.calls[1] as [{ sections?: Record<string, unknown> }];
    expect(secondPayload.sections).toMatchObject({
      hero: { visible: false },
      stats: { visible: false },
    });
  });
});

// ---------------------------------------------------------------------------
// Test 5: ManagerDashboardPage shows Customize link for org_admin, hides for resident
// ---------------------------------------------------------------------------
describe('Test 5: Customize link role-gated on ManagerDashboardPage', () => {
  it('shows Customize link for org_admin', async () => {
    mockAuthRole = 'org_admin';
    renderManagerDashboard();
    await waitFor(() => {
      expect(screen.getByRole('link', { name: 'layout.customize.title' })).toBeInTheDocument();
    });
  });

  it('hides Customize link for resident', async () => {
    mockAuthRole = 'resident';
    renderManagerDashboard();
    // Give it a moment then assert absent
    await waitFor(() => {
      // Dashboard title should be visible (page rendered)
      expect(screen.getByText('dashboard.managerDashboard')).toBeInTheDocument();
    });
    expect(screen.queryByRole('link', { name: 'layout.customize.title' })).toBeNull();
  });
});
