/**
 * Regression guard: the maintenance / kill-switch fallback document must
 * source its user-facing copy from the next-intl `layout.maintenance` catalog,
 * not from hardcoded English literals.
 *
 * The `next-intl/server` mock below echoes the translation key (and surfaces
 * the interpolation values), so asserting on the key (`title`) and on the
 * injected tenant name proves the strings are wired through
 * `getTranslations('layout.maintenance')` rather than falling back to the
 * literal English ("Service temporarily unavailable", "… is offline for
 * maintenance …").
 *
 * Before the i18n fix these assertions failed: the component rendered the
 * hardcoded English copy, so neither the `title` key nor a `body`-keyed string
 * ever appeared in the DOM.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

// Echo the key so we can assert the copy is catalog-sourced. For `body` the
// interpolated tenant name is appended so we can also prove it still flows in.
vi.mock('next-intl/server', () => ({
  getTranslations: async (_namespace: string) => (key: string, values?: Record<string, unknown>) =>
    values && 'name' in values ? `${key}:${String(values.name)}` : key,
}));

import { MaintenanceNotice } from './MaintenanceNotice';

describe('MaintenanceNotice — i18n-wired kill-switch fallback', () => {
  it('sources the heading from the layout.maintenance catalog, not a hardcoded literal', async () => {
    render(await MaintenanceNotice({ locale: 'sk', tenantName: 'Acme Reality' }));

    // The echoed `title` key proves the heading comes from getTranslations.
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('title');
    // The pre-fix hardcoded English string must no longer be rendered.
    expect(screen.queryByText('Service temporarily unavailable')).not.toBeInTheDocument();
  });

  it('sources the body from the catalog and keeps the tenant-name interpolation', async () => {
    render(await MaintenanceNotice({ locale: 'sk', tenantName: 'Acme Reality' }));

    // `body` key echoed + interpolated tenant name proves both the i18n wiring
    // and that the tenant name is passed through as an ICU value.
    expect(screen.getByText('body:Acme Reality')).toBeInTheDocument();
    expect(
      screen.queryByText('Acme Reality is offline for maintenance. Please check back shortly.')
    ).not.toBeInTheDocument();
  });
});
