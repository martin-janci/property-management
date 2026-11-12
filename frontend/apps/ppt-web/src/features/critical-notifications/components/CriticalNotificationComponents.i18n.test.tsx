/// <reference types="vitest/globals" />
/**
 * Critical-notification component i18n regression test.
 *
 * Guards against the four presentational components under
 * `features/critical-notifications/components/` hard-coding English
 * user-facing copy — the state they shipped in before the i18n pass
 * (`CriticalNotificationBanner`, `CriticalNotificationModal`,
 * `NotificationStatsCard`, `CreateNotificationForm` each rendered literal
 * English regardless of the active locale, while the sibling *pages* already
 * routed through `t('criticalNotifications.*')`).
 *
 * The Slovak cases fail on the pre-fix implementation (which rendered English
 * even under `sk`) and pass once every string routes through
 * `t('criticalNotifications.{banner,modal,statsCard,form}.*')`. The
 * `criticalNotifications` namespace intentionally ships in en/sk/cs only
 * (de/pl/hu fall back to en — see `localeNamespaceParity.test.ts`), so this
 * exercises the Slovak bundle.
 */
import type {
  CreateCriticalNotificationRequest,
  CriticalNotificationResponse,
  CriticalNotificationStats,
} from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import sk from '../../../../messages/sk.json';
import { CreateNotificationForm } from './CreateNotificationForm';
import { CriticalNotificationBanner } from './CriticalNotificationBanner';
import { CriticalNotificationModal } from './CriticalNotificationModal';
import { NotificationStatsCard } from './NotificationStatsCard';

const notification: CriticalNotificationResponse = {
  id: 'n1',
  title: 'Roof repair',
  message: 'The roof will be repaired on Monday.',
  createdAt: '2026-09-01T10:00:00Z',
  isAcknowledged: false,
} as unknown as CriticalNotificationResponse;

const stats: CriticalNotificationStats = {
  totalUsers: 10,
  acknowledgedCount: 4,
  pendingCount: 6,
} as unknown as CriticalNotificationStats;

beforeAll(() => {
  i18n.addResourceBundle('sk', 'translation', sk, true, true);
});

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('critical-notification components i18n (Slovak)', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('sk');
  });

  it('CriticalNotificationBanner renders translated copy', () => {
    render(<CriticalNotificationBanner count={1} onClick={() => {}} />);

    expect(screen.getByText('1 kritické upozornenie')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Zobraziť teraz' })).toBeInTheDocument();
  });

  it('CriticalNotificationModal renders translated copy', () => {
    render(
      <CriticalNotificationModal
        notifications={[notification]}
        onAcknowledge={() => Promise.resolve()}
      />
    );

    expect(screen.getByRole('heading', { name: 'Kritické upozornenie' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Potvrdzujem' })).toBeInTheDocument();
  });

  it('NotificationStatsCard renders translated copy', () => {
    render(<NotificationStatsCard stats={stats} />);

    expect(screen.getByText('Stav potvrdení')).toBeInTheDocument();
    expect(screen.getByText('Používatelia celkom')).toBeInTheDocument();
  });

  it('CreateNotificationForm renders translated copy', () => {
    render(
      <CreateNotificationForm
        onSubmit={(_data: CreateCriticalNotificationRequest) => Promise.resolve()}
        onCancel={() => {}}
      />
    );

    expect(
      screen.getByRole('button', { name: 'Odoslať kritické upozornenie' })
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Zrušiť' })).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Zadajte názov upozornenia')).toBeInTheDocument();
  });
});
