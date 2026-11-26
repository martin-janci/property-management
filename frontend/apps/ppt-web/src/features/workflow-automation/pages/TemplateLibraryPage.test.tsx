/// <reference types="vitest/globals" />
/**
 * TemplateLibraryPage "Use Template" failure-surfacing regression tests.
 *
 * History: handleUseTemplate once awaited `createFromTemplate.mutateAsync(...)`
 * with NO try/catch and no error rendering. On a rejected mutation (network
 * error / 4xx / 5xx) the await threw, navigate() never ran, and the user got a
 * silent no-op — the spinner cleared, nothing happened, and the promise
 * rejected unhandled. The query error WAS surfaced, but the mutation error was
 * swallowed (see the 2026-07-31 ppt-web-ui code-review finding).
 *
 * The flow now wraps mutateAsync in try/catch and shows an error Toast. These
 * tests lock that contract in so the mutation failure can never again be
 * swallowed: a rejection surfaces a visible error and never navigates, while a
 * success routes to the new rule's editor. Mirrors the sibling guard in
 * CreateRulePage.test.tsx, whose presence the finding explicitly contrasted
 * against this page's (previous) lack of coverage.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { TemplateLibraryPage } from './TemplateLibraryPage';

const createMutate = vi.fn();
const navigate = vi.fn();

const template = {
  id: 'tpl-1',
  name: 'Overdue payment reminder',
  description: 'Notify tenants when rent is overdue',
  category: 'payments',
  triggerType: 'time_based',
  actionsCount: 1,
  tags: ['rent'],
  isPopular: false,
  usageCount: 3,
};

// A single template is always available so the "Use Template" card button
// renders; the mutation hook is driven per-test via `createMutate`.
vi.mock('@ppt/api-client', () => ({
  useAutomationTemplates: vi.fn(() => ({
    data: { data: [template], total: 1, page: 1, pageSize: 50, totalPages: 1 },
    isLoading: false,
    error: null,
  })),
  useCreateRuleFromTemplate: vi.fn(() => ({
    mutateAsync: createMutate,
    isPending: false,
    error: null,
  })),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

function renderPage() {
  return render(
    <MemoryRouter>
      <ToastProvider>
        <TemplateLibraryPage />
      </ToastProvider>
    </MemoryRouter>
  );
}

function alerts() {
  return screen.queryAllByRole('alert');
}

function hasToast(pattern: RegExp) {
  return alerts().some((el) => pattern.test(el.textContent ?? ''));
}

function clickUseTemplate() {
  fireEvent.click(screen.getByRole('button', { name: 'Use Template' }));
}

describe('TemplateLibraryPage "Use Template" failure surfacing', () => {
  beforeEach(() => {
    createMutate.mockReset();
    navigate.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('navigates to the new rule editor on success', async () => {
    createMutate.mockResolvedValueOnce({ id: 'rule-9' });
    renderPage();

    clickUseTemplate();

    await waitFor(() => expect(createMutate).toHaveBeenCalledTimes(1));
    expect(createMutate).toHaveBeenCalledWith({
      templateId: 'tpl-1',
      name: 'Overdue payment reminder',
    });
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/automations/rules/rule-9/edit'));
  });

  it('surfaces an error Toast and does not navigate when the mutation rejects', async () => {
    createMutate.mockRejectedValueOnce(new Error('network down'));
    renderPage();

    clickUseTemplate();

    await waitFor(() => expect(createMutate).toHaveBeenCalledTimes(1));
    // Regression: the failure must be visible, not a silent no-op.
    await waitFor(() => expect(hasToast(/could not use template/i)).toBe(true));
    // The thrown Error's own message is shown verbatim.
    expect(hasToast(/network down/i)).toBe(true);
    expect(navigate).not.toHaveBeenCalled();
  });

  it('falls back to the generic error message when a non-Error is thrown', async () => {
    createMutate.mockRejectedValueOnce('plain string failure');
    renderPage();

    clickUseTemplate();

    await waitFor(() => expect(createMutate).toHaveBeenCalledTimes(1));
    // Non-Error rejections use the translated fallback, not the raw value.
    await waitFor(() =>
      expect(hasToast(/a rule could not be created from this template/i)).toBe(true)
    );
    expect(hasToast(/plain string failure/)).toBe(false);
    expect(navigate).not.toHaveBeenCalled();
  });
});
