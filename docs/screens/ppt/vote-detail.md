---
id: ppt/vote-detail
name: Vote Detail
product: ppt
implementations:
  ppt-web:
    route: /voting/:voteId
    component: VoteDetailPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    component: VoteDetailScreen
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
relatedScreens:
  - id: ppt/voting
    rel: parent
  - id: ppt/vote-create
    rel: sibling
sharedComponents:
  - ballot-radio-cards
  - ballot-yes-no
  - ballot-multi-select
  - ballot-ranked
  - quorum-tile
  - results-bar
  - thread
  - timeline
  - kv-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-vote-detail.html
    frame: open-single-choice-not-voted-7-of-13 / yes-no-voted-yes / closed-approved-67% / loading
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobVoteDetailScreen
useCases:
  - UC-04
endpoints: []
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header (state-aware)
- [ ] [w] Manager chrome + breadcrumb `Hlasovania / <category> / V-2026-0042`
- [ ] [w] H1 + meta line (status pill · category · author · "Otvorené 14. 3." · close countdown chip)
- [ ] [w] State-aware action toolbar:
  - **Koncept**: Náhľad · Publikovať teraz primary · Plánovať
  - **Plánované**: countdown "Otvorí sa o 1 deň" + Otvoriť teraz primary + Upraviť + Zrušiť ghost danger
  - **Otvorené/Kvórum**: Pripomenúť hlasujúcim · Predĺžiť deadline · Uzavrieť teraz ghost danger
  - **Uzavreté/Zrušené**: Exportovať výsledky · Otvoriť pokračovanie ghost (creates successor pre-filled)

### Left column · body
- [ ] [w] **Description card** — proposal paragraphs + attachments
- [ ] [w] **Ballot UI card** — 4 variants:
  - **Single choice**: stack of large radio-cards; selected brand-soft + outline; "Hlasovať za <selected>" full-width primary; "Vymazať voľbu" ghost link if voted
  - **Multi-select**: same as single but checkbox icon; "Vyberte 1–N max" sub-line; counter pill "Vybraté: 2 z 3 max"
  - **Yes-No**: 2 large 50/50 cards (Áno success-soft / Nie danger-soft); selected full opacity, unselected fades 60%; optional "Zdržať sa" ghost
  - **Ranked**: vertical list with drag-handle (desktop) or up/down arrows + position number (touch); Borda or IRV tally
- [ ] [w] **Already-voted state** (inline at top of ballot): success-soft banner with check icon + "Hlasovali ste 14. 3. 11:42 · za <choice>" + ghost "Zmeniť voľbu"
- [ ] [w] **Results card** (visible when manager OR after close OR partial=true): per-choice horizontal stacked bar + total turnout + quorum status pill
- [ ] [w] **Audit log card** (collapsed by default; manager-only fully expanded): last 5 events + "Plný log →"
- [ ] [w] **Discussion thread** (mirrors `ppt/dispute-detail`): day dividers + author/voter/manager bubbles (manager violet-tinted) + "iba pre managera" toggle

### Right rail · sticky
- [ ] [w] **Stav hlasovania**: large countdown ("Končí o 3 dni · 14h 22m") + quorum bar (matches manager-dashboard tile) + total turnout %
- [ ] [w] **Prílohy** card
- [ ] [w] **Súvisiace** card
- [ ] [w] **Účasť podľa skupín** (manager-only): % účasť by audience segment

### Mobile (RN — `MobVoteDetailScreen`)
- [ ] [m] Header sticky + hero strip (status + countdown + quorum bar full-width)
- [ ] [m] Description card + Ballot card (touch-optimized variants — radio-cards min 60pt tap; ranked uses up/down arrows not drag)
- [ ] [m] Sticky bottom action bar: full-width "Hlasovať za <choice>" primary; if voted → "Zmeniť voľbu" ghost; sub-line muted "Zmeny do <close datetime>"

## States

- **Otvorené · Single-choice · not-yet-voted** (default ballot UI, 7/13 quorum)
- **Otvorené · Yes-No · already-voted** (success banner + composer)
- **Uzavreté · Schválené 67%** (read-only, full results card visible, turnout 79%)
- **Loading**: skeleton

## Notes

### Broader context

UC-04 deep view + ballot interaction. The act of voting happens here — friction-free single-choice ballot is the core conversion. Quorum tile must round-trip with manager-dashboard right-rail tile (same logic, same colors).

### Specific (recent)

- Ballot card has 4 visual variants depending on `vote.type`. Implementation should render via discriminated-union switch, not a generic ballot component with mode prop.
- Already-voted state is **inline at top of ballot** (NOT replacing the ballot). Voter can see their choice + still has access to change-vote affordance.
- Discussion thread uses 4 speaker types: vote-author (brand-tinted), other voter (surface), manager (violet event-soft), system (neutral).
- Mobile ballot ranked uses **up/down arrows**, not drag handle — touch drag-to-reorder is unreliable on phones.

## Agent Log

<!-- newest entries on top -->

- 2026-06-08 — agent (CTO/PAP-19): built ppt-web `VoteDetailPage` — state-aware manager toolbar (publish/close/cancel), per-unit per-question ballot (4 ballot variants via discriminated switch), live/final results bars, quorum tile, discussion thread — wired to `useVote`/`useVoteEligibility`/`useVoteResults`/`useCastVote`/`usePublishVote`/`useCloseVote`/`useCancelVote`/`useAddVoteComment`; route `/voting/:voteId`; buildStatus planned→shipped, apiStatus stub→complete. Sticky right-rail, audit-log card, and ranked drag-handle (desktop) remain design follow-ups.
- 2026-05-09 — agent: bootstrapped from Batch E (pages/ppt-vote-detail.html — 4 artboards) + Batch F1 (MobVoteDetailScreen); 4 sections + 4 states + 4 notes; declared 9 sharedComponents; parent ppt/voting; sibling ppt/vote-create
