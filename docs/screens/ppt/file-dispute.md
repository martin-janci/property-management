---
id: ppt/file-dispute
name: File Dispute (5-step wizard)
product: ppt
sitemapRefs:
  ppt-web: ppt-dispute-new
implementations:
  ppt-web:
    component: FileDisputeWizard
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - disputes_create
relatedScreens:
  - id: ppt/disputes
    rel: parent
  - id: ppt/dispute-detail
    rel: sibling
epics:
  - Epic-77
sharedComponents:
  - wizard
  - stepper
  - radio-cards
  - chip-group
  - address-combobox
  - file-upload
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-file-dispute.html
    frame: step1-category+severity / step3-attachments-uploading / step5-review / submitted-D-2026-0058 / step3-validation-errors
useCases:
  - UC-38
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Wizard chrome
- [ ] [w] Manager chrome + breadcrumb `Hlásenia / Spory / Nový`
- [ ] [w] Stepper: 1 Kategória → 2 Strany → 3 Opis → 4 Riešenie → 5 Súhrn
- [ ] [w] Auto-save indicator: "Návrh uložený · pred 14 sekundami"
- [ ] [w] Right toolbar: ghost "Uložiť návrh a zavrieť" + "Zrušiť" link

### Step 1 · Kategória + Závažnosť
- [ ] [w] 6 category radio-cards (Hluk · Poškodenie · Fakturácie · Spoločné priestory · Domáce zvieratá · Iné)
- [ ] [w] 4 severity radio-cards (Nízka / Stredná / Vysoká / Eskalujúce)

### Step 2 · Strany sporu
- [ ] [w] Sťažovateľ pre-filled (combobox per `forms/address-combobox.html`)
- [ ] [w] Druhá strana combobox
- [ ] [w] Optional witnesses/experts repeating add-row

### Step 3 · Opis + prílohy
- [ ] [w] Title (required, 120 char limit)
- [ ] [w] Description textarea (min 30 chars)
- [ ] [w] Date/time of incident
- [ ] [w] Repeating? checkbox + frequency text
- [ ] [w] Attachments (JPG/PNG/PDF/MP3/MP4, max 50 MB/file)

### Step 4 · Preferovaný spôsob riešenia
- [ ] [w] 4 radio-cards: Dohovor s druhou stranou · Mediácia správcom · Formálne hlasovanie · Eskalácia mimo systému
- [ ] [w] Optional consent checkbox-card "Súhlasím že obe strany dostanú prístup k spisu"

### Step 5 · Súhrn (Review)
- [ ] [w] Recap card with 4 sections + per-section "Upraviť →" links
- [ ] [w] Confirm checkbox: "Potvrdzujem že informácie sú pravdivé..."
- [ ] [w] Submit "Otvoriť spor" disabled until checkbox

## States

- **Step 1**: category + severity selected, "Pokračovať" enabled
- **Step 3 · attachments uploading**: 1 done + 2 uploading queue rows
- **Step 5 · review**: final state before submit
- **Submitted · success**: success card "Spor otvorený · D-2026-0058" + 2 actions
- **Step 3 · validation errors**: top banner.err + inline field errors

## Notes

### Broader context

UC-38 dispute creation flow. 5-step wizard balances thoroughness (legal record) with speed (resident must be willing to complete). Auto-save between steps prevents loss.

### Specific (recent)

- Resolution preference option 4 ("Eskalácia mimo systému") is the legally-sensitive choice — disclaimer copy may need legal review.
- Attachments support audio (MP3) and video (MP4) — useful for noise complaints with recordings.
- Auto-save fires on step transition, not on every keystroke (rate-limit consideration).

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: integrated Batch C (pages/ppt-file-dispute.html — 5 artboards: step 1 / step 3 uploading / step 5 review / submitted / step 3 validation errors); flipped redesignStatus → in-progress; attached designSource; populated 6 sections + 5 states + 3 notes; declared 7 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
