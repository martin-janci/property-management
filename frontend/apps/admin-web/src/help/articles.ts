/**
 * Static help articles for the admin contextual help system.
 *
 * Each article has a stable `id`, a `title`, and `body` (plain text / simple
 * markdown rendered as-is in the HelpSidebar). Articles are keyed by page
 * context so the AdminLayout can surface the right one automatically.
 */

export interface HelpArticle {
  id: string;
  title: string;
  /** Plain text with light markdown (# heading, - list, **bold**). */
  body: string;
  /** Optional external docs URL. */
  docsUrl?: string;
}

// ---------------------------------------------------------------------------
// Individual articles
// ---------------------------------------------------------------------------

const DASHBOARD: HelpArticle = {
  id: 'dashboard',
  title: 'Admin Dashboard',
  body: `The dashboard gives a real-time overview of platform health.

**Stat tiles (top row)**
- **Tenants active** — total registered tenants with at least one active user.
- **Operators online** — platform principals with a valid session token.
- **Pending merges** — membership merge collisions awaiting manual resolution.
- **High-risk · 24h** — purge / kind-escalate events fired in the last 24 hours.

**High-risk events**
Shows the 10 most recent audit events with severity=high. Requires the \`audit_read\` capability. Click *View all →* to open the full Audit log.

**Active impersonation**
Lists all live impersonation sessions. Visible with \`users_impersonate\` or \`audit_read\`. Click *Stop* to navigate to the Impersonation page and end a session.

**Quick actions**
Fast navigation links scoped to your capabilities. Only shown when at least one action is available.`,
  docsUrl: '/docs/admin/dashboard',
};

const AGENCIES: HelpArticle = {
  id: 'agencies',
  title: 'Agencies',
  body: `Agencies are top-level tenant organisations that group multiple buildings and users.

**Listing**
The table shows every agency with its slug, member count, and status. Use the search bar (top-right) to filter by name or slug.

**Suspending an agency**
Suspending prevents all users in the agency from logging in. Active sessions are not terminated immediately; they expire normally. Requires the \`agencies_read\` capability to view and a destructive-confirm dialog to proceed.

**Adding a domain**
Domains are used to auto-assign new user registrations to an agency. Only one domain per TLD suffix is allowed across the platform.`,
  docsUrl: '/docs/admin/agencies',
};

const USERS: HelpArticle = {
  id: 'users',
  title: 'Users',
  body: `The Users page lets you search for and manage platform principals.

**Search**
Enter an email address or display name fragment. Results are returned from the API — the list is not pre-loaded.

**Principal kinds**
- **Tenant** — normal property-management user.
- **Agency** — agency-level operator.
- **Platform** — super-admin (this console). Use *Change kind* carefully.
- **Service** — machine principal used by integrations.

**Impersonation**
Click *Impersonate* to start a time-limited session as that user. The session appears in Active Impersonation on the Dashboard. Requires the \`users_impersonate\` capability.

**Change kind**
Permanently changes how the user authenticates and which capabilities they may hold. This action is irreversible without a further change. Requires \`principal_kind_escalate\`.`,
  docsUrl: '/docs/admin/users',
};

const MEMBERSHIPS: HelpArticle = {
  id: 'memberships',
  title: 'Memberships',
  body: `Memberships link a user (principal) to an agency or building with a specific role.

**Granting a membership**
Click *Grant* and select the target user, agency/building, and role. Requires the \`memberships_grant\` capability.

**Revoking a membership**
Select one or more rows and click *Revoke*. Active sessions are not terminated; the next API call from that user will be denied. Requires \`memberships_revoke\`.

**Merge collisions**
When two users' memberships conflict during a tenant merge, a collision record is created. Resolve these from the Dashboard *Pending merge collisions* card or on this page.`,
  docsUrl: '/docs/admin/memberships',
};

const CAPABILITIES: HelpArticle = {
  id: 'capabilities',
  title: 'Capabilities',
  body: `Capabilities are named permission flags assigned to platform principals.

**Viewing**
The table lists every capability that exists in the system alongside the principals who currently hold it.

**Granting**
Click *Grant capability* and choose the target principal and capability name. Requires \`memberships_grant\`.

**Revoking**
Click the revoke button next to a row. Requires \`memberships_revoke\`.

**Common capabilities**
| Capability | Effect |
|---|---|
| \`audit_read\` | Read the audit log |
| \`users_read\` | Search and view users |
| \`users_impersonate\` | Start impersonation sessions |
| \`memberships_grant\` | Grant capabilities/memberships |
| \`memberships_revoke\` | Revoke capabilities/memberships |
| \`site_settings_write\` | Manage platform settings |
| \`feature_flags_write\` | Toggle feature flags |
| \`oauth_client_write\` | Manage OAuth clients |`,
  docsUrl: '/docs/admin/capabilities',
};

const AUDIT: HelpArticle = {
  id: 'audit',
  title: 'Audit Log',
  body: `The Audit log records every significant action performed by any principal.

**Filters**
- **Actor** / **Actor ID** — who performed the action.
- **Target** / **Target type** — what was affected.
- **Action** — the specific operation name.
- **Outcome** — \`success\`, \`failure\`, or \`denied\`.
- **From / Until** — ISO date range.

**Severity**
Events are tagged high / medium / low. High-severity events (purge, escalate, impersonation) appear on the Dashboard.

**Pagination**
Results are paginated server-side. Use *Previous* / *Next* to navigate. The API returns up to 50 items per page.

Requires the \`audit_read\` capability.`,
  docsUrl: '/docs/admin/audit',
};

const IMPERSONATION: HelpArticle = {
  id: 'impersonation',
  title: 'Impersonation',
  body: `Impersonation lets a platform operator act as another user temporarily.

**Starting a session**
From the Users page, find the target user and click *Impersonate*. A time-limited JWT is issued; the operator's browser is redirected to the property-management SPA as that user.

**Session limits**
Sessions last a maximum of 1 hour and cannot be renewed. Every impersonation start/end is recorded in the audit log.

**Ending a session**
Click *End impersonation* in the banner that appears across the top of the SPA, or use the *Stop* button on the Dashboard or this page.

Requires the \`users_impersonate\` capability.`,
  docsUrl: '/docs/admin/impersonation',
};

const FEATURE_FLAGS: HelpArticle = {
  id: 'feature-flags',
  title: 'Feature Flags',
  body: `Feature flags enable or disable functionality per tenant in real time.

**Available flags**
- **AI fault triage (beta)** — enables the AI-powered fault severity classifier for a tenant's operators.
- **New listings search (beta)** — activates the redesigned property search UI in the Reality Portal for this tenant.
- **Disable this building (kill switch)** — immediately prevents all API access for the selected building. Use in emergencies.

**Effect**
Changes take effect on the next API request from any user in the affected tenant — no deployment is needed.

Requires the \`feature_flags_write\` capability.`,
  docsUrl: '/docs/admin/feature-flags',
};

const PLATFORM_SETTINGS: HelpArticle = {
  id: 'platform-settings',
  title: 'Platform Settings',
  body: `Global settings that apply to the entire PPT platform.

**Maintenance mode**
When enabled, all non-admin API requests return 503. Existing sessions are preserved. Use this before database migrations or infrastructure changes.

**Signup enabled**
Controls whether new user registrations are accepted. Disabling this prevents new accounts without affecting existing users.

**Support email**
Shown in error pages and automated emails sent to end users. Must be a valid email address.

Changes are applied immediately without requiring a server restart.

Requires the \`site_settings_write\` capability.`,
  docsUrl: '/docs/admin/platform-settings',
};

const MOBILE_CONFIG: HelpArticle = {
  id: 'mobile-config',
  title: 'Mobile Config',
  body: `Remote configuration delivered to the Property Management mobile app on startup.

**Config keys**
The mobile app polls this endpoint every 60 seconds. Keys control feature availability, API endpoint overrides, and minimum app version requirements.

**Minimum app version**
Set \`min_version\` to force users on older builds to update before they can connect. The app compares this against the build number embedded in the binary.

**Force update URL**
When \`min_version\` is enforced, the app shows a prompt and links to this URL (usually the App Store / Google Play listing).

Requires the \`mobile_config_write\` capability.`,
  docsUrl: '/docs/admin/mobile-config',
};

const HEALTH: HelpArticle = {
  id: 'health',
  title: 'Platform Health',
  body: `Real-time monitoring of API server metrics and alert conditions.

**Metrics grid**
Each tile shows the current value and status (Normal / Warning / Critical). Click a metric name to open the time-series drill-down panel.

**Time ranges**
Select 1h, 6h, 24h, 7d, or 30d to view historical values. The panel shows min, max, and average alongside the raw data points.

**Alerts**
Active alerts require attention. Toggle *Active only* to hide resolved alerts. Click *Acknowledge* to mark an alert as reviewed; this records your identity in the audit log.

**Thresholds**
Edit Warning and Critical threshold values inline. Threshold changes take effect on the next metric evaluation cycle (every 60 seconds).

Auto-refreshes every 60 seconds. Requires the \`audit_read\` capability.`,
  docsUrl: '/docs/admin/health',
};

const ANNOUNCEMENTS: HelpArticle = {
  id: 'system-announcements',
  title: 'System Announcements',
  body: `System-wide banners shown to all users of the Property Management application.

**Creating an announcement**
Click *New announcement* and fill in the title, body, and optional call-to-action URL. Set the severity (info / warning / critical) to control the banner color.

**Scheduling**
Announcements can have a start and end time. Outside these times the banner is not shown. Leave end time blank for an indefinite announcement.

**Targeting**
An announcement can target all tenants or a specific tenant slug.

**Deleting**
Announcements can be deleted at any time. This immediately removes the banner for all users.

Requires the \`site_settings_write\` capability.`,
  docsUrl: '/docs/admin/system-announcements',
};

const ONBOARDING_TOURS: HelpArticle = {
  id: 'onboarding-tours',
  title: 'Onboarding Tours',
  body: `Step-by-step tours that guide new admin users through key platform workflows.

**Tour list**
Each card shows the tour name, description, progress bar, and current status (Not started / In progress / Completed / Skipped).

**Starting a tour**
Click *Start tour* to open the interactive overlay. Complete each step in order by clicking *Complete step*. The tour remembers your progress between sessions.

**Skipping a step**
Click *Skip step* to advance without marking the step complete. This counts as completing the step for progress tracking.

**Finishing a tour**
After the last step, click *Finish tour*. The tour status changes to Completed.

**Resetting a tour**
Completed or skipped tours can be restarted. Open the tour overlay and click *Restart tour*.

**Admin preview**
As a platform admin you can run any tour yourself to verify the content before publishing it to end users.

Requires the \`site_settings_write\` capability.`,
  docsUrl: '/docs/admin/onboarding-tours',
};

const OAUTH_CLIENTS: HelpArticle = {
  id: 'oauth-clients',
  title: 'OAuth Clients',
  body: `Third-party applications that obtain access tokens on behalf of your users via OAuth 2.0.

**Registering a client**
Click *Register client* and provide a display name and list of redirect URIs. The server generates a client ID and secret — **save the secret immediately**, it is shown only once.

**Regenerating the secret**
If a secret is compromised, click *Regenerate secret*. Old secrets are immediately invalidated. Requires an MFA confirmation.

**Revoking a client**
Revoking permanently deactivates the client. Existing access tokens continue to work until they expire (max 15 minutes). No new tokens can be issued.

Requires the \`oauth_client_write\` capability.`,
  docsUrl: '/docs/admin/oauth-clients',
};

// ---------------------------------------------------------------------------
// Route → article map
// ---------------------------------------------------------------------------

/**
 * Maps admin route prefixes to the help article to show for that page.
 * Matched with `location.pathname.startsWith(prefix)`, longest-match wins.
 */
export const ROUTE_HELP_MAP: Array<{ prefix: string; article: HelpArticle }> = [
  { prefix: '/tenants/agencies', article: AGENCIES },
  { prefix: '/identity/users', article: USERS },
  { prefix: '/identity/memberships', article: MEMBERSHIPS },
  { prefix: '/identity/capabilities', article: CAPABILITIES },
  { prefix: '/identity/oauth-clients', article: OAUTH_CLIENTS },
  { prefix: '/ops/audit', article: AUDIT },
  { prefix: '/ops/impersonation', article: IMPERSONATION },
  { prefix: '/ops/feature-flags', article: FEATURE_FLAGS },
  { prefix: '/platform/settings', article: PLATFORM_SETTINGS },
  { prefix: '/platform/mobile', article: MOBILE_CONFIG },
  { prefix: '/platform/health', article: HEALTH },
  { prefix: '/platform/announcements', article: ANNOUNCEMENTS },
  { prefix: '/platform/onboarding', article: ONBOARDING_TOURS },
  { prefix: '/', article: DASHBOARD },
];

/** Returns the best-matching article for the given pathname. */
export function getArticleForPath(pathname: string): HelpArticle {
  // Sort by prefix length descending so the most specific match wins.
  const sorted = [...ROUTE_HELP_MAP].sort((a, b) => b.prefix.length - a.prefix.length);
  const match = sorted.find((entry) => pathname.startsWith(entry.prefix));
  return match?.article ?? DASHBOARD;
}
