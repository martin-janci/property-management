---
id: ppt/document-detail
name: Document Detail
product: ppt
sitemapRefs:
  ppt-web: ppt-document-detail
implementations:
  ppt-web:
    component: DocumentDetailPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    component: DocumentDetailScreen
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
endpoints:
  - documents_get
  - documents_get_versions
relatedScreens:
  - id: ppt/documents
    rel: parent
  - id: ppt/upload-document
    rel: sibling
epics:
  - Epic-39
sharedComponents:
  - file-icon
  - timeline
  - data-table
  - status-pill
  - pdf-preview
  - kv-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-document-detail.html
    frame: loaded-v3-published
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobDocumentDetailScreen
useCases:
  - UC-08
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with Dokumenty tab active
- [ ] [w] Breadcrumb path "Dokumenty / Knižnica / Schôdze (AGM) / DOC-2026-0042" — last segment monospace ID, bold

### Document header
- [ ] [w] Large file-icon block (PDF/DOCX/XLSX/JPG/PNG/ZIP coloring)
- [ ] [w] Title h1 + meta line (category pill + version + size + page count + last-updated relative time)
- [ ] [w] Right toolbar: prev/next ghost icon-btns (navigate within current list filter) + "Stiahnuť (1,2 MB)" secondary + "Nahrať novú verziu" primary

### Split layout
- [ ] [w] 2-column flex: PDF preview left (flex 1) + 320px right rail
- [ ] [w] Below 1024px collapses to single column with rail above preview

### PDF preview pane
- [ ] [w] Preview toolbar: page navigator (`<` <b>3</b> z <b>14</b> `>`) + zoom controls (− / 100% / +)
- [ ] [w] Stage shows rendered PDF page (mock content for AGM minutes: header, agenda item, voting table, signatures, page footer "Strana 3 / 14")
- [ ] [w] Click page → fullscreen view (TBD per implementation)

### Right rail (3 cards)
- [ ] [w] **Metadáta** — kv-list (`dl`): Kategória / Publikum (with eye icon) / Rok / Nahral (avatar+name) / Zdroj (Manuálny upload · web) / ID (monospace selectable)
- [ ] [w] **Akcie** — vertical action list (5 rows): Zdieľať s rezidentmi · Pripojiť k oznamu · Premenovať / upraviť metadáta · Archivovať · Vymazať (danger ink)
- [x] [w] **História verzií · 4** — timeline with current version highlighted (brand-600 dot), prior versions with neutral dots; each entry: version tag (v3/v2/v1) + commit-style title + sub-line (date · time · author · size) — list-only (no restore) shipped via `useDocumentVersions`

### Locale + theme switcher
- [ ] [-] preview-bar with Theme + Locale toggles (SK/CS/DE/EN)

## States

- **Loading**: not depicted; recommend split-skeleton (PDF placeholder with shimmer + 3-card right rail with line skeletons)
- **Error**: not depicted; per voice → "Detail dokumentu sa nepodarilo načítať" + retry primary, retain breadcrumb + header
- **Permission denied**: not depicted; if user lacks audience access, show forbidden state (lock icon + "Tento dokument je viditeľný iba pre {audience}" + back-to-library CTA)
- **Loaded · v3 published (default)**: full layout — header with all actions, PDF page 3 visible, right rail with 4-version timeline (v3 current, v2 oprava preklepu, v1 prvý upload, vytvorenie účtu)
- **Archived**: not depicted; recommend muted layout treatment + "Archivované" status pill in header + actions row hides Edit/Delete + adds "Obnoviť" primary

## Notes

### Broader context

UC-08 single-document detail. Manager-side full editing; resident-side filtered (no version timeline, no edit actions). PDF preview is the killer feature — viewing without download. Version history is the **immutability anchor** — once a version is replaced, the prior file remains downloadable forever (audit + legal trail).

### Specific (recent)

- PDF preview uses an embedded rendering library (PDF.js most likely); production must lazy-load on viewport entry to keep initial load light. Page navigator + zoom must work without re-fetching the file (client-side rendering).
- Sample PDF content models real AGM minutes: building address + meeting metadata + agenda items + voting tables (Za / Proti / Zdržal sa percentages + counts) + signatures (Predseda / Zapisovateľ / Overovateľ). When porting, ensure typography matches: h1 building-meeting + h2 agenda items + h3 sub-sections + dense tables.
- Version timeline uses `cur` modifier on the active version (brand-600 fill dot + "Aktuálna verzia" label); prior versions have neutral dots and concise diff-style titles ("Oprava preklepu v hlasovaní"). Implementation: capture commit-style messages on every upload-new-version action.
- "Pripojiť k oznamu" action is a cross-screen affordance — links to UC-02 announcement-create with this document pre-attached.
- Delete action requires confirmation (modal) per the privacy-settings pattern. Don't allow direct trash-icon delete; document deletion has GDPR + audit implications.
- "Predchádzajúci / Ďalší" navigation is bound to the current `ppt/documents` filter context — clicking next jumps to the next document in the same filter view, not the next document by ID.
- Right-rail cards are sticky on tall content (`position: sticky; top: 84px`) — must match header height.
- Archive flow is non-destructive (sets `status=archived`, hides from default search, retains all data). Restore from archive is a single-click via the documents list filter.
- 4 locales: SK/CS/DE/EN with full string maps inline.

## Agent Log

<!-- newest entries on top -->

- 2026-06-03 — agent: #974.1 — fixed the version-history binding to match the real backend `VersionHistoryResponse` (the generated client mis-models this endpoint as a bare camelCase array; runtime returns `{ history: { versions: [...] } }` snake_case). Page now unwraps `history.versions`, prefers the server's `is_current_version` flag for the current-version highlight, and reads `created_by_name` for the uploader. Added `VersionHistoryResponse`/`DocumentVersionHistory`/`DocumentVersion` types and typed the `useDocumentVersions` hook to the backend shape. Still list-only (no restore).
- 2026-06-03 — agent: gap-7b-1 — added "História verzií" list-only version-history timeline to DocumentDetail (via new `useDocumentVersions` hook); each row shows version number + uploader + date + size, newest first, highest version highlighted as current. No restore (excluded from fixable-now scope).
- 2026-05-27 — agent: gap-7a-4 review fixes — useDocumentDownload hook extracted and wired into DownloadButton; download errors now surface via toast; getDownloadUrl imperative call retained (same fetchApi transport as all api-client ops)
- 2026-05-27 — agent: gap-7a-4 — added DocumentPreviewModal (PDF inline preview via PdfPreview + image preview via <img> + fallback + download button in header); preview eye + download action buttons added to each document row in DocumentsBrowse; useDownloadUrl + usePreviewUrl presigned-URL hooks wired; modal is accessible (Escape key close, backdrop click, auto-focus close button, role=dialog aria-modal); apiStatus remains complete
- 2026-05-27 — agent: gap-7a-2 review fixes — useDocumentDownload hook extracted; download error toast added; MoveFolderDialog focus trap completed
- 2026-05-24 — agent: gap-7a-5 — added DocumentSharePanel (user/role/building/link share types) to DocumentDetail; Share toggle button; share hooks in @ppt/api-client; apiStatus remains complete

- 2026-05-24 — agent: gap-7a-3 — added RLS-aware permission-denied state to DocumentDetail (403 → lock icon + Slovak message); promoted ppt-web.apiStatus partial→complete
- 2026-05-09 — agent: design analyzed (pages/ppt-document-detail.html — single artboard: loaded-v3-published with PDF preview + metadata + 5 actions + 4-version timeline); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (5 sections), 5 states (recommended where not depicted), design-specific notes (PDF.js lazy-load + version-immutability + cross-screen attach + archive flow); declared 6 sharedComponents; added 1 relatedScreen (upload-document sibling)
- 2026-05-08 — init: created from scan (source: sitemap)
