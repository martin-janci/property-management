---
id: ppt/accessibility-settings
name: Accessibility Settings
product: ppt
sitemapRefs:
  ppt-web: ppt-settings-accessibility
implementations:
  ppt-web:
    component: AccessibilitySettingsPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
epics:
  - Epic-60
relatedScreens:
  - id: ppt/privacy-settings
    rel: sibling
  - id: ppt/notification-settings
    rel: sibling
sharedComponents:
  - switch
  - banner
  - toast
  - section-card
  - settings-side-nav
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-accessibility-settings.html
    frame: loaded / loading+saved-toast / empty+error
useCases:
  - UC-25
endpoints: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Layout shell
- [ ] [w] 2-column shell (~240px settings side-nav + flexible content)
- [ ] [w] Settings side-nav: brand mark + "Account" group with 5 items: Profil / Predvoľby / Prístupnosť (active) / Súkromie / Upozornenia — each row icon + label, brand-soft-bg + brand-600 ink on active

### Page header
- [ ] [w] Breadcrumb "Nastavenia / Prístupnosť" (last bold + primary)
- [ ] [w] H1 "Prístupnosť"
- [ ] [w] Lede paragraph: "Prispôsobte si veľkosť textu, kontrast, animácie a tipy pre čítače obrazovky. Zmeny sa uložia po kliknutí na Uložiť."

### Section: Zrak (Vision)
- [ ] [w] Section card with head (h3 "Zrak" + sub "Veľkosť písma a kontrast prvkov.")
- [ ] [w] **Veľký text** row: meta (h4 + body "Zväčší veľkosť písma o ~25 %. Aplikuje sa na celú aplikáciu vrátane formulárov.") + live preview tile (`<span class="pv-text lg">` showing actual enlarged sample text "Vitajte vo Vašej správe") + switch toggle right
- [ ] [w] **Vysoký kontrast** row: meta + preview tile (button rendered with high-contrast styling) + switch toggle

### Section: Pohyb (Motion)
- [ ] [w] **Obmedziť animácie** row: meta describing what's affected (transitions, card animations, auto-rotating carousels — but NOT loaders) + preview tile (static dot vs. animated dot) + switch
- [ ] [w] Preview must show the actual `still` state when toggle is on; respects user's `prefers-reduced-motion` automatically

### Section: Čítač obrazovky (Screen reader)
- [ ] [w] **Tipy pre čítač obrazovky** row: meta (adds contextual `aria-describedby` text) + preview showing example mark-up: `aria-describedby="Otvorí modál — formulár hlásenia"` + switch
- [ ] [w] **Zvukové signály po uložení** row: meta (short success tone on save toast; longer tone for errors) + preview with speaker icon + state label ("Vypnuté" / "Zapnuté") + switch

### Save bar (sticky bottom)
- [ ] [w] Idle state: "Všetky zmeny uložené · pred chvíľou" + disabled save button
- [ ] [w] Dirty state: "<b>2 zmeny</b> · posledné uloženie pred 14 minútami" + "Zahodiť zmeny" secondary + "Uložiť" primary (check icon)
- [ ] [w] Saving state: "Ukladám zmeny…" + disabled button with spinner + "Ukladám…"
- [ ] [w] Error state: danger-600 "2 neuložené zmeny" + retry CTA

### Loading + saved toast
- [ ] [w] During save: switches go `dis aria-busy=true`; preview tiles still render but inputs locked; spinner on save button
- [ ] [w] On success: toast slides in from corner with check icon + "Nastavenia prístupnosti uložené · synchronizujem na ostatné zariadenia"; auto-dismiss after 4s; respects reduced-motion (fade not slide if motion disabled)

### Empty (fresh account)
- [ ] [w] All switches off
- [ ] [w] `banner.info` with circle-info icon: "<b>Žiadne preferencie nie sú nastavené.</b> Detekujeme váš operačný systém — môžete preniesť jeho nastavenia (OS hlási: <b>vysoký kontrast: zapnutý · obmedziť pohyb: zapnutý</b>)."
- [ ] [w] Banner action: "Použiť systémové" — single-click ports OS-detected prefs into the form

### Error (sync failed)
- [ ] [w] `banner.err` (circle-info) at top: "<b>Zmeny sa nepodarilo uložiť.</b> Server odpovedal HTTP 503. Vaše lokálne zmeny zostávajú aktívne — skúste uložiť znova alebo si <a>stiahnite súbor s nastaveniami</a>."
- [ ] [w] Banner action: "Skúsiť znova"
- [ ] [w] Save bar shows "<b style='danger-600'>2 neuložené zmeny</b>" + retry primary
- [ ] [w] Switch positions retain user's local choices (not server values) until retry succeeds

### Locale + theme switcher (artboard chrome)
- [ ] [-] preview-bar with Theme + Locale toggles

## States

- **Empty (fresh account)**: all switches off; OS-detection info banner with "Použiť systémové" CTA. No save activity.
- **Loaded · synced (default)**: 2 switches on (Veľký text + Obmedziť animácie), live previews reflect each toggle, save bar idle with "Všetky zmeny uložené · pred chvíľou".
- **Loading (saving)**: switches `aria-busy=true`, save bar shows "Ukladám zmeny…" with spinner, fields disabled but still display current chosen values.
- **Saved (toast)**: toast slides in confirming save + sync; save bar returns to idle; switches re-enabled.
- **Error (sync failed)**: error banner at top with retry CTA; save bar shows danger "2 neuložené zmeny"; local changes preserved until retry succeeds; download-settings-file fallback offered.

## Notes

### Broader context

UC-25 accessibility preferences. Per-user, server-synced across devices. The killer feature: **live previews** on every switch row — the user sees the effect (large-text sample, high-contrast button render, motion-still dot, aria attribute example, speaker-on/off label) before committing the save. Reduces "save → test → revert" loops.

### Specific (recent)

- The "Empty" state is the most thoughtful: it actively detects OS prefs (`prefers-contrast`, `prefers-reduced-motion`) and offers a one-click port. Implementation must use the matching CSS media queries + a server stub to confirm OS values weren't already explicitly overridden.
- Live preview tiles are NOT just CSS samples — they use the actual app's design tokens applied with the proposed setting. Implementation should reuse the production `Button`, `Heading`, etc. components with a scoped `[data-preview-mode]` attribute that mimics the would-be applied state.
- Switch is the standard `forms/switch.html` primitive: 34×20 with circular knob, `.on` brand-fill, `.dis` muted with `aria-busy=true` during save.
- Save bar is sticky at viewport bottom inside the content column (not full-window). 4 distinct visual states (idle / dirty / saving / error). Idle disables the save button via `disabled` attribute, not opacity.
- Toast pattern: bottom-right anchored, surface bg + 1px subtle border, check icon in success-soft tile + ink. Auto-dismiss 4s; respects `prefers-reduced-motion: reduce` (no slide-in animation, fade only).
- Sound-on-save behavior: the design states a short tone for success, a longer one for errors. Must be opt-in (default off) — do not auto-play sound. Use `Audio` API, not generated tones, to avoid violating user autoplay policies.
- Settings side-nav has 5 items but only Accessibility is implemented in this artboard. Profile / Predvoľby (Preferences) / Súkromie / Upozornenia are placeholders pointing to other screen-maps (privacy-settings + notification-settings exist; profile + preferences are TBD per project scope).
- "Pred chvíľou" / "pred 14 minútami" relative-time strings need `Intl.RelativeTimeFormat` for sk/cs/de/en/pl/hu — match other screens.
- Error fallback "stiahnite súbor s nastaveniami" generates a JSON download of the user's current preference state — important for accessibility users who can't lose 2 minutes of toggling. Implement before shipping.
- Switch tokens: bg = `--bg-input`, on-bg = `--accent`, knob = white, knob-shadow `0 1px 3px rgba(0,0,0,.2)`. Disabled state must remain readable in dark mode.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (pages/ppt-accessibility-settings.html — 5 artboards: loaded / loading-saving / saved-toast / empty / error); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (10 sections), all 5 states, design-specific notes including live-preview tile pattern + OS-detection empty state + sticky save-bar 4-variant; linked UC-25; declared 5 sharedComponents; added 1 relatedScreen (privacy-settings sibling)
- 2026-05-08 — init: created from scan (source: sitemap)
