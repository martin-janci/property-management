# pm-security — 2026-07-06

**Summary.** PR #2120 fixed the outages.rs JWT-role-vs-DB-role authz bug, but the same root cause (`TenantExtractor.role` always resolves to Guest because the login flow never populates `AuthUser.role`) still gates manager-only mutations/overrides in ~9 other route files across Documents, Announcements, Templates, and Granular Notifications — all shipped as "done" this sprint. Separately, PII-exposing OCR draft PR #1797 remains unmerged after 13 days.

## Next actions

| Priority | Action | Owner dep | DoD |
|---|---|---|---|
| high | Migrate all `tenant.role.is_manager()` mutation/override gates in `routes/documents/{core,folders,shares,versions}.rs`, `announcements/{comments,engagement,lifecycle,crud,ai_draft,stats}.rs`, `templates.rs`, `granular_notifications.rs` to DB-validated role (`ValidatedTenantExtractor` / `RlsConnection.role()`), mirroring PR #2120 | pm-backend | All listed manager-gated handlers read role from RLS-validated org membership, not the JWT claim; regression tests drive real login flow per PR #1979 |
| high | Escalate stalled draft PR #1797 (missing auth on OCR endpoints + missing manager-gate on rental guest PII) | pm-scrum-master | PR merged, or endpoint feature-flagged off / manager-gated in production until merge |
| medium | Verify `dev` branch protection actually enforces "Require review from Code Owners" so the new `backend/deny.toml` CODEOWNERS gate (PR #2111) is a real, not advisory, control | pm-devops | `gh api repos/.../branches/dev/protection` confirms CODEOWNERS review is a required check |
| medium | Add a CI lint that flags a route handler holding both `TenantExtractor` and a mutating verb gated on `.role.is_manager()` without an accompanying RLS-derived role | pm-tech-lead | Lint/clippy rule merged and passing on dev |
| low | Add a second reviewer/team to the `/backend/deny.toml`, `Cargo.toml`, `Cargo.lock` CODEOWNERS lines currently owned solely by @martin-janci | pm-devops | CODEOWNERS entry lists ≥2 reviewers/team for supply-chain-critical files |
| low | Confirm `assign_fault` recipient-guard extraction (#2095) left no un-migrated caller of the old inline logic | pm-qa | `faults.rs` recipient_policy_tests + existing fault-notification tests green; no divergent inline copy remains |

## Risks

- **[high/high]** `TenantExtractor.role` defaults to Guest for every request; fixed only in outages.rs, still gates manager-only mutations/overrides across documents/announcements/templates/granular_notifications — features marked "done" this sprint (Epic 6, 7A) likely fail closed for real managers in production.
- **[high/high]** Draft PR #1797 (auth on OCR endpoints + manager-gate on rental guest PII) has sat unmerged 13+ days, leaving a live PII exposure vector.
- **[medium/high]** `backend/deny.toml` CODEOWNERS gate (PR #2111) is only effective if "Require review from Code Owners" is enabled on dev branch protection — unverifiable in-repo.
- **[low/medium]** Single-owner (@martin-janci) CODEOWNERS on Cargo.toml/Cargo.lock/deny.toml creates a bus-factor risk for supply-chain review.

## Open questions

- Is "Require review from Code Owners" actually enabled on `dev` branch protection?
- What is the specific blocker on draft PR #1797 — technical rework or review-queue backlog?
- Do Epic 6/7A stories currently marked `done` need re-verification given the same JWT-role-vs-DB-role bug class likely affects their manager-only mutation paths?
- Are there other extractors/services outside `servers/api-server/src/routes` (reality-server, mobile-native BFF calls) that assume `TenantExtractor.role` is populated?

## Decisions needed

- Fast-track merge vs. temporary production disable/gate for #1797's OCR/rental-guest-PII exposure — owner: pm-security / pm-scrum-master
- Schedule a dedicated remediation pass for the JWT-role-vs-DB-role mutation-gate class across documents/announcements/templates/notifications (single tracked issue vs. per-file follow-ups) — owner: pm-tech-lead
