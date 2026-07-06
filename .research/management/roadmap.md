# PPT Delivery Roadmap

_Generated: 2026-07-06T03:15:55Z — daily upkeep (Scrum Master + pm-security rotation). Coverage `scan_kind=upkeep`, rotating epic-8a re-checked (idx 12→0). Underlying deep scan: 2026-07-02._

## State of the project

**49 stories classified: 40 done · 9 partial · 0 not-started.** MVP feature-complete; residual work is a small tail of reconciles, one mobile-native slice, and two feature gaps.

| Platform | done | partial |
|----------|-----:|--------:|
| backend  | 31 | 6 |
| frontend | 27 | 6 |
| mobile   | 8 | 4 |

_(stories span multiple platforms, so columns overlap)_

**Screen coverage:** 0 stories without screen-map · 2 orphan epics · 29 orphan screens · 4 missing UC links.

**Top 3 gaps by impact:**
1. **Mobile OS push (FCM/APNs)** deferred — pipeline dispatch is serial and the FCM stub swallows failures (`8a-3`).
2. **iOS Account + Android/KMP inquiry UI** — Account Profile/Settings are stub `Text` views (`82-5`).
3. **Report "create schedule"** — editing/pause/resume shipped but `POST /reports/schedules` still missing (`81-1`).

## Ranked plan

### MVP
- **[high]** Land PR #1797 (OCR auth + rental guest PII manager-gate, 13-day stale draft) — owner: pm-security — why: security-sensitive; closes #1772 (unauthenticated OCR) + #1766 (guest PII exposure).
- **[high]** Fix issue #2107 (fabricated-JWT test bypass in outages happy-path) — owner: pm-security — why: masks real DB-role vs JWT-role drift.
- **[high]** Reconcile + verify **permission-based document access** (`7a-3`) — owner: pm-backend — why: authz-sensitive; sprint-status stale.
- **[high]** Ship **mobile OS push** for notification-preference sync (`8a-3`) — owner: pm-frontend — why: FCM/APNs deferred; pipeline dispatch serial.
- **[medium]** Land the 4 in-flight follow-up drafts (#2115/#2116/#2117/#2089) — owner: pm-backend/pm-qa — why: closes 4 of 5 post-merge review issues in one pass.
- **[medium]** Fix red CI on **folder organization** (`7a-2`) — owner: pm-backend — why: `document_folder_tests` blocks 7a-2 promotion review→done.
- **[medium]** Reconcile **document sharing** (`7a-5`) — owner: pm-backend — why: mobile parity landed (PR #2003), web + sprint-status still lagging.
- **[medium]** Apply the **5-step dispute-filing wizard** redesign (`80-2`) — owner: pm-frontend — why: current single-page form ships MVP but design flag still partial.

### Phase 2
- **[medium]** Implement **create-schedule** for reports (`81-1`) — owner: pm-frontend — why: only `PUT`/pause/resume exist; `POST /api/v1/reports/schedules` missing (screen `apiStatus=partial` pinned on this).
- **[medium]** Add OAuth Provider security tests (PKCE + refresh-rotation + introspection) — owner: pm-security — why: closes `pm-security-oauth-10a-untested-security-contract` risk before epic-10a promotion.

### Phase 4
- **[medium]** Finish **inquiries & account** on mobile-native (`82-5`) — owner: pm-frontend — why: build the Android/KMP Compose inquiry thread (+ calendar); replace stub Account Profile/Settings destinations.
- **[low]** Push stalled churn-hotspot draft #1812 (reality_portal.rs split, ~12d stale) — owner: pm-backend — why: use PR #2100 rental split as template.

### Screen-map drift
- 2 orphan epics · 29 orphan screens · 4 missing UC links. Systemic frontmatter backfill still owed (deep-scan 2026-07-02 finding: 0/~120 screen-maps populate `epics:`).

**Buffer:** 36/36 open · 4 candidates ranked but unqueued.
