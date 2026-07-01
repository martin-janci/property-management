---
id: ppt/onboarding-tour
name: User Onboarding Tour
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    component: TourPage
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
  mobile:
    screen: HelpCenterScreen
    component: TourOverlay
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - get_onboarding_status
  - get_onboarding_tours
  - start_onboarding_tour
  - complete_onboarding_step
  - complete_onboarding_tour
  - skip_onboarding_tour
  - reset_onboarding_tour
relatedScreens:
  - id: ppt/admin-onboarding-tours
    rel: sibling
  - id: ppt/admin-contextual-help
    rel: sibling
sharedComponents:
  - TourOverlay
  - WelcomeModal
diagrams: []
useCases:
  - UC-42.1
epics:
  - Epic-10B-6
designSources: []
owner: pm-frontend
---

## Functionality Checklist

Tag each item with the platforms it applies to: `[w]`, `[m]`, `[w,m]`, or `[-]`.

- [w,m] Welcome modal greets a new user and offers the available tours (`WelcomeModal` on web; first-run prompt on mobile)
- [w,m] Onboarding status overview: list every tour with status badge (Not started / In progress / Completed / Skipped) and overall "Pending onboarding / All complete" chip
- [w,m] Per-tour card: name, description, step count, completed count, target roles, progress bar
- [w,m] Interactive step-by-step tour walkthrough with progress indicator and target hint per step
- [w,m] Start / Resume / View action per tour (label adapts to display status)
- [w,m] Complete-step action (label becomes "Finish tour" on the last step)
- [w,m] Skip-tour action (tour-level only — backend has no skip-step endpoint)
- [w,m] Reset-tour action (restarts progress from the beginning)
- [w,m] Completed and skipped banners inside the tour view/overlay
- [w] `WelcomeModal`, `TourCard`, `TourStep`, `OnboardingProgress` components + `TourPage` / `OnboardingStatusPage` pages under `features/onboarding/`
- [w] Help-center adjacency: `HelpCenterPage`, `VideoTutorialsPage`, `FAQPage`, `SupportPage`, `FeedbackPage`
- [m] `TourManager` singleton drives tour state, persisted via AsyncStorage; default tours: main-onboarding, fault-reporting, voting
- [m] `TourOverlay` highlights the targeted UI element; `ContextualHelp` shows context-aware help
- [m] `HelpCenterScreen` + `FeedbackScreen` consume `helpCenter` / `feedbackManager` from the onboarding module
- [-] Route wiring: end-user pages/screens are NOT yet registered in ppt-web `AppRoutes` or the mobile navigator (see Notes); only the admin variant (`ppt/admin-onboarding-tours`) is routed today

## States

- **Loading**: spinner / "Loading…" while fetching `/api/v1/onboarding/status`
- **Error**: error banner with Retry
- **Empty**: "No tours available" placeholder
- **Tours list**: filterable by status (all / not_started / in_progress / completed / skipped)
- **Tour — in progress**: active step highlighted; previous steps checked; future steps dimmed
- **Tour — completed**: completed banner with Restart / Close
- **Tour — skipped**: skipped banner with Restart / Close
- **Mutating**: action buttons disabled while a start/complete/skip/reset mutation is pending

## Notes

### Broader context

End-user side of the onboarding tour system (Epic 10B, Story 10B.6 / UC-42.1). Backed
by the same `/api/v1/onboarding/*` endpoints as the admin view (`ppt/admin-onboarding-tours`).
The shared web client lives in `@ppt/api-client` `onboarding/` (api + TanStack Query hooks
`useOnboardingStatus`, `useOnboardingTours`, `useStartOnboardingTour`, …). Web UI is in
`frontend/apps/ppt-web/src/features/onboarding/`; mobile UI is in
`frontend/apps/mobile/src/onboarding/` + `src/components/onboarding/` + `src/screens/help/`.

### Specific (recent)

- 2026-06-28 — agent: screen doc created to close the "no screen-map (orphan epic)" coverage
  gap for the *end-user* onboarding tour. The admin management view already had a doc
  (`ppt/admin-onboarding-tours`); the end-user experience did not. buildStatus = `in-progress`
  (not `shipped`): the components/pages and the mobile managers/screens exist and are
  type-complete, but the end-user pages are not yet wired into `ppt-web` `AppRoutes` nor the
  mobile navigator — they are presentational (props-driven) and barrel-exported only. apiStatus
  = `partial`: api-client hooks exist and are wired for the admin view, but the end-user
  containers that call them are not routed. Follow-up task: wire `TourPage`/`OnboardingStatusPage`
  into a route + container (and register the mobile help/feedback screens in the navigator).

## Agent Log

<!-- newest entries on top -->
- 2026-06-28 — agent: initial screen doc added for the end-user onboarding tour (Epic 10B, Story 10B.6 / UC-42.1) to resolve the orphan-epic / no-screen-map coverage gap. Endpoints, hooks, and components confirmed present on dev; route wiring confirmed absent (no `features/onboarding` reference in `ppt-web/src/routes/`). Backend routes at `backend/servers/api-server/src/routes/onboarding.rs`.
