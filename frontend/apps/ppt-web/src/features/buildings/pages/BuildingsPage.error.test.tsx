/**
 * Error-state i18n + info-leak regression (code-review ppt-web-ui, vector i18n).
 *
 * The buildings list error panel must render translated copy only — a localized
 * heading and a generic translated body — and must NOT leak the raw API/network
 * error text (e.g. a connection string or internal host) into the DOM.
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

const buildingsResult = vi.hoisted(() => ({
  current: { data: undefined as unknown, isLoading: false, error: null as unknown },
}));

vi.mock('../hooks', () => ({
  useBuildings: () => buildingsResult.current,
}));

vi.mock('../components/BuildingList', () => ({
  BuildingList: () => null,
}));

import { BuildingsPage } from './BuildingsPage';

function renderPage() {
  return render(
    <MemoryRouter>
      <BuildingsPage onNavigateToView={() => {}} />
    </MemoryRouter>
  );
}

const SENSITIVE = 'connect ECONNREFUSED postgres://ppt:s3cr3t@10.0.0.5:5432/ppt';

describe('BuildingsPage — error state i18n + info-leak', () => {
  it('renders the translated heading, a generic translated body, and no raw error text', () => {
    buildingsResult.current = {
      data: undefined,
      isLoading: false,
      error: new Error(SENSITIVE),
    };

    renderPage();

    // Localized heading (not a bare key, not hardcoded outside i18n).
    expect(screen.getByText('Error loading buildings')).toBeInTheDocument();
    // Generic translated body is shown instead of the raw message.
    expect(screen.getByText('An unexpected error occurred')).toBeInTheDocument();
    // The raw API/network detail must never reach the DOM.
    expect(screen.queryByText(SENSITIVE)).not.toBeInTheDocument();
    expect(document.body.textContent).not.toContain('ECONNREFUSED');
    expect(document.body.textContent).not.toContain('10.0.0.5');
  });
});
