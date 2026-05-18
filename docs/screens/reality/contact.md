---
id: reality/contact
name: Contact
product: reality
implementations:
  reality-web:
    component: ContactPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
  mobile-native:
    component: MContact
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/help
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - text-input
  - select
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/contact.html
    frame: contact-page
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MContact (KMP)
useCases:
  - UC-42
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [ ] [w,m] H1 "Kde nás nájdete" + lede

### Office locations
- [ ] [w] Card grid: Bratislava (HQ) · Praha · Viedeň — each with address, hours, phone, email, embedded map preview
- [ ] [m] Vertical card list with same data

### Contact form
- [ ] [w,m] Subject select (Všeobecné · Sťažnosť na inzerát · Pre realitky · Tlačové oddelenie · Iné) + Name + Email + Phone (optional) + Message textarea + GDPR consent checkbox + Submit primary
- [ ] [w,m] Inline validation per `forms/validation-patterns.html`

### States
- [ ] [w,m] Submitting · Success (toast + form reset) · Error (banner + retry, fields preserved)

### Footer
- [ ] [w] Standard portal footer; [m] System bottom-nav

## States

- **Default**: form empty, submit disabled until required fields validate
- **Submitting**: form fields disabled, spinner on submit
- **Success**: toast + thank-you message + "Pošleme odpoveď do 1 pracovného dňa" promise
- **Error (validation)**: inline field errors below failing fields
- **Error (server)**: top banner "Server momentálne neodpovedá. Skúste to znova alebo nás kontaktujte priamo na <email>."

## Notes

### Broader context

UC-42 contact entry-point. SLA "1 pracovný deň" must be honored server-side; consider auto-routing by subject (e.g. Sťažnosť → moderation queue, Pre realitky → sales).

### Specific (recent)

- 3-office layout reflects the SK/CZ/AT geo footprint mentioned in `reality/about` milestones.
- Contact form is generic — different from agent-specific inquiry forms (which live on listing-detail).
- Map previews on each office card are placeholders; production needs Mapbox/Leaflet (same provider as listings map).

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/contact.html + mobile-new-pages.html MContact frame)
