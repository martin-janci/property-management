/// <reference types="vitest/globals" />
/**
 * ConfirmationDialog i18n regression tests.
 *
 * The confirm button's loading text used to be a hardcoded 'Processing...'
 * with no translation and no override. It now reads the shared
 * `common.processing` translation and accepts a `loadingLabel` prop escape
 * hatch. These tests lock in both behaviours.
 */

import { render, screen } from '@testing-library/react';
import { ConfirmationDialog } from './ConfirmationDialog';

describe('ConfirmationDialog i18n', () => {
  const baseProps = {
    isOpen: true,
    title: 'Confirm Action',
    message: 'Are you sure you want to proceed?',
    onConfirm: vi.fn(),
    onCancel: vi.fn(),
  };

  it('renders the translated loading label (not a bare i18n key) while loading', () => {
    render(<ConfirmationDialog {...baseProps} confirmLabel="Delete" isLoading />);

    // Resolves via common.processing -> English bundle ("Processing...").
    expect(screen.getByText('Processing...')).toBeInTheDocument();
    // The raw key must never leak into the UI.
    expect(screen.queryByText('common.processing')).not.toBeInTheDocument();
    // The idle confirm label is replaced while loading.
    expect(screen.queryByText('Delete')).not.toBeInTheDocument();
  });

  it('honours the loadingLabel prop escape hatch', () => {
    render(<ConfirmationDialog {...baseProps} isLoading loadingLabel="Saving…" />);

    expect(screen.getByText('Saving…')).toBeInTheDocument();
    expect(screen.queryByText('Processing...')).not.toBeInTheDocument();
  });

  it('shows the confirm label (not the loading label) when idle', () => {
    render(<ConfirmationDialog {...baseProps} confirmLabel="Delete" />);

    expect(screen.getByText('Delete')).toBeInTheDocument();
    expect(screen.queryByText('Processing...')).not.toBeInTheDocument();
  });
});
