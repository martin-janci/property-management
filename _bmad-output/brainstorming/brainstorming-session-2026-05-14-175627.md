---
stepsCompleted: [1, 2, 3, 4, 5]
session_continued: true
continuation_date: 2026-05-16
inputDocuments: []
session_topic: 'Multitenant SaaS for property-management agencies — super-admin control plane, agency tenancy, per-agency subdomain/custom-domain hosting (shared frontend now, customizable later), shared super-admin across Reality Portal + PPT'
session_goals: 'Architecture options and roadmap sequencing, MVP-first'
selected_approach: 'ai-recommended'
techniques_used: ['First Principles Thinking', 'Morphological Analysis', 'Reverse Brainstorming']
ideas_generated: 70
context_file: ''
technique_execution_complete: true
facilitation_notes: 'User drove fast and decisively, consistently requesting and accepting reasoned recommendations. First Principles + a codebase audit reframed the problem early (4 false "shoulds"). Reverse Brainstorming surfaced leak #11 as a Phase-2 design decision. Session converged cleanly.'
session_active: false
workflow_completed: true
---

# Brainstorming Session Results

**Facilitator:** Mjanci
**Date:** 2026-05-14

## Session Overview

**Topic:** Multitenant SaaS for property-management agencies. The base platform and a largely multitenant-ready DB exist. Focus areas: (1) tenancy & hosting — each agency as a tenant reachable via its own subdomain or a brought custom domain, reusing the main Reality frontend (web + mobile) per-agency now, with per-agency customized frontends as a future horizon; (2) a super-admin control plane managing agencies, users, site settings, mobile-app settings, etc.; (3) cross-product reuse — the same super-admin pattern repeats in the PPT management app, with shared super-admin functionality.

**Goals:** Architecture options and roadmap sequencing, MVP-first.

### Session Setup

_Outcomes targeted: concrete architecture approaches (subdomain/custom-domain routing, tenant isolation, shared-vs-per-product code) and a prioritized roadmap separating MVP from later phases._

## Technique Selection

**Approach:** AI-Recommended Techniques
**Analysis Context:** Complex architectural topic with a divergent goal (architecture options) followed by a convergent goal (MVP-first roadmap sequencing).

**Recommended Techniques:**

- **First Principles Thinking** *(Phase 1 — Foundation)*: Strip multitenancy + super-admin to bedrock truths; separate what the current DB/system genuinely guarantees from what is assumed. Grounds later options in reality.
- **Morphological Analysis** *(Phase 2 — Idea Generation)*: Name the key architecture parameters (tenant resolution, isolation level, frontend-sharing, custom-domain handling, super-admin scope, cross-product reuse), list options per parameter, and combine into comparable architecture candidates. A Cross-Pollination prompt is injected mid-phase to force orthogonal options.
- **Resource Constraints** *(Phase 3 — Refinement & Action)*: Impose hard limits (ship in one month, no new infra, one engineer) to force the MVP boundary and let the phased roadmap fall out.

**AI Rationale:** The flow mirrors the user's two goals end-to-end — diverge to a real option space, then converge to a defensible MVP-first sequence — while building each phase on the previous one.

## Technique Execution Results

### Phase 1 — First Principles Thinking

#### Verified bedrock (assets to build on)

- RLS pattern is real and CI-tested in **api-server**: `is_super_admin() OR organization_id = get_current_org_id()`, ~66 tables, connection-scoped via `RlsConnection` + `set_request_context`. Deprecated middleware path fully removed.
- `X-Tenant-ID` → UUID extraction works, with a membership-validating `ValidatedTenantExtractor`.

#### Foundation findings — the "shoulds" that broke under inspection

**[Foundation #1]: The RLS Blanket Has Holes**
_Concept_: "Every tenant-scoped table has org_id + RLS" is FALSE. `listings`, `disputes` (+~12 child tables), and ~10 other migrations carry `organization_id` with no RLS policy.
_Why it matters_: Isolation guarantee is incomplete exactly where the agency-facing product lives (`listings`).

**[Foundation #2]: Tenancy Is api-server-Only**
_Concept_: RLS context is never set in reality-server — raw `self.db.acquire()`, plain queries, zero DB-layer tenant enforcement on the public portal.
_Why it matters_: The product targeted for per-agency multitenancy currently has no tenancy plumbing.

**[Foundation #3]: portal_users Are Global, Not Tenants**
_Concept_: `portal_users` has no `organization_id`, no RLS — global identities. Refined in Element 2: this is arguably correct *by design* — the global portal is a shared marketplace of house-hunters; agencies publish into it.
_Why it matters_: Central architecture fork — portal user belongs to the platform, agency staff belong to a tenant (see I-B).

**[Foundation #4]: The Super-Admin Column Is Orphaned**
_Concept_: Bypass works via JWT role claims. The `users.is_super_admin` DB column + `is_current_user_super_admin()` function are dead code, used by no policy.
_Why it matters_: Two competing super-admin mechanisms; one must be retired (one mechanism, auditable — see I-C).

**[Foundation #5]: Listings Are the Boundary-Crossing Entity**
_Concept_: An agency owns its listings (isolated from other agencies), but can *publish* a listing to the global portal. A listing therefore carries tenancy + a visibility/publish state. The agency's own surface (subdomain/custom domain) shows its listings; the global portal shows the union of published listings across agencies.
_Novelty_: Multitenancy here is not "everything per-agency" — it's "agency-owned data with an explicit, revocable publish projection into a shared global layer."

#### Invariants (any acceptable architecture must satisfy)

- **I-A:** An agency's data is never visible to another agency, enforced at the DB layer — *except* the deliberate, revocable "publish to global portal" channel for listings. Ownership/separation is always preserved.
- **I-B:** A public portal user (house-hunter) and an agency's staff are different identity types and may follow different identity rules.
- **I-C:** Super-admin sees across all tenants through exactly one mechanism, and it is auditable.
- **I-D:** A listing exists once (one source of truth). The agency's own surface and the global portal are both views of the same row, filtered by publish state — never copies, no sync job.
- **I-E:** A request's tenant is resolved from its host (subdomain or custom domain) → exactly one agency → before any data is touched. Unresolved host fails closed, not open.

### Phase 2 — Morphological Analysis

**Axis map:** A1 Tenant resolution · A2 Identity model · A3 Isolation enforcement · A4 Listing publish model · A5 Frontend hosting · A6 Super-admin control plane · A7 Cross-product reuse.

#### Axis A1 — Tenant resolution → DECIDED

Options considered: A1-1 edge injects trusted header · A1-2 app resolves from Host via DB table · A1-3 wildcard subdomains + alias rows · A1-4 path-based `/a/{slug}` · A1-5 token-carried (≈ current state).

**Decision — [A1-2] core, delivered as shared middleware, Caddy on-demand TLS:**
- `agency_domains` table (subdomain + custom domain → `org_id`) is the single source of truth.
- Host→tenant resolution lives in a **thin shared middleware in BOTH servers** (read `Host` → cached lookup → set RLS context → fail closed). This also closes Foundation #2 (reality-server gains RLS context) as a side effect.
- **Caddy with on-demand TLS** in front; its "ask" endpoint queries `agency_domains` — so one table drives both tenant resolution AND cert issuance. Wildcard cert for subdomains, on-demand for custom domains.
- **[A1-4] path-based `/a/{slug}` kept as dev-mode + MVP fallback** — works today with zero DNS/TLS, lets all tenancy plumbing be built/tested before touching DNS.
- **[A1-5] killed as primary**, retained only as a defense-in-depth cross-check (resolved-host tenant and JWT-claimed tenant must agree for authenticated agency-staff requests).
- _Honest tradeoff:_ per-request lookup + cache-invalidation burden on the app; Caddy becomes a hard dependency.

#### Axis A2 — Identity model → DECIDED

Options considered: A2-1 two-table split kept · A2-2 unified `users`, tenancy = membership rows · A2-3 unified + `principal_kind` discriminator · A2-4 federated (agency brings own IdP).

**Decision — [A2-3] unified `users` table with a `principal_kind` discriminator (public | staff | platform):**
- One identity table, one auth stack, one account per human — the "is-both" house-hunter/agency-staff case is free.
- `principal_kind` lets different identity rules apply per kind (password policy, e-mail verification, MFA) — operationalizes I-B and is the natural home for the per-org auth-policy work already started.
- Tenancy expressed via membership rows; super-admin = a `platform`-kind principal / platform-scoped membership.
- A2-4 (federated IdP) bolts on later for `staff` principals only.
- Migration cost (merging `portal_users` into `users`, reality-server adopting the full identity stack) accepted by user as fine.
- _Risk to manage:_ the discriminator must be rigorously enforced — a `public` row must never silently gain staff powers.

#### Axis A3 — Isolation enforcement → DECIDED

Options considered: A3-1 extend existing RLS everywhere · A3-2 schema-per-tenant · A3-3 database-per-tenant · A3-4 app-layer only · A3-5 publish exception as a fourth RLS context.

**Decision — [A3-1] + [A3-5]:**
- Keep one Postgres DB. Backfill RLS policies on the gap tables (`listings`, `disputes` + children, ~10 others). Flip reality-server on via the A1 shared middleware. Retire the orphaned `is_super_admin` column. Add a real CI check that every tenant table has a policy.
- **[A3-5] — the global portal is a fourth RLS context, not a separate system.** `listings` policy becomes `is_super_admin() OR organization_id = get_current_org_id() OR (is_published AND <global-read-context>)`. Four contexts, one policy set: global-read (published everywhere) · agency-subdomain (that agency's published rows) · agency staff (full tenant context) · super-admin (bypass).
- A3-2 / A3-4 killed. **A3-3 (database-per-tenant) parked as the enterprise/compliance upsell** for a whale customer.
- _Unlock:_ Foundation #5 + I-D collapse into one extra OR-clause instead of a sync pipeline.

#### Axis A4 — Listing publish model → DECIDED

Cross-pollination: Shopify (product + sales channels), Airbnb/Booking (one listing syndicated to many OTAs); the existing `integrations` crate already has external real-estate-portal clients.

Options considered: A4-1 binary `is_published` · A4-2 channel/target set · A4-3 lifecycle state machine + opt-in moderation · A4-4 derived search index (orthogonal scale decision).

**Decision — [A4-2] channel model, evolve from binary:**
- A listing carries a set of publish targets: `{agency_site, global_portal, external:<portal>, …}`. A3-5's global-read context checks "is `global_portal` in the set." Subdomain, global portal, and existing external integrations all become one channel abstraction.
- **[A4-3] moderation as an opt-in per-agency flag** — trusted agencies auto-publish to global; new ones can be gated. Becomes a super-admin / site-settings lever (ties to A6).
- **[A4-4] derived search index deferred** until global-portal traffic demands it (an index is a derived projection, not a copy — compatible with I-D).
- _Sequencing:_ ship a binary `is_published` column first, evolve it into a channel set later — same UI story, bigger backend later.

#### Axis A5 — Frontend hosting → DECIDED

Cross-pollination: WordPress multisite / Discourse hosting / Shopify storefronts — one codebase, runtime-themed per site; code customization comes much later.

Options considered: A5-1 single deployment + runtime theming · A5-2 multi-zone / per-agency builds · A5-3 edge rewrites · A5-4 tenant-config API + design tokens (mechanism) · A5-5 per-agency component slots.

**Decision — [A5-1] + [A5-4]:**
- One Next.js deployment. Middleware reads `Host` → resolves agency (A1 lookup) → loads branding → injects theme/context. Global portal = the platform-host case of the same app.
- **[A5-4]** branding (logo, colors, fonts, CSS vars, feature flags) served from a `/tenant-config` endpoint keyed by resolved host; app consumes design tokens. The existing `@ppt/ui-kit` + `--ppt-*` token system IS the customization layer — per-agency look = a config row, never a code fork.
- **Gotcha to manage:** Next.js SSG/ISR caching must be tenant-keyed or branding leaks across domains.
- **Mobile:** for MVP, the KMP/RN apps = global portal only. Agency-specific mobile (in-app selection or white-label builds) is a future axis.
- A5-2 / A5-5 parked as the explicit "future customization" horizon.

#### Axis A6 — Super-admin control plane → DECIDED

Discovery: a `platform_admin` table (migration 00029) + `SupportActivityLog` / `SupportUserSession` / `SupportUserInfo` repos already exist — audit infra is partly built; A6 consolidates onto it.

Options considered: A6-1 ppt-web section + api-server `/admin` · A6-2 separate super-admin service+app · A6-3 shared mechanism, both servers honor it, UI in ppt-web, admin API split by domain · A6-4 capability-based not role-boolean (mechanism refinement).

**Decision — [A6-3] + [A6-4]:**
- Super-admin *mechanism* (context flag + audit log) lives in shared crates (`common`/`api-core`/`db`) so **both** servers honor it identically — satisfies I-C and pre-wires A7.
- **Capability-gated, not boolean:** named capabilities (`agencies:write`, `site_settings:write`, `mobile_config:write`, `impersonate`); every action writes to `SupportActivityLog`. Supports tiered support staff.
- UI = a ppt-web section. Admin API split by domain: api-server owns PM-domain admin, reality-server owns portal/agency admin, both behind the same shared guard.
- **Retire the orphaned `users.is_super_admin` column (Foundation #4)** — the `platform_admin` table is the home.
- A6-2 (separate service) parked as a future hardening step.

#### Axis A7 — Cross-product reuse → DECIDED

Options considered: A7-1 mechanism shared only · A7-2 shared `admin-core` crate + `@ppt/admin-ui` library · A7-3 fully schema-driven admin · A7-4 one unified super-admin console.

**Decision — [A7-4] + [A7-2], explicitly NOT [A7-3]:**
- **[A7-4]** One unified super-admin console (the ppt-web section) with domain areas (Reality / PPT / platform-wide) — super-admin is platform-level, the cross-product split disappears at the UX layer. Both servers' admin APIs feed it.
- **[A7-2]** Backed by a shared `admin-core` crate (generic CRUD-over-tenant-resources, audit middleware, capability registry, settings-store) + `@ppt/admin-ui` component library (resource tables, settings forms, audit viewer, impersonation). Each domain registers its resources.
- **A7-3 skipped** — fully schema-driven admin is the over-engineering trap; A7-2 components may evolve toward config-driven for trivial CRUD later.

---

### Phase 2 Synthesis — Architecture Candidate: "Single-Core, Host-Resolved Multitenancy"

The 7 axis decisions combine into one coherent architecture:

- **Infra:** one Postgres DB, one api-server, one reality-server, one Next.js deployment, one mobile app — no per-tenant infra.
- **Tenant resolution (A1):** per-request from `Host` via an `agency_domains` table, in a thin **shared middleware both servers run**. Caddy on-demand TLS; its ask-endpoint queries the same table.
- **Isolation (A3):** RLS everywhere — existing pattern extended to gap tables; reality-server gains RLS context via that same middleware. Four contexts: global-read / agency-subdomain / agency-staff / super-admin.
- **Identity (A2):** one `users` table with `principal_kind` (public | staff | platform); tenancy via membership rows; `portal_users` merged in.
- **Publishing (A4):** a channel set on listings; global portal is the fourth RLS context reading `global_portal ∈ channels`. Ships binary, evolves to channels.
- **Frontend (A5):** one deployment, runtime-themed via `/tenant-config` + design tokens; global portal = platform-host case.
- **Super-admin (A6 + A7):** capability-gated mechanism in shared crates, honored by both servers, mandatory audit-log writes; ONE unified console (ppt-web) backed by `admin-core` + `@ppt/admin-ui`.

**Keystone — the shared host-resolution middleware.** It is the single piece that ties A1 (resolve) → A3 (set RLS context) → A5 (theme) → A6 (super-admin context). Build it once, four axes light up.

**New data-model / code surface required:**
- `agency_domains` table (host → org, verification state, cert state)
- `users.principal_kind` + merge `portal_users` → `users`
- `listings` channel column (start as binary `is_published`) + global-read RLS context
- RLS policies backfilled on gap tables (`listings`, `disputes` + children, ~10 others) + real CI coverage check
- capability registry; consolidate onto `platform_admin` + `SupportActivityLog`; retire `users.is_super_admin`
- `/tenant-config` endpoint + agency branding consumption in the frontend
- `admin-core` crate + `@ppt/admin-ui` package; unified console in ppt-web

**Future horizon (explicitly parked):** per-agency code customization (A5-2/A5-5), federated IdP (A2-4), database-per-tenant for enterprise (A3-3), separate super-admin service (A6-2), derived search index (A4-4), schema-driven admin (A7-3).

### Phase 3 — Resource Constraints → Durable Roadmap Split

_Reframed by user: not a "fake it" MVP cut — a dependency-ordered phase split where every phase is done properly (TDD, complete), so no feature is forgotten. "MVP-first" = sequencing order only. No week estimates (AI-assisted speed); each phase is a scope chunk, quality bar = "done well."_

**Phase 0 — Foundation hardening** _(no user-visible change; unblocks everything)_
Backfill RLS on gap tables (`listings`, `disputes` + children, ~10 others) · add real CI check (every tenant table has a policy) · retire orphaned `is_super_admin` column. _Fixes Foundation #1._

**Phase 1 — Keystone: tenant resolution**
`agency_domains` table · shared host-resolution middleware in both servers · reality-server adopts RLS context (closes Foundation #2) · path-based `/a/{slug}` dev mode (testable with zero DNS) · **+ minimal "agency create" sliver pulled forward** (see decision below).

**Phase 2 — Identity unification**
One `users` table + `principal_kind` · merge `portal_users` in · membership rows · per-principal-kind auth rules — absorbs the per-org password/verification work already started (paused tasks #89–92).

**Phase 3 — Agency goes live: hosting + theming**
Caddy + on-demand TLS · `/tenant-config` endpoint + branding · frontend runtime theming. First externally-visible milestone — pilot agency gets subdomain + custom domain.

**Phase 4 — Publishing & the global portal**
`listings.is_published` (binary) · global-read RLS context · global portal reads published-everywhere. The Foundation #5 / I-D payoff.

**Phase 5 — Super-admin control plane**
Capability registry + mechanism in shared crates · mandatory audit-log writes · `admin-core` + `@ppt/admin-ui` · unified console (agencies, users, site/mobile settings).

**Phase 6+ — Future horizon** _(listed so it is not forgotten — see parked list above)_

**Cross-phase dependency — DECIDED:** the pilot agency must exist before Phase 3 can onboard it. Decision: **pull a minimal "agency create" sliver forward into Phase 1** rather than a throwaway seed script — the sliver becomes the first piece of Phase 5's console, so no wasted work.

### Reverse Brainstorming — Leak Hunt (architecture stress-test)

_Goal: deliberately attack "Single-Core, Host-Resolved Multitenancy" to surface defenses before code exists. Each leak → a defense folded into a roadmap phase._

#### Cluster 1 — Tenant resolution + RLS context

- **#1 Host header spoofing** — app trusts raw `Host` → pick any tenant. _Defense: trust only proxy-validated host._
- **#2 Connection-pool context bleed** — pooled connection keeps prior tenant's RLS context after a panic/missed release. _Defense: guaranteed context-clear (Drop/finally), not best-effort._
- **#3 `agency_domains` cache poisoning** — released domain re-registered by attacker, stale cache still maps to old org. _Defense: TTL + invalidation on domain release; treat removal as a security event._
- **#4 Caddy on-demand TLS as DoS amplifier** — flood of unknown domains → Let's Encrypt rate-limit ban. _Defense: ask-endpoint hard-rejects hosts not in `agency_domains`._
- **#5 Raw-query backdoor** — `sqlx::query(&self.db)` bypasses RLS; reality-server does this today. _Defense: `check-rls-enforcement.sh` CI gate must cover reality-server._
- **#6 Dev-mode `/a/{slug}` ships to prod** — URL-editable tenant switch. _Defense: env-gated off in prod, with a test asserting it._

#### Cluster 2 — Identity (all land in Phase 2)

- **#7 Merge collision** — `portal_users`→`users` merge collapses two different humans sharing an email. _Defense: email collision = manual review, never auto-merge._
- **#8 `principal_kind` escalation** — public row's kind flipped to staff via mass-assignment. _Defense: never client-settable; one audited transition path; DB-level guard._
- **#9 Membership row injection** — unguarded path creates a membership → join any org. _Defense: capability-gated + audited; invites single-use, expiring, email-bound._
- **#10 Stale authz in a live token** — revoked membership, token still acts. _Defense: authz resolved per-request from DB, not baked in token._
- **#11 Skeleton-key token** — unified identity → one token valid across all orgs/products. _Defense (a DECISION, not a patch): token carries user only; tenant authority = resolved-host ∩ memberships, fail closed on mismatch._
- **#12 Escalation to `platform` = escalation to God** — #8 landing on the worst value. _Defense: `platform` principal creation is its own out-of-band, heavily-audited path._
- **#13 Policy resolved under wrong tenant** — spoofable host / attacker org-hint → weakest org's password rules; ties to paused tasks #89–92. _Defense: re-evaluate auth policy at privilege-change._

#### Triage — P0 (architectural, design-it-in) vs P1 (discrete control)

_For tenant isolation there is no "nice-to-have" tier — a leak is a leak. P0 = the design is wrong without it; P1 = a specific guard you add._

**P0 (8):** #1 (Ph1) · #2 (Ph1) · #5 (Ph0) · #7 (Ph2) · #8 (Ph2) · #9 (Ph2) · #11 (Ph2) · #12 (Ph2/5)
**P1 (5):** #3 (Ph1/3) · #4 (Ph3) · #6 (Ph1) · #10 (Ph2) · #13 (Ph2)

**Fallout:**
1. **Phase 2 carries 7 of 13 leaks** — highest-risk phase; warrants the most TDD rigor + a dedicated security-review gate before merge.
2. **#11 is a design decision, not a defense** — "what is a token scoped to?" must be answered in Phase 2 design.
3. **Unifying rule:** every request resolves tenant + authz from trusted server-side state, never from client input, and fails closed. Build each phase to that one sentence and most leaks never exist.

#### Cluster 3 — Operational blast radius (the dark side of "single-core")

_A different kind of leak: mostly not instant breach, but amplified consequences — when it goes wrong, it goes wrong for everyone._

- **#14 Bad migration = total outage** — single DB/deploy. _Defense: expand-contract migrations only, prod-mirror staging, fast rollback._
- **#15 Noisy neighbor** — one agency starves compute for all. _Defense: per-tenant rate limits + query-cost budgets; pool fairness._
- **#16 Restore-one rolls back everyone** — _Defense: soft-delete + audit over hard-restore; per-tenant logical export/snapshot tooling._
- **#17 GDPR delete-one misses a table** — purge spans ~70 tables + S3 + index + caches. _Defense: CI-enforced tenant-data manifest + tested purge routine._
- **#18 Backup is the whole breach** — one file = every agency. _Defense: encrypted backups, tight access; per-tenant logical exports._
- **#19 No per-tenant metering = blind** — can't bill, can't spot abuse. _Defense: per-tenant usage metering from day one._
- **#20 Shared Redis cross-tenant** — un-namespaced keys collide. _Defense: tenant-prefixed keys via a cache wrapper, never raw client._
- **#21 Super-admin = single catastrophic point** — one credential = everything. _Defense: mandatory MFA for `platform` principals, short sessions, audit + alerting, break-glass._
- **#22 No per-tenant kill switch** — can't disable one agency's feature without a full deploy. _Defense: per-tenant feature flags._

#### Consolidated triage — operational cluster

_Different axis than Cluster 1–2: build-it-in-early vs discipline vs later-tooling._

**O-A — Hooks to build early (cheap now, painful to retrofit):**
- #15 noisy-neighbor hook → **Phase 1** (the keystone middleware already runs per-request per-tenant)
- #19 per-tenant metering hook → **Phase 1** (same place)
- #20 tenant-prefixed cache wrapper → **Phase 1** (when reality-server goes tenant-aware)
- #17 tenant-data manifest → **Phase 0** (piggyback the RLS-coverage CI work — same table list)
- #22 per-tenant kill switch → **Phase 3** (piggyback `/tenant-config` feature flags)

**O-B — Discipline / practice (process, continuous):**
- #14 expand-contract migrations + staging + rollback → from **Phase 0**, every phase
- #18 encrypted backups + access control → ops setup
- #21 MFA + audit/alerting for `platform` principals → non-negotiable in **Phase 5**

**O-C — Tooling, its own late phase:**
- #16 per-tenant restore/export + #17 purge routine → new **Phase 5.5 — "Tenant Lifecycle & Operability"** (soft-delete-over-hard-delete is a *now* decision; the tooling is later)

**Fallout — the keystone gets even more leverage:** the shared host-resolution middleware is also where the #15 rate-limit hook and #19 metering hook live. It now ties: resolve → RLS context → theme → super-admin context → rate-limit → metering. Build it well; six concerns light up.

**Roadmap addition:** insert **Phase 5.5 — Tenant Lifecycle & Operability** (per-tenant export, restore, GDPR purge routine) so the operability family is not forgotten.

## Idea Organization and Prioritization

### Thematic Organization (7 themes)

1. **The Keystone** — the shared host-resolution middleware. Ties A1 (resolve) → A3 (set RLS context) → A5 (theme) → A6 (super-admin context) → #15 (rate-limit hook) → #19 (metering hook). Highest-leverage component in the system.
2. **Isolation Integrity** — RLS coverage (Foundation #1), reality-server gap (Foundation #2), leaks #1–6 + #20. Unifying rule: resolve tenant + authz server-side, never from client input, fail closed.
3. **Identity Unification** — A2-3 (one `users` table + `principal_kind`), the `portal_users` merge, leaks #7–13. Highest-*risk* theme — 7 of 13 cluster-1/2 leaks live here.
4. **The Publish Boundary** — Foundation #5, invariant I-D, A4 channel model, A3-5 "fourth RLS context." The agency↔global seam.
5. **Hosting & Theming** — A1 Caddy/on-demand TLS, A5-1/A5-4, `/tenant-config` + design tokens.
6. **Super-Admin Control Plane** — A6 + A7, capability model, mandatory audit, unified console, leak #21.
7. **Operability & Blast Radius** — operational cluster #14–#22, logical per-tenant operations, Phase 5.5.

### Breakthrough Concepts

- **Global portal = a fourth RLS context** (A3-5) — collapses what looked like a sync pipeline into a single OR-clause in one policy.
- **The keystone middleware** — one component, six concerns; the single highest-ROI thing to build well.
- **Publish = a channel set** (A4-2) — absorbs subdomain, global portal, *and* existing external integrations into one abstraction.
- **First Principles caught 4 false "shoulds"** before any code — the RLS holes, the api-server-only tenancy, global `portal_users`, the orphaned super-admin column.

### Prioritization Results

The roadmap **is** the prioritization — dependency-ordered, MVP-first:
**Phase 0** (foundation hardening) → **Phase 1** (keystone: tenant resolution) → **Phase 2** (identity unification) → **Phase 3** (agency goes live: hosting + theming) → **Phase 4** (publishing + global portal) → **Phase 5** (super-admin control plane) → **Phase 5.5** (tenant lifecycle & operability) → **Phase 6+** (parked future horizon).

- **Top priority / immediate:** Phase 0 — blocking, invisible, cheap.
- **Highest leverage:** Phase 1's keystone middleware.
- **Highest risk (most rigor + a security-review gate):** Phase 2.

### Action Plan — Phase 0 (Foundation Hardening)

**Why it matters:** every later phase builds on tenant isolation; doing it on Swiss-cheese RLS is building on sand. Also unblocks the tenant-data manifest (leak #17).

**Next steps:**
1. Produce the definitive gap list — every table with `organization_id` but no RLS policy (audit already found: `listings`, `disputes` + ~12 children, ~10 other migrations).
2. Write RLS policies for each gap table following the existing pattern: `is_super_admin() OR organization_id = get_current_org_id()`.
3. Upgrade `check-rls-enforcement.sh`: (a) cover reality-server, not just api-server; (b) actually verify every tenant table has a policy — this script doubles as the tenant-data manifest (#17).
4. Retire `users.is_super_admin` + `is_current_user_super_admin()`; commit to the single JWT-role-driven mechanism.
5. TDD: extend `rls_smoke_tests` to cover every newly-protected table.

**Success indicators:** CI fails if any tenant table lacks a policy · `rls_smoke_tests` green on all gap tables · zero raw-pool query paths in reality-server.

## Session Summary and Insights

**Key Achievements:**
- Verified ground truth via codebase audit — separated 4 false assumptions from real assets before designing.
- Produced one coherent architecture candidate ("Single-Core, Host-Resolved Multitenancy") across 7 fully-decided axes.
- Stress-tested it with 22 leak modes, triaged into P0/P1 + an operational tier, each mapped to a roadmap phase.
- Delivered a dependency-ordered, MVP-first 7-phase roadmap with nothing forgotten — including a parked future-horizon list.

**Session Reflections:**
- First Principles earned its keep immediately — the audit-driven "4 false shoulds" reframed the whole problem (the real work is closing the reality-server tenancy gap, not greenfield design).
- The user drove fast and decisively, consistently asking for and accepting reasoned recommendations — the session converged cleanly rather than sprawling.
- The Reverse Brainstorming pass paid off: leak #11 (token scope) surfaced as a *design decision* that must be made in Phase 2, not a bug to patch later — exactly the kind of thing that's expensive to discover after code exists.
- Cross-product reuse (A7) and the keystone middleware both pointed at the same conclusion: invest in shared crates early, and many axes light up at once.

**Open thread (paused, not lost):** the per-org password/verification work on branch `feature/per-org-auth-policy` (tasks #89–92) maps directly into **Phase 2** — Phase 1 (`AuthPolicy` type + reality-server wiring) is done and compiles; Phase 2 (email-verification toggle) is partially built.

---

## Continuation — 2026-05-16

**Trigger:** Phases 0–5.5 implemented (PR #260 ready). User's question: _"finish missing pieces and gaps in multitenant"_ — enumerate every gap, triage P0/P1/parked, dispatch P0 fixes immediately.

**Scope:** all four lenses — (a) deferred Phase 2–5.5 follow-ups, (b) operational gaps not yet built, (c) cross-cutting integration gaps across reality-web/server + mobile + admin-ui, (d) parked future-horizon items.

**Technique:** Reverse Brainstorming v2 — same lens as the original 22-leak hunt, applied to "claimed completeness" of each phase. Anti-bias enforced by rotating creative domain every cluster (deferral → operational → integration → horizon).

### Phase 5.6 — Reverse Brainstorming v2 (gap hunt)

#### Cluster A — Deferred Phase 2–5.5 follow-ups (named in code/docs)

- **A1 portal_users still alive at write paths.** Phase 2 unification stopped at the read layer — `listing_inquiries.realtor_id_fkey → portal_users` forced the smoke-test helper to dual-write; ~8 write call-sites + 16 FK columns remain. _Identity is bifurcated under the hood — the unification claim is partial._
- **A2 token_scope_tests #[ignore] ×3.** The defining test for leak #11 (token = user only, authority = host ∩ memberships) does not run in CI. Local-only verification is not verification. _Phase 2's keystone invariant is unproven end-to-end._
- **A3 smoke_test_phase0_rls_cross_tenant_isolation skipped.** Legacy `rfqs` FK forced a `--skip`. The test that asserts the entire Phase 0 promise (cross-tenant isolation) has a deliberate hole.
- **A4 caddy_ask prod_mode tests #[ignore].** Env-var race. The ask-endpoint's prod-only "reject unknown host" behavior — the leak #4 defense — is unverified.
- **A5 reality-server endpoint sweep partially finished.** Some routes still acquire raw pool. _Leak #5 is closed where audited, open everywhere else; the CI gate covers the static check but not behaviour._
- **A6 perf_portfolios was the canary, not the only case.** Migration 00146 fixed one pre-Phase-0 policy that crashed on empty UUID GUC. No systematic audit has been done for other policies that predate the `is_super_admin() OR NULLIF(...)` convention.

#### Cluster B — Operational gaps (Phase 5.5 landed as schema/library, not as run-it operability)

- **B1 GDPR purge routine not E2E tested.** Manifest exists; CI checks the manifest stays in sync. But "delete tenant N from postgres + S3 + Redis + audit" has no green integration test. Leak #17 defense is theoretical.
- **B2 Per-tenant restore drill missing.** `tenant-ops::export` and `tenant-ops::restore` exist as code; no scheduled drill, no "we restored a real tenant from a real export in staging" record. _Backup that hasn't been restored is a wish._
- **B3 Backup-restore drill cadence.** Even encrypted backups (#18 defense) untested in restore. No staging-restore runbook in `docs/multitenancy/operability.md`.
- **B4 Per-tenant rate limiter has one tier.** `governor` wired in middleware, default 600 rpm uniformly. No tier table, no per-tenant override, no "starter vs pro vs enterprise" budgets. Hook exists, policy doesn't.
- **B5 Metering pipeline has no consumer.** `meter_request` emits Prometheus counters. Nothing aggregates them per-tenant per-day. Can't bill, can't spot abuse — leak #19 partially closed.
- **B6 MFA enforcement without enrollment flow.** `admin-core::mfa` enforces challenge on capability use; no enrollment flow for new platform principals, no recovery-code generation, no "step-up before sensitive op" UX. Leak #21 defense incomplete.
- **B7 Per-tenant feature flags table has no admin UI.** Migration 00134 created `tenant_feature_flags`; admin-ui has a `feature-flags` page (per summary), but the kill-switch (#22) needs flip-and-invalidate-cache wiring + audit-log entry per flip.
- **B8 Caddy on-demand TLS prod config missing.** Ask-endpoint exists in code; no committed Caddyfile, no cert-storage location decided, no cert-renewal monitoring, no cert-storage backup procedure.
- **B9 Soft-delete grace-period scheduler not wired.** Tables tagged `deleted_at`; no scheduled job that purges (or warns about) records past their grace window. Soft-delete without a sweeper = unbounded soft-delete table growth + unfulfilled GDPR clock.
- **B10 Audit-log retention/rotation missing.** `audit_logs` + `SupportActivityLog` grow unbounded. No archive policy, no time-partition. The compliance lever becomes the cost lever.
- **B11 Alerting on platform-admin actions absent.** Audit row written → no alerting pipeline. An impersonation goes unobserved unless someone reads the table. Leak #21's "audit + alerting" pair is half-done.
- **B12 Break-glass procedure undocumented.** Single super-admin lock-out scenario has no runbook in `docs/multitenancy/operability.md`.
- **B13 Connection-pool RLS context bleed test missing.** Leak #2 defense relied on `Drop` + `clear_request_context()`. No property test that injects panic mid-request and asserts the next checkout has a clear GUC. Defense exists in code; absence-of-bleed is an assertion-free claim.
- **B14 agency_domains cache invalidation on domain release untested.** Leak #3 defense was "TTL + invalidation on release." TTL is in code (`TenantResolutionCache::new(300, ...)`); the invalidation-on-release path needs a test that releasing a domain immediately revokes resolution, not eventually.

#### Cluster C — Cross-cutting integration gaps

- **C1 reality-web `/tenant-config` not consumed.** Phase 3 shipped the endpoint + design tokens; Next.js middleware for Host → tenant → branding injection has no integration test, possibly no implementation in reality-web. _The user-facing payoff of Phase 3 is unrealised on the public portal._
- **C2 reality-web SSR/ISR cache not tenant-keyed.** Explicit "gotcha to manage" from the original session — and the most likely production-breach vector after rollout. `revalidate` keys, edge cache, image cache must all encode the resolved host.
- **C3 Mobile RN (PPT mgmt) tenant context not addressed.** App targets one api-server URL today. With per-agency hosts, does the mobile pick a base URL? Auth-domain selector at first launch? No design exists.
- **C4 Mobile KMP (Reality) — same gap.** White-label readiness undefined; current app = global portal only (per session decision), but per-agency mobile has no path.
- **C5 admin-ui coverage gaps.** Phase 5 console covers agencies/users/feature-flags/audit/platform. `site_settings:write` and `mobile_config:write` capabilities exist with no UI to exercise them. Capabilities without UI = unused capabilities.
- **C6 reality-server admin endpoints — A6 said reality-server owns portal/agency admin; nothing actually mounted.** Whole admin surface lives in api-server today.
- **C7 Cross-server super-admin context propagation untested.** When admin clicks a reality-server-backed action from the unified console, does the super-admin context flow? Shared JWT suggests yes, but no test asserts it.
- **C8 SDK regen for new admin surface.** `@hey-api/openapi-ts` consumes utoipa output. memberships, invites, capabilities, audit, settings, impersonation, MFA-challenge endpoints — all need annotations + regen committed; otherwise the frontend hand-writes types and drifts.
- **C9 WebSocket tenant context.** WS connections may bypass `host_tenant_middleware` (axum WS handlers are in a separate route layer). Every other request enforces; one unenforced path = full breach class for any tenant-scoped pub/sub.
- **C10 Redis pub/sub tenant prefixing.** `TenantedRedis` wraps the GET/SET path; pub/sub channels likely raw. Leak #20 partially closed.
- **C11 admin-ui i18n consumer.** Package is i18n-agnostic via `labels` prop; ppt-web needs to actually pass localized strings. Untranslated admin-ui = English-only super-admin in a `sk/cs/de/en` product.
- **C12 OpenAPI annotations for new admin endpoints.** utoipa coverage check — every new route present in spec? Drives C8.
- **C13 CORS for new admin routes.** If admin-ui ever moves off ppt-web (subdomain split for security), CORS allowlist needs to know.
- **C14 Per-tenant CSP allowlist.** Subdomains may load tenant logos / fonts from per-tenant CDNs; one global CSP either blocks them or whitelists everything.
- **C15 Tenant-branded email sender.** lettre 0.11.22; invites currently `noreply@platform`. Spec implies tenant-branded sender — and DKIM/SPF per tenant domain — is unaddressed.
- **C16 `/tenant-config` cacheability.** No `Cache-Control` strategy. Either every page-load makes a DB hit through middleware, or a stale config persists past a flag flip.
- **C17 OAuth provider principal_kind enforcement.** api-server is the OAuth provider (per CLAUDE.md). Login path needs to enforce `principal_kind` in the token-issuance step, not only at the request middleware.
- **C18 UC-01..UC-32 regression sweep.** Phase 2 changed identity shape (added `principal_kind`, membership rows). No systematic check that the 32 documented UCs still pass. _Likely silent breakage._

#### Cluster D — Parked future-horizon items (with explicit unparking triggers — current gap is that no trigger is defined)

- **D1 A5-2 multi-zone / per-agency builds** — Trigger? "Pilot agency demands code-level UI fork" — capture as a written threshold so the team doesn't drift into ad-hoc forking earlier.
- **D2 A5-5 per-agency component slots** — extension API not designed. Trigger: 3 agencies request the same custom widget.
- **D3 A2-4 Federated IdP for staff principals** — SAML/OIDC adapter not started. Trigger: first enterprise prospect with SSO mandate.
- **D4 A3-3 DB-per-tenant for whales** — promotion path missing. Trigger: signed enterprise contract with data-residency clause.
- **D5 A6-2 Separate super-admin service** — hardening; trigger: first compliance audit asks for blast-radius separation.
- **D6 A4-4 Derived search index** — trigger: global-portal P95 search latency exceeds X (define X).
- **D7 A7-3 Schema-driven admin** — only when ≥10 trivial CRUD resources in the unified console; today we have 5.
- **D8 Mobile per-agency builds (white-label)** — vs in-app agency selection. Decide before C3/C4 is built — they branch here.

#### Triage — three tiers

**P0 (architectural / breach-class — must close before declaring multitenant production-ready):**
A1 · A5 · C1 · C2 · B6 · B13 · B14 · C9 · C17

**P1 (discrete control / pre-launch must-have):**
A2 · A3 · A4 · A6 · B1 · B2 · B7 · B8 · B9 · B11 · B12 · C5 · C8 · C10 · C12

**P2 / Phase 6 backlog (operability + nice-to-have):**
B3 · B4 · B5 · B10 · C3 · C4 · C6 · C7 · C11 · C13 · C14 · C15 · C16 · C18

**Parked (with the gap = no defined unpark trigger):**
D1 · D2 · D3 · D4 · D5 · D6 · D7 · D8

#### Pattern observation

- **Phase 5.5 has the most "schema-but-not-operability" gaps** (B1, B2, B3, B5, B6, B9, B10, B11) — the library landed; the run-it muscle did not.
- **Cross-product surfaces lag the backend** (C1, C3, C4, C6, C11, C18) — Phase 3's branding contract has no consumer; mobile is unaddressed.
- **Verification gaps cluster on the highest-stakes invariants** (A2 = leak #11; A3 = Phase 0 promise; B13 = leak #2; B14 = leak #3; C9 = leak #5/#20). _Code without test ≈ code without commitment._


