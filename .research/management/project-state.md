# Project State — PPT (Property Management)

<sub>Generated: 2026-11-22T03:06:30Z — Scrum Master synthesis + pm-frontend (rotation index 2 → 3). Coverage cursor advanced idx 8 → 9 (epic-83 re-checked, next epic-84).</sub>

## Executive summary

**26 PRs merged in-window (#3687–#3712), 12 of them frontend/mobile.** The 2026-10-28 carried ppt-web logout cache-leak landed as **#3706** (25-day-stale item closed). A WS token-in-URL leak (**#3696**), an invoice empty-items guard (**#3697**) + wire-format fix (**#3703**), facility availability loading (**#3707**), Create Lease pickers empty states (**#3700**), reality-web /report attachments via evidence links (**#3695**), and admin-web console-contract alignment (**#3710**) all shipped. Mobile-rn i18n continued burning down with **#3699** (WidgetSettings) and **#3705** (list/search placeholders). Backend tail: accounting MVP trio #2558 closed via **#3691**, 201 release-248 tests un-quarantined via **#3692**, reality-server SSO timeout guaranteed via **#3701**. Sprint spine (6/6) remains `done`; hardening track active. 5 drafts open; 1 new follow-up issue (#3711).

## Sprint progress

- **Sprint:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Epics done:** 6 of 6 (100%)
- **Coverage (upkeep this run):** 49/49 stories done; rotating cursor re-checked **epic-83** (3/3 stories done; `last_checked` advanced).

## Shipped since last run (2026-11-21T03:06:30Z → 2026-11-22T03:06:30Z)

- #3687 — pm-integration slug client propagation (closes 2026-11-21 roadmap item)
- #3688 — pm-integration compliance 400 spec (RFC-6749-style error envelope)
- #3689 — pm-integration voice OAuth RFC-6749 shape (closes action item)
- #3690 — pm-scrum-master cross-tenant 404 sweep (violations/moderation/aml_dsa)
- #3691 — pm-backend accounting MVP trio #2558 (closes 110-day stalled slice)
- #3692 — pm-backend un-quarantine 201 release-248 tests
- #3693 — pm-backend regression coverage for reality-server #3669 surface
- #3694 — fix RAG embedding BIT-352
- #3695 — reality-web /report attachments via evidence links
- #3696 — ppt-web WS notifications token-in-URL leak stripped
- #3697 — ppt-web AccountingInvoiceForm empty-items guard
- #3698 — api-handlers template-import workflow-enable silent fail surfaced
- #3699 — mobile-rn WidgetSettings i18n (object-form t(key,{defaultValue}))
- #3700 — ppt-web Create Lease pickers empty/error states (retry1)
- Plus 12 more PRs in-window (#3687-#3712) — the full list is in `roles/pm-scrum-master.md`.

## What's next (top 5)

1. **[high · pm-scrum-master]** Land the 2026-12-01 i18n warn→error lint-flip burn-down: merge or close the 10+ open `code-review-*-i18n-retry1` items + drafts #3676/#3677 within 9 days · dep: pm-frontend
2. **[high · pm-scrum-master]** Resolve dispatcher stall #3357 — 3 approved PRs still blocked on non-mechanical conflicts across TWO merge waves (32 + 26 PRs) · dep: pm-tech-lead
3. **[high · pm-scrum-master]** Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable · dep: pm-devops
4. **[high · pm-frontend]** Burn down the frontend-i18n slice (10+ `code-review-*-i18n-retry1` items + drafts #3676/#3677) before 2026-12-01 CI gate flip
5. **[high · pm-frontend]** Promote drafts #3714 (ppt-web date/number locale formatters) and #3715 (ppt-web native confirm dialogs) to ready-for-review within 48h — both are from the 2026-11-19/20 action-list and are the only non-i18n frontend drafts in flight

## Blockers

- **dispatcher #3357** — 3 approved PRs blocked on non-mechanical conflicts; two merge waves failed to drain. Owner: `pm-tech-lead`.
- **mobile-native KMP backlog (4 items)** — cloud-runner KMP + swagger-ui egress blocked (#2652, #2966). Owner: `pm-devops`.
- **5 open drafts** — promote-or-close policy from 2026-11-21 not yet enforced. Owner: `pm-scrum-master`.

## Role focus today

`pm-scrum-master` (always-on), `pm-frontend` (rotation index 2 → 3; was 25 days stale, now caught up).

### pm-frontend — one-line

12/26 merged PRs were frontend/mobile — the 2026-10-28 carried logout cache-leak landed via #3706; two accounting UX fixes + two i18n PRs + three empty-state wires. 2026-12-01 mobile i18n lint flip is 9 days out with 10+ open retry1 items.

### pm-scrum-master — one-line

26 PRs merged, three 2026-11-21 action-list items closed via #3687/#3689/#3704, 5 drafts still open. Pipeline-side the constraint remains #3357 + cloud-runner egress — unchanged across two merge waves.
