# Project state — PPT delivery

_Generated 2026-08-28T02:35Z by Phase 1.6 (pm-scrum-master synthesis, pm-devops rotating role)._

## Executive summary

Compliance hardening remained the dominant workstream since 2026-08-25 08:50Z, with 11 human-authored merges (#2848–#2856, #2858, #2860, #2863, #2864) refactoring ContentModerationPage/AmlDashboardPage/DsaReportsPage and one security fix (#2858, cap-grant email-verify fail-open). Three dependabot bumps (#2865, #2866, #2867) also landed. All 6 open PRs are dependabot bumps from 2026-08-27 with no human review pending, and 6 closed issues this window were labeled follow-up / from-merged-review / security / backend with no untriaged intake. The buffer signal is the story of the day: `claimable=6/72`, with all 6 remaining backlog items in `mobile-native/KMP` and therefore unlandable in the cloud runner — the dispatcher is starving on infra, not on ideas. Today's rotating role slot is `pm-devops` (last-run 2026-06-16, 2+ months overdue), and their recommendation drives the "what's next" list below.

## Sprint progress

**Sprint:** compliance-hardening-q3 — **epics done:** 47/49 (unchanged — the 2 MVP holdouts remain `84-1` direct-to-S3 upload wiring and `84-2` document-sign page, both frontend-only on shipped APIs).

## Shipped since last run

- #2848 — ContentModerationPage: extract sections + shared data hook
- #2849 — AML decision dialogs on AmlDashboardPage
- #2850 — locale updates (i18n)
- #2851 — locale updates (i18n)
- #2852 — locale updates (i18n)
- #2853 — compliance hardening (window)
- #2854 — compliance hardening (window)
- #2855 — DSA reports: i18n toast on submit
- #2856 — moderation overdue: server-side filter
- #2858 — **security:** cap-grant email-verify fail-open fix
- #2860 — DB test hardening
- #2863 — moderation overdue: truncation UX
- #2864 — compliance hardening (window)
- #2865 / #2866 / #2867 — dependabot bumps

## What's next (top 5)

1. **(pm-devops, high)** Add a self-hosted Kotlin/Gradle/Android-SDK runner to the dispatcher's runner pool (label `kmp-cloud`) so mobile-native/KMP plans can be claimed in cloud mode — unblocks the 6 stuck backlog items and prevents future KMP-only buffer starvation.
2. **(pm-devops, medium)** Add a `runner_requires` field to plan frontmatter so the dispatcher can pre-filter unclaimable-in-cloud plans (`runner_requires: kmp-cloud`) instead of exhausting the queue and going quiet.
3. **(pm-devops, medium)** Split the mobile-native/KMP backlog into a local-only lane surfaced via `/next-plan --local` so the user can consume it manually from a workstation with a KMP toolchain.
4. **(pm-frontend)** Land the two long-aging partial MVP stories (`84-1` direct-to-S3 upload wiring, `84-2` signer-facing document-sign page) — both frontend-only on shipped APIs, would move MVP to 49/49. (Carried from 2026-08-25 pm-scrum-master risk.)
5. **(pm-backend)** `mobile-native-kmp InquiriesResponse` required `page_size` mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call (score=3, highest-priority KMP bug in the backlog).

## Blockers

- **Buffer low:** `claimable=6/72` — all remaining backlog is `mobile-native/KMP`, unlandable in the cloud runner (no Gradle/JDK/Android SDK). Owner: **pm-devops**.
- **84-1 / 84-2 aging:** 3 upkeep windows now (2026-07-30 / 08-06 / 08-25) without dispatcher progress on the last 2 MVP partial stories. Owner: **pm-frontend**.
- **Accounting MVP-loop trio (#2555 / #2558 / #2559):** carried risk of reviewer-slot starvation. Owner: **pm-tech-lead**.

## Role focus today

**pm-devops** — 2+ months overdue on rotation; today's buffer-low signal is a DevOps concern (cloud CI can't build KMP; the local-only lane can).

## Per-role summary

- **pm-devops (today):** Cloud runner cannot build `mobile-native/` (KMP: Kotlin 2.3.21 + AGP 8.7.3 + KSP 2.1.0 + Android SDK 34), so all 6 remaining backlog items are stuck. Recommended: provision a self-hosted `kmp-cloud` runner, add a `runner_requires` field so the dispatcher can pre-filter, and either split the KMP queue into a local-only lane or formally defer to an infra sprint. Two decisions surfaced today (see `decisions.md`).
