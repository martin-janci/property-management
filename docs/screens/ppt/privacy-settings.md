---
id: ppt/privacy-settings
name: Privacy Settings
product: ppt
sitemapRefs:
  ppt-web: ppt-settings-privacy
implementations:
  ppt-web:
    component: PrivacySettingsPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
epics:
  - Epic-63
relatedScreens:
  - id: ppt/accessibility-settings
    rel: sibling
  - id: ppt/notification-settings
    rel: sibling
sharedComponents:
  - section-card
  - banner
  - modal-drawer
  - checkbox-cards
  - audit-log
  - settings-side-nav
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-privacy-settings.html
    frame: loaded / export-3-state / delete-modal-2 / sessions-3 / consents-just-changed+audit / page-error
useCases:
  - UC-23
endpoints: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Layout shell
- [ ] [w] Settings shell with side-nav (Profil / Predvoľby / Prístupnosť / **Súkromie** active / Upozornenia)
- [ ] [w] Breadcrumb "Nastavenia / Súkromie"; H1 "Súkromie"; lede paragraph

### A · Vaše dáta · Export
- [ ] [w] Section card with description (JSON + ZIP s prílohami)
- [ ] [w] Idle state: "Pripraviť úplný export" h4 + "Posledný export: 12. 1. 2026 (148 MB)" meta + ETA "~10–15 minút" + primary "Pripraviť export" CTA (download icon)
- [ ] [w] Preparing state: progress bar with % + ETA "zostáva ~12 minút"
- [ ] [w] Ready state: "Export pripravený" h4 + "Vytvorené 14. 3. 2026 · 12:42 · platné do 21. 3. 2026" + primary "Stiahnuť (148 MB)" + tiny secondary link "Pripraviť nový export"

### B · Vymazanie účtu (DESTRUCTIVE)
- [ ] [w] Section card with `.danger` modifier (red-tinted left edge); h3 "Vymazanie účtu" + `Nezvratné` tag
- [ ] [w] Description references **GDPR čl. 17 ods. 3** retention rule (audit + regulatory data persists)
- [ ] [w] Right-side `btn.danger` "Vymazať účet" (trash icon)
- [ ] [w] Click → modal (`forms/modal-drawer.html` modal variant, ~520 max-width)
- [ ] [w] Modal header: alert-triangle icon + "Vymazať účet · trvalá akcia" h3 + subline
- [ ] [w] Modal body: 4-bullet list (logout / anonymise / notifications stop / GDPR-protected residue) + label "Pre potvrdenie napíšte presne: <code>jana.kovacova@example.sk</code>" + text input + optional `opt-cb` checkbox "Pošlite mi e-mailom potvrdenie o vymazaní (do 30 dní)"
- [ ] [w] Submit (`btn.danger-solid` "Trvale vymazať účet") **disabled** until typed value === user email; "ok" class on input when match
- [ ] [w] Modal footer: "Zrušiť" ghost + danger primary

### C · Aktívne prihlásenia
- [ ] [w] Section card with description "Zariadenia, kde ste momentálne prihlásení."
- [ ] [w] Session row anatomy: device-icon (laptop / phone / tablet — Lucide) + h4 (device + OS) + sub-line (browser + city + masked IP + last-active relative time, separated by dots) + right-side: `cur-pill` (success-soft "Aktuálna" with pulsing dot) on current OR `btn.ghost` "Ukončiť" on others
- [ ] [w] 4 sample rows: MacBook (current) · iPhone 15 (12 min) · iPad Air Trnava (2d) · ThinkPad Praha (11d)
- [ ] [w] Footer link: "Ukončiť všetky ostatné relácie"
- [ ] [w] Variant: Loading — 3 skeleton rows (avatar circle + 2 skeleton lines + skeleton pill)
- [ ] [w] Variant: Single — only the current session shown with sub-text "Aktuálne ste prihlásení iba na tomto zariadení."
- [ ] [w] Variant: Error 503 — section header retained + `banner.err` inline with retry CTA

### D · Marketingové súhlasy
- [ ] [w] `cc-list` of 3 `checkbox-card` rows (NOT switches — checkbox stores explicit GDPR consent timestamp): E-mailové novinky · SMS upozornenia · Anonymizovaná analytika
- [ ] [w] Each card: large check `.box` left + body (h4 + p description + `.ts` timestamp footer "Súhlas udelený 14. 3. 2026 · 12:08" or "Bez súhlasu" warn variant)
- [ ] [w] Toggling auto-saves (no save bar). Just-changed row gets a 600ms brand-glow ring (`box-shadow: 0 0 0 3px var(--accent-soft-bg), 0 0 0 4px var(--accent)` fading out) + new timestamp "Súhlas udelený pred 2 sekundami"
- [ ] [w] Footer link: "Zobraziť históriu súhlasov →"

### E · Audit log card
- [ ] [w] Section card with h3 "História súhlasov" + sub "Posledných 8 zmien · v plnej histórii sú dostupné cez Export"
- [ ] [w] Vertical list, each entry: `dt` left (date · time, tabular-nums) + `ev` right (event type bold + dash + topic). Variants: Súhlas · Odvolanie · Export dokončený · Vytvorenie účtu

### F · Page-level error
- [ ] [w] Top of content: `banner.err` (circle-info) "Zmenu súhlasu sa nepodarilo uložiť. Server odpovedal HTTP 503. Vaša voľba zostáva lokálne aktívna — pokúsime sa uložiť automaticky pri ďalšej akcii alebo si <a>stiahnite súbor s nastaveniami</a>."
- [ ] [w] Banner action: "Skúsiť znova"
- [ ] [w] Local toggle state preserved on the consent that triggered the error

### Locale + theme switcher (artboard chrome)
- [ ] [-] preview-bar with Theme + Locale toggles (SK/CS/DE/EN)

## States

- **Empty (fresh account)**: not depicted; assumed minimal — all consents off, no sessions list (or just current), no last-export, no audit entries beyond "Vytvorenie účtu". Recommend onboarding-style hint banner pointing to first consent.
- **Loaded · synced (default)**: all 4 sections populated; export idle with last-export meta; sessions show 4 devices; 2 of 3 consents on; audit log present.
- **Loading**: per-section. Sessions card has its own skeleton variant (3 rows). Export has prepare-in-flight progress. Consent toggle has its own ring-and-timestamp animation. No global loading state — each section independent.
- **Error · per-section**: sessions 503 banner inside the card · consent save 503 page-level banner · export prep failure inferred (not depicted explicitly — recommend reusing the same `banner.err` pattern).
- **Destructive flow · delete-account modal**: 2 explicit variants — Empty (input blank, submit disabled) and Ready (input matches email + checkbox ticked + danger primary enabled).
- **Just-changed (consents)**: brand-ring glow on the toggled row + "pred 2 sekundami" timestamp; no save bar; auto-persists.

## Notes

### Broader context

UC-23 GDPR + session management. The page bundles 4 distinct concerns: data export (Article 15 right-to-access), data deletion (Article 17 right-to-erasure), session management (security), and marketing consents (Article 6 lawful basis). The destructive flow is the sharpest moment — the design uses confirm-typed-name + audit-stops-anyway disclosure to balance UX with regulatory truth.

### Specific (recent)

- **Confirm-typed-name pattern** (modal): submit stays disabled until input value `===` user email exactly. Type comparison is case-sensitive — production must trim whitespace + normalize Unicode but NOT lowercase (emails can be case-significant under RFC 5321, even though most providers lowercase locals).
- **Optional confirmation email checkbox** generates an audit-trail email sent within 30 days. This is a **deliberate** flow extension — even if the user dismisses the toast, they get evidence in their inbox. Implementation should send to the **email of record at deletion time**, not any later email.
- GDPR retention: the modal explicitly states "Auditné záznamy a regulačné dáta zostanú podľa GDPR čl. 17 ods. 3" — implementation must NOT delete audit_log or regulatory tables. Anonymize by setting `user_id → 'anonymized-<random-uuid>'` in user-facing tables only.
- Sessions sub-line uses dot separators: `Browser · City · masked IP fragment` + relative time. Masked IP shows only the first two octets ("78.99.•••.•••") — implementation must server-side mask, not just visually obscure.
- Current-session pill uses **pulsing green dot** (success-500) — animate `0.5 → 1 → 0.5` opacity at 2s; respect `prefers-reduced-motion: reduce`.
- "Ukončiť všetky ostatné relácie" is a single-click destructive action — design doesn't add a confirm modal because the action is recoverable (user can sign back in). Implementation may surface a less-disruptive `confirm()` toast.
- Consent rows use **checkbox-cards**, not switches, because each toggle records a GDPR consent timestamp + IP + user-agent in the audit log. Switches feel toggleable; checkboxes feel deliberate. The visual token differs slightly: checkbox-card has a dedicated `.box` quadrant with a check glyph.
- Auto-save on consent change: no save bar, no spinner. The optimistic UI updates immediately, the timestamp is shown right away, and the just-changed row gets a 600ms brand-glow that fades. If the server rejects, the page-level error banner appears at top while the local toggle state stays — user can retry.
- Audit-log card shows last 8 entries — full history is downloadable via Export. Implementation: load on demand (collapse/expand); don't fetch full history on page load.
- 4 locales: SK (canonical) + CS + DE + EN with full string maps inline. Production should pull these into `messages/{loc}.json`. Note: legal phrases ("GDPR čl. 17 ods. 3") must NOT be translated automatically — get a localized legal review for each market.
- Export "preparing" duration is server-calculated based on attachments size; design shows "~12 minút" as illustrative. Implementation must show a real ETA from the export job's progress endpoint.

## Agent Log

<!-- newest entries on top -->

- 2026-06-02 — agent (gap-sweep): fixed critical auth bug — page used bare `fetch('/api/v1/gdpr/*')` with no Authorization header (401 in prod). Added `features/privacy/gdprClient.ts` (base URL + bearer token) and routed all GDPR calls through it. Account deletion now sends the required `confirmation` (email) instead of `{}` (was always 400). Save now sends only backend-supported fields (profile_visibility, show_contact_info). Marketing/analytics consent still not persisted by backend — tracked as a follow-up issue; apiStatus stays `partial`.
- 2026-05-09 — agent: design analyzed (pages/ppt-privacy-settings.html — 6 rows: loaded / export-3-state / delete-modal-2 / sessions-3 / consents-just-changed+audit / page-error); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (8 sections covering 6 destructive/non-destructive flows), all states + 6 destructive-flow variants, design-specific notes (confirm-typed-name + GDPR čl. 17 ods. 3 + IP masking + auto-save consent UX); declared 6 sharedComponents; added 1 relatedScreen (accessibility-settings sibling)
