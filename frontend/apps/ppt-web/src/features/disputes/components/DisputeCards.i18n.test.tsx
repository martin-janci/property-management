/// <reference types="vitest/globals" />
/**
 * Dispute cards render through the real (en) i18n bundle: plural forms,
 * interpolated names and translated enum labels instead of hard-coded copy.
 */
import { render, screen } from '@testing-library/react';
import { type ActionItem, ActionItemCard } from './ActionItemCard';
import { DisputeCard, type DisputeSummary } from './DisputeCard';
import { ResolutionCard } from './ResolutionCard';

const DAY = 86_400_000;

const summary: DisputeSummary = {
  id: 'd-1',
  referenceNumber: 'DSP-1',
  category: 'common_area',
  title: 'Bikes in the hallway',
  status: 'awaiting_response',
  priority: 'urgent',
  filedByName: 'Fiona',
  partyCount: 1,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

describe('DisputeCard (en)', () => {
  it('renders translated labels and a singular party count', () => {
    render(<DisputeCard dispute={summary} onView={() => {}} onManage={() => {}} />);
    expect(screen.getByText('Awaiting Response')).toBeInTheDocument();
    expect(screen.getByText('Common Area')).toBeInTheDocument();
    expect(screen.getByText('Urgent Priority')).toBeInTheDocument();
    expect(screen.getByText('1 party')).toBeInTheDocument();
    expect(screen.getByText('Filed by Fiona')).toBeInTheDocument();
    expect(screen.getByText('Today')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Manage' })).toBeInTheDocument();
  });

  it('pluralises the party count and shows a relative age', () => {
    render(
      <DisputeCard
        dispute={{
          ...summary,
          partyCount: 3,
          createdAt: new Date(Date.now() - 3 * DAY - 1000).toISOString(),
        }}
        onView={() => {}}
      />
    );
    expect(screen.getByText('3 parties')).toBeInTheDocument();
    expect(screen.getByText('3 days ago')).toBeInTheDocument();
  });
});

const action: ActionItem = {
  id: 'a-1',
  disputeId: 'd-1',
  assignedTo: 'u-1',
  assignedToName: 'Ann',
  title: 'Move the bikes',
  description: 'Clear the hallway',
  dueDate: new Date(Date.now() - 2 * DAY - 1000).toISOString(),
  status: 'overdue',
};

describe('ActionItemCard (en)', () => {
  it('pluralises the overdue days and translates the status', () => {
    render(<ActionItemCard action={action} isManager onSendReminder={() => {}} />);
    expect(screen.getByText('Overdue by 2 days')).toBeInTheDocument();
    expect(screen.getByText('Overdue')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Send Reminder' })).toBeInTheDocument();
  });

  it('uses the singular for a one-day due date window', () => {
    render(
      <ActionItemCard
        action={{
          ...action,
          status: 'pending',
          dueDate: new Date(Date.now() + 2 * DAY + 1000).toISOString(),
        }}
      />
    );
    expect(screen.getByText('Due in 3 days')).toBeInTheDocument();
    expect(screen.getByText('Pending')).toBeInTheDocument();
  });
});

describe('ResolutionCard (en)', () => {
  it('formats the approval rate and translates the status', () => {
    render(
      <ResolutionCard
        resolution={{
          id: 'r-1',
          disputeId: 'd-1',
          proposedBy: 'u-1',
          proposedByName: 'Ann',
          resolutionText: 'Store bikes in the cellar',
          terms: [],
          status: 'proposed',
          proposedAt: '2026-07-01T10:00:00Z',
        }}
        acceptanceRate={0.5}
        canVote
        onVote={() => {}}
      />
    );
    expect(screen.getByText('Proposed')).toBeInTheDocument();
    expect(screen.getByText('50% approval')).toBeInTheDocument();
    expect(screen.getByText('Resolution Proposal')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Accept' })).toBeInTheDocument();
  });

  it('offers "Mark as Implemented" on an accepted resolution without terms', () => {
    const onImplement = vi.fn();
    render(
      <ResolutionCard
        resolution={{
          id: 'r-2',
          disputeId: 'd-1',
          proposedBy: 'u-1',
          proposedByName: 'Ann',
          resolutionText: 'Store bikes in the cellar',
          terms: [],
          status: 'accepted',
          proposedAt: '2026-07-01T10:00:00Z',
        }}
        canImplement
        onImplement={onImplement}
      />
    );
    screen.getByRole('button', { name: 'Mark as Implemented' }).click();
    expect(onImplement).toHaveBeenCalledWith('r-2');
  });
});
