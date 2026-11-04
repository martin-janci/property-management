/// <reference types="vitest/globals" />
/**
 * Role gating of the app shell navigation (round-3 audit #13, #15, #22, #40).
 *
 * Owners and tenants must not be offered operator-only links (each one ends on
 * "Access denied"), every role must reach the core modules from the sidebar,
 * and the platform-admin-only notification analytics link is gone for everyone.
 */
import { fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';
import { MANAGER_ROLES } from '../../routes/shared';
import { AppShell } from './AppShell';
import { ACCOUNT_ITEMS, NAV_SECTIONS } from './navConfig';

let currentRole: string | undefined = 'manager';

vi.mock('../../contexts', () => ({
  useAuth: () => ({
    isAuthenticated: true,
    user: { id: 'u1', email: 'user@example.com', firstName: 'Jana', role: currentRole },
    logout: vi.fn(),
  }),
}));

// The connection pill needs the WebSocket provider; it is not under test here.
vi.mock('../ConnectionStatus', () => ({ ConnectionStatus: () => null }));

/** Links that only managers may see (each route is manager-gated). */
const MANAGER_ONLY = [
  '/leases',
  '/rentals',
  '/iot',
  '/accounting/invoices',
  '/reports',
  '/automations/rules',
  '/notifications/triggers',
];

/** Core modules every signed-in role reaches from the sidebar. */
const CORE_FOR_ALL = [
  '/dashboard',
  '/faults',
  '/voting',
  '/announcements',
  '/financial',
  '/community',
  '/messages',
  '/meters',
];

function renderShellAs(role: string | undefined) {
  currentRole = role;
  render(
    <MemoryRouter initialEntries={['/dashboard']}>
      <AppShell>
        <div />
      </AppShell>
    </MemoryRouter>
  );
}

function sidebarHrefs(): string[] {
  const nav = screen.getByRole('navigation', { name: 'Main navigation' });
  return within(nav)
    .getAllByRole('link')
    .map((a) => a.getAttribute('href') ?? '');
}

function accountMenuHrefs(): string[] {
  fireEvent.click(screen.getByRole('button', { expanded: false, name: /Jana/ }));
  return within(screen.getByRole('menu'))
    .getAllByRole('menuitem')
    .map((el) => el.getAttribute('href'))
    .filter((href): href is string => href != null);
}

describe('AppShell navigation per role', () => {
  it.each(['owner', 'tenant', 'resident'])('hides operator-only links from "%s"', (role) => {
    renderShellAs(role);
    const hrefs = sidebarHrefs();
    for (const href of MANAGER_ONLY) {
      expect(hrefs).not.toContain(href);
    }
    expect(accountMenuHrefs()).not.toContain('/settings/integrations');
  });

  it.each(['owner', 'tenant'])('shows "%s" the core modules and My unit', (role) => {
    renderShellAs(role);
    const hrefs = sidebarHrefs();
    for (const href of [...CORE_FOR_ALL, '/my-unit']) {
      expect(hrefs).toContain(href);
    }
  });

  it.each([...MANAGER_ROLES])('shows the manager role "%s" the operator links', (role) => {
    renderShellAs(role);
    const hrefs = sidebarHrefs();
    for (const href of [...CORE_FOR_ALL, ...MANAGER_ONLY]) {
      expect(hrefs).toContain(href);
    }
    // "My unit" is a resident view.
    expect(hrefs).not.toContain('/my-unit');
    expect(accountMenuHrefs()).toContain('/settings/integrations');
  });

  it.each(['manager', 'owner', 'tenant'])(
    'never links "%s" to the platform-admin notification analytics',
    (role) => {
      renderShellAs(role);
      expect(sidebarHrefs()).not.toContain('/notifications/analytics');
    }
  );
});

const LOCALES = { cs, de, en, hu, pl, sk } as Record<string, Record<string, unknown>>;

function lookup(bundle: Record<string, unknown>, key: string): unknown {
  return key
    .split('.')
    .reduce<unknown>(
      (node, part) =>
        node && typeof node === 'object' ? (node as Record<string, unknown>)[part] : undefined,
      bundle
    );
}

describe('nav labels are translated in every locale (audit #29)', () => {
  const keys = [
    ...NAV_SECTIONS.flatMap((section) => [
      section.labelKey,
      ...section.items.map((i) => i.labelKey),
    ]),
    ...ACCOUNT_ITEMS.map((i) => i.labelKey),
  ];

  it.each(Object.keys(LOCALES))('"%s" has a value for every nav label key', (locale) => {
    const missing = keys.filter((key) => typeof lookup(LOCALES[locale], key) !== 'string');
    expect(missing).toEqual([]);
  });
});
