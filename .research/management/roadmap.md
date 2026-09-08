# PPT Roadmap — upkeep 2026-09-08

⚠ Buffer below half — consider running `/ppt-project-management scan` to refresh coverage (16/36 open; but 47/49 stories already `done` so the queue is genuinely draining, not stale). Note: 4 of the 16 open items are gated on `pm-devops-vendor-swagger-ui-unblock-api-server-cloud-build`, so effective claimable is ~12 until #2949 lands.

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan; the auto-review loop closes follow-ups faster than they arrive on landable stacks, but three cross-tenant IDOR bugs opened 2026-09-06 cannot land because api-server is cloud-verify-blocked (#2949).
- Delta vs 2026-08-31 upkeep: **14 PRs merged**, 5 with real security wins on the frontend/reality-server surface (typed error, raw-DB leak fix, single-flight refresh + 401 replay, cold-boot JWT-exp, allowlist auto-derivation, shared-device cache purge). Zero regressions detected.
- Remaining gaps (unchanged from prior windows):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete).
- **New this window:** 3 open IDOR issues on the repo (#2944/#2945/#2946); 2 new infra blockers (#2949 api-server cloud build, #2951 mobile jest infra).
- Screen coverage: 0 orphan screens · 0 validation errors · **3 missing UC links** (UC-33.1/33.2/33.3 all still queued). No change from prior window.
- Merged-PR keyword sweep: no coverage story flipped status — the 14 merged PRs were all follow-up hardening (compliance leak fix, invoice PDF tests, ppt-web auth path, i18n, cache purge, allowlist auto-discovery, typed saved-search error).

## Ranked plan

### mvp / security — top priority (score 9-10)

- [high] Land infra #2949 (vendor swagger-ui offline) so `cargo build -p api-server` succeeds in the cloud sandbox — every other high-priority item is downstream — owner: pm-devops — why: unblocks the IDOR trio and every future api-server security patch
- [high] Fix IDOR #2946 — apply computed `org_id` as scope filter in portfolio_analytics `upsert_property_metrics` + `get_property_metrics` (routes/portfolio_analytics.rs:282,314) + sqlx cross-tenant regression test — owner: pm-security — why: confirmed by direct code read; public issue; cross-tenant data disclosure
- [high] Fix IDOR #2945 — verify org ownership of target property/`building_id` in portfolio_properties handlers + sqlx test — owner: pm-security — why: public issue; cross-tenant read+write
- [high] Fix IDOR #2944 — org-scope violations comments/evidence/payments reads + role-gate internal-notes visibility + tests — owner: pm-security — why: public issue; largest surface of the trio
- [high] Sweep `_org_id` / `_tenant_id` compute-then-discard antipattern (known: portfolio_analytics.rs:282,314; migration.rs:1472; faults.rs:642) + `just verify` grep gate — owner: pm-security — why: systemic footgun; clippy misses `_`-prefixed names

### mvp / finish-what's-started (score 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url (84-1 partial) — api-client binding + UploadDocument integration + regression test — owner: pm-frontend — why: aging 5+ windows; backend shipped
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API (84-2 partial); flip screen-map ppt/document-sign buildStatus planned→shipped — owner: pm-frontend — why: closes 49/49 delivery when paired with #6

### security / carried (score 7)

- [high] Resolve cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) — every backend PR ships against a waivered advisory — owner: pm-security — why: standing since 2026-08-18; upstream watch

### infra / cloud-verify unblock (score 6-7)

- [high] Unblock mobile-native/KMP cloud builds (issue #2652) — 6 of 9 pre-run action-list items structurally unclaimable — owner: pm-devops — why: chronic buffer starvation
- [medium] Resolve infra #2951 (jest-expo/RN version rot) so mobile jest suite loads in cloud; PR #2950 merged red-CI on this — owner: pm-devops — why: future mobile-rn security patches will land red-CI otherwise

### post-merge follow-ups from this window (score 5-6)

- [medium] Device-handoff regression test companion to PR #2950 — asserts `TENANT_SCOPED_EXACT_KEYS` covers each new tenant-scoped namespace on session change — owner: pm-qa — why: reviewer-memory gap otherwise
- [medium] Add authz/rotation test for JWT `exp` cold-boot path in AuthContext (PR #2941) — near-expiry-inside-skew case + opaque-token trust case are worth pinning — owner: pm-qa — why: subtle auth-boot behaviour

### quality / lint prevention (score 5, carried)

- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings ESLint config in frontend/apps/mobile — owner: pm-frontend — why: 3 mobile-rn PRs earlier this cycle fixed defects a lint would catch

### bug / mobile-native-kmp reliability (score 3-4, carried; cloud-unclaimable until #2652 lands)

- [medium] mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — owner: pm-backend
- [low] mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — owner: pm-backend
- [low] getPortfolioAnalytics() truncates realtor portfolio at 100 listings — owner: pm-backend
- [low] getPortfolioAnalytics() unbounded fan-out (retry 1/2) — owner: pm-backend
- [low] Shared repositories swallow CancellationException in catch(e: Exception) — owner: pm-backend
- [low] SsoService has zero direct tests — owner: pm-qa
- [low] KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — owner: pm-backend

### Screen-map drift (score 2-3, carried)

- [low] screen-map-drift: PR #2894 touched reality-web routes without updating docs/screens — owner: pm-qa

Buffer: **16/36 open** · effective claimable ~12 (4 items gated on `pm-devops-vendor-swagger-ui-unblock-api-server-cloud-build`) · 6 items added this run (IDOR trio + swagger-ui unblock + antipattern sweep + jest infra + handoff test + h2 RUSTSEC re-queue). Project at 47/49 delivery; the near-term lever is unblocking api-server cloud builds so the IDOR trio can ship.
