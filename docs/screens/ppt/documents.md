---
id: ppt/documents
name: Documents
product: ppt
sitemapRefs:
  ppt-web: ppt-documents
  mobile: mobile-documents
implementations:
  ppt-web:
    component: DocumentsPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    component: DocumentsScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
endpoints:
  - documents_list
relatedScreens:
  - id: ppt/document-detail
    rel: child
  - id: ppt/upload-document
    rel: child
  - id: ppt/document-folders
    rel: child
epics:
  - Epic-39
  - Epic-7
sharedComponents:
  - data-table
  - filter-sidebar
  - search-bar
  - segmented-control
  - bulk-action-bar
  - status-pill
  - file-icon
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-documents.html
    frame: loaded-12-3-selected / empty / error / loading
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobDocumentsScreen
useCases:
  - UC-08
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with Dokumenty tab active
- [ ] [w] Breadcrumb "Dokumenty / Knižnica"

### Page header
- [ ] [w] H1 "Dokumenty" + count chip "40 dokumentov · 12 zobrazených"
- [ ] [w] Right toolbar: "Nahrať dokument" primary (upload icon) → ppt/upload-document

### Filter sidebar (left rail)
- [ ] [w] `fbar-head` with "Filtre" + active count + "Vymazať" link
- [ ] [w] Group **Rok**: checkbox list with year + per-year count (2026 · 3, 2025 · 5, 2024 · 8, 2023 · 7, Staršie · 12)
- [ ] [w] Group **Kategória**: checkbox list (AGM · 6, Ročné správy · 5, Poistenie · 3, Faktúry · 14, Zmluvy · 7)
- [ ] [w] Group **Status**: 3-state machine — Publikované · Návrhy · Archivované with counts
- [ ] [w] Group **Publikum**: Všetci rezidenti · Iba vlastníci · Iba správcovia with counts
- [ ] [w] Multi-select within and across groups (AND between groups, OR within)

### Toolbar
- [ ] [w] Segmented chips with counts: Všetky · 40 / AGM · 6 / Ročné · 5 / Poistenie · 3 / Faktúry · 14
- [ ] [w] Search input "Hľadať podľa názvu, kategórie, autora…"

### Bulk-action bar (visible when ≥1 row selected)
- [ ] [w] Brand-soft bg strip with check icon + "<b>3</b> vybraté" + actions: "Stiahnuť ZIP (3,4 MB)" + "Presunúť do…" + "Archivovať"
- [ ] [w] Slides in below toolbar; preserves table position

### Documents table
- [ ] [w] Header row with checkbox column + sortable: Názov ↓ / Kategória / Nahral / Veľkosť (right-aligned) / Dátum (right-aligned) + actions cell
- [ ] [w] Row anatomy: bulk checkbox + file-icon block (PDF / DOCX / XLSX / JPG / PNG / ZIP coloring) + 2-line file info (title + monospace ID + version) + category pill + uploader avatar + name + size (tabular-nums right) + date right + 3-dot menu
- [ ] [w] Selected row gets brand-soft bg + brand-600 left bar (3px inset)
- [ ] [w] Click row → `ppt/document-detail`

### Empty (fresh building)
- [ ] [w] Centered card: file-tile + "Žiadne dokumenty" headline + body + primary "Nahrať prvý dokument" CTA + secondary "Importovať priečinok" link

### Loading
- [ ] [w] Toolbar + filter sidebar remain interactive
- [ ] [w] 8 skeleton rows: file-icon skel + 2-line title skel + pill skel + avatar+name skel + size+date skels

### Error
- [ ] [w] Toolbar + filter sidebar remain interactive
- [ ] [w] Where table would be: danger-tile + "Dokumenty sa nepodarilo načítať." + retry primary

### Locale + theme switcher
- [ ] [-] preview-bar with Theme + Locale toggles

## States

- **Empty (fresh building)**: file tile + headline + body + "Nahrať prvý dokument" primary + "Importovať priečinok" secondary.
- **Loading**: 8 skeleton rows; filter sidebar + toolbar interactive.
- **Error**: danger tile + retry; sidebar + toolbar interactive.
- **Loaded · 12 of 40 (3 selected)**: filter sidebar with 2 active filters (Rok 2026+2025, Status Publikované); table with 3 selected rows (brand-soft bg) showing AGM zápis + ročná správa + poistná zmluva; bulk-action bar visible.

## Notes

### Broader context

UC-08 central document repository. Manager-side CRUD; resident-side filtered read by audience. Filter density (year × category × status × audience) is the key affordance — managers should be able to find a 2-year-old AGM minutes in <5 seconds via combined filters.

### Specific (recent)

- **Drift note**: existing screen-map showed `ppt-web.buildStatus: shipped`, but the bundle now provides the canonical redesign — flipping to `in-progress` reflects the redesign-shipped-but-implementation-pending state.
- File-icon block uses 3-letter colored tags (PDF red / XLS green / DOC blue / JPG amber / PNG cyan / ZIP gray) — same convention as `forms/file-upload.html`.
- Document ID `DOC-2026-0042` is monospace + muted — must be selectable for support tickets. Display under the title in 11px size.
- Version indicator (`v3`) shown next to ID — clicking opens detail with version timeline (designed in `ppt/document-detail`).
- Bulk-action bar slides into view when >0 rows selected; collapses when count returns to 0. Hide-on-hide animation respects reduced-motion.
- "Stiahnuť ZIP" action computes total size client-side from selected rows; shows estimated size in button label ("3,4 MB"). Server generates the ZIP server-side (don't ZIP in browser for large sets).
- Sort indicator ↓ / ↑ uses Unicode glyphs — design intent is muted secondary color when not actively sorting.
- Audience filter — Iba správcovia (5) is intentionally smaller because most documents are resident-visible. The implementation must enforce audience server-side (RLS or row-level scope), not just hide in UI.
- Mobile (RN) docs list: `mobile-documents` sitemap ref exists but no design in this bundle iteration. Mobile redesign-status stays `not-started` until a mobile-specific artboard ships.
- Filter "Status" 3-state (Publikované / Návrhy / Archivované) maps to the document state machine — Návrhy are visible only to managers and reviewers, Archivované are read-only and excluded from default search.
- 5-category default segmented (Všetky / AGM / Ročné / Poistenie / Faktúry) covers ~75% of typical building documents. Zmluvy + Technická dokumentácia accessed via sidebar checkbox or "+ ďalšie" overflow chip.
- 2026-08-31 — realtime sync (PR #2889): the `documents` query root now auto-refetches on a `notification.created` frame with `category: documents`. Previously `WebSocketContext.eventToQueryKeys` keyed on dead `entity:*` names the api-server never emits (100% dead sync); PR #2889 added `categoryToQueryKeys.documents → ['documents']` and wired `App.tsx`'s `onEntityEvent`. REST wiring unchanged.

## Agent Log

<!-- newest entries on top -->

- 2026-08-31 — agent: screen-map-drift-pr-2889-ppt — reconcile drift from PR #2889 (realtime ws→query-invalidation fix). ppt-web `WebSocketContext` re-keyed cache invalidation to canonical `domain.action` events and its `notification.created` subscriber routes by `payload.category`, so `category=documents` now invalidates the `documents` root; `App.tsx` wires `onEntityEvent → queryClient.invalidateQueries`. No route/component/endpoint/status change — frontmatter unchanged; docs-only.

- 2026-05-27 — agent: gap-7a-4 review fixes — extracted useDocumentDownload hook (error toast on failure, blob URL); download now shared between RowDownloadButton and DownloadButton via single hook
- 2026-05-27 — agent: gap-7a-4 — added preview (eye) + download action buttons to each doc row in DocumentsBrowse; DocumentPreviewModal renders PDF inline (react-pdf via PdfPreview) or image (<img>) with download button in header; action buttons hidden by default, revealed on row hover/focus/selected via CSS opacity transition
- 2026-05-27 — agent: gap-7a-2 review fixes — extracted useDocumentDownload hook (error toast on failure, blob URL pattern); added full Tab/Shift-Tab focus trap to MoveFolderDialog
- 2026-05-24 — agent: gap-7a-2 follow-up — added ppt/document-folders as child relatedScreen (FolderTreePage ships on /documents/folders; omitted from PR #451 commit)
- 2026-05-24 — agent: gap-7a-3 — added DocumentsBrowse component wired to RLS-aware GET /api/v1/documents; surfaces audience (access_scope) filter chips and status segmented control (Publikované/Návrhy/Archivované); promoted ppt-web.apiStatus partial→complete; DocumentsPage browse tab now uses real data, not placeholder
- 2026-05-09 — agent: design analyzed (pages/ppt-documents.html — 4 artboards: loaded-12-3-selected / empty / loading / error); flipped ppt-web redesignStatus → in-progress (drift note: was shipped, now redesign in flight); attached designSource; populated functionality checklist (8 sections), 4 states, design-specific notes (filter-AND-OR semantics + ZIP server-side + audience RLS); declared 7 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
