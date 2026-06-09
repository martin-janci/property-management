---
id: ppt/admin-onboarding-tours
name: Onboarding Tours (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: platform/onboarding
    component: OnboardingToursPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - get_onboarding_status
  - get_onboarding_tours
  - start_onboarding_tour
  - complete_onboarding_step
  - complete_onboarding_tour
  - skip_onboarding_tour
  - reset_onboarding_tour
relatedScreens:
  - id: ppt/admin-system-announcements
    rel: sibling
  - id: ppt/admin-platform-health
    rel: sibling
sharedComponents:
  - TourOverlay
  - Toast
diagrams: []
useCases: []
epics:
  - Epic-10B-6
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [w] List all onboarding tours for the current admin user with status badges (Not started / In progress / Completed / Skipped)
- [w] Overall onboarding status chip: "Pending onboarding" or "All complete"
- [w] Per-tour card showing name, description, step count, completed count, target roles, and a progress bar
- [w] Start / Resume / View button per tour (label adapts to display status)
- [w] Tour overlay modal: step-by-step progress with step dots, step content, target hint
- [w] Complete step action button (label changes to "Finish tour" on the last step)
- [w] Skip tour action (tour-level, no per-step skip — backend has no skip-step endpoint)
- [w] Reset tour action (restarts progress from the beginning)
- [w] Completed and skipped banners inside the overlay
- [w] Accessibility: focus trap on the overlay, `role="dialog"`, `aria-modal`, `aria-labelledby`
- [w] Overlay z-index 1050 (above Toast z-1000, below DestructiveConfirmDialog z-9999)
- [w] Complete-step → complete-tour mutations sequenced (POST complete-step onSuccess chains POST complete-tour) to avoid race conditions
- [w] Query invalidation on all mutations: tours list + status + individual tour
- [w] Toast notifications on success/error for start, complete, skip, reset
- [w] Loading state while fetching onboarding status
- [w] Error state with Retry button
- [w] Empty state when no tours are available
- [w] Route gated by `site_settings_write` capability
- [w] Navigation item in PLATFORM sidebar group

## States

- **Loading**: "Loading…" placeholder while fetching `/api/v1/onboarding/status`
- **Error**: Error banner with message and Retry button
- **Empty**: Icon + "No tours configured" message
- **Tours grid**: Auto-fill grid of tour cards (minmax 360px)
- **Overlay open**: Fixed backdrop with tour panel, step list, footer actions
- **Overlay — in progress**: Active step highlighted; previous steps checked; future steps dimmed
- **Overlay — completed**: Green completed banner; Restart Tour + Close buttons in footer
- **Overlay — skipped**: Amber skipped banner; Restart Tour + Close buttons in footer
- **Mutating**: Action buttons disabled with reduced opacity while any mutation is pending

## Notes

### Broader context

Admin view for the user onboarding tour system (Epic 10B, Story 10B.6). Platform admins can preview and manage their own tour progress state against the same `/api/v1/onboarding/*` endpoints that end-users see. This allows admins to verify tour content and progression before rolling out to users.

### Specific (recent)

- 2026-06-08 — agent: created screen doc; component (OnboardingToursPage + TourOverlay + api-client hooks) already shipped on dev in PR #840. Route `platform/onboarding` registered in App.tsx with `site_settings_write` cap gate. Nav link in AdminLayout PLATFORM group.

## Agent Log

<!-- newest entries on top -->
- 2026-06-08 — agent: initial screen doc added for feat-admin-web-onboarding-tour-component; implementation was already merged (PR #840 fix + PR #730-era feat commits). Backend routes confirmed at `backend/servers/api-server/src/routes/onboarding.rs` (Epic 10B, Story 10B.6).
