# EPIC-ACC-01 — Accounts, Organizations & Access · Stories

> Covers `UC-ACC-01.1–.13`. Format per repo story convention. Platform mechanisms (2FA, audit, GDPR, backups) are
> implemented in [EPIC-ACC-16](EPIC-ACC-16-platform-security.md) and *consumed* here.
> **Shared DoD (all stories):** AC pass · unit+integration tests green · **strict per-company isolation verified (RLS)** ·
> audit-log entry on mutation · i18n externalized · mobile parity where flagged.

---

## STORY-ACC-01-001 — Register company & log in
*Covers UC-ACC-01.1, .13*

**User Story:** As a new **Owner**, I want to sign up and get a first company, so that I can start using the product immediately.

**Acceptance Criteria**
- **Given** valid sign-up details, **when** I register, **then** an account + first company (agenda) are created and I land logged-in on an empty dashboard.
- **Given** an existing account, **when** I log in (credentials or SSO), **then** I resume my last active company.
- **Given** wrong credentials or an unverified email, **when** I attempt login, **then** access is denied with a clear, non-enumerating message.

**Technical Notes:** account ≠ company; auth via shared `api-core` (api-server issues JWT, accounting-server validates). Password hashing Argon2-class.
**UI/UX:** minimal sign-up; "remember last company"; mobile-flagged.
**Test Cases:** duplicate-email rejected; SSO path; last-company resume; rate-limited login.

## STORY-ACC-01-002 — Create & switch companies (agendas)
*Covers UC-ACC-01.2, .3*

**User Story:** As an **Owner/Accountant**, I want several companies under one login, so that I manage all my entities (or clients) in one place.

**Acceptance Criteria**
- **Given** my account, **when** I create another company, **then** it is fully isolated — no data bleeds between companies.
- **Given** multiple companies, **when** I switch the active one, **then** all lists, documents, settings, and numbering scope to it.
- **Given** a plan company-count limit, **when** I exceed it, **then** creation is blocked with an upgrade prompt.

**Technical Notes:** company = tenant boundary; every query RLS-scoped by company_id; active-company in session context.
**Test Cases:** cross-company isolation (the release gate, NFR-ACC-01); switch updates scope; plan-limit enforcement.

## STORY-ACC-01-003 — Users & role-based access
*Covers UC-ACC-01.4, .5, .6*

**User Story:** As an **Owner/Admin**, I want to invite teammates with roles, so that they do their work without over-broad access.

**Acceptance Criteria**
- **Given** a company, **when** I invite a user by email, **then** they receive an invite and join scoped to that company.
- **Given** a member, **when** I assign a role (Admin / Standard / Read-only), **then** their visible modules and allowed actions match the role exactly.
- **Given** a member, **when** I deactivate them, **then** their access is revoked immediately but their authored documents remain intact.
- **Given** a plan user-count limit, **when** exceeded, **then** invites are blocked with an upgrade prompt.

**Technical Notes:** RBAC enforced server-side (see EPIC-ACC-16.2), not just UI hiding; invitations are expiring tokens.
**Test Cases:** read-only cannot mutate (server-enforced); deactivation revokes live sessions; authored docs preserved; seat-limit.

## STORY-ACC-01-004 — Accountant access
*Covers UC-ACC-01.7*

**User Story:** As an **Owner**, I want to grant my accountant ongoing remote access, so that they work without me shuffling files.

**Acceptance Criteria**
- **Given** an accountant email, **when** I grant access, **then** they get a (typically read+export) role across the company's documents and exports.
- **Given** an accountant working multiple clients, **when** they log in, **then** they can switch between the companies that granted them access.
- **Given** I revoke access, **when** done, **then** the accountant immediately loses access to that company only.

**Technical Notes:** accountant is a cross-company principal; access grants are per-company edges; export scope (EPIC-ACC-12.7).
**Test Cases:** multi-client switch; revoke isolates to one company; export-only role cannot issue documents.

## STORY-ACC-01-005 — Subscription & plan limits
*Covers UC-ACC-01.8*

**User Story:** As an **Owner**, I want to manage my plan, so that I pay for what I use and understand my limits.

**Acceptance Criteria**
- **Given** my account, **when** I view billing, **then** I see current plan, usage vs limits (users, companies, API, metered AI reads), and renewal.
- **Given** a plan change, **when** I upgrade/downgrade, **then** entitlements (and gated features) update accordingly, prorated where applicable.
- **Given** a metered feature (e.g., AI document reads), **when** I exhaust the free quota, **then** I am offered paid overage and usage is metered.

**Technical Notes:** entitlement checks gate feature access; metered counters per company/period.
**Test Cases:** downgrade re-gates premium features; overage metering; limit displays match enforcement.

## STORY-ACC-01-006 — Account security & data rights
*Covers UC-ACC-01.9, .10, .11, .12 (consumes EPIC-ACC-16)*

**User Story:** As an **Owner**, I want 2FA, an activity log, and the ability to export or delete my data, so that I stay secure and in control.

**Acceptance Criteria**
- **Given** my account, **when** I enable 2FA, **then** subsequent logins require the second factor (mechanism: EPIC-ACC-16.1).
- **Given** company activity, **when** I open the audit log, **then** I see who changed what and when (source: EPIC-ACC-16.7).
- **Given** a data request, **when** I export, **then** I receive a complete portable archive (EPIC-ACC-16.4/.6); **when** I delete a company/account, **then** confirmations + retention rules apply (EPIC-ACC-16.6).

**Technical Notes:** thin account-level surface over EPIC-ACC-16 platform capabilities; deletion is guarded + logged.
**Test Cases:** 2FA enforced on next login; audit entries present for mutations; export completeness; delete confirmation + retention.

---

## Coverage
| Story | UCs |
|-------|-----|
| 001 Register & login | 01.1, 01.13 |
| 002 Multi-company | 01.2, 01.3 |
| 003 Users & RBAC | 01.4, 01.5, 01.6 |
| 004 Accountant access | 01.7 |
| 005 Subscription | 01.8 |
| 006 Security & data rights | 01.9, 01.10, 01.11, 01.12 |
