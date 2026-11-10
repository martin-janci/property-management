/// <reference types="vitest/globals" />
/**
 * AiChatPage insight links (nav-fixes): /ai/sentiment and
 * /ai/predictive-maintenance had no inbound link; managers get them here.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { AiChatPage } from './AiChatPage';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));
vi.mock('../components', () => ({ ChatInterface: () => <div data-testid="chat" /> }));

const auth = vi.hoisted(() => ({ role: 'manager' as string | undefined }));
vi.mock('../../../contexts', () => ({ useAuth: () => ({ user: { role: auth.role } }) }));

function renderPage() {
  return render(
    <MemoryRouter>
      <AiChatPage />
    </MemoryRouter>
  );
}

describe('AiChatPage', () => {
  it('shows AI insight links to managers', () => {
    auth.role = 'manager';
    renderPage();
    expect(screen.getByRole('link', { name: 'aiDashboards.nav.sentiment' })).toHaveAttribute(
      'href',
      '/ai/sentiment'
    );
    expect(
      screen.getByRole('link', { name: 'aiDashboards.nav.predictiveMaintenance' })
    ).toHaveAttribute('href', '/ai/predictive-maintenance');
    expect(screen.getByTestId('chat')).toBeInTheDocument();
  });

  it('hides them from residents', () => {
    auth.role = 'resident';
    renderPage();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
    expect(screen.getByTestId('chat')).toBeInTheDocument();
  });
});
