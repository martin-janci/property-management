/// <reference types="vitest/globals" />
/**
 * DisputeDetailPage action visibility, aligned with the backend role checks:
 *  - status changes (`PATCH /disputes/{id}`) are manager-only and limited to
 *    the backend state machine's allowed transitions;
 *  - evidence upload has no backend route, so there is no "Add Evidence";
 *  - parties and the mediator can open the mediation workspace.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import {
  allowedStatusTransitions,
  type DisputeDetail,
  DisputeDetailPage,
} from './DisputeDetailPage';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}));

const dispute: DisputeDetail = {
  id: 'd-1',
  organizationId: 'org-1',
  referenceNumber: 'DSP-2026-00007',
  category: 'noise',
  title: 'Loud music',
  description: 'Every night',
  status: 'under_review',
  priority: 'medium',
  filedBy: 'user-2',
  filedByName: 'Fiona Filer',
  createdAt: '2026-07-01T10:00:00Z',
  updatedAt: '2026-07-01T10:00:00Z',
};

const noop = () => {};

function renderPage(props: Partial<React.ComponentProps<typeof DisputeDetailPage>> = {}) {
  const onUpdateStatus = vi.fn();
  render(
    <DisputeDetailPage
      dispute={dispute}
      parties={[]}
      evidence={[]}
      timeline={[]}
      resolutions={[]}
      actionItems={[]}
      onBack={noop}
      onUpdateStatus={onUpdateStatus}
      onDeleteEvidence={noop}
      onProposeResolution={noop}
      onVoteResolution={noop}
      onAcceptResolution={noop}
      onImplementResolution={noop}
      onCreateAction={noop}
      onCompleteAction={noop}
      onSendReminder={noop}
      onEscalate={noop}
      onNavigateToMediation={noop}
      {...props}
    />
  );
  return { onUpdateStatus };
}

describe('DisputeDetailPage', () => {
  it('mirrors the backend dispute state machine', () => {
    expect(allowedStatusTransitions.filed).toEqual([
      'under_review',
      'awaiting_response',
      'withdrawn',
    ]);
    expect(allowedStatusTransitions.resolved).toEqual(['closed']);
    expect(allowedStatusTransitions.closed).toEqual([]);
  });

  it('lets a manager move the dispute to an allowed next status', () => {
    const { onUpdateStatus } = renderPage({ isManager: true });
    const select = screen.getByLabelText('disputes.detail.changeStatus');
    const options = Array.from((select as HTMLSelectElement).options).map((o) => o.value);
    expect(options).toEqual(['', 'mediation', 'awaiting_response', 'resolved', 'escalated']);

    const apply = screen.getByRole('button', { name: 'disputes.detail.applyStatus' });
    expect(apply).toBeDisabled();
    fireEvent.change(select, { target: { value: 'mediation' } });
    fireEvent.click(apply);
    expect(onUpdateStatus).toHaveBeenCalledWith('mediation');
  });

  it('does not offer status changes to a mediator (backend is manager-only)', () => {
    renderPage({ isMediator: true });
    expect(screen.queryByLabelText('disputes.detail.changeStatus')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'disputes.detail.mediation' })).toBeInTheDocument();
  });

  it('offers no status change once the dispute is closed', () => {
    renderPage({ isManager: true, dispute: { ...dispute, status: 'closed' } });
    expect(screen.queryByLabelText('disputes.detail.changeStatus')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'disputes.detail.mediation' })
    ).not.toBeInTheDocument();
  });

  it('lets a party open the mediation workspace but not change status', () => {
    renderPage({ isParty: true });
    expect(screen.getByRole('button', { name: 'disputes.detail.mediation' })).toBeInTheDocument();
    expect(screen.queryByLabelText('disputes.detail.changeStatus')).not.toBeInTheDocument();
  });

  it('has no evidence upload affordance', () => {
    renderPage({ isManager: true, isParty: true });
    expect(screen.queryByText('disputes.addEvidence')).not.toBeInTheDocument();
    expect(document.querySelector('input[type="file"]')).toBeNull();
  });

  it('links evidence only when the storage URL is a web URL', () => {
    renderPage({
      evidence: [
        {
          id: 'e-1',
          uploadedBy: 'u',
          uploaderName: 'U',
          filename: 'a',
          originalFilename: 'web.jpg',
          contentType: 'image/jpeg',
          sizeBytes: 1,
          storageUrl: 'https://cdn.example.com/web.jpg',
          createdAt: '2026-07-01T10:00:00Z',
        },
        {
          id: 'e-2',
          uploadedBy: 'u',
          uploaderName: 'U',
          filename: 'b',
          originalFilename: 'key.jpg',
          contentType: 'image/jpeg',
          sizeBytes: 1,
          storageUrl: 'org/abc/key.jpg',
          createdAt: '2026-07-01T10:00:00Z',
        },
      ],
    });
    expect(screen.getAllByRole('link', { name: 'disputes.detail.viewEvidence' })).toHaveLength(1);
  });

  it('renders status, category, priority and copy through i18n keys', () => {
    renderPage();
    expect(screen.getByText('disputes.statusUnderReview')).toBeInTheDocument();
    expect(screen.getByText('disputes.categoryNoise')).toBeInTheDocument();
    expect(screen.getByText('disputes.priorityWithLabel')).toBeInTheDocument();
    expect(screen.getByText('common.backToDisputes')).toBeInTheDocument();
    expect(screen.getByText('disputes.detail.noEvidence')).toBeInTheDocument();
    expect(screen.getByText('disputes.timeline.title')).toBeInTheDocument();
  });
});
