/**
 * Information architecture of the authenticated app shell.
 *
 * Single source of truth for the sidebar. Items are grouped by the job a
 * property-management user is doing (running the building, talking to
 * residents, governance, money, renting) rather than by backend module, and
 * each item declares who may see it so residents are not shown operator-only
 * tools that would bounce them to `/forbidden`.
 *
 * `audience`:
 *  - `all`      — every authenticated user
 *  - `manager`  — {@link MANAGER_ROLES} only (operator tooling)
 *  - `resident` — non-manager roles only (e.g. "My unit")
 */
export type NavAudience = 'all' | 'manager' | 'resident';

export interface NavItem {
  to: string;
  /** i18n key under `nav.*`. */
  labelKey: string;
  /** English fallback so a missing key never renders the raw key. */
  defaultLabel: string;
  audience?: NavAudience;
  /** Treat the item as active only on an exact path match (e.g. `/iot` vs `/iot/alerts`). */
  end?: boolean;
}

export interface NavSection {
  id: string;
  /** i18n key under `shell.sections.*`. */
  labelKey: string;
  defaultLabel: string;
  items: NavItem[];
}

export const NAV_SECTIONS: NavSection[] = [
  {
    id: 'overview',
    labelKey: 'shell.sections.overview',
    defaultLabel: 'Overview',
    items: [
      { to: '/dashboard', labelKey: 'nav.dashboard', defaultLabel: 'Dashboard' },
      {
        to: '/my-unit',
        labelKey: 'nav.myUnit',
        defaultLabel: 'My unit',
        audience: 'resident',
      },
      { to: '/announcements', labelKey: 'nav.announcements', defaultLabel: 'Announcements' },
      { to: '/messages', labelKey: 'nav.messages', defaultLabel: 'Messages' },
    ],
  },
  {
    id: 'operations',
    labelKey: 'shell.sections.operations',
    defaultLabel: 'Building operations',
    items: [
      { to: '/buildings', labelKey: 'nav.buildings', defaultLabel: 'Buildings' },
      { to: '/faults', labelKey: 'nav.faults', defaultLabel: 'Faults' },
      { to: '/meters', labelKey: 'nav.meters', defaultLabel: 'Meters' },
      { to: '/outages', labelKey: 'nav.outages', defaultLabel: 'Outages' },
      {
        to: '/iot',
        labelKey: 'nav.iot',
        defaultLabel: 'Smart building',
        audience: 'manager',
        end: true,
      },
      { to: '/emergency', labelKey: 'nav.emergency', defaultLabel: 'Emergency contacts' },
    ],
  },
  {
    id: 'governance',
    labelKey: 'shell.sections.governance',
    defaultLabel: 'Owners & governance',
    items: [
      { to: '/voting', labelKey: 'nav.voting', defaultLabel: 'Voting' },
      { to: '/documents', labelKey: 'nav.documents', defaultLabel: 'Documents' },
      { to: '/disputes', labelKey: 'nav.disputes', defaultLabel: 'Disputes' },
    ],
  },
  {
    id: 'community',
    labelKey: 'shell.sections.community',
    defaultLabel: 'Community',
    items: [
      { to: '/news', labelKey: 'nav.news', defaultLabel: 'News' },
      { to: '/community', labelKey: 'nav.community', defaultLabel: 'Community' },
      { to: '/neighbors', labelKey: 'nav.neighbors', defaultLabel: 'Neighbors' },
    ],
  },
  {
    id: 'finance',
    labelKey: 'shell.sections.finance',
    defaultLabel: 'Finance',
    items: [
      { to: '/financial', labelKey: 'nav.financial', defaultLabel: 'Finance' },
      {
        to: '/accounting/invoices',
        labelKey: 'nav.accounting',
        defaultLabel: 'Accounting',
        audience: 'manager',
      },
      { to: '/reports', labelKey: 'nav.reports', defaultLabel: 'Reports', audience: 'manager' },
    ],
  },
  {
    id: 'rentals',
    labelKey: 'shell.sections.rentals',
    defaultLabel: 'Leasing & rentals',
    items: [
      { to: '/leases', labelKey: 'nav.leases', defaultLabel: 'Leases', audience: 'manager' },
      {
        to: '/rentals',
        labelKey: 'nav.rentals',
        defaultLabel: 'Short-term rentals',
        audience: 'manager',
      },
    ],
  },
  {
    id: 'tools',
    labelKey: 'shell.sections.tools',
    defaultLabel: 'Tools',
    items: [
      { to: '/ai-assistant', labelKey: 'nav.aiChat', defaultLabel: 'AI assistant' },
      {
        to: '/automations/rules',
        labelKey: 'nav.automations',
        defaultLabel: 'Automations',
        audience: 'manager',
      },
      // No /notifications/analytics link: its API is platform-admin only
      // (GET /admin/notifications/analytics → 403 for org managers); the
      // analytics live in admin-web.
      {
        to: '/notifications/triggers',
        labelKey: 'nav.notificationTriggers',
        defaultLabel: 'Notification triggers',
        audience: 'manager',
      },
    ],
  },
];

/** Account / settings links shown in the user menu. */
export const ACCOUNT_ITEMS: NavItem[] = [
  { to: '/settings/profile', labelKey: 'nav.profile', defaultLabel: 'Profile' },
  { to: '/settings/password', labelKey: 'shell.account.password', defaultLabel: 'Password' },
  {
    to: '/settings/two-factor',
    labelKey: 'shell.account.twoFactor',
    defaultLabel: 'Two-factor authentication',
  },
  { to: '/settings/sessions', labelKey: 'nav.sessions', defaultLabel: 'Active sessions' },
  {
    to: '/settings/notifications',
    labelKey: 'shell.account.notifications',
    defaultLabel: 'Notification preferences',
  },
  { to: '/settings/privacy', labelKey: 'nav.privacy', defaultLabel: 'Privacy' },
  { to: '/settings/accessibility', labelKey: 'nav.accessibility', defaultLabel: 'Accessibility' },
  {
    to: '/settings/integrations',
    labelKey: 'nav.integrations',
    defaultLabel: 'Integrations',
    // Airbnb / Booking.com channel sync is an operator tool.
    audience: 'manager',
  },
  {
    to: '/settings/portal-webhooks',
    labelKey: 'nav.portalWebhooks',
    defaultLabel: 'Portal webhooks',
    audience: 'manager',
  },
];

/** Whether an item is visible for the given operator flag. */
export function isVisibleFor(item: Pick<NavItem, 'audience'>, isManager: boolean): boolean {
  const audience = item.audience ?? 'all';
  if (audience === 'manager') return isManager;
  if (audience === 'resident') return !isManager;
  return true;
}

/** Sections filtered for the viewer; empty sections are dropped. */
export function sectionsFor(isManager: boolean): NavSection[] {
  return NAV_SECTIONS.map((section) => ({
    ...section,
    items: section.items.filter((item) => isVisibleFor(item, isManager)),
  })).filter((section) => section.items.length > 0);
}
