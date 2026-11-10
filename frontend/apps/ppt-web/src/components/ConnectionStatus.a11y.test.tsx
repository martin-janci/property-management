/// <reference types="vitest/globals" />
/**
 * ConnectionStatus Accessibility Tests
 *
 * Regression guard for the role mismatch where the `role="status"` live-region
 * `<div>` also doubled as the interactive reconnect control (onClick/onKeyDown/
 * tabIndex on the status node). The reconnect action must be a real `<button>`
 * (button semantics), distinct from the `role="status"` live-region.
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ConnectionStatus } from './ConnectionStatus';

const reconnect = vi.fn();
let wsState = {
  isConnected: false,
  isConnecting: false,
  error: null as Error | null,
  send: vi.fn(),
  reconnect,
};

vi.mock('../hooks/useWebSocket', () => ({
  useWebSocketState: () => wsState,
}));

describe('ConnectionStatus accessibility', () => {
  beforeEach(() => {
    reconnect.mockClear();
    wsState = {
      isConnected: false,
      isConnecting: false,
      error: null,
      send: vi.fn(),
      reconnect,
    };
  });

  it('exposes the reconnect control as a real button, distinct from the status live-region', () => {
    render(<ConnectionStatus />);

    // The reconnect control must have button semantics...
    const button = screen.getByRole('button');
    // ...and must NOT be the status live-region node itself.
    const status = screen.getByRole('status');
    expect(button).not.toBe(status);
    expect(status).not.toHaveAttribute('onclick');
    // The live-region must not be independently focusable / clickable.
    expect(status).not.toHaveAttribute('tabindex');
  });

  it('reconnects when the button is activated', async () => {
    const user = userEvent.setup();
    render(<ConnectionStatus />);

    await user.click(screen.getByRole('button'));
    expect(reconnect).toHaveBeenCalledTimes(1);
  });

  it('does not render a reconnect button while connected', () => {
    wsState = {
      isConnected: true,
      isConnecting: false,
      error: null,
      send: vi.fn(),
      reconnect,
    };
    render(<ConnectionStatus />);

    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.getByRole('status')).toBeInTheDocument();
  });
});
