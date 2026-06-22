/// <reference types="vitest/globals" />
/**
 * Tests for the Story 7B.2 document-template generate dialog.
 *
 * Pins the contractual behavior of the generate flow:
 *   1. live preview substitutes `{{name}}` placeholders with current values;
 *   2. required-placeholder validation blocks submit and surfaces the names;
 *   3. a valid submit POSTs only non-empty values (server applies defaults)
 *      plus the title + category, and navigates to the generated document.
 *
 * Only the network boundary (`@ppt/api-client`) and context hooks
 * (`../../buildings/hooks`) are mocked; the dialog logic runs for real.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GenerateDocumentDialog } from './GenerateDocumentDialog';

// ─── network boundary: @ppt/api-client ──────────────────────────────────────
const useTemplateMock = vi.fn();
const mutateMock = vi.fn();

vi.mock('@ppt/api-client', () => ({
  useTemplate: (id: string) => useTemplateMock(id),
  useGenerateDocument: () => ({
    mutate: mutateMock,
    isPending: false,
    isError: false,
    error: null,
  }),
}));

// ─── context hooks ──────────────────────────────────────────────────────────
vi.mock('../../buildings/hooks', () => ({
  useBuildings: () => ({ data: { items: [] } }),
  useUnits: () => ({ data: { units: [] } }),
}));

// ─── leaf helpers ───────────────────────────────────────────────────────────
vi.mock('../../../hooks/useFocusTrap', () => ({ useFocusTrap: () => {} }));

const navigateMock = vi.fn();
vi.mock('react-router-dom', () => ({
  useNavigate: () => navigateMock,
}));

const TEMPLATE = {
  id: 'tpl-1',
  organization_id: 'org-1',
  name: 'Notice',
  template_type: 'notice',
  content: 'Dear {{tenant}}, your rent is {{amount}}.',
  placeholders: [
    { name: 'tenant', type: 'text', required: true, default_value: null, description: null },
    { name: 'amount', type: 'currency', required: false, default_value: '500', description: null },
  ],
  usage_count: 0,
  created_by: 'u-1',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  created_by_name: 'Manager',
};

function renderDialog() {
  return render(
    <GenerateDocumentDialog templateId="tpl-1" templateName="Notice" onClose={vi.fn()} />
  );
}

describe('GenerateDocumentDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useTemplateMock.mockReturnValue({
      data: { template: TEMPLATE },
      isLoading: false,
      isError: false,
      refetch: vi.fn(),
    });
  });

  it('renders a live preview, seeding defaults and substituting placeholders', () => {
    renderDialog();
    // `amount` default (500) is seeded; `tenant` is still empty.
    expect(screen.getByText(/Dear , your rent is 500\./)).toBeInTheDocument();

    fireEvent.change(screen.getByRole('textbox', { name: /tenant/i }), {
      target: { value: 'Jane' },
    });
    expect(screen.getByText(/Dear Jane, your rent is 500\./)).toBeInTheDocument();
  });

  it('blocks submit and lists missing required placeholders', () => {
    renderDialog();
    fireEvent.click(screen.getByRole('button', { name: 'Generate document' }));
    expect(mutateMock).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent('tenant');
  });

  it('submits non-empty values + metadata and navigates on success', () => {
    renderDialog();
    fireEvent.change(screen.getByRole('textbox', { name: /tenant/i }), {
      target: { value: 'Jane' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Generate document' }));

    expect(mutateMock).toHaveBeenCalledTimes(1);
    const [body, opts] = mutateMock.mock.calls[0];
    expect(body).toMatchObject({
      values: { tenant: 'Jane', amount: '500' },
      title: 'Notice',
      category: 'other',
    });

    // Drive the success callback and assert navigation to the new document.
    opts.onSuccess({ document_id: 'doc-9' });
    expect(navigateMock).toHaveBeenCalledWith('/documents/doc-9');
  });
});
