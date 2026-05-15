# Multitenancy Phase Integration Plan

**Status as of 2026-05-15:**

All five phases (2, 3, 4, 5, 5.5) are committed in isolated worktrees branched off Phase 1 head `3f289dc9`. Each phase touches **non-overlapping files** by design (the prompts forbade cross-phase file writes), so most chunks integrate as straight cherry-picks. The only true integration points are the few files Phase 1 left open for downstream extension.

## Worktree → branch map

| Phase | Branch | Commits ahead of base | Verify fixes |
|-------|--------|------------------------|--------------|
| 2 | `phase2/identity-work` | 9 | `f7705b91` UserInvite Eq derive, `df06884c` exhaustive match, `2ec71fab` rand 0.10 SysRng |
| 3 | `feature/phase-3-hosting-theming` | 6 | `6dd3300d` AgencyBranding name collision |
| 4 | `feature/phase-4-publish-channels` | 5 | `a38fe559` AppState::new 4-arg test fixture |
| 5 | `feature/phase-5-superadmin` | 6 | `b67a6d9e` admin-core re-exports (require_capability, AdminDeps, IMPERSONATION_TTL) |
| 5.5 | `feature/phase-5p5-tenant-lifecycle` | 8 | `21d75679` metrics 0.21 syntax + manifest stale string + cleanups |

> Phase 2 branch named `phase2/identity-work` because main repo already had `feature/phase-2-identity-unification` (placeholder for roadmap doc). Rename target branch on integration.

## Migration number ranges (claim table — no overlaps)

| Phase | Migrations |
|-------|------------|
| 2 | 00127–00132 |
| 3 | 00133–00134 |
| 4 | 00135–00136 |
| 5 | 00137–00138 |
| 5.5 | 00139–00140 |

Schema-level integration is **purely additive** — no two phases write to the same table column at the same time. Only Phase 4 modifies an existing RLS policy (listings); only Phase 5.5 modifies all RLS policies (soft-delete filter). Their edits are applied via separate ALTER POLICY / replace statements at separate migration numbers, so they apply in order without conflict.

## Cross-phase file collisions (small, expected)

These files were touched by more than one phase. Each line indicates the resolution:

| File | Phases | Resolution |
|------|--------|-----------|
| `backend/Cargo.toml` (workspace `members`) | 5 (admin-core), 5.5 (tenant-ops) | Both add a new crate to `members[]`. Conflict is line-additive; merge takes both. |
| `backend/Cargo.lock` | 2, 3, 5, 5.5 | Regenerate after final merge: `cargo update -w` once at end. |
| `backend/crates/api-core/src/middleware/host_tenant.rs` | 3 (tweaks), 4 (PlatformHost variant) | Apply Phase 4 first (adds variant), then Phase 3's tweaks (compose). |
| `backend/crates/api-core/src/middleware/mod.rs` | 5.5 (registers tenant_ops) | Phase 5.5 only. No conflict. |
| `backend/crates/api-core/src/extractors/auth.rs` | 2 (token-trust hardening) | Phase 2 only. |
| `backend/crates/api-core/src/extractors/rls_connection.rs` | 4 (global_read context) | Phase 4 only. |
| `backend/crates/api-core/src/lib.rs` | 5.5 (re-export cache) | Phase 5.5 only. |
| `backend/crates/db/src/models/mod.rs` | 2, 3 | Both add new `pub mod` lines. Line-additive. |
| `backend/crates/db/src/repositories/mod.rs` | 2, 3 | Same. Line-additive. |
| `backend/servers/api-server/src/routes/mod.rs` | 3 (mount /tenant-config + /admin/tenants), 5 (replaces admin.rs with admin/) | Apply Phase 5 first (it does the file move admin.rs → admin/mod.rs), then Phase 3's additional mount lines. |
| `backend/servers/api-server/src/main.rs` | 3 (wire tenant_config state) | Phase 3 only. |
| `backend/servers/api-server/src/lib.rs` | 5 (mount admin tree) | Phase 5 only. |

No frontend file is touched by two phases (admin-ui = Phase 5; reality-web theming = Phase 3; ppt-web admin = Phase 5).

## Recommended integration order

The order matches the roadmap's dependency chain — each step takes ~2 minutes plus one cargo check between merges:

```
main
  └─ feature/phase-1-tenant-resolution                (already done)
       └─ feature/phase-2-identity-unification        (1) ← rename phase2/identity-work
            └─ feature/phase-3-hosting-theming        (2)
                 └─ feature/phase-4-publish-channels  (3)
                      └─ feature/phase-5-superadmin   (4)
                           └─ feature/phase-5p5-tenant-lifecycle (5)
                                └─ INTEGRATION       merge to main as one PR per phase
```

### Step-by-step

```bash
# Pre: confirm Phase 1 head matches what each worktree branched from
PHASE1_HEAD=3f289dc9
git fetch
git checkout main
git checkout -b integration/multitenancy-phases-2-5p5 $PHASE1_HEAD

# 1) Phase 2 — identity (the riskiest, lands first so all others rebase onto its principal_kind types)
git merge --no-ff phase2/identity-work -m "Merge Phase 2 — identity unification"
cargo check --workspace --tests   # gate

# 2) Phase 3 — hosting & theming (depends on nothing in Phase 2, but lands here for convenience)
git merge --no-ff feature/phase-3-hosting-theming -m "Merge Phase 3 — hosting & theming"
cargo check --workspace --tests

# 3) Phase 4 — publish state + 4th RLS context (touches host_tenant.rs already modified by Phase 3)
git merge --no-ff feature/phase-4-publish-channels -m "Merge Phase 4 — publish + global portal"
# expected conflict: host_tenant.rs (Phase 3 tweaks vs Phase 4 PlatformHost variant)
# resolution: keep Phase 4's TenantSource variant block + Phase 3's tweaks
cargo check --workspace --tests

# 4) Phase 5 — admin-core + admin routes
git merge --no-ff feature/phase-5-superadmin -m "Merge Phase 5 — super-admin console"
# expected conflict: routes/mod.rs (admin.rs deletion + tenant_config mount)
# resolution: take Phase 5's admin/ tree + Phase 3's tenant_config mount
# wire Phase 5's RequireCapability into Phase 3's stub `require_platform_principal`
cargo check --workspace --tests

# 5) Phase 5.5 — tenant lifecycle + manifest + ops middleware
git merge --no-ff feature/phase-5p5-tenant-lifecycle -m "Merge Phase 5.5 — tenant lifecycle & operability"
cargo check --workspace --tests

# Final: regenerate lockfile, push as ONE PR per phase (5 PRs, stacked) OR one big PR
cargo update -w
git push -u origin integration/multitenancy-phases-2-5p5
gh pr create --title "Multitenancy phases 2-5.5: identity, hosting, publish, super-admin, lifecycle" \
             --body-file docs/multitenancy/INTEGRATION-PLAN.md
```

## Cleanup after Phase 2 merges

Phase 2 leaves several known stubs/TODOs that are explicitly deferred to subsequent phases or post-merge cleanup:

- **portal_users physical table** — still present after Phase 2's merge migration. Drop in a Phase 2.5 cleanup PR after the back-pointer FK rewrites are complete.
- **JWT `tenant_id`/`role` claims** — still emitted but ignored at the extractor. Remove from token issuance after Phase 5 lands (no caller relies on them).
- **Phase 5's `RequireCapability` extractor stub** — Phase 3's admin tenant routes use a local `require_platform_principal` stub. After Phase 5 lands, swap the stub for `RequireCapability(Capability::SiteSettingsWrite)` etc. Grep: `TODO(Phase 5)`.

## Test gates between merges

Every merge step ends with `cargo check --workspace --tests`. If any merge step fails:

1. **Compile errors** → fix in the integration branch directly, commit as `fix(integration): <reason>`.
2. **Logic conflicts** → the host_tenant.rs and routes/mod.rs collisions are the only expected ones. Anything else means a phase wrote outside its territory — investigate.

DB migration order is enforced by file-name sort; no integration step alters that.

## Rollback

Each phase merge is a `--no-ff` merge commit, so `git revert -m 1 <merge-sha>` cleanly undoes one phase without touching the others. Order of revert = reverse of merge order.

## Final acceptance gates

Before opening the integration PR:

- [ ] `cargo check --workspace --tests` clean.
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` clean (deferred per individual phase agents — verify here).
- [ ] `cargo fmt --all --check` clean.
- [ ] `bash backend/scripts/check-rls-coverage.sh` exit 0.
- [ ] `bash backend/scripts/lints/no-raw-redis.sh` exit 0 (Phase 5.5 added).
- [ ] `cd frontend && pnpm install && pnpm -F @ppt/admin-ui build && pnpm -F reality-web build && pnpm -F @ppt/web build` clean.
- [ ] All migration numbers contiguous 00127 → 00140, no skips.

## Defenses & invariants checklist (carry through merge)

The 22 leak triage items from the brainstorming session must each have a defense in the integrated branch:

| Leak | Phase | Defense |
|------|-------|---------|
| #1 host spoofing | 1 | Trusted proxy host only |
| #2 conn-pool bleed | 1 | Drop guard in RlsConnection (extended in Phase 4 for global_read) |
| #3 cache poisoning | 1 | TTL + invalidation in TenantResolutionCache |
| #4 Caddy DoS | 3 | ask-endpoint rejects unknown hosts |
| #5 raw query bypass | 0 | check-rls-enforcement.sh covers reality-server |
| #6 dev `/a/{slug}` in prod | 1 | Env-gated off in prod |
| #7 merge collision | 2 | user_merge_collisions table; never auto-merge |
| #8 principal_kind escalation | 2 | DB trigger + SECURITY DEFINER set_principal_kind() |
| #9 membership injection | 2 | Capability-gated invites; single-use email-bound tokens |
| #10 stale token authz | 2 | Authz re-derived per request from DB |
| #11 skeleton-key token | 2 | Token carries user only; tenant = host ∩ memberships |
| #12 platform escalation | 2/5 | Out-of-band path + `Capability::PrincipalKindEscalate` |
| #13 wrong-tenant policy | 2 | Auth policy re-evaluated at privilege change |
| #14 bad migration | ops | Expand-contract; covered by docs/multitenancy/operability.md |
| #15 noisy neighbor | 5.5 | Per-tenant rate limit in tenant_ops middleware |
| #16 restore-one rolls all | 5.5 | Soft-delete + restore-to-new-org-id |
| #17 GDPR purge gap | 5.5 | Manifest-driven purge; CI fails on new tenant table without entry |
| #18 backup is breach | ops | docs/multitenancy/operability.md |
| #19 no per-tenant metering | 5.5 | Prometheus counters tagged by org_id in tenant_ops middleware |
| #20 shared Redis cross-tenant | 5.5 | TenantedRedis + no-raw-redis CI gate |
| #21 super-admin catastrophic | 5 | MFA-gated capability grants; audit-of-self-grant forbidden |
| #22 no per-tenant kill switch | 3 | tenant_feature_flags.building_disabled |

If any leak number above lacks a corresponding code artifact in the integrated branch at PR time, that's a release blocker.
