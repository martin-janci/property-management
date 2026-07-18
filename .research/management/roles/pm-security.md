# pm-security — 2026-07-18

<sub>Rendered from role return by ppt-project-management at 2026-07-18T12:00:00Z.</sub>

## Summary
Recent PRs (#2416/#2421/#2419) hardened government_portal + rag/migrate RLS with the correct explicit org_id-predicate pattern, but two sibling IDOR plans (ai.rs LLM-document handlers, realtors.rs mark_inquiry_read) remain open/unverified, and today's dispatcher findings surface a live-webhook data-integrity issue and a DNS-rebinding SSRF that should block release.

## Next actions
- [high] Fix cross-tenant IDOR in routes/ai.rs publish_description (ai.rs:2620, state-mutating), list_listing_descriptions (ai.rs:2599), get_photo_enhancement (ai.rs:2847) per .research/plans/security-llm-doc-idor.md — bind principal, add organization_id predicate to llm_document.rs:397/358/977 — dependency: rust-backend — DoD: org-scoped SQL + regression tests mirroring government_portal_connection_cross_org_idor_tests.rs; cross-tenant publish/read return 404
- [high] Verify actual dev-branch state of realtors.rs:250 mark_inquiry_read — backlog.json marks security-realtors-mark-inquiry-read-idor plan archived but the plan file is still in .research/plans/ (not _archive/), and no fixing PR appears in this window's merged list — dependency: rust-backend — DoD: confirm handler binds principal + calls mark_inquiry_read_for_realtor; if not, apply fix and correct backlog.json/archive
- [high] Replace voice_commands.rs action_check_balance (line 248) and action_report_fault (line 278) fabricated-success stubs with real repo calls or fail-closed error on the live voice-webhook path (routes/voice_webhooks.rs -> process_command) — dependency: rust-backend — DoD: balance queried from real repo or endpoint returns explicit not-implemented; fault reports persist a real ticket or the call fails loudly instead of confirming a fake ticket number
- [high] Harden actions/api_call.rs validate_external_url (line 179) to resolve DNS hostnames and re-check every resolved address against the private/link-local/metadata blocklist, not just literal-IP hosts — dependency: rust-backend — DoD: hostname resolving to 169.254.169.254/10.x/fc00::/7 is rejected pre-request; regression test with a mock resolver
- [medium] Fix authorization.rs require_permission() (line 180) fail-open .unwrap_or(TenantRole::Guest) default, or delete the unused exported middleware so it can't be wired later as-is — dependency: rust-backend — DoD: missing-role requests reject (401/403) instead of defaulting to Guest; if kept, add a test asserting reject-on-missing-role
- [medium] Add per-email/per-IP rate limiting to /forgot-password and /resend-verification per .research/plans/security-forgot-password-no-rate-limit.md — dependency: rust-backend — DoD: check_email_rate_limit wired at auth.rs:1461 + resend handler; regression tests in auth_tests.rs

## Risks
- **Cross-tenant write/read IDOR in LLM-document endpoints (ai.rs) lets any authenticated user publish or read another org's generated listing description/photo enhancement** (prob: high · impact: high)
  Mitigation: apply org_id predicate per security-llm-doc-idor.md before next release
- **Voice webhook stubs fabricate success on financial-balance queries and silently drop fault reports (including potentially safety-relevant ones like gas smell) while confirming a fake ticket number** (prob: high · impact: high)
  Mitigation: fail closed or implement real persistence before this ships wider
- **DNS-rebinding SSRF in workflow api_call action reaches internal/cloud-metadata IPs when hostname (not literal IP) resolves privately** (prob: medium · impact: high)
  Mitigation: resolve-then-check all addresses, not just literal-IP hosts
- **backlog.json claims security-realtors-mark-inquiry-read-idor is archived/fixed but the plan file was never moved to _archive and no corresponding PR shows in this window — status may be stale/incorrect, giving false confidence the IDOR is closed** (prob: medium · impact: medium)
  Mitigation: directly verify realtors.rs:250 on dev before trusting backlog status
- **Two low-exploitability fail-open/fail-unsafe patterns (authz Guest default, idempotency lock never released on request cancellation) are dead-code/latent today but are exactly the kind of pattern that gets copy-pasted into new active routes** (prob: low · impact: medium)
  Mitigation: fix or delete now rather than leaving as a template for future handlers

## Open questions
- Is realtors.rs:250 mark_inquiry_read actually fixed on dev, or did backlog.json mark it archived before the code change landed?
- Is security-llm-doc-idor (score 3, high confidence) scheduled for this sprint, or still just backlogged with no owner?
- Do reality-server portal_listings.rs and inquiries.rs have any remaining unscoped single-row queries analogous to the fixed government_portal pattern? (not directly inspected this run — only agencies.rs was spot-checked and looks correctly scoped)
- Should api_core::middleware::authorization::require_permission be deleted (currently unused — route modules define local equivalents) rather than patched, to avoid maintaining two authz code paths?
- Are api_call.rs workflow action URLs restricted to an admin-curated allowlist anywhere upstream of validate_external_url, or is trigger-substituted user data the only gate?

## Decisions needed
- Prioritize security-llm-doc-idor and verification of the realtors mark_inquiry_read fix into this sprint vs. next — owner: eng-lead/pm-security
- Decide fail-closed vs. implement-for-real for voice_commands.rs balance/fault-report stubs before any further voice-webhook rollout — owner: rust-backend lead
