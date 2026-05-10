---
id: ppt/emergency-contacts
name: Emergency Contacts
product: ppt
sitemapRefs:
  ppt-web: ppt-emergency
implementations:
  ppt-web:
    component: EmergencyContactDirectoryPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - emergency_list
epics:
  - Epic-62
sharedComponents:
  - status-pill
  - data-table
  - modal-drawer
  - inline-edit
  - radio-cards
  - phone-input
  - search-bar
  - segmented-control
  - color-picker
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-emergency-contacts.html
    frame: loaded-8-rows / inline-edit / add-drawer-empty+filled / delete-confirm / empty / loading / error
useCases:
  - UC-39
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with nav (Dashboard / Hlásenia / Rezidenti active / Dokumenty / Oznamy)
- [ ] [w] User chip (avatar + name + role) right-aligned
- [ ] [w] Breadcrumb "Rezidenti / Núdzové kontakty"

### Page header
- [ ] [w] H1 "Núdzové kontakty"
- [ ] [w] Count chip "8 kontaktov · 2 primárne"
- [ ] [w] Right toolbar: "Importovať" secondary (upload icon) + "+ Pridať kontakt" primary (opens drawer)

### Filter strip
- [ ] [w] Segmented chips with counts: Všetci · 8 / Primárne · 2 / Sekundárne · 4 / Mimo hodín · 2
- [ ] [w] Search input "Hľadať podľa mena, role alebo čísla…"
- [ ] [w] Sort dropdown ("Dôležitosť ↓" default; alternatives: Name ↑, Recently updated)

### Contacts table (loaded)
- [ ] [w] Columns: Meno (avatar + name) / Rola (b + sub muted) / Telefón (formatted SK + click-to-call icon) / Poznámka (truncated 1-line, hover full) / Dôležitosť pill / Akcie (right-aligned icon-btns)
- [ ] [w] Avatar mark uses 1-of-6 gradient color (`c1`–`c6`) hashed from contact ID — stable across renders
- [ ] [w] Importance pill variants: Primárny (brand-soft + brand-600 dot) / Sekundárny (neutral-soft + neutral dot) / Mimo hodín (violet-soft + violet dot)
- [ ] [w] Per-row actions: pencil icon-btn (opens inline edit) + trash icon-btn (opens confirm modal)
- [ ] [w] Row hover bg-subtle; click anywhere except action cell opens detail (or inline edit if shipped without detail)
- [ ] [w] Keyboard nav: ↑↓ select, Enter open, Delete confirm-delete

### Inline edit
- [ ] [w] Per `forms/inline-edit.html`: edited row replaces with `tr.edit > td colspan=6 > ie-grid` containing 4 inline inputs (Name / Role / Phone / Notes) + importance radio-cards + Save (primary) + Cancel (ghost) right-aligned
- [ ] [w] Adjacent rows remain visible + interactive — preserves position context
- [ ] [w] ESC cancels; ⌘/Ctrl+Enter saves

### Add-contact drawer
- [ ] [w] Right-side drawer per `forms/modal-drawer.html` (~440px, full-height, shadow)
- [ ] [w] Header: "Pridať núdzový kontakt" + close ×
- [ ] [w] Avatar/mark color picker — 6 swatches matching `c1`–`c6` palette
- [ ] [w] Name text-input
- [ ] [w] Role text-input with autocomplete suggestions: Vodár · Plynár · Kúrič · Výťah · servis · Elektrikár · Bezpečnostná služba · Polícia · Hasiči · Záchranná zdravotná služba
- [ ] [w] `forms/phone-input.html` with SK country code default (+421)
- [ ] [w] Notes textarea (3 rows, optional)
- [ ] [w] Importance radio-cards (Primary / Secondary / Off-hours) with 1-line description each
- [ ] [w] Visibility segmented: Všetci rezidenti / Iba správcovia
- [ ] [w] Footer: Zrušiť (secondary) + "Pridať kontakt" (primary)
- [ ] [w] Variants: Empty (default form, primary disabled) + Filled (validation-passing, primary enabled)

### Delete confirmation
- [ ] [w] Modal (~480 max-width); danger styling
- [ ] [w] Body: "Vymazať '{Name} · {Role}'? Rezidenti tento kontakt prestanú vidieť. Histórie hovorov a hlásení zostanú zachované."
- [ ] [w] Footer: Zrušiť (secondary) + danger "Vymazať"

### Empty (no contacts yet)
- [ ] [w] Center card: phone tile icon + "Zatiaľ žiadne núdzové kontakty" h2 + body advising at-least-one primary contact for water/gas/heating + primary "+ Pridať prvý kontakt" CTA + secondary "Importovať z CSV" link

### Loading
- [ ] [w] Toolbar + filter strip remain interactive
- [ ] [w] 6 skeleton rows: avatar circle skel + 4 line skels (name/role/phone/notes) + pill skel + 2 icon-btn skels

### Error
- [ ] [w] Toolbar + filter strip remain interactive
- [ ] [w] Where table would be: centered danger-tile + "Núdzové kontakty sa nepodarilo načítať." + "Skontrolujte pripojenie a skúste to znova." + primary "Skúsiť znova" + secondary "Zobraziť posledný známy stav" (cached fallback)

### Locale + theme switcher (artboard chrome)
- [ ] [-] preview-bar with Theme + Locale toggles (SK/CS/DE/EN)

## States

- **Empty (fresh building)**: phone tile + headline + body + primary "Pridať prvý kontakt" + secondary "Importovať z CSV". No filter strip activity.
- **Loading**: 6 skeleton rows; toolbar + filter strip stay interactive.
- **Error**: danger tile + retry primary + cached-fallback secondary; toolbar + filter strip preserved.
- **Loaded · 8 rows**: 2 primary (BVS, SPP) + 4 secondary (Veolia kúrenie, OTIS, Elektro Mlynár, Securitas) + 2 off-hours (Nonstop dispečing, building manager); each row has avatar + role + phone + notes + importance pill + actions.
- **Inline edit (1 row)**: edited row expands in-place; adjacent rows readable.
- **Add drawer**: 2 variants (empty form / filled form ready to submit).
- **Delete confirm**: modal with name-aware body copy + danger primary.

## Notes

### Broader context

UC-39 emergency contact directory. Manager-side CRUD; resident-side read (rezidenti vidia tento zoznam vo svojich aplikáciách). Importance pills (Primary / Secondary / Off-hours) drive both **sort order** and **resident-app surfacing** — primary contacts appear in the resident's bottom-sheet emergency drawer first.

### Specific (recent)

- The design uses **manager chrome** (PPT top header + Rezidenti tab active) instead of the settings side-nav — this is intentional. Emergency contacts are a **building asset** managed by managers, not a personal preference, so the surface should live under the manager nav, not under "Account · Súkromie · Prístupnosť · ...".
- Sample row contacts cover real Slovak emergency lines: BVS (water), SPP (gas), Veolia (heating), OTIS (lift), Securitas (security), plus a building-manager fallback. Production seed data should match real per-region utilities.
- Phone numbers use formatted SK style with optional country code: `+421 800 121 333`, `0850 111 727`, `+421 905 412 008`. The `phone-input` primitive (`forms/phone-input.html`) must accept both formats and normalise to E.164 for storage.
- Click-to-call icon (small phone glyph) inline next to the number — opens `tel:` link on mobile, copies to clipboard on desktop with toast confirmation.
- Importance pill colors are the 8-state-machine tokens: Primary uses `--status-primary-*` (brand), Secondary uses `--status-neutral-*`, Off-hours uses `--status-event-*` (violet). Don't ad-hoc colors.
- Avatar gradient palette is 6 colors hashed from contact ID — same approach as the inquiries page agent avatars. Hash function must be stable across server restarts.
- Notes column truncates at 1 line via `text-overflow: ellipsis`; hover shows full text via `title` attribute (browser-native tooltip). For long notes, consider expanding into a popover with `forms/hover-card.html` pattern.
- Inline edit pattern preserves table layout (uses `colspan=6` on the edit row's TD); adjacent rows remain in their normal layout. Don't try to push the edit row into a sidebar.
- Add-drawer slides from right at 320ms ease-standard; respect `prefers-reduced-motion` (fade only, no slide).
- Drawer vs. modal split: **drawer for non-destructive create/edit** (can stay open while user references the table behind), **modal for delete** (blocks the page until decision made). This pattern follows `forms/modal-drawer.html` recommendations.
- "Importovať z CSV" empty-state link points to a yet-to-design CSV import flow — placeholder for a future screen-map (similar to reality/agency-import).
- Resident view of this list is NOT yet designed — assumed to be a simple read-only mobile screen sourced from this directory. Flag at implementation if the mobile RN side needs its own variant.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (pages/ppt-emergency-contacts.html — 7 artboards: loaded-8 / inline-edit / add-drawer empty+filled / delete-confirm / empty / loading / error); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (10 sections), 7 states, design-specific notes (manager-chrome decision + 8-state pill tokens + drawer-vs-modal split + phone-input E.164 normalisation); declared 9 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
