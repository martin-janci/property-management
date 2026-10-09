/// <reference types="vitest/globals" />
/**
 * CreateRulePage create-payload boundary regression tests.
 *
 * History: handleSave forwarded the builder's `Partial<AutomationRule>` to the
 * create mutation with a blanket `rule as AutomationRule` cast. That cast
 * silently (a) pretended server-owned fields (id/createdAt/updatedAt/createdBy)
 * were present, so any stray value on those keys was sent to the API, and
 * (b) masked missing client-required fields (name/trigger/actions) — an
 * incomplete rule was posted rather than rejected.
 *
 * These tests lock in the typed boundary: incomplete rules are rejected with an
 * error Toast and never reach the mutation, and a valid submission sends only
 * the client-supplied CreateAutomationRuleInput shape (no server-owned fields).
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { CreateRulePage } from './CreateRulePage';

const createMutate = vi.fn();
const navigate = vi.fn();

// Drive handleSave directly through a stubbed RuleBuilder: one button submits a
// complete rule carrying stray server-owned fields, the other an incomplete one.
vi.mock('../components/RuleBuilder', () => ({
  RuleBuilder: ({ onSave }: { onSave: (rule: Record<string, unknown>) => void }) => (
    <div>
      <button
        type="button"
        onClick={() =>
          onSave({
            // stray server-owned fields that must NOT be forwarded
            id: 'should-not-be-sent',
            createdAt: '2020-01-01T00:00:00Z',
            updatedAt: '2020-01-01T00:00:00Z',
            createdBy: 'ghost',
            // client-supplied fields
            name: 'My rule',
            description: 'desc',
            isEnabled: true,
            trigger: { type: 'manual', name: 'Manual' },
            actions: [{ type: 'send_notification', name: 'Notify', config: {}, order: 0 }],
          })
        }
      >
        submit-complete
      </button>
      <button type="button" onClick={() => onSave({ name: 'no trigger or actions' })}>
        submit-incomplete
      </button>
    </div>
  ),
}));

vi.mock('@ppt/api-client', () => ({
  useCreateAutomationRule: vi.fn(() => ({
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
        <CreateRulePage />
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

describe('CreateRulePage create-payload boundary', () => {
  beforeEach(() => {
    createMutate.mockReset();
    navigate.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('rejects an incomplete rule without calling the mutation', async () => {
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: 'submit-incomplete' }));

    await waitFor(() => expect(hasToast(/create failed/i)).toBe(true));
    expect(createMutate).not.toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();
  });

  it('sends only the client-supplied CreateAutomationRuleInput shape', async () => {
    createMutate.mockResolvedValueOnce({ id: 'rule-1' });
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: 'submit-complete' }));

    await waitFor(() => expect(createMutate).toHaveBeenCalledTimes(1));
    const payload = createMutate.mock.calls[0][0];
    // Server-owned fields are not forwarded.
    expect(payload).not.toHaveProperty('id');
    expect(payload).not.toHaveProperty('createdAt');
    expect(payload).not.toHaveProperty('updatedAt');
    expect(payload).not.toHaveProperty('createdBy');
    // Client-supplied fields are preserved.
    expect(payload).toMatchObject({
      name: 'My rule',
      description: 'desc',
      isEnabled: true,
      trigger: { type: 'manual', name: 'Manual' },
      actions: [{ type: 'send_notification', name: 'Notify', config: {}, order: 0 }],
    });
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/automations/rules'));
  });
});
