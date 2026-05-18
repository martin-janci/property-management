---
id: ppt/dispute-detail
name: Dispute Detail
product: ppt
sitemapRefs:
  ppt-web: ppt-dispute-detail
implementations:
  ppt-web:
    component: DisputeDetailPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - disputes_get
relatedScreens:
  - id: ppt/disputes
    rel: parent
  - id: ppt/file-dispute
    rel: sibling
epics:
  - Epic-77
sharedComponents:
  - status-pill
  - timeline
  - thread
  - file-upload
  - kv-list
  - state-aware-toolbar
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-dispute-detail.html
    frame: loaded-V-mediacii / loaded-vyriesene-readonly / loading
useCases:
  - UC-38
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header (state-aware)
- [ ] [w] Manager chrome + breadcrumb `Hlásenia / Spory / D-2026-0042`
- [ ] [w] Title H1 + meta (status pill · severity · category · "Otvorené pred 4 dňami" · mediator)
- [ ] [w] Right toolbar varies by state:
  - In V hodnotení/Otvorený: "Priradiť mediátora" primary + "Eskalovať" ghost danger
  - In V mediácii: "Vyriešiť spor" primary + "Eskalovať" + "Stiahnuť mediáciu"
  - In Eskalovaný/Súdny: "Pridať aktualizáciu" primary + "Uzavrieť" ghost danger
  - In Vyriešený/Uzavretý/Stiahnutý: read-only "Znovu otvoriť" ghost only

### Left column · body
- [ ] [w] **Description** card (paragraphs + attachments)
- [ ] [w] **Časová os stavov** card — timeline of every status transition (who, when, optional note)
- [ ] [w] **Diskusia** thread (per `ppt/inquiries` pattern):
  - Day dividers, plaintiff brand-tinted bubbles, defendant surface bubbles, mediator violet-tinted, system muted
  - Composer with "iba pre mediátora" toggle for private notes

### Right rail · sticky
- [ ] [w] **Strany sporu** — Sťažovateľ + Druhá strana cards with avatar, role, contact link, participation pill (Aktívny/Reagovali/Nereagovali/Odmietli)
- [ ] [w] **Mediátor** card (assigned-from date, change-mediator link)
- [ ] [w] **Prílohy** card (4–6 thumbs)
- [ ] [w] **Súvisiace** card (related faults, announcements, documents, other disputes)
- [ ] [w] **Audit log** mini-card (last 4 entries + "Plný log →")

## States

- **Loaded · V mediácii**: mediator assigned, 3 thread messages, 5 attachments, full toolbar
- **Loaded · Vyriešený · read-only**: composer hidden, success-soft status pill, audit shows resolution, "Znovu otvoriť" ghost in toolbar
- **Loading**: body skeleton + rail skeleton

## Notes

### Broader context

UC-38 single-dispute deep view. Combines patterns from `ppt/announcements` (chrome) + `pages/inquiries.html` (thread). State machine drives action toolbar visibility — most-likely actions float to primary slot.

### Specific (recent)

- Discussion bubbles: 4 distinct speaker types (plaintiff/defendant/mediator/system). Mediator gets violet (event-soft); system messages are neutral muted with icon prefix.
- Audit log writes are append-only — design assumes immutability; flag for engineering.
- Sticky rail uses `position: sticky; top: 84px` matching header height.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: integrated Batch C (pages/ppt-dispute-detail.html — 3 artboards: V mediácii / Vyriešený read-only / Loading); flipped redesignStatus → in-progress; attached designSource; populated 4 sections + 3 states + 3 notes; declared 6 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
