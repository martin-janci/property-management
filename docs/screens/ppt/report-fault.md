---
id: ppt/report-fault
name: Report Fault
product: ppt
sitemapRefs:
  mobile: mobile-report-fault
implementations:
  ppt-web:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
  mobile:
    component: ReportFaultScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
endpoints:
  - faults_create
relatedScreens:
  - id: ppt/dashboard
    rel: parent
  - id: ppt/faults-list
    rel: sibling
sharedComponents:
  - radio-cards
  - chip-group
  - text-input
  - file-upload
  - segmented-control
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobReportFaultScreen
useCases:
  - UC-03
epics:
  - Epic-4
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Modal-style header
- [ ] [m] iOS-modal pattern: surface bg with bottom border, 14×20 padding
- [ ] [m] Left "Cancel" link (accent-500, 15/500); centered title "Report a fault" (16/600); right "Submit" link (accent-500, 15/700 — bold to indicate primary)
- [ ] [m] Submit disabled (gray) until required fields filled

### Category (3-column grid of 6 chips)
- [ ] [m] Section eyebrow "Category" (uppercase 11/700/.06em)
- [ ] [m] 6 category cards (radius 10, 14×8 padding, surface bg + border, brand-soft-bg + brand-border on selected): Plumbing (default selected) / Electric / Heating / Door/lock / Cleaning / Other — each with stroke icon + 12/600 label

### Title input
- [ ] [m] Eyebrow "Title" + single-line text input (12×14 padding, radius 10, surface bg + border)
- [ ] [m] Default placeholder shows example "Heating failure, no hot water"

### Description textarea
- [ ] [m] Eyebrow "Description" + 4-row textarea (12×14 padding, lh 1.5, no resize)
- [ ] [m] Sample copy shows resident-style detail: "No hot water since this morning. Radiator in living room also stays cold. Already checked the breaker."

### Photos (optional)
- [ ] [m] Eyebrow "Photos (optional)"
- [ ] [m] Horizontal strip of 72×72 photo tiles (radius 10) — each with 18px circular × remove-button top-right (rgba(0,0,0,.5) bg)
- [ ] [m] Trailing dashed-border tile with center "+" → opens camera/library picker

### Priority (4-pill segmented)
- [ ] [m] Eyebrow "Priority"
- [ ] [m] 4 equal-flex pills (radius 8, 9×0 padding): Low (gray) · Normal (blue) · High (amber) · Urgent (red, default selected)
- [ ] [m] Each pill has its own bg+border colors when selected (priority-tinted); inactive pill is surface + neutral border

## States

- **Empty (initial)**: Plumbing pre-selected, title/description empty, no photos, Urgent pre-selected (per design). Submit may be disabled until title is non-empty.
- **Submitting**: Submit button shows spinner; form fields disabled. Header should not allow Cancel during in-flight submit; instead show "Cancel submission?" confirm if pressed.
- **Validation error**: failing required field shows danger-600 inline message + danger ring; auto-scroll to first error.
- **Network error**: per voice → "Couldn't submit fault. Try again." inline above the action bar; retains all field values.
- **Success**: navigate back to faults-list with new card at top, success toast "Fault reported · #F-XXX assigned"; haptic feedback on submit.

## Notes

### Broader context

UC-03 fault creation — single-step modal form. The mobile design favors speed (3-col category grid, 4-priority quick-select, optional photos) over comprehensive metadata. Building/unit context is inferred from the resident's authenticated profile, not asked. Photo capture is the highest-value affordance (visual evidence) — make camera path 1-tap.

### Specific (recent)

- Modal header pattern is iOS-native style (Cancel left / Title center / Submit right) — Android variant should follow Material modal pattern (X close + title-bar + submit FAB or bottom action) per platform conventions; design only shows iOS frame.
- Category icons currently inline path strings — RN production must use a shared icon library (Lucide-RN ideally). Plumbing path is a water-drop, Electric is a bolt, Heating is a sun-ray, Door/lock is a door, Cleaning is a trash bin, Other is a plus.
- Selected category styling: brand-soft-bg + brand-600 ink + brand-600 border + brand-600 stroke on icon. Inactive: surface bg + neutral border + dark ink + dark stroke. Single-select.
- Priority pill colors come from the 8-state-machine fault priority tokens — must align with the same pills used in `ppt/faults-list`. Currently inline hex (`#fee2e2`/`#991b1b` etc.) — replace with `--status-fault-priority-{low|normal|high|urgent}-{bg|ink}` tokens.
- Photo strip currently uses inline gradients as placeholders — production captures real images via `expo-image-picker` (RN) or Photos.framework. Limit ≤5 photos and ≤8MB total to control upload time on cellular.
- Title field has no maxLength enforced; should cap at ~120 chars to prevent abuse + ensure list rendering doesn't overflow.
- Description textarea has `resize: none` to keep mobile layout stable; on web/desktop this would resize.
- Submit action: optimistic — show fault card immediately in faults-list with `Submitting...` pill, then resolve to `Reported` on success or `Failed` (with retry) on failure.
- Form should retain unsubmitted state on app background → resume; use AsyncStorage or in-memory persisted form per RN best practice.
- Building/Unit auto-fill from auth context; design doesn't show selector — confirm there's no multi-unit-per-resident edge case (UC-28 delegated permissions might require it).
- Eyebrow uppercase 11/700/.06em pattern is consistent across all sections — extract to a shared `<SectionEyebrow>` RN component.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (ui_kits/mobile/screens.jsx — MobReportFaultScreen); flipped mobile redesignStatus → in-progress; attached designSource; populated functionality checklist (6 sections), all 5 states (initial/submitting/validation/network-error/success), design-specific notes; declared 5 sharedComponents; added 2 relatedScreens (dashboard parent + faults-list sibling)
- 2026-05-08 — init: created from scan (source: sitemap)
