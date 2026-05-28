# pm-backend — 2026-05-24

**Summary:** The active sprint carries three backend risk clusters: (1) two open security/correctness issues from the PR #435 post-merge review (#438, #439) — P1-05 SSRF confirmed still open at `routes/integrations.rs:2743` and `routes/signatures.rs:628`; (2) four 10B handler stubs that are mounted routes with no real logic; and (3) a hard dependency on the un-built Epic 2B infra that blocks 8a-3 WS sync and 6-5 DM realtime. PR #437 (dead AuthHandler/BuildingHandler deletion) closed the duplicate-handler divergence risk this run.

## Next actions
1. **[high]** Fix P1-05 SSRF — extract `validate_external_url`/`is_blocked_v4` from `services/actions/api_call.rs:152` into a shared module and apply it before `signatures.rs:628` `client.get(signed_url)` and `integrations.rs:2743` `client.post(&subscription.url)`. _DoD:_ URL parsed, host resolved, rejected for 127.0.0.1 / 10.x / 169.254.x / [::1]; unit test passes.
2. **[high]** Triage and close #438/#439 — give each of the named findings (P1-01 ordering, P1-04 Debug-format hash, P0-12 cookie scope, IG3 test gap, plus the 4 from #438) a sprint slot; P0/P1 land before Epic 7A review.
3. **[high]** Replace/guard the four 10B stub handlers (10b-4 system announcements, 10b-5 support data access, 10b-6 onboarding tour, 10b-7 contextual help) — return 501 with a documented timeline or ship real handlers with happy-path tests, so callers fail loudly rather than silently.
4. **[high]** Implement the missing Epic 81 endpoints (`/schedules/{id}/pause`, `/resume`, `/executions`) the frontend already calls — currently 404 in production.
5. **[medium]** Plan the module-split for documents.rs (3559), organizations.rs (4018), integrations.rs (4327) — each exceeds safe review/diff size; RLS-predicate omission risk grows with file size.
6. **[medium]** Decide Epic 2B WebSocket infra scheduling vs. formal deferral of 8a-3 + 6-5; update sprint-status.yaml blockers.

## Risks
- **SSRF (P1-05)** — `subscription.url` / `signed_url` reach `reqwest` with no private-range validation; an org member or poisoned provider can target `169.254.169.254` / internal services. Redirect-following is disabled on the webhook path but not on signatures (`reqwest::Client::new()`). probability medium · impact high.
- **10B silent stubs** — mounted routes return 200/204 with no logic → silent data loss / wrong UI state. probability high · impact medium.
- **RLS predicate omission in large route files** — a handler using a raw pool connection instead of `RlsConnection` silently bypasses row-level security. probability medium · impact high.
- **#438/#439 unscheduled** — Debug-format audit hash can silently break the integrity chain on a variant rename/toolchain bump; cookie scope misconfig enables sibling-subdomain credential theft. probability high · impact high.

## Decisions needed
- Schedule Epic 2B WebSocket infra next sprint OR formally defer 8a-3 + 6-5 with owner + date — owner: pm-tech-lead.
- Treat P0-12 cookie scope + P1-04 Debug-hash as a hotfix off dev, or batch into next release — owner: pm-tech-lead.
- 10B stub handlers: return 501 until implemented, or remain silent no-ops — owner: pm-tech-lead.

## Open questions
- Exact file:line for P1-01 (ordering), P1-04 (Debug-format hash), P0-12 (cookie scope) — named in #438/#439 but not yet pinned in code this run.
- Do the documents.rs RLS checks cover the share/download paths (7a-4/7a-5), or only list + get-by-id?
