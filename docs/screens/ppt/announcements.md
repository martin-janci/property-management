---
id: ppt/announcements
name: Announcements
product: ppt
sitemapRefs:
  mobile: mobile-announcements
implementations:
  ppt-web:
    component: AnnouncementsPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
  mobile:
    component: AnnouncementsScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
endpoints:
  - announcements_list
epics:
  - Epic-6
relatedScreens:
  - id: ppt/home
    rel: parent
sharedComponents:
  - status-pill
  - data-table
  - filter-sidebar
  - search-bar
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/ppt-web/announcement-detail.html
    frame: announcement-detail (single — list design pending)
    note: Bundle covers DETAIL surface only; list-view design pending. Pattern established for the list (status pill, pinned indicator, audience badge, ack-rate) extracted from this detail view.
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: mobile-announcements
useCases:
  - UC-02
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header (web — pattern from detail view)
- [ ] [w] PPT manager header (60px, sticky), Announcements tab active

### List view (web — design TBD; spec inferred from detail patterns)
- [ ] [w] Toolbar: search input + segmented filter (All / Published / Draft / Archived / Mine) + "+ New announcement" primary
- [ ] [w] Sidebar: Category (Maintenance / Outage / Vote / Community / General), Audience (All residents / By unit / By role), Status (Draft / Scheduled / Published / Archived)
- [ ] [w] Table or card list with columns: Title (+ pinned indicator) / Status pill / Audience pill / Published / Read rate / Actions (Edit / Pin / Archive)

### Detail view (web — directly designed)
- [ ] [w] Breadcrumb "Announcements / <Title>" + H1 + meta line (Published pill, Pinned indicator, posted-by + relative time, "Delivered to N residents")
- [ ] [w] Right-aligned actions: Unpin (ghost) / Edit (secondary) / Archive (secondary, danger ink)
- [ ] [w] Body card (radius 12, padded 28×32) with rich-text content: paragraphs, h2 sub-headings, ul lists
- [ ] [w] **Callout** block (warning-50 bg + warning-500 left border, 12×16 padding, 6px radius) for safety/action info
- [ ] [w] **Attachment** card (dashed border, 14×16 padding): file icon + filename + size · attached-by + Download button
- [ ] [w] Right rail (300px) — 3 stacked tiles:
  - **Delivery status**: 2 stat columns (Delivered / Acknowledged) + horizontal progress bar + "N% read rate · M pending" sub-line
  - **Details**: kv table (Audience · Category · Published · Pinned until · Language)
  - **Recent acknowledgements**: vertical list (max-height 220 with mask-image fade) of mini-avatar + name + relative time

### Mobile (RN)
- [ ] [m] Announcements tab in bottom-nav (Lucide megaphone icon, replacing legacy 📢)
- [ ] [m] List of announcement cards: pinned-on-top section, then chronological; each card with title + body preview + meta + status pill
- [ ] [m] Tap → detail view (same content as web detail, single-column scroll)

### Status pill set (UC-02)
- [ ] [w,m] Draft → neutral gray
- [ ] [w,m] Scheduled → blue soft + clock indicator
- [ ] [w,m] Published → success-soft + green dot
- [ ] [w,m] Archived → muted gray + archive icon
- [ ] [w,m] Pinned → brand-soft + 📌 indicator (pin glyph or Lucide pin)

## States

- **Empty (list)**: per project voice → "No announcements yet" + "Post one to keep your residents informed." + primary "New announcement"
- **Loading (list)**: skeleton rows or cards (TBD with list design)
- **Error (list)**: "Unable to load announcements. Try again."
- **Detail**: as designed — published with delivery + ack stats; loading / error states for detail not depicted, recommend placeholder skeleton + blunt error banner.

## Notes

### Broader context

UC-02 announcements — manager-published, resident-acknowledged messages. The detail view exposes the **read-rate** as a primary KPI, making it the manager's accountability surface (did the message land?). Pin + audience-targeting are the differentiated affordances over plain email.

### Specific (recent)

- **Drift note**: Like faults, sitemap doesn't include a `ppt-announcements` route, but design + state machine exist. Bumped `ppt-web.buildStatus` from `n/a` → `planned`.
- **Bundle gap**: Only the **detail** view is designed. List-view patterns are inferred from the detail's chrome (header, status pill colors, side-rail style); a list-view artboard is needed before implementation. Surface this as a design ask at next sync.
- Pinned indicator uses Unicode 📌 emoji in the design — per SKILL.md, must migrate to Lucide `pin` (or inline SVG with the same stroke). Don't ship 📌.
- Callout block styling uses `--warning-50` and `--warning-500` (with fallbacks `#fffbeb` / `#f59e0b`) — token-driven; ink `#78350f` is hard-coded; surface a `--warning-ink-strong` token for this.
- Read-rate progress bar uses `--success-500` fill — only because high read-rate is good. If read-rate is low, no design treatment for that — flag at implementation: should we color-shift below a threshold (e.g. <50% gets warning bg)?
- Ack list `mask-image: linear-gradient(#000 75%, transparent)` for fade-out — works but Safari needs `-webkit-mask-image` prefix; respect `prefers-reduced-motion` (no animated reveal).
- Audience meta currently free text ("All residents") — must become a tokenized chip set (All residents / Owners only / By unit / By role) tied to the audience-selector when composing.
- Multi-language announcements (`Slovak · English` in details kv) — implementation must support per-language body editing + per-language read receipts; v1 may ship single-language only.
- Mobile bottom-nav must drop the legacy 📢 emoji per SKILL.md non-negotiable.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (ui_kits/ppt-web/announcement-detail.html — DETAIL only; list pending + ui_kits/mobile/screens.jsx for mobile list); flipped ppt-web from n/a → planned + redesignStatus in-progress (drift: route not in sitemap); flipped mobile redesignStatus → in-progress; attached 2 designSources (with note on detail-only coverage); populated functionality checklist (5 sections + 5-state pill set), states, design-specific notes; declared 4 sharedComponents; added 1 relatedScreen
- 2026-05-08 — init: created from scan (source: sitemap)
