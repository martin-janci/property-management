# Code Review #3 — Phase 2 Identity Unification (Deep Dive)

**Branch:** `integration/multitenancy-phases-2-5p5`
**Reviewer:** Code Reviewer #3
**Date:** 2026-05-15
**Scope:** Phase 2 carries 7 of 13 P0 cluster-1/2 leak defenses (#7, #8, #9, #10, #11, #12, #13). This is the security-critical phase.

---

## 1. Algorithm trace — `RequestPrincipal` extractor

File: `backend/crates/api-core/src/extractors/principal.rs`.

```
[L80–L97]  Read `Authorization: Bearer …` header. Read `JWT_SECRET` env var.
           No header / wrong prefix → 401. Missing secret → 500 (never silently
           "open").
[L99–L107] `decode::<PrincipalClaims>` with `Validation::default()`.
           Default validation enforces HS256 + `exp` + `nbf`. Audience and
           issuer are NOT checked — see Issue T1.
           On any decode error → 401 ("Invalid or expired token").
[L108]     Bind `user_id = token_data.claims.sub`. From here on, every other
           JWT claim (`tenant_id`, `role`, `kind`) is IGNORED. ✅ Defense for
           leaks #10/#11.
[L111–L124] `SELECT principal_kind FROM users WHERE id=$1 AND status!='deleted'`.
           DB error → 500 + log. NOT cached. ✅ Re-derives kind from trusted
           server-side state on EVERY request — defends leak #8 (a stale
           "platform" claim in a JWT cannot promote a user).
[L126–L129] No row → 401 "Unknown principal" (deleted/never-existed user).
           ✅ Soft-deleted users cannot present a JWT.
[L130]     `PrincipalKind::parse(&kind_str)` — note the parser FALLS BACK to
           `Staff` for unknown strings (`models/user.rs:140`). Defensive
           default is fine; the CHECK constraint already restricts the value.
[L133]     `parts.extensions.get::<ResolvedTenant>().copied()` — Option<…>.
           Note: `ResolvedTenant.organization_id` is `Uuid::nil()` for
           `PlatformHost` (per `host_tenant.rs:75`). The match below treats
           any `Some(rt)` as a real org, including the platform host. See
           Issue C2.
[L135–L180] Match `(resolved, kind)`:
   • `(Some(rt), Public|Staff)`  → `MembershipRepository::is_active(user, rt.org)`.
       not active → 403 "no active membership in this organization". ✅ leak #11.
       Active → `effective_org = Some(rt.org)`.
   • `(Some(rt), Platform)`      → trust the platform principal: `effective_org
                                    = Some(rt.org)`. NO membership check.
       ✅ Intended (platform principals can act on any tenant), but see C2.
   • `(None, Platform)`          → `effective_org = None`. Platform host case.
   • `(None, _)`                 → 403 "tenant not resolved". Fail-closed if
                                    a non-platform user reaches a route with
                                    no host resolution. ✅ Defense in depth.
[L184]     `parts.extensions.insert(user_id)` — back-compat for downstream
           extractors that read user_id from extensions.
[L186–L190] Return `RequestPrincipal { user_id, kind, effective_org }`.
```

**No audit row is written on any of the 403 branches.** Per the brainstorming session, defense #10/#11 calls for audit visibility on cross-tenant rejections. Currently we only emit `tracing::warn!`. See Issue O1.

---

## 2. Trigger mechanism analysis — `users_principal_kind_guard`

File: `backend/crates/db/migrations/00129_principal_kind_guards.sql`.

### How it works

1. `BEFORE UPDATE OF principal_kind ON users FOR EACH ROW`. The trigger fires
   ONLY when the `principal_kind` column appears in the SET list (Postgres
   semantics for `BEFORE UPDATE OF col`).
2. If `NEW.principal_kind IS NOT DISTINCT FROM OLD.principal_kind` → no-op
   (lets touch-only updates pass).
3. Re-validates the value against the `('public','staff','platform')` set
   (defense in depth on top of the column-level CHECK).
4. Reads session GUC `app.principal_kind_change_authorized`. Anything other
   than the literal string `'true'` → `RAISE EXCEPTION` with SQLSTATE
   `insufficient_privilege` and a message naming the guard + the leak ids.
5. `set_principal_kind(target, new_kind, actor, reason)` is the ONLY caller
   that arms the GUC. It uses `set_config(..., TRUE)` — the `TRUE` third
   arg means **transaction-local**, not session-local. The flag is auto-
   cleared at COMMIT/ROLLBACK regardless of whether the function is exited
   normally. Belt-and-braces line 98 explicitly resets it within the same
   tx for the (rare) sub-transaction case.
6. `set_principal_kind` is `SECURITY DEFINER` — callers don't need raw
   `UPDATE` on `users`; they invoke the function and the function runs under
   the function-owner's privileges.
7. The function INSERTs an `audit_logs` row tagged
   `'account_updated'::audit_action` with `resource_type='users.principal_kind'`,
   the old and new kind in `details/old_values/new_values`, and a
   `leak_guard='principal_kind_change'` discriminator for greppability.

### What this defends

- ✅ A raw `UPDATE users SET principal_kind=…` from any session that has not
  armed the GUC fails. Verified by `principal_kind_guard_tests.rs::raw_update_to_principal_kind_is_rejected`.
- ✅ A direct `UPDATE users SET email=…` (a column other than principal_kind)
  is unaffected — the trigger does not fire (`BEFORE UPDATE OF principal_kind`).
- ✅ Mass-assignment via Rust `UpdateUser` (`models/user.rs:255`) cannot
  reach `principal_kind`: the struct has no such field.

### Theoretical bypasses

| Vector | Defended? | Notes |
|---|---|---|
| Raw UPDATE from a tenant-scoped pooled connection | ✅ Yes | GUC is unset → trigger raises. |
| `RESET ROLE` after acquiring connection | ✅ Yes | Trigger fires irrespective of session role; only the GUC matters. |
| `SET ROLE` to `users` table owner | ⚠️ Partial | Postgres BEFORE-row triggers fire under the function's `SECURITY INVOKER` semantics by default — i.e. they fire *for the role doing the UPDATE*. A `SET ROLE` to a role that owns `users` does NOT bypass the trigger (PG triggers are not skippable by ownership). The only true escape is a superuser running `ALTER TABLE ... DISABLE TRIGGER` — out of scope of in-app code. |
| Inserting a row with the column set + `INSERT … ON CONFLICT DO UPDATE` | ⚠️ Worth a test | Trigger fires on the UPDATE path of upserts; INSERTs of fresh rows go through the column DEFAULT (`'staff'`) unless the INSERT specifies the value. The merge migration 00132 DOES set `principal_kind='public'` on insert; this is allowed because there is no UPDATE trigger on INSERT — the CHECK constraint is the only gate. Confirmed correct. ✅ |
| Setting GUC manually from a hostile route (`SELECT set_config(...)`) | ✅ Yes (in practice) | Routes do not have raw SQL surface to the connection; `RlsConnection` returns a typed `PoolConnection<Postgres>` but the route-layer code paths that exist do not run user-controllable SQL. A hostile *handler* could in theory set the GUC and run UPDATE in the same transaction — but at that point the attacker is already inside the codebase. |
| Pooled-connection state bleed (set GUC in tx 1, exploit in tx 2) | ✅ Yes | `set_config(..., TRUE)` is transaction-local; the flag dies at COMMIT. Even if it were session-local, `clear_request_context()` runs on connection release (`tenant_context.rs:127`). |
| `set_principal_kind` called by an unprivileged caller | ⚠️ See P1 | The function is `SECURITY DEFINER` with no in-function caller-privilege check. Anyone with EXECUTE on the function can call it. The `actor` and `reason` args are passed in by the caller — not validated against any session identity. **There is no application-layer route that exposes this function**, but the in-DB authorization story is "EXECUTE perm = god mode." |

### Issues

- **P1 (medium):** `set_principal_kind` accepts an arbitrary `actor` UUID. It should at minimum cross-check against `current_setting('app.current_user_id')` and refuse if they disagree, OR accept no `actor` arg and read the session GUC directly. Right now an attacker who reaches the function-call surface can attribute the audit row to any user.
- **P2 (low):** The trigger reads `app.principal_kind_change_authorized` via `current_setting(..., TRUE)` (the `TRUE` makes a missing GUC return NULL). `clear_request_context()` does NOT explicitly reset `app.principal_kind_change_authorized` — it only resets `current_org_id`, `current_user_id`, `is_super_admin`, `global_read`. The `set_config(..., TRUE)` transaction-locality saves us today, but if a future maintainer changes that to session-local (`FALSE`), the cleanup hook will silently leak the flag. **Recommend extending `clear_request_context()` to also `set_config('app.principal_kind_change_authorized', 'false', FALSE)`** as defense-in-depth.

---

## 3. Token / invite hardness analysis

### Invite tokens (`user_invite.rs` + `00130_user_invites.sql` + `routes/admin/memberships.rs::generate_token`)

| Property | Status | Detail |
|---|---|---|
| Plaintext storage | ✅ Never | Only `token_hash = sha256_hex(token)` is persisted (`user_invite.rs:41-45`, `:73`). `token_hash` is `UNIQUE` so a hash collision (or replay-on-disk leak) is structurally suppressed. Verified by `user_invite_tests.rs::token_is_stored_only_as_hash`. |
| Hash algorithm | ⚠️ SHA-256 (raw) | SHA-256 is fast; an attacker who exfiltrates the table can offline-brute-force short tokens. Mitigated entirely by the 256-bit token size — see below. If the token size is ever reduced, switch to `argon2` or `blake3-keyed`. |
| Entropy at the source | ✅ 256 bits | `generate_token()` (`memberships.rs:249-255`) draws 32 bytes from `rand::rngs::SysRng` via `try_fill_bytes` (rand 0.10 OS-RNG path). This is `getrandom(2)` on Linux — CSPRNG-grade. base64-url-no-pad encoding → 43 chars, no padding. ✅ |
| Single-use | ✅ Atomic | `consume_by_token` (`user_invite.rs:81-144`) opens a tx, `SELECT … FOR UPDATE` on the row, then `UPDATE … WHERE accepted_at IS NULL`. The `accepted_at IS NULL` clause closes the TOCTOU window: even if two threads pass the in-memory `is_accepted()` check, only one wins the UPDATE; the loser sees `Option::None` and returns `AlreadyAccepted` (lines 137-140). Verified by `user_invite_tests.rs::second_accept_with_same_token_is_rejected`. |
| Email-bound | ✅ Case-insensitive | `invite.email.to_lowercase() != accepting_email.to_lowercase()` (`user_invite.rs:114`). `INSERT … VALUES (LOWER($1), …)` (`user_invite.rs:65`) so the stored side is normalized at write. ✅ Defends against case-shift smuggling. Verified by `user_invite_tests.rs::accept_with_wrong_email_is_rejected`. |
| Expiry | ✅ Enforced | `is_expired()` checks `expires_at <= Utc::now()` (`models/membership.rs:69`); used inside the FOR-UPDATE tx (line 110). Verified by `accept_after_expiry_is_rejected`. |
| Email match → membership write | ✅ | After `Accepted`, the route immediately writes `MembershipRepository::grant(...)` (`memberships.rs:179-195`). Note: this is in a **separate transaction** from the invite consume — see Issue I1. |
| Defense against unknown tokens | ✅ | `consume_by_token` returns `UnknownToken` for non-matching hash (`user_invite.rs:103`). No timing-distinguishable path between unknown-token and accepted-token (both run a query and then return; constant-time hash compare via PG = string compare on the fixed 64-char hash, side-channel inside Postgres). |

### TOCTOU & ordering issues

- **I1 (low):** `accept` route (`memberships.rs:152-201`) consumes the invite in tx-A and writes the membership in tx-B. If the membership write fails (DB outage), the invite is consumed BUT the user has no membership. The user can re-invite themselves out of this only by an operator regenerating a new invite. Not a security bug (fail-closed), but an operability foot-gun. The two writes should be wrapped in one transaction — easy refactor in a follow-up.
- **I2 (low):** `accept` does NOT verify the request principal's `principal_kind`. The handler allows ANY non-platform principal whose `user_id == req.user_id` to accept. That's the intent (a public/staff principal redeems their own invite), but a public-portal-user could in principle be granted an admin-role membership in a PM org via this flow. Capability gating per role-string lands in Phase 5; today, the only check is "platform principal can mutate, others can self-accept." Document this in the route comment.

### Token-scope leak #11 at HTTP layer

`token_scope_tests.rs` — three scenarios, all green expectations:

1. `token_works_for_org_with_active_membership` — JWT for user with grants in A and B; requests to `/a/agency-a` and `/a/agency-b` both 200.
2. `token_is_rejected_for_org_without_active_membership` — same JWT against `/a/scope-c` (no grant) returns 403. ✅ Defends leak #11 ("skeleton-key token").
3. `token_is_rejected_after_membership_revoked` — pre-revoke 200, post-revoke 403 with the SAME JWT. ✅ Defends leak #10 ("stale authz in a live token"). Confirms per-request DB resolution; no JWT-claim caching.

The HTTP test uses the real `host_tenant_middleware` + dev-mode `/a/{slug}` path — a production-equivalent wiring. Strong test.

---

## 4. Merge semantics audit — does Phase 2 achieve "one identity per human"?

### What the migration does (`00132_merge_portal_users_into_users.sql`)

- Adds `users.portal_origin_id` (UUID, NULLable, UNIQUE-where-not-null, FK → `portal_users(id)` ON DELETE SET NULL).
- Two-step body:
  1. **Collision queue:** `INSERT INTO user_merge_collisions … FROM portal_users JOIN users ON LOWER(email)=LOWER(email)` filtered by "not already merged" + "not already queued". Status `'pending'`. ✅ NO `ON CONFLICT DO UPDATE`.
  2. **Non-collision insert:** `INSERT INTO users … FROM portal_users WHERE NOT EXISTS (matching email in users) AND NOT EXISTS (already merged)`. ✅ Email collisions are filtered out — they are NEVER inserted into `users`.
- **Critical correctness check:** The merge has NO `ON CONFLICT` clause. A pre-existing `users` row with the same email is left UNTOUCHED. ✅ Defends leak #7. Verified by `portal_user_merge_tests.rs::collision_writes_collision_row_no_silent_merge`: post-merge there is exactly ONE `users` row for the colliding email (the pre-existing staff one), kind unchanged.
- **Idempotent:** verified by `portal_user_merge_tests.rs::merge_is_idempotent`.

### The dual-write window — is "one identity per human" actually enforced?

**Verdict: NOT YET. Phase 2 deliberately leaves a dual-write window open.**

- `portal_users` table is NOT dropped. Per the migration header comment and ROADMAP, "physical removal of `portal_users` is a future cleanup phase."
- `PortalRepository::create_user` (`portal.rs:46-108`) still INSERTs into `portal_users` AND mirrors a row into `users` (with `principal_kind='public'`, `portal_origin_id` back-pointer, `ON CONFLICT (email) DO NOTHING`). This is the **forward-going invariant** — every NEW portal signup writes both tables atomically (within a single tx). ✅
- BUT: `update_user`, `update_password_hash`, and the SSO upsert path (`portal.rs:840-857`) ONLY touch `portal_users`. **A password change on the portal-side is NOT mirrored into `users`.** A login attempt that comes through `RequestPrincipal` will check `users.password_hash` (which is now stale). Unless the reality-server's auth path was also rewritten to use `users` — and **it was not** (see below) — this is benign for now: the unified-identity authn for public users is not yet wired.
- **`reality-server` does NOT use `RequestPrincipal`.** Grep for `RequestPrincipal` in `backend/servers/reality-server/src` returns ZERO hits. `host_tenant_middleware` IS wired in `reality-server/src/main.rs:500-503`, but the user identity flow still goes through `PortalUserRepository` / `PortalUser` (`handlers/users/mod.rs`, `routes/users.rs`, `routes/articles.rs`, `routes/agent_reviews.rs`). Per the ROADMAP, "reality-server identity stack: adopt the same `RequestPrincipal` extractor and the unified `users` table" was a Phase 2 deliverable — **it is NOT done in this branch.** This is the single largest gap in Phase 2.

### Consequences of the gap

- Public portal users authenticated by reality-server still have a `portal_users` row as the source of truth for their password and session. The mirrored `users` row exists but is unused for authn.
- Cross-server: `RequestPrincipal` works in api-server (Phase 2 admin/membership routes). It is callable in reality-server (the trait + extractor are public), but NO route uses it. So a public user holding a reality-server-issued JWT cannot exercise the `users.principal_kind=public` paths because no such path exists yet on reality-server.
- The merge migration is correct in its narrow contract (no silent collapse), but the unified-identity goal — one human, one row, one credential — is only half-built. Remaining work is Phase 2.5 / future cleanup.

---

## 5. Test coverage table

| Leak | Defense | Test | Strength |
|---|---|---|---|
| **#7** | No silent merge of two humans sharing email | `portal_user_merge_tests.rs::collision_writes_collision_row_no_silent_merge` (positive) + `non_collision_inserts_users_row_with_origin_pointer` + `merge_is_idempotent` | **Strong.** Direct contract check on the migration's INSERT shape, plus idempotence. Tests replicate the migration body verbatim. ✅ |
| **#8** | `principal_kind` not flippable via mass-assignment | `principal_kind_guard_tests.rs::raw_update_to_principal_kind_is_rejected` | **Strong.** Asserts both rejection AND that the value did not change. ✅ |
| **#9** | Membership only via single-use, expiring, email-bound invite | `user_invite_tests.rs` (5 tests: accept-once, second-accept-rejected, expired, email-mismatch, unknown-token, hash-only-storage) | **Strong.** Covers all four invariants + storage. Race-condition test uses repeated single-thread `consume_by_token` calls — concurrent acceptance race is NOT directly tested (would need two tasks against the same pool), but the SQL `FOR UPDATE` + `WHERE accepted_at IS NULL` pattern is correct by inspection. ⚠️ Marginal: add a true two-task race test. |
| **#10** | Membership revocation kills next request even with live JWT | `membership_revocation_tests.rs::revoke_flips_is_active_immediately` (repo level) + `token_scope_tests.rs::token_is_rejected_after_membership_revoked` (HTTP level) | **Very strong.** Both layers covered; HTTP test mints a real JWT and presents it across the revoke. ✅ |
| **#11** | JWT carries no authority — re-derive per request | `token_scope_tests.rs::token_is_rejected_for_org_without_active_membership` + `token_works_for_org_with_active_membership` | **Strong.** Same JWT, different host slugs, opposite outcomes by membership presence alone. ✅ |
| **#12** | Promotion to `platform` is audited and gated | `principal_kind_guard_tests.rs::set_principal_kind_succeeds_and_writes_audit_row` | **Medium.** Verifies `set_principal_kind` writes ONE audit row with old/new kind. **Missing:** no test that an unprivileged HTTP route can NOT call this function. **Missing:** no test of the brainstorming session's "hardware-MFA verification step" — that path is not implemented at all. ⚠️ |
| **#13** | Auth-policy re-evaluated on every privilege change | **No test. No code.** The ROADMAP names `feature/per-org-auth-policy` as the source for the `AuthPolicy` type to adopt. Grep for `AuthPolicy` / `auth_policy` in `backend/` returns zero hits in this branch. | **❌ Not implemented.** Deferred without explicit acknowledgement in Phase 2 deliverables. |

---

## 6. Verdict — per-leak

| Leak | Status | Reason |
|---|---|---|
| **#7** Merge collision | ✅ | Migration has no `ON CONFLICT DO UPDATE`; collisions queued, not merged. Tests prove no silent collapse. |
| **#8** Mass-assignment to `principal_kind` | ✅ | DB trigger rejects raw UPDATEs; `UpdateUser` Rust struct has no field. Tests prove rejection. |
| **#9** Membership row injection | ✅ | All four invariants (single-use / expiring / email-bound / hash-only storage) are enforced atomically. Strong test coverage. Minor: full concurrency race not directly tested. |
| **#10** Stale authz in live token | ✅ | Per-request `is_active` lookup; revoke kills the next request on the same JWT. HTTP-layer test confirms. |
| **#11** Skeleton-key token | ✅ | Token only carries `sub`; effective org derived from `ResolvedTenant ∩ memberships`. No-membership host returns 403. HTTP-layer test confirms. |
| **#12** Platform escalation | ⚠️ | The DB-level guard is solid and audited, but: (a) no HTTP route or admin path actually exposes promotion (good for now, but the brainstorming session's "out-of-band, hardware-MFA verification step" is unbuilt); (b) `set_principal_kind` accepts a caller-supplied `actor` UUID without cross-check — an in-DB attacker can attribute the audit row to anyone. (c) Promotion path is not exercised end-to-end by tests. |
| **#13** Auth-policy re-evaluation | ❌ | No `AuthPolicy` code exists in this branch. `feature/per-org-auth-policy` referenced in ROADMAP was not adopted. Per-org password/verification policy + privilege-change re-eval is entirely absent. This is a P0 leak left unaddressed in the phase that owns it. |

### Cross-cutting issues

| Id | Severity | Issue |
|---|---|---|
| **T1** | low | `RequestPrincipal` uses default `Validation` — no audience/issuer checks. The legacy `AuthUser` extractor uses the same default; both should pin `aud="api-server"` (or per-server) so a token minted for reality-server cannot be replayed against api-server when the JWT_SECRET is the same. (Defense in depth.) |
| **C2** | low | `RequestPrincipal` treats `(Some(rt), Platform)` as "platform principal acting on tenant `rt.org`" — but if `rt.is_platform_host()` is true (the nil UUID), `effective_org` is set to `Uuid::nil()`. Downstream code that reads `effective_org` and uses it as a real org id will misbehave on the platform-host case. The extractor should branch on `rt.is_platform_host()` and set `effective_org = None` in that case. |
| **O1** | low | The 403 branches in `RequestPrincipal` only emit `tracing::warn!`. No `audit_logs` row is written. Cross-tenant rejection is exactly the signal a SOC wants in audit; the brainstorming session calls this out as part of leak #11's defense. |
| **P1** | medium | `set_principal_kind(target, new_kind, actor, reason)` takes a caller-supplied `actor` with no validation. Cross-check against `app.current_user_id` GUC, or remove the param and read the GUC directly. |
| **P2** | low | `clear_request_context()` does NOT reset `app.principal_kind_change_authorized`. Today the `TRUE` (transaction-local) third arg to `set_config` saves us. Add the reset for defense-in-depth in case a future change makes the GUC session-local. |
| **R1** | high | **reality-server has not adopted `RequestPrincipal` or the unified `users` table for portal user identity.** `portal_users` remains the source of truth for portal authn. Phase 2 ROADMAP explicitly lists this as a deliverable — it was not done. The dual-write mirror in `PortalRepository::create_user` covers new signups but `update_user` / `update_password_hash` / SSO upsert do NOT mirror updates. |
| **D1** | medium | Leak #13 (auth-policy re-evaluation) has no implementation and no test. ROADMAP mentions `feature/per-org-auth-policy` adoption — not done. Either build the AuthPolicy type now, or explicitly mark it deferred to Phase 5 in the ROADMAP and add a tracking issue. Silently dropping a P0 defense from the phase that owns it is the worst outcome. |

---

## 7. Top-line summary

Phase 2 lands the **schema + DB-trigger + extractor spine** of the unified identity model with high quality. The migrations are precise and idempotent; the trigger guard is real and well-tested; the per-request authority model in `RequestPrincipal` is the right design and is exercised end-to-end by `token_scope_tests.rs` against a real `host_tenant_middleware` pipeline. Five of seven owned P0 leaks (#7, #8, #9, #10, #11) are defended with strong coverage.

The phase is incomplete in three meaningful ways: (1) reality-server has not been switched onto the unified identity stack — `portal_users` remains live and authoritative for portal authn (R1); (2) the platform-promotion HTTP path and hardware-MFA step that the brainstorming session calls out as part of leak #12's defense are unbuilt — only the DB function exists, with a caller-supplied `actor` arg that should be cross-checked (P1, #12); (3) leak #13 (per-org auth-policy re-evaluation on privilege change) has no code and no test — it appears to have been silently dropped from the phase (D1, #13).

These gaps do NOT block merging Phase 2 into the integration branch — the defenses that ARE landed are the load-bearing ones. They DO need to be tracked explicitly in the ROADMAP (R1, D1, #12 hardware-MFA path) so they are not lost between phases. The ROADMAP currently lists them as Phase 2 deliverables marked complete; that is misleading.

**Recommendation:** merge with the gaps documented and tracked. Do NOT close out Phase 2 until R1 (reality-server adoption), P1 (set_principal_kind hardening), and D1 (leak #13 implementation or explicit deferral with rationale) are resolved. Add a follow-up story for I1 (single-tx invite-accept), O1 (audit on 403), T1 (audience/issuer pinning), C2 (platform-host effective_org), and P2 (clear principal_kind GUC).
