# pm-security — 2026-09-25

## Summary

Both draft / `needs-human-review` retry PRs (#2977, #2976) target confirmed, still-live cross-org IDORs on `dev`:

- `backend/servers/api-server/src/routes/violations.rs` — `list_evidence` / `list_payments` / `list_comments` drop `AuthUser` entirely (handler binds `_user: AuthUser`) and the underlying repo methods in `backend/crates/db/src/repositories/violations.rs` use plain `&self.pool` with **no org filter** and **no RLS**, unlike `delete_evidence` / `create_appeal` in the same file which correctly scope by `org_id`.
- `backend/servers/api-server/src/routes/portfolio_performance.rs` — property sub-resource endpoints (`list_properties` / `get_property` / `update_property` / `remove_property`) bind `_auth: AuthUser` but skip the `org_id` check that sibling portfolio-level handlers (`get_portfolio` / `update_portfolio` / `delete_portfolio`) correctly apply via `get_org_id()`. This is a **write-capable** cross-tenant IDOR.

No merges landed this run — both fixes sit exposed in draft on `needs-human-review`.

## Next actions

1. **[high]** Prioritize human review + merge of PR #2977 — org-scope fix on violations.rs list_evidence/list_payments/list_comments. DoD: handlers derive `org_id` from `AuthUser` (not `_user`); repo methods filter by org; merged to `dev` with regression tests.
2. **[high]** Prioritize human review + merge of PR #2976 — portfolio_performance property sub-resource IDOR fix. DoD: `list_properties` / `get_property` / `update_property` / `remove_property` validate that `portfolio_id` belongs to `auth.tenant_id` (mirroring the sibling handlers in the same file); merged with regression tests.
3. **[high]** Add IDOR regression tests before merge — mirror `payment_management_tests::list_payments_is_org_scoped_and_counted` and `dispute_cross_org_idor_tests::list_evidence_is_scoped_to_owning_org`; each of the 7 vulnerable handlers gets a cross-org 403/404 test.
4. **[medium]** Audit `dev` tree for the discarded-AuthUser IDOR anti-pattern (handler signature `_auth: AuthUser` / `_user: AuthUser` beside a path-supplied `Uuid` lookup) across all api-server + reality-server route files. Pattern recurred independently in two unrelated features this sprint alone.
5. **[medium]** Confirm with whoever closed issues #2944 / #2945 on 2026-09-23 whether closure was tied to a merged fix or just to PR existence — the vulnerable code is still on `dev` today. Document / tighten security-issue-closure criteria: merged + tested, not draft PR creation.

## Risks

1. **[high · high]** Cross-org read of violation evidence, fine payments, and comments via violations.rs list handlers. Mitigation: merge PR #2977 after human review; add cross-org regression tests.
2. **[high · high]** Cross-org read + write of portfolio property financial records via portfolio_performance.rs property sub-resource endpoints. Mitigation: merge PR #2976 after human review; validate portfolio ownership before any property read/write.
3. **[medium · high]** Same discarded-AuthUser IDOR pattern likely exists elsewhere in the route tree — surfaced independently in two unrelated feature areas this sprint. Mitigation: repo-wide grep audit for `_auth: AuthUser`/`_user: AuthUser` combined with entity-by-Uuid lookups.
4. **[medium · medium]** Process churn — security-fix issues #2944 / #2945 were marked closed/completed on 2026-09-23 while the underlying vulnerable code remained unmerged on `dev`, causing a drop-then-resurrect (retry1) cycle that cost a day of exposure. Mitigation: gate security-issue closure on verified merge.
5. **[medium · high]** Both fixes are draft + `needs-human-review` with zero security-reviewer throughput this run — exposure continues each cycle they sit unreviewed while lower-severity correctness PRs (#2969/#2968/#2967 reality-web silent-fail) compete for review attention. Mitigation: route #2976/#2977 to a human security reviewer ahead of the correctness-only backlog.

## Open questions

- GitHub GraphQL was 403 in the cloud sandbox this run — could not pull the actual diffs of #2976 / #2977 to confirm they fix exactly the vulnerable line ranges identified here (violations.rs L322-333, L475-486, L625-637; portfolio_performance.rs L300-361). Reviewer should diff against these ranges.
- Who is the assigned human reviewer for `needs-human-review` security PRs, and what is the SLA — both PRs have sat in draft since at least 2026-09-25 02:44.
- Were issues #2944 / #2945 closed by an automated process without a merged fix present on `dev`?
- Do the `violation_evidence` / `fine_payments` / `violation_comments` / `portfolio_properties_perf` tables carry any DB-level RLS policies at all, or is org isolation entirely app-layer here (no `_rls`-suffixed repo methods exist in violations.rs, unlike the documents repo pattern)?

## Decisions needed

- Fast-track #2976 / #2977 through human review ahead of the correctness-only backlog (#2969 / #2968 / #2967) given confirmed live, write-capable cross-tenant IDOR. Owner: security / eng-lead.
- Open a follow-up repo-wide audit ticket for the discarded-AuthUser IDOR pattern beyond these two files. Owner: pm-security / rust-backend.
