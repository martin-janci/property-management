/**
 * NotificationTriggersPage tests (Story 84.4 — Notification Trigger System,
 * PM gap 84-4). Covers forbidden/loading/error states, per-category grouping,
 * channel toggles (incl. forced priority in-app), and the reset action.
 */

/// <reference types="vitest/globals" />
import type { EventTriggersResponse } from '@ppt/api-client';
import { fireEvent, render, screen, within } from '@testing-library/react';
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
    expect(screen.getByRole('heading', { name: 'Fault' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Critical' })).toBeInTheDocument();
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
    const faultSection = screen.getByRole('heading', { name: 'Fault' }).closest('section');
    expect(faultSection).not.toBeNull();
    expect(within(faultSection as HTMLElement).getByText('1 of 1 enabled')).toBeInTheDocument();
  });
});
