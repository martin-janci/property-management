---
id: ppt/disputes
name: Disputes
product: ppt
sitemapRefs:
  ppt-web: ppt-disputes
implementations:
  ppt-web:
    component: DisputesPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - disputes_list
relatedScreens:
  - id: ppt/dispute-detail
    rel: child
  - id: ppt/file-dispute
    rel: child
  - id: ppt/home
    rel: parent
epics:
  - Epic-77
sharedComponents:
  - status-pill
  - data-table
  - search-bar
  - segmented-control
  - sidebar-filter
  - bulk-action-bar
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-disputes.html
    frame: loaded-3-selected / empty / loading-6-skel / error-503
useCases:
  - UC-38
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with `Hlásenia` tab active
- [ ] [w] Breadcrumb `Hlásenia / Spory`

### Page header
- [ ] [w] H1 "Spory" + count chip "12 otvorených · 3 v mediácii · 2 eskalované"
- [ ] [w] Right toolbar: "Exportovať CSV" secondary + "+ Nový spor" primary → `ppt/file-dispute`

### Filter sidebar (220px)
- [ ] [w] **Stav** 8-state machine: Otvorený · V hodnotení · V mediácii · Vyriešený · Stiahnutý · Eskalovaný · Súdny spor · Uzavretý — counts per state
- [ ] [w] **Kategória**: Hluk · Poškodenie · Fakturácie · Spoločné priestory · Domáce zvieratá · Iné
- [ ] [w] **Závažnosť**: Nízka · Stredná · Vysoká · Eskalujúce
- [ ] [w] **Priradenie**: Mne · Bez priradenia · Tím

### Toolbar
- [ ] [w] Search "Hľadať podľa ID, mena rezidenta alebo opisu…"
- [ ] [w] Segmented "Všetky · Otvorené · V mediácii · Eskalované · Mine"
- [ ] [w] Sort dropdown (Najnovšie ↓ default)

### Data table
- [ ] [w] Columns: ID (mono) · Spor (title + truncated body) · Strany (2 avatars + names) · Kategória · Stav (8-state pill) · Závažnosť · Mediátor · Vek
- [ ] [w] Row selected → brand-soft + brand-600 inset bar

### Bulk-action bar
- [ ] [w] Visible when ≥1 selected: Priradiť mediátora · Eskalovať · Hromadne uzavrieť · Exportovať vybrané

## States

- **Empty**: phone-tile + "Zatiaľ žiadne spory" + body + primary "+ Otvoriť spor" + secondary "Importovať z papierového zoznamu"
- **Loading**: 8 skeleton rows; toolbar + sidebar interactive
- **Error 503**: danger tile + retry; toolbar + sidebar interactive
- **Loaded**: 8 sample disputes covering all 8 statuses; 3 selected with bulk bar visible

## Notes

### Broader context

UC-38 dispute management hub. Manager-side CRUD; resident-side filtered to "my disputes only". 8-state machine + severity + mediator assignment drive the workflow. Audit immutability matters — dispute records may be subpoenaed.

### Specific (recent)

- 8-state pillset uses `--status-dispute-{state}-{bg|ink}` token pairs; ensure tokens defined in `colors_and_type.css` extension.
- Mediator column shows avatar + name when assigned, italic muted "Bez mediátora" when not.
- Multi-select within and across filter groups: AND between groups, OR within (matches `ppt/documents` filter semantics).
- Audit log immutability is engineering concern — flag during implementation kickoff.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: integrated Batch C delivery (pages/ppt-disputes.html — 4 artboards: loaded-3-selected / empty / loading / error); flipped redesignStatus → in-progress; attached designSource; populated 6-section checklist + 4 states + design-specific notes; declared 6 sharedComponents; linked UC-38
- 2026-05-08 — init: created from scan (source: sitemap)
