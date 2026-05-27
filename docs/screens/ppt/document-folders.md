---
id: ppt/document-folders
name: Document Folders
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/documents/folders"
    component: FolderTreePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/documents
    rel: parent
  - id: ppt/document-detail
    rel: sibling
epics:
  - Epic-39
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Layout
- [x] [w] Left sidebar: FolderTree (15rem fixed) with all-documents root entry
- [x] [w] Right content panel: document list scoped to selected folder (DocumentsBrowse for root, FolderDocuments for folder)
- [x] [w] Breadcrumb: Dokumenty / Priečinky (page nav); FolderBreadcrumb in content header (folder path)
- [x] [w] Responsive: sidebar collapses to 12rem max-height on mobile, stacks vertically

### FolderTree
- [x] [w] 5-level max nesting (add-child button hidden at depth 4)
- [x] [w] Expand / collapse chevron per node; leaf nodes show dot instead
- [x] [w] Inline rename: click pencil icon → input → Enter to commit / Escape to cancel
- [x] [w] Delete: click trash icon → modal with two options (keep docs / cascade delete)
- [x] [w] Add child: click + icon on any non-max-depth node
- [x] [w] Add root: + button in tree header
- [x] [w] New folder inline input with autoFocus; blur commits if non-empty
- [x] [w] Document count badge per folder node
- [x] [w] "Všetky dokumenty" root selection (clears folder filter)

### Document panel
- [x] [w] Loading skeleton (5 rows)
- [x] [w] Error state with retry
- [x] [w] Empty state with link to /documents/upload
- [x] [w] Document rows: file icon + title + category + size
- [x] [w] "Show all N documents →" link when folder has >50 docs
- [x] [w] FolderBreadcrumb: shows ancestor path of selected folder; each crumb navigates
- [x] [w] Move-to-folder button (folder icon) on each document row (hover-visible)
- [x] [w] MoveFolderDialog: inline tree picker, destination label, confirm/cancel buttons

### Move documents
- [x] [w] useMoveDocument hook from @ppt/api-client wired to POST /api/v1/documents/{id}/move
- [x] [w] Success/error toast after move; folder tree + document lists invalidated on success
- [x] [w] Available from DocumentsBrowse (root view) and FolderDocuments (folder-scoped view)

### Backend constraints respected
- [x] [w] Circular reference prevention: handled by backend 409 (UI shows api error)
- [x] [w] 5-level depth enforced client-side (add-child disabled) + server-side (422)

## States

- **Empty tree**: "Žiadne priečinky" + create CTA.
- **Loading**: shimmer skeleton rows.
- **Error (tree)**: danger text + retry.
- **Folder selected**: content panel shows scoped document list.
- **Root selected**: content panel shows DocumentsBrowse (full list).
- **Delete confirm**: modal overlay with cascade / non-cascade options.

## Notes

### Specific (recent)

- Backend `GET /api/v1/documents/folders/tree?building_id=<id>` returns up to 5 levels of `FolderTreeNode[]` with nested `children[]` + `document_count`.
- Circular check is backend-enforced on folder `PUT` (parent_id change) — the UI doesn't special-case it beyond showing the API error message.
- The `useDeleteFolder(cascade=true)` removes documents too — hence the 2-option delete dialog to prevent accidental data loss.
- Route: `/documents/folders` — accessible via "Priečinky" button on DocumentsPage header.

## Agent Log

<!-- newest entries on top -->

- 2026-05-26 — agent: gap-7a-2-folder-ui — added MoveFolderDialog (folder picker modal), FolderBreadcrumb (path navigation), buildFolderCrumbs helper; wired move-documents action in DocumentsBrowse and FolderTreePage; success/error toasts on move; breadcrumb in content-panel header shows selected folder path; apiStatus remains complete
- 2026-05-24 — agent: gap-7a-2 — created FolderTree component (5-level expand/collapse, inline rename, delete dialog, new-folder inline input, document count badges); created FolderTreePage with left sidebar + right document panel; wired /documents/folders route; added Priečinky button to DocumentsPage header; created this screen-map
