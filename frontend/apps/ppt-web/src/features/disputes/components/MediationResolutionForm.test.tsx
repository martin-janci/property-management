/// <reference types="vitest/globals" />
/**
 * MediationResolutionForm component tests (fix-569).
 * Validates react-hook-form + zod migration.
 */

import type { ResolveDisputeRequest } from '@ppt/api-client';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MediationResolutionForm } from './MediationResolutionForm';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      const map: Record<string, string> = {
        'disputes.mediation.resolutionType': 'Resolution Type',
        'disputes.mediation.resolutionMutualAgreement': 'Mutual Agreement',
        'disputes.mediation.resolutionMutualAgreementDesc': 'Mutual agreement desc',
        'disputes.mediation.resolutionFavorFiler': 'In Favour of Filer',
        'disputes.mediation.resolutionFavorFilerDesc': 'Favour filer desc',
        'disputes.mediation.resolutionFavorRespondent': 'In Favour of Respondent',
        'disputes.mediation.resolutionFavorRespondentDesc': 'Favour respondent desc',
        'disputes.mediation.resolutionWithdrawn': 'Withdrawn',
        'disputes.mediation.resolutionWithdrawnDesc': 'Withdrawn desc',
        'disputes.mediation.resolutionDismissed': 'Dismissed',
        'disputes.mediation.resolutionDismissedDesc': 'Dismissed desc',
        'disputes.mediation.selectedType': `Selected: ${params?.label}`,
        'disputes.mediation.resolutionDetails': 'Resolution Details',
        'disputes.mediation.resolutionDetailsPlaceholder': 'Describe resolution...',
        'disputes.mediation.charMinimum': `${params?.count} chars (10 minimum)`,
        'disputes.mediation.errors.detailsMinLength': 'Details must be at least 10 characters.',
        'disputes.mediation.termsConditions': 'Terms / Conditions',
        'disputes.mediation.optional': 'optional',
        'disputes.mediation.termsPlaceholder': 'List terms...',
        'disputes.mediation.requiresConfirmation': 'Requires party confirmation',
        'disputes.mediation.requiresConfirmationDesc': 'Both parties must acknowledge.',
        'disputes.mediation.cancel': 'Cancel',
        'disputes.mediation.recording': 'Recording...',
        'disputes.mediation.recordResolutionBtn': 'Record Resolution',
      };
      return map[key] ?? key;
    },
  }),
}));

describe('MediationResolutionForm', () => {
  const mockOnResolve = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders all resolution type radio options', () => {
    render(<MediationResolutionForm onResolve={mockOnResolve} onCancel={mockOnCancel} />);
    expect(screen.getByText('Mutual Agreement')).toBeInTheDocument();
    expect(screen.getByText('In Favour of Filer')).toBeInTheDocument();
    expect(screen.getByText('In Favour of Respondent')).toBeInTheDocument();
    expect(screen.getByText('Withdrawn')).toBeInTheDocument();
    expect(screen.getByText('Dismissed')).toBeInTheDocument();
  });

  it('shows validation error when details field has fewer than 10 chars and form is submitted', async () => {
    render(<MediationResolutionForm onResolve={mockOnResolve} onCancel={mockOnCancel} />);

    const detailsField = screen.getByLabelText(/resolution details/i);
    fireEvent.change(detailsField, { target: { value: 'short' } });
    fireEvent.click(screen.getByRole('button', { name: /record resolution/i }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Details must be at least 10 characters.'
      );
    });
    expect(mockOnResolve).not.toHaveBeenCalled();
  });

  it('calls onResolve with form data when valid', async () => {
    render(<MediationResolutionForm onResolve={mockOnResolve} onCancel={mockOnCancel} />);

    const detailsField = screen.getByLabelText(/resolution details/i);
    fireEvent.change(detailsField, {
      target: { value: 'Both parties agreed to a new payment schedule.' },
    });

    fireEvent.click(screen.getByRole('button', { name: /record resolution/i }));

    await waitFor(() => {
      expect(mockOnResolve).toHaveBeenCalledWith(
        expect.objectContaining<Partial<ResolveDisputeRequest>>({
          resolutionType: 'mutual_agreement',
          resolutionDetails: 'Both parties agreed to a new payment schedule.',
          requiresConfirmation: false,
        })
      );
    });
  });

  it('calls onCancel when cancel button is clicked', () => {
    render(<MediationResolutionForm onResolve={mockOnResolve} onCancel={mockOnCancel} />);
    fireEvent.click(screen.getByRole('button', { name: /cancel/i }));
    expect(mockOnCancel).toHaveBeenCalled();
  });

  it('disables submit button while isSubmitting is true', () => {
    render(
      <MediationResolutionForm onResolve={mockOnResolve} onCancel={mockOnCancel} isSubmitting />
    );
    expect(screen.getByRole('button', { name: /recording/i })).toBeDisabled();
  });

  it('changes selected resolution type when radio is clicked', async () => {
    render(<MediationResolutionForm onResolve={mockOnResolve} onCancel={mockOnCancel} />);

    const favorFilerRadio = screen.getByRole('radio', { name: /in favour of filer/i });
    fireEvent.click(favorFilerRadio);

    await waitFor(() => {
      expect(screen.getByText(/Selected: In Favour of Filer/)).toBeInTheDocument();
    });
  });
});
