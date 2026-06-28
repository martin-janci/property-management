# EPIC-ACC-16 — Platform: Security, Data & Compliance · Stories

> Covers `UC-ACC-16.1–.9`. Cross-cutting foundation consumed by all epics (esp. EPIC-ACC-01).
> **Shared DoD:** AC pass · tests green · **tenant isolation is a release gate (NFR-ACC-01)** · changes audited · secrets never logged.

---

## STORY-ACC-16-001 — 2FA & session/login security
*Covers UC-ACC-16.1, .9*

**User Story:** As a **platform**, I want strong login security, so that accounts and their financial data are protected.

**Acceptance Criteria**
- **Given** a user, **when** they enable 2FA, **then** subsequent logins require the second factor; recovery codes are issued once.
- **Given** repeated failed logins, **when** a threshold is hit, **then** the account is rate-limited/locked and the event is audited.
- **Given** active sessions, **when** a user reviews them, **then** they can see and revoke sessions; password change revokes others.

**Technical Notes:** TOTP-class 2FA; lockout + anomaly signals; session store supports revocation.
**Test Cases:** 2FA enforced next login; lockout after N fails; session revoke kills tokens; recovery-code single-use.

## STORY-ACC-16-002 — Role-based access control (enforcement)
*Covers UC-ACC-16.2*

**User Story:** As a **platform**, I want server-side RBAC, so that permissions can't be bypassed by the client.

**Acceptance Criteria**
- **Given** a role, **when** any API action is attempted, **then** authorization is checked **server-side** and denied actions return 403 regardless of UI state.
- **Given** a read-only role, **when** a mutation is attempted via API directly, **then** it is rejected.
- **Given** cross-company access, **when** a user requests another company's data, **then** RLS + authz deny it (no leakage).

**Technical Notes:** centralized authorization in `api-core` middleware/extractors; RLS as defense-in-depth.
**Test Cases:** direct-API privilege escalation blocked; cross-company denied; 403 vs 404 semantics consistent.

## STORY-ACC-16-003 — Daily backups & restore
*Covers UC-ACC-16.3*

**User Story:** As an **Owner**, I want automatic backups with proven restore, so that I never lose my financial data.

**Acceptance Criteria**
- **Given** the platform, **when** a day passes, **then** an automated backup is taken and retained per policy.
- **Given** a backup, **when** a restore is exercised, **then** data is recovered intact within the recovery objective.

**Technical Notes:** restore must be **tested**, not assumed (NFR-ACC-05); per-tenant restore considered.
**Test Cases:** scheduled backup present; restore drill succeeds; retention pruning.

## STORY-ACC-16-004 — Data export, migration & import
*Covers UC-ACC-16.4, .5*

**User Story:** As an **Owner**, I want full data export and import, so that I'm never locked in and can onboard from another system.

**Acceptance Criteria**
- **Given** my company, **when** I export, **then** I receive a complete, portable archive (documents, contacts, items, payments) in standard formats.
- **Given** a source file/system, **when** I import, **then** records validate, conflicts are reported, and a summary is produced.

**Technical Notes:** export = no lock-in (NFR-ACC-11); import shares validation with EPIC-ACC-03/-04 importers.
**Test Cases:** export round-trips into import; conflict reporting; large-dataset export.

## STORY-ACC-16-005 — GDPR data-subject requests
*Covers UC-ACC-16.6*

**User Story:** As an **Owner/platform**, I want to fulfill access and erasure requests, so that we comply with data-protection law.

**Acceptance Criteria**
- **Given** a data-subject access request, **when** processed, **then** all personal data for the subject is exported.
- **Given** an erasure request, **when** processed, **then** personal data is erased/anonymized **subject to legal retention** of accounting documents, and the action is logged.

**Technical Notes:** reconcile erasure with statutory retention of invoices (anonymize vs delete); audit every DSAR.
**Test Cases:** access export completeness; erasure respects retention; DSAR audited.

## STORY-ACC-16-006 — Audit trail
*Covers UC-ACC-16.7*

**User Story:** As an **Owner/platform**, I want an immutable change history, so that every mutation is accountable.

**Acceptance Criteria**
- **Given** any document/setting mutation, **when** it occurs, **then** an audit entry records who/what/when/old→new.
- **Given** the audit log, **when** viewed (EPIC-ACC-01.10), **then** it is filterable and tamper-evident.

**Technical Notes:** append-only audit store; powers EPIC-ACC-01.10 view.
**Test Cases:** entry on every mutation type; immutability; filter.

## STORY-ACC-16-007 — Legislative & template updates
*Covers UC-ACC-16.8*

**User Story:** As a **platform**, I want tax rules, statutory formats, and templates kept current, so that customers stay compliant without manual upkeep.

**Acceptance Criteria**
- **Given** a legislative change (rates, filing formats), **when** an update ships, **then** new documents/filings use the new rules from the effective date while historical ones keep their original rules.
- **Given** templates, **when** updated centrally, **then** companies pick up improvements without losing custom branding.

**Technical Notes:** versioned, date-effective rule sets (ties to EPIC-ACC-10 e-file formats); pluggable per jurisdiction.
**Test Cases:** effective-date switchover; historical immutability; branding preserved across template update.

---

## Coverage
| Story | UCs |
|-------|-----|
| 001 2FA & sessions | 16.1, 16.9 |
| 002 RBAC enforcement | 16.2 |
| 003 Backups & restore | 16.3 |
| 004 Export/migration/import | 16.4, 16.5 |
| 005 GDPR DSARs | 16.6 |
| 006 Audit trail | 16.7 |
| 007 Legislative/template updates | 16.8 |
