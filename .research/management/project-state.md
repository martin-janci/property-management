# Project State — 2026-06-24

## Executive summary

Sprint (Epic 6, 7A, 8A, 10A) is significantly behind its YAML label — `coverage.json` (generated 2026-06-23) shows most Epic 6 and 7A stories as **partial** or **done** in code while `sprint-status.yaml` still reads `ready-for-dev` or `review`. The status file has not been reconciled with merged work for multiple cycles. Outside the active sprint, 95 PRs merged in 8 days delivered Epic 11 financial scheduler/reports, Epic 14 IoT (BIT-145 WebSocket + alerts), Epic 13 Sentiment/Predictive dashboards, Epic 18 ID-doc OCR, messaging N-party / camelCase wire flip, a new ACC (Invoicing/Accounting) epic scaffold (PR #1821, :8082), and a large "split route monolith" churn-hotspot wave. Cloud cron ran 8 days late — `state.json` `last_run_iso` was 2026-06-16; today is 2026-06-24.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics done: **1 / 6** (per sprint-status.yaml; coverage.json suggests reconciliation would shift this)

## Shipped since last run (2026-06-16 → 2026-06-24)

- PR #1822 — story 79-2-authentication-flow: reality-web SSO `/auth/callback` e2e integration tests (8 cases). Closes the gap-79-2-auth-callback-e2e.
- PR #1821 — Epic ACC (Invoicing & Accounting MVP) initial scaffold + standalone accounting-server (:8082).
- Churn-hotspot route/repo split wave: PRs #1796 (subscription.rs repo), #1798 (emergency.rs route), #1800 (enhanced_tenant_screening.rs route), #1810 (sensor.rs repo), #1813 (vendors.rs route), #1815 (iot.rs route), #1816 (reserve_funds.rs route). PRs #1812 (reality_portal.rs) and #1814 (form.rs) remain blocked — see *Blockers*.
- Story 11.6 + 11.7 — financial scheduler + reports/PDF/xlsx export.
- Epic 14 IoT — BIT-145 WebSocket channel + alerts page.
- Epic 13 — Sentiment & Predictive Maintenance dashboards.
- Epic 18 Story 18.2 — Guest ID-doc OCR seam.
- Messaging — camelCase wire flip + N-party plumbing (#1802, #1768, #1756, #1689, #1696, #1702).
- BIT-185 fault notifications (#1705); follow-up gap (`#1793`) filed for two missing transitions (`triage_fault`, `confirm_fault`).
- PR #1713 — revert #1690 delegations frontend (BIT-213 retirement reconcile, intentional).

## What's next (top 5 actions)

1. **High** — Merge or close PR #1797 (OCR endpoints unauthenticated + manager-gate on persistent rental/guest PII reads) before any rental-flow story ships to prod. *owner: pm-security / rust-backend*
2. **High** — Resolve issue #481 (OAuth refresh-token revocation bypass; `revoked_at IS NULL` removed from token-repo query). Gates stories 10A-1, 10A-3. *owner: pm-security / rust-backend*
3. **High** — Resolve issue #480 (WebSocket auth token in query-param logged + session not re-validated after JWT expiry). Gates story 8A-3. *owner: pm-security / rust-backend*
4. **High** — Merge PRs #1799 (msg attachment IDOR — bind file_key to thread + MIME validation) and #1823 (rental guest ID-doc PII audit + content-sniff + manager-gate parity). *owner: rust-backend / pm-security*
5. **High** — Reconcile sprint-status.yaml with coverage.json: promote 7A-1/-2/-3, 6-5 (verify done), 6-2/6-3/6-4 (partial). Stale labels for ≥1 cycle across most Epic 7A and half of Epic 6. *owner: pm-backend + pm-frontend sign-off per story*

## Blockers

- **PR #1812 (reality_portal.rs churn-split)** — CI failing exit 126; check-rls-enforcement.sh lost exec bit (100644 vs 100755 on dev). MCP push_files cannot restore exec bit; manual `chmod +x` + push required. *owner: pm-tech-lead*
- **PR #1814 (form.rs churn-split)** — Real modify/delete conflict against dev post-#1781; needs manual rebase of the form/ split onto the current form.rs head. *owner: pm-tech-lead*
- **Stories 10A-1, 10A-3, 8A-3** — Blocked from done by test-hardening-batch items #480 (WS JWT log) and #481 (revoked-token reuse). Open since 2026-05-25 (30 days) with no fix or explicit defer. *owner: pm-security*

## Role focus today

- **pm-scrum-master** — delivery synthesis (always-on) + reconciliation surface.
- **pm-security** — rotation slot (`pm_cursor.next_index=5`). Backed up at 30 days since previous run on 2026-05-27.

## Per-role summaries

- **pm-scrum-master** — 95 PRs in 8d; sprint-status drift wide; 2 churn-split PRs stuck (#1812/#1814); 31 untriaged issues from post-merge follow-ups; ACC epic scaffold landed without sprint planning (PR #1821); cron 8d late.
- **pm-security** — 8 open test-hardening items (#480–#487); 7 in-flight security-flavored draft PRs (#1797/#1799/#1823/#1824/#1825/#1806/#1795); residual cross-tenant IDOR `security-llm-doc-idor` (status: ready) still has no PR.
