---
id: ppt/vote-create
name: Vote Create (5-step wizard)
product: ppt
implementations:
  ppt-web:
    route: /voting/new
    component: VoteCreatePage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: ppt/voting
    rel: parent
  - id: ppt/vote-detail
    rel: sibling
sharedComponents:
  - wizard
  - stepper
  - radio-cards
  - chip-group
  - slider
  - date-range
  - file-upload
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-vote-create.html
    frame: step1-yes-no-pick / step2-single-choice-3-suppliers / step3-owners-quorum-60-15-of-24 / step4-now-7days-blind-tally-off / step5-review / confirm-published
useCases:
  - UC-04
endpoints: []
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Wizard chrome
- [ ] [w] Manager chrome + breadcrumb `Hlasovania / Nové`
- [ ] [w] H1 "Vytvoriť hlasovanie"
- [ ] [w] Right toolbar: "Uložiť návrh a zavrieť" ghost + "Zrušiť" link
- [ ] [w] Stepper per `forms/stepper.html`: 1 Téma → 2 Typ → 3 Publikum + Kvórum → 4 Plán → 5 Súhrn
- [ ] [w] Auto-save indicator: "Návrh uložený · pred 14 sekundami"

### Step 1 · Téma
- [ ] [w] Title input (required, 120 char, char counter)
- [ ] [w] Description rich-text textarea (4-row min, basic markdown)
- [ ] [w] Category radio-cards (5): Plán opráv · Domový poriadok · Dodávatelia · Financie · Iné
- [ ] [w] Optional cover image dropzone + attachments multi-file dropzone

### Step 2 · Typ + možnosti
- [ ] [w] 4 vote-type radio-cards with example diagrams: Jediná voľba · Viacero možností · Áno-Nie · Poradie preferencií (Ranked)
- [ ] [w] Choices builder (state-aware):
  - Single/Multi/Ranked: vertical list with drag-handle + label + optional 1-line description; min 2, max 12; "Pridať možnosť" ghost
  - Multi-select: extra "Maximálny počet vybratých" number input
  - Yes-No: choices pre-filled (Áno + Nie) locked; optional "Povoliť 'Zdržať sa'" toggle
  - Ranked: "Tally metóda" segmented (Borda · IRV) default Borda

### Step 3 · Publikum + Kvórum
- [ ] [w] Audience radio-cards: Všetci vlastníci (24) · Vlastníci + nájomcovia (32) · Vybraní vlastníci custom-modal
- [ ] [w] Quorum slider per `forms/slider.html`: range 1 to total-eligible; live label "Vyžaduje kvórum: 13 z 24 (54.2%)"
- [ ] [w] Legal preset chips: 50%+1 · 2/3 · 100% (clicking jumps slider)
- [ ] [w] Voting privacy segmented: Tajné hlasovanie · Otvorené (default)
- [ ] [w] Show partial results toggle: ON default (residents see live) · OFF (blind tally)

### Step 4 · Plán
- [ ] [w] Open at: datetime picker per `forms/date-range.html` reduced to single; default "Hneď po publikovaní"; option "Plánovať na neskôr"
- [ ] [w] Close at: datetime (≥24h after open); quick presets +7 dní / +14 dní / +30 dní / do najbližšej schôdze
- [ ] [w] Reminders: 24h-before push toggle (default ON), 3-days-before email (default ON for >7d votes), custom repeating add-row
- [ ] [w] Auto-extend toggle: "Predĺžiť deadline o 48h pri nedosiahnutí kvóra" (sub-line: max 1×, notification sent)

### Step 5 · Súhrn (Review)
- [ ] [w] Read-only recap with 4 sections + per-section "Upraviť →" links
- [ ] [w] Final preview: embedded mini-ballot showing voter view (read-only)
- [ ] [w] Confirm checkboxes (both required): "v súlade s domovým poriadkom..." + "po publikovaní nie je možné meniť otázku..."
- [ ] [w] Footer: "Späť" + "Publikovať hlasovanie" primary (disabled until both checkboxes) + "Uložiť ako návrh" ghost

### Submitted (success)
- [ ] [w] Success card replacing wizard: large icon + "Hlasovanie publikované · V-2026-0058" + "<n> oprávnených vlastníkov dostalo notifikáciu" + 2 actions

## States

- **Step 1**: title + description + category filled
- **Step 2 · Single choice**: 4 choices entered
- **Step 3**: audience + quorum slider at 60%
- **Step 4**: open now, ends in 7 days, blind-tally OFF, 2× reminder
- **Step 5**: review, confirm checkboxes ticked
- **Confirm · published**: success card

## Notes

### Broader context

UC-04 vote creation. 5-step wizard balances thoroughness (binding decision) with speed (manager publishes weekly). Auto-save between steps prevents loss on flaky network.

### Specific (recent)

- Vote-type Ranked: tally method (Borda vs. IRV) produces different winners on same data — get legal/governance owner confirmation before defaulting Borda.
- Quorum legal preset chips (50%+1, 2/3, 100%) are common but Slovak ZoVB §14b specifies different thresholds per decision class — get localized legal review.
- "Show partial results" defaults ON; blind tally is statistically more honest. Trade-off worth surfacing to users when configuring vote.
- Auto-extend max 1× rule prevents perpetual extension; design respects this.

## Agent Log

<!-- newest entries on top -->

- 2026-06-08 — agent (CTO/PAP-19): built ppt-web `VoteCreatePage` (sectioned form: topic, per-type question/option builder for yes-no/single/multiple/ranked, quorum + close schedule) wired to `useCreateVote` + `addQuestion` + `publishVote`; route `/voting/new`; buildStatus planned→shipped, apiStatus stub→complete, component renamed VoteCreateWizard→VoteCreatePage. 5-step stepper chrome + auto-save + legal-preset chips remain a design follow-up.
- 2026-05-09 — agent: bootstrapped from Batch E (pages/ppt-vote-create.html — 6 artboards: steps 1-5 + confirm); 7 sections + 6 states + 4 notes (with Ranked tally + ZoVB legal flags); declared 8 sharedComponents; parent ppt/voting; sibling ppt/vote-detail
