/**
 * NotificationTriggersPage tests (Story 84.4 — Notification Trigger System,
 * PM gap 84-4). Covers forbidden/loading/error states, per-category grouping,
 * channel toggles (incl. forced priority in-app), and the reset action.
 */

/// <reference types="vitest/globals" />
import type { EventTriggersResponse } from '@ppt/api-client';
import { fireEvent, render, screen, within } from '@testing-library/react';
import i18n from 'i18next';
import sk from '../../../../messages/sk.json';
import { NotificationTriggersPage } from './NotificationTriggersPage';

const noop = () => {};

const baseProps = {
  onToggle: noop,
};

const data: EventTriggersResponse = {
  preferences: [
    {
      eventType: 'fault_status_changed',
      category: 'fault',
      displayName: 'Fault status changed',
      description: 'When a reported fault changes status.',
      isPriority: false,
      pushEnabled: true,
      emailEnabled: false,
      inAppEnabled: true,
    },
    {
      eventType: 'emergency_alert',
      category: 'critical',
      displayName: 'Emergency alert',
      isPriority: true,
      pushEnabled: true,
      emailEnabled: true,
      inAppEnabled: true,
    },
  ],
  categories: [
    { category: 'fault', displayName: 'fault', totalEvents: 1, enabledEvents: 1 },
    { category: 'critical', displayName: 'critical', totalEvents: 1, enabledEvents: 1 },
  ],
};

describe('NotificationTriggersPage', () => {
  it('shows the forbidden notice when isForbidden', () => {
    render(<NotificationTriggersPage {...baseProps} isForbidden />);
    expect(screen.getByRole('alert')).toHaveTextContent('do not have permission');
  });

  it('renders a loading spinner when isLoading', () => {
    const { container } = render(<NotificationTriggersPage {...baseProps} isLoading />);
    expect(container.querySelector('[aria-busy="true"]')).toBeInTheDocument();
  });

  it('renders an error state with a retry button', () => {
    const onRetry = vi.fn();
    render(<NotificationTriggersPage {...baseProps} isError onRetry={onRetry} />);
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to load notification triggers');
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it('renders the empty state when there are no triggers', () => {
    render(<NotificationTriggersPage {...baseProps} data={{ preferences: [], categories: [] }} />);
    expect(
      screen.getByText('No notification triggers are available for your account.')
    ).toBeInTheDocument();
  });

  it('groups triggers by category and shows the priority badge', () => {
    render(<NotificationTriggersPage {...baseProps} data={data} />);
    expect(screen.getByRole('heading', { name: 'Faults' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Critical alerts' })).toBeInTheDocument();
    expect(screen.getByText('Priority')).toBeInTheDocument();
  });

  it('invokes onToggle with the event type, channel, and next value', () => {
    const onToggle = vi.fn();
    render(<NotificationTriggersPage {...baseProps} data={data} onToggle={onToggle} />);
    // Email is currently off for the fault trigger — clicking enables it.
    const emailBox = screen.getByRole('checkbox', {
      name: 'Fault status changed — Email',
    });
    fireEvent.click(emailBox);
    expect(onToggle).toHaveBeenCalledWith('fault_status_changed', 'email', true);
  });

  it('forces the in-app channel on and disabled for priority triggers', () => {
    render(<NotificationTriggersPage {...baseProps} data={data} />);
    const inAppBox = screen.getByRole('checkbox', {
      name: 'Emergency alert — In app',
    }) as HTMLInputElement;
    expect(inAppBox).toBeChecked();
    expect(inAppBox).toBeDisabled();
  });

  it('disables a row while it is saving', () => {
    render(
      <NotificationTriggersPage {...baseProps} data={data} savingEventType="fault_status_changed" />
    );
    const pushBox = screen.getByRole('checkbox', {
      name: 'Fault status changed — Push',
    });
    expect(pushBox).toBeDisabled();
  });

  it('renders the reset button and calls onReset', () => {
    const onReset = vi.fn();
    render(<NotificationTriggersPage {...baseProps} data={data} onReset={onReset} />);
    fireEvent.click(screen.getByRole('button', { name: 'Reset to defaults' }));
    expect(onReset).toHaveBeenCalledOnce();
  });

  it('shows the per-category enabled count', () => {
    render(<NotificationTriggersPage {...baseProps} data={data} />);
    const faultSection = screen.getByRole('heading', { name: 'Faults' }).closest('section');
    expect(faultSection).not.toBeNull();
    expect(within(faultSection as HTMLElement).getByText('1 of 1 enabled')).toBeInTheDocument();
  });

  describe('seeded catalogue text is translated by event-type key', () => {
    const seeded: EventTriggersResponse = {
      preferences: [
        {
          eventType: 'fault.created',
          category: 'fault',
          // English text as seeded in notification_event_types.
          displayName: 'New Fault Report',
          description: 'A new fault has been reported in your building',
          isPriority: false,
          pushEnabled: true,
          emailEnabled: true,
          inAppEnabled: true,
        },
        {
          eventType: 'custom.unknown_event',
          category: 'custom',
          displayName: 'Server-only trigger',
          description: 'Only the server knows this one',
          isPriority: false,
          pushEnabled: false,
          emailEnabled: false,
          inAppEnabled: true,
        },
      ],
      categories: [],
    };

    beforeAll(() => {
      i18n.addResourceBundle('sk', 'translation', sk, true, true);
    });

    afterEach(async () => {
      await i18n.changeLanguage('en');
    });

    it('renders the Slovak name, description and category for a known event type', async () => {
      await i18n.changeLanguage('sk');
      render(<NotificationTriggersPage {...baseProps} data={seeded} />);
      expect(screen.getByRole('heading', { name: 'Poruchy' })).toBeInTheDocument();
      expect(screen.getByText('Nová porucha')).toBeInTheDocument();
      expect(screen.getByText('Vo vašom dome bola nahlásená nová porucha')).toBeInTheDocument();
      expect(screen.queryByText('New Fault Report')).not.toBeInTheDocument();
    });

    it('falls back to the server text for an event type the client does not know', async () => {
      await i18n.changeLanguage('sk');
      render(<NotificationTriggersPage {...baseProps} data={seeded} />);
      expect(screen.getByText('Server-only trigger')).toBeInTheDocument();
      expect(screen.getByText('Only the server knows this one')).toBeInTheDocument();
    });
  });
});
