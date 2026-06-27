# Backend Endpoint Checklist — api-server + reality-server

> **Audit:** BIT-247 (parent BIT-245). Owner: CTO. Generated 2026-06-27.
> **Method:** read-only fan-out classification of every route file, cross-referenced against the `tests/*.rs` suites. See [`_RUBRIC.md`](./_RUBRIC.md) for the status taxonomy and path-derivation method. Reproducible: re-run the per-group classification against `backend/servers/*/src/routes/**`.

## Headline

**12.9% of backend endpoints are DONE (implemented *and* test-verified).**

| Status | Count | Share | Meaning |
|--------|------:|------:|---------|
| ✅ `done` | **262** | **12.9%** | Implemented **and** exercised by a passing test |
| 🟡 `partial` | **1654** | 81.4% | Implemented (real DB/logic) but no behaviour-asserting test |
| 🔧 `stub` | **117** | 5.8% | Placeholder handler (501 / mock data) or unmounted ROADMAP router |
| ❌ `missing` | **0** | 0.0% | No handler for a spec/use-case endpoint *(code-first scope — see caveat)* |
| **Total** | **2033** | 100% | Distinct method+path across both servers |

The headline finding: **functionality is broad and largely implemented — the dominant gap is test coverage, not missing features.** 81% of endpoints work but are unproven by tests, so they cannot be counted `done` under the board's "verified by tests" bar.

## Per-group summary

| # | Group | File | Total | ✅ done | 🟡 partial | 🔧 stub | % DONE |
|---|-------|------|------:|------:|----------:|--------:|-------:|
| 01 | Maintenance & Operations | [01-maintenance-ops.md](./01-maintenance-ops.md) | 282 | 52 | 230 | 0 | 18.4% |
| 02 | Governance & Community | [02-governance.md](./02-governance.md) | 172 | 30 | 142 | 0 | 17.4% |
| 03 | Announcements, Messaging & Notifications | [03-announcements-notifications.md](./03-announcements-notifications.md) | 98 | 31 | 67 | 0 | 31.6% |
| 04 | Documents, Forms & Signatures | [04-documents-forms-signatures.md](./04-documents-forms-signatures.md) | 63 | 19 | 42 | 2 | 30.2% |
| 05 | Auth, OAuth & MFA | [05-auth-oauth-mfa.md](./05-auth-oauth-mfa.md) | 36 | 19 | 17 | 0 | 52.8% |
| 06 | Compliance, Legal, AML/DSA & Insurance | [06-compliance-legal-aml.md](./06-compliance-legal-aml.md) | 143 | 12 | 98 | 33 | 8.4% |
| 07 | Organizations & Admin | [07-organizations-admin.md](./07-organizations-admin.md) | 74 | 9 | 65 | 0 | 12.2% |
| 08 | Platform Admin, Features, Subscriptions & Tenant Config | [08-platform-admin-features.md](./08-platform-admin-features.md) | 90 | 1 | 89 | 0 | 1.1% |
| 09 | AI & Voice | [09-ai-voice.md](./09-ai-voice.md) | 86 | 24 | 60 | 2 | 27.9% |
| 10 | Integrations & API Ecosystem | [10-integrations.md](./10-integrations.md) | 241 | 16 | 159 | 66 | 6.6% |
| 11 | Financial, Accounting & Reporting | [11-financial-accounting.md](./11-financial-accounting.md) | 227 | 21 | 206 | 0 | 9.3% |
| 12 | Leasing, Listings, Vendors & Buildings | [12-leasing-buildings.md](./12-leasing-buildings.md) | 318 | 19 | 285 | 14 | 6.0% |
| 13 | Portals, Analytics & Health | [13-portals-analytics.md](./13-portals-analytics.md) | 112 | 4 | 108 | 0 | 3.6% |
| 14 | Reality Server (public listings + SSO) | [14-reality-server.md](./14-reality-server.md) | 91 | 5 | 86 | 0 | 5.5% |
| | **Total** | | **2033** | **262** | **1654** | **117** | **12.9%** |

> api-server alone: 1942 endpoints, 257 done (13.2%). reality-server: 91 endpoints, 5 done (5.5%).
> Best-covered surface is **Auth/OAuth/MFA (52.8%)** — security-critical paths are the most-tested. Worst is **Platform Admin (1.1%)**.

## Gaps & stubs (117 stub endpoints)

| Cluster | Count | Kind | Owning area / epic |
|---------|------:|------|--------------------|
| `public_api.rs` → `/api/v1/developer/*` | 43 | **Unmounted** ROADMAP (PAP-24) — router not nested in `lib.rs` | Public Developer API (roadmap) |
| `migration.rs` → `/api/v1/migration/*` | 23 | **Mounted but mock** — handlers return hardcoded data, `State(_)` unused, "in a real implementation" comments | Tenant migration / onboarding |
| `regional_compliance.rs` | 19 | **Mounted but placeholder** — handlers ignore `AppState`, return hardcoded jurisdiction/SK/CZ config | Regional compliance epic |
| `data_residency.rs` → `/api/v1/data-residency/*` | 14 | **Mounted but placeholder** — hardcoded config/regions/audit responses | Data residency epic |
| `vendor_portal.rs` → `/api/v1/vendor-portal/*` | 14 | **Unmounted** ROADMAP, handlers return 501 (`vendors.rs` is the live surface) | Vendor portal (roadmap) |
| `ai/ocr.rs` (`process_meter_reading`, `submit_correction`) | 2 | **501 / accept-and-discard** | AI OCR |
| `signatures.rs` `document_signature_router` (`list`/`create` doc-scoped) | 2 | **Defined but never mounted** in `lib.rs` | E-signature |

Of the 117 stubs: **59 are unmounted dead/roadmap routers** (43 developer-API + 14 vendor-portal + 2 doc-signature) and **58 are mounted placeholders returning fake data** (23 migration + 19 regional-compliance + 14 data-residency + 2 OCR). The mounted placeholders are the higher risk — they are reachable and return plausible-but-fake responses.

### `missing` caveat (important)

This audit is **code-first**: it enumerates every handler that exists and classifies it. It does **not** exhaustively diff the 508-use-case catalog / OpenAPI spec to find spec'd endpoints with *no* handler, so `missing = 0` means "no missing handler was found within the routed surface", **not** "the spec is 100% covered". A spec→handler diff is a recommended follow-up (see roadmap Phase 4) before treating `missing = 0` as authoritative.

## Current vs future state — roadmap to 100% DONE

**Current state:** the platform is feature-broad (2033 endpoints, ~94% implemented) but test-thin (only 12.9% test-verified). The path to a higher DONE% is overwhelmingly a **testing** effort, with a smaller stub-completion effort.

The remaining work, grouped:

- **Phase 1 — Eliminate mounted placeholder stubs (58).** Finish `migration` (23), `regional_compliance` (19), `data_residency` (14), `ai/ocr` (2). These return fake data today and are a correctness/compliance risk if surfaced in the UI. Highest priority because they are *reachable*.
- **Phase 2 — Decide the 59 unmounted ROADMAP endpoints.** Either implement+mount or delete the dead `public_api` (43), `vendor_portal` (14), and doc-scoped signature (2) routers. A product call (CMO/Atlas) on whether the Developer API and Vendor Portal are on the near roadmap.
- **Phase 3 — Test backfill for high-risk partials.** Prioritise the 1654 partials by blast radius: **financial/accounting (206), compliance (98), platform-admin/admin (154), organizations (65)** first — money, compliance, and privileged surfaces. Each test moves an endpoint `partial → done`; this is the single biggest lever on the headline %.
- **Phase 4 — Test backfill for remaining partials + spec diff.** Cover the long tail (maintenance, governance, leasing, reality-server) and run the spec/use-case→handler diff to confirm `missing = 0`.

**Indicative DONE% trajectory** (test-backfill driven): clearing Phase 1+2 stubs lifts the "live" denominator; Phase 3 on the ~520 high-risk partials would roughly triple DONE to ~38%; full Phase 3+4 backfill is the only route to >90%.

## How this was produced

14 read-only classification passes (one per domain group), each enumerating every `.route(...)` registration, resolving the full path from the `lib.rs`/`main.rs` mount prefixes, opening suspected stub handlers to confirm, and grepping the `tests/*.rs` suites for path/handler coverage. Per-group tallies were summed into the totals above. Integration tests are matched only in top-level `tests/*.rs` (the `tests/integration/` dir is dead/uncompiled).
