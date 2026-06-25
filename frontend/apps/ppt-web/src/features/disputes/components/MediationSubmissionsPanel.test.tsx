/// <reference types="vitest/globals" />
/**
 * MediationSubmissionsPanel component tests (Story 80.3).
 * Covers list rendering, empty state, private-note badge, and the party
 * composer (visible only when canSubmit, submits via useSubmitResponse).
 */
import type { PartySubmission } from '@ppt/api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { MediationSubmissionsPanel } from './MediationSubmissionsPanel';

let mockSubmissions: PartySubmission[] = [];
let mockIsLoading = false;
const mutateAsync = vi.fn().mockResolvedValue({});

vi.mock('@ppt/api-client', () => ({
  useDisputeSubmissions: () => ({ data: mockSubmissions, isLoading: mockIsLoading }),
  useSubmitResponse: () => ({ mutateAsync, isPending: false }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'disputes.mediation.submissions.title': 'Party Submissions',
        'disputes.mediation.submissions.subtitle': 'Statements and responses.',
        'disputes.mediation.submissions.empty': 'No submissions yet.',
        'disputes.mediation.submissions.privateNote': 'Visible to mediator only',
        'disputes.mediation.submissions.composerTitle': 'File a submission',
        'disputes.mediation.submissions.typeLabel': 'Submission type',
        'disputes.mediation.submissions.typeStatement': 'Statement',
        'disputes.mediation.submissions.typeEvidence': 'Evidence',
        'disputes.mediation.submissions.typeResponse': 'Response',
        'disputes.mediation.submissions.typeRequest': 'Request',
        'disputes.mediation.submissions.contentLabel': 'Content',
        'disputes.mediation.submissions.contentPlaceholder': 'Write your submission…',
        'disputes.mediation.submissions.visibleToAll': 'Visible to all parties',
        'disputes.mediation.submissions.submit': 'Submit',
        'disputes.mediation.submissions.submitting': 'Submitting…',
      };
      return map[key] ?? key;
    },
    i18n: { language: 'en' },
  }),
}));

function makeSubmission(overrides: Partial<PartySubmission>): PartySubmission {
  return {
    id: 's-1',
    dispute_id: 'd-1',
    party_id: 'p-1',
    submission_type: 'statement',
    content: 'My statement',
    is_visible_to_all: true,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

const noop = () => {};

describe('MediationSubmissionsPanel', () => {
  beforeEach(() => {
    mockSubmissions = [];
    mockIsLoading = false;
    mutateAsync.mockClear();
  });

  it('shows the empty state when there are no submissions', () => {
    render(
      <MediationSubmissionsPanel
        disputeId="d-1"
        canSubmit={false}
        onToastSuccess={noop}
        onToastError={noop}
      />
    );
    expect(screen.getByText('No submissions yet.')).toBeInTheDocument();
  });

  it('renders submissions and the private-note badge for restricted ones', () => {
    mockSubmissions = [
      makeSubmission({ id: 'pub', content: 'Public one', is_visible_to_all: true }),
      makeSubmission({ id: 'priv', content: 'Private one', is_visible_to_all: false }),
    ];
    render(
      <MediationSubmissionsPanel
        disputeId="d-1"
        canSubmit={false}
        onToastSuccess={noop}
        onToastError={noop}
      />
    );
    expect(screen.getByText('Public one')).toBeInTheDocument();
    expect(screen.getByText('Private one')).toBeInTheDocument();
    // Only one private-note badge (for the restricted submission).
    expect(screen.getAllByText('Visible to mediator only')).toHaveLength(1);
  });

  it('hides the composer when canSubmit is false', () => {
    render(
      <MediationSubmissionsPanel
        disputeId="d-1"
        canSubmit={false}
        onToastSuccess={noop}
        onToastError={noop}
      />
    );
    expect(screen.queryByText('File a submission')).not.toBeInTheDocument();
  });

  it('submits a response via useSubmitResponse when a party files one', async () => {
    const onToastSuccess = vi.fn();
    render(
      <MediationSubmissionsPanel
        disputeId="d-1"
        canSubmit
        onToastSuccess={onToastSuccess}
        onToastError={noop}
      />
    );
    fireEvent.change(screen.getByLabelText('Content'), {
      target: { value: 'Here is my response' },
    });
    fireEvent.click(screen.getByText('Submit'));
    expect(mutateAsync).toHaveBeenCalledWith({
      submission_type: 'statement',
      content: 'Here is my response',
      is_visible_to_all: true,
    });
  });

  it('disables submit when content is empty', () => {
    render(
      <MediationSubmissionsPanel
        disputeId="d-1"
        canSubmit
        onToastSuccess={noop}
        onToastError={noop}
      />
    );
    expect(screen.getByText('Submit')).toBeDisabled();
  });
});
