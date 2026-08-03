---
id: reality/agency-import
name: Agency Import (UC-50)
product: reality
implementations:
  reality-web:
    route: "/agency/import"
    component: AgencyImportPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/agency-dashboard
    rel: parent
  - id: reality/agency-branding
    rel: sibling
sharedComponents:
  - wizard
  - stepper
  - radio-cards
  - file-upload
  - data-table
  - status-pill
  - validation-patterns
  - next-intl
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/agency-import.html
    frame: 9-step-wizard-step1-connect / step2-map-fields / step3a-preview / step3b-source-vs-imported / step4-schedule / confirm / history-7-rows / empty-no-imports / error-states-4
useCases:
  - UC-50
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Wizard chrome
- [ ] [w] Manager chrome + agency-dashboard tab strip with `Import` active
- [ ] [w] H1 "Import inzerátov" + count chip + "+ Nový import" primary

### Step 1 · Connect
- [ ] [w] Provider grid (5 radio-cards): Reas · Bazoš · Topreality · Custom CRM · Generic XML
- [ ] [w] Dynamic form per provider: API key, endpoint URL, login credentials
- [ ] [w] "Test connection" button → inline result (success/error within card)

### Step 2 · Map fields
- [ ] [w] 2-column field mapper: source fields left → portal fields right
- [ ] [w] Required fields starred; optional fields ghosted; auto-suggest flags shown
- [ ] [w] Warning banner if any required field unmapped (e.g. "2 nemapované povinné polia")

### Step 3a · Preview
- [ ] [w] 5 sample listings rendered as cards in portal style with provider import badge

### Step 3b · Source vs imported diff
- [ ] [w] Toggle between RAW source fields (left, monospace) and imported model (right, formatted)
- [ ] [w] Highlights mismatches in amber

### Step 4 · Schedule
- [ ] [w] 4 radio-cards: Manual · Daily · Hourly · Webhook
- [ ] [w] Time picker for Daily; webhook URL field for Webhook

### Confirm
- [ ] [w] Success card with green check + summary + actions: "Run now" primary · "Zobraziť v histórii" secondary · "Späť" ghost

### Import history (populated)
- [ ] [w] Filter segmented (Všetky / Beží / Úspešné / Čiastočné / Zlyhalo)
- [ ] [w] Table: timestamp · provider · status pill (Úspešný/Čiastočne/Zlyhanie/Beží with pulsing dot) · imported count · skipped count (clickable link if >0) · duration · actions
- [ ] [w] Sample 7 rows including 1 currently-running import

### Empty state
- [ ] [w] "No imports yet" icon + "Pridajte prvý import alebo CSV nahrať priamo" primary CTA

### Error states (4)
- [ ] [w] 4 error cards: API key invalid · Source feed unreachable · 5 preskočených (chýba pole) · Limit prekročený

## States

- **Step 1–4** (4 wizard steps with provider-specific variations)
- **Confirm**: post-create success card
- **History · loaded** with 7 rows
- **History · empty**
- **History · error variants** (4 distinct cards)

## Notes

### Broader context

UC-50 agency property import. Connects to external CRMs / XML feeds to bulk-import listings. Mapping step is the most error-prone — auto-suggest based on field names + ML helps but can't be perfect.

### Specific (recent)

- i18n (PR #2636): the whole agency/import cluster is now fully localized via next-intl — no hardcoded English left. Route `/agency/import` renders `AgencyImportPage` (a 3-tab layout: CSV · CRM · Feed) mounting `CsvImport` / `CrmConnection` / `FeedImport`; `SyncSchedule` exists in `components/import` (exported, uses `import.schedule`) but is not yet mounted in the page. Note: the shipped tab layout differs from the 9-step wizard in the design bundle above — the wizard checklist remains aspirational (`buildStatus: in-progress`).
- i18n message keys live under the `import` namespace with nested `page` / `steps` / `csv` / `crm` / `feed` / `schedule` groups, present in all 6 reality-web locales (en, sk, cs, de, pl, hu — superset of the sk/cs/de/en named in UC-50). Follow the sibling `useTranslations('import.<group>')` pattern when adding strings; auto-derived CSV/CRM/feed field-name labels are intentionally left untranslated (technical schema identifiers, not UI copy).
- "Beží" status pill uses pulsing dot animation (success-500); respects reduced-motion → static green dot.
- Skipped-count link drilldown shows which fields blocked import for each skipped row (useful for fixing source data).
- Webhook URL must be HTTPS only; show inline validation error for HTTP.
- Auto-suggest flag accuracy: ~75% on first time, improves with usage; surface confidence to user when offering suggestion.
- Provider-specific dynamic forms — keep as JSON config not hardcoded so adding 6th provider doesn't require redeploy.

## Agent Log

<!-- newest entries on top -->

- 2026-08-03 — agent: synced to PR #2636 i18n rewrite. Corrected component `AgencyImportWizard` → `AgencyImportPage`, added route `/agency/import`, added `next-intl` to sharedComponents, documented `import.*` namespace (page/steps/csv/crm/feed/schedule × 6 locales) and the shipped 3-tab (CSV/CRM/Feed) vs design-bundle 9-step-wizard divergence. buildStatus/apiStatus unchanged (pure i18n refactor, no behavior/API change).
- 2026-05-09 — agent: bootstrapped from bundle (pages/agency-import.html — 9 sections: 4 steps + confirm + history loaded/empty + 4 error cards); UC-50; parent agency-dashboard; sibling agency-branding
