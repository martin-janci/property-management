---
id: ppt/upload-document
name: Upload Document
product: ppt
sitemapRefs:
  ppt-web: ppt-document-upload
implementations:
  ppt-web:
    component: DocumentUploadPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - documents_upload
relatedScreens:
  - id: ppt/documents
    rel: parent
  - id: ppt/document-detail
    rel: sibling
epics:
  - Epic-39
sharedComponents:
  - file-upload
  - dropzone
  - text-input
  - select
  - radio-cards
  - switch
  - toast
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-upload-document.html
    frame: loaded-4-files-mixed-states / empty-drag-over / success-toast
useCases:
  - UC-08
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with Dokumenty active
- [ ] [w] Breadcrumb "Dokumenty / Knižnica / Nahrať"
- [ ] [w] Page header: H1 "Nahrať dokument" + right "Zrušiť" ghost (returns to documents list)

### Split layout
- [ ] [w] 2-column: form column (flex 1) + right rail (320px) with tips + summary
- [ ] [w] Below 1024px: rail collapses below form

### Files card (dropzone + queue)
- [ ] [w] Card heading "Súbory" + sub copy listing supported types (PDF, DOCX, XLSX, JPG, PNG, ZIP) + max-size (25 MB per file)
- [ ] [w] Dropzone tile (`.dz`): icon + h4 "Pretiahnite súbory sem" + "alebo <a>vyberte z disku</a>" + meta line
- [ ] [w] Drag-over state: brand-soft bg + brand-600 dashed border + accent icon + "Pustite pre nahranie" + "<n> súbory pripravené · <total>"
- [ ] [w] File queue (per `forms/file-upload.html`): each file row has type-icon (3-letter colored) + info (filename + status line + progress bar) + actions
- [ ] [w] Done state: success-soft border-left + check + "Dokončené · <size>"; first done file gets "HLAVNÝ" pill (drives auto-fill of title)
- [ ] [w] Uploading state: progress bar fills + "Nahráva sa · <up> kB / <total> kB · <pct>%"; cancel × icon-btn
- [ ] [w] Failed state: danger-soft border-left + circle-info icon + "Zlyhalo: prekročený limit (25 MB)" or other reason; "Skúsiť znova" retry button + remove ×

### Metadata card
- [ ] [w] Heading "Metadáta" + sub "Vyplnené metadáta sa použijú pre všetky súbory."
- [ ] [w] Title text-input (required) — auto-filled from "HLAVNÝ" file name; help text below explains
- [ ] [w] Row of 2: Category select (AGM / Ročné správy / Poistenie / Faktúry / Zmluvy / Technická dokumentácia) + Year number-input
- [ ] [w] Audience radio-cards (3-up, per `forms/radio-cards.html`): Všetci rezidenti (32) · Iba vlastníci (24) · Iba správcovia (3) — counts come from current building roster
- [ ] [w] Description textarea (optional)
- [ ] [w] Switch row: "Poslať push notifikáciu" + sub "Rezidenti dostanú upozornenie hneď po publikovaní" — default ON
- [ ] [w] Switch row: "Uložiť ako návrh" + sub "Dokument zostane neviditeľný pre rezidentov, kým ho nepublikujete" — default OFF

### Submit bar
- [ ] [w] Helper text: "<b>3 zo 4 súborov pripravené</b> · odhad 12 s do dokončenia"
- [ ] [w] "Uložiť návrh" ghost (saves draft regardless of upload completion)
- [ ] [w] "Publikovať" primary — disabled while files in flight or any failure unresolved; label changes to "Publikovať (čaká na upload)" during in-flight

### Right rail · Tips
- [ ] [w] Card "Tipy" with 3 illustrated tips:
  - **Hlavný súbor** — first done file becomes the preview; rename in queue if needed
  - **Verzie** — for updates, open detail and use "Nahrať novú verziu" instead of duplicating
  - **Veľké súbory** — over 25 MB → split or ZIP-with-password (password in notes)

### Right rail · Súhrn
- [ ] [w] kv list: Súbory (4) · Celková veľkosť (2,3 MB tabular-nums) · Publikum (Všetci rezidenti, brand-600) · Notifikácia (32 príjemcov, success-600)

### Empty (drag-over)
- [ ] [w] Standalone card variant: dropzone in `.over` state with brand-soft bg + accent icon + "Pustite pre nahranie" + count of dragged files

### Success (post-publish)
- [ ] [w] Toast slides in: success-soft tile + check icon + "Dokument publikovaný · <count> rezidentov upozornených" + link to detail
- [ ] [w] Auto-dismiss 5s; respects reduced-motion (fade only)

### Error (per-file)
- [ ] [w] Each failed file shows danger-soft state inline; aggregate failure surfaces in submit bar count ("3 zo 4 pripravené") — submit stays disabled until cleared

### Locale + theme switcher
- [ ] [-] preview-bar with Theme + Locale toggles (SK/CS/DE/EN)

## States

- **Empty (drag-over)**: dropzone with `.over` styling and brand-tinted icon + count of dragged files. No queue.
- **Initial (no files)**: dropzone idle + empty form fields with placeholders + submit disabled.
- **Loaded · 4 files mixed**: 1 done (PDF AGM minutes, marked HLAVNÝ) + 2 uploading (XLS rozpočet 78%, PDF poistka 24%) + 1 failed (DOCX over-size 31.4 MB); metadata auto-filled from first done; submit disabled with "(čaká na upload)" label.
- **Submitting**: all uploads complete, submit clicked, button shows spinner; metadata locked.
- **Success**: toast confirmation + redirect to detail OR documents list (TBD per UX preference).
- **Error per-file**: failed row gets retry CTA; aggregate count in submit-bar reflects pending failures.

## Notes

### Broader context

UC-08 document upload — single-step form. Multi-file with per-file progress and retry. Metadata applies to all files in the batch (single category, single audience, single notification setting); per-file overrides happen later via detail page. The "HLAVNÝ" pill on the first-done file drives auto-fill of the title — important UX shortcut, but allow override.

### Specific (recent)

- **Direct-to-S3 upload wired (gap-84-1):** `DocumentUpload` now uploads via
  `useUploadDocumentDirect` (api-client), which does `POST /api/v1/documents/upload-url`
  → presigned `PUT` straight to S3 → `POST /api/v1/documents` to register. Bytes
  no longer proxy through the api-server multipart `/upload` route. Per-file
  progress is driven by the S3 PUT phase. The legacy multipart `useUploadDocument`
  hook is still exported for callers that need the byte-proxy path. NB: a backend
  PR (#2339) is hardening the presigned endpoint (org-scoped `file_key` +
  signed Content-Length) — the client already sends the signed `Content-Type`.
- The bundle uses **single-screen layout** (dropzone + metadata + submit on one page) instead of a multi-step wizard. This is intentional for the common case (1–4 files, single batch). For 10+ files or split-batch needs, document a future "advanced upload" flow.
- File-icon block uses 3-letter colored tags matching `forms/file-upload.html`: PDF red, XLS green, DOC blue, JPG amber, PNG cyan, ZIP gray, generic gray for unknown types.
- "HLAVNÝ" pill is auto-assigned to the first successful upload — the design states "Premenujte ho v zozname, ak chcete iný" (rename in list to swap). Production must allow per-file context menu to manually set "Make primary".
- Per-file states: `.done` (success-soft border + check + 100% bar), default uploading (brand-600 progress bar fill), `.err` (danger-soft + alert + retry button). The bar fill respects `--accent` token in light, brightened in dark.
- Error message specificity matters: "prekročený limit (25 MB)" not "upload failed". Implementation must surface real reason (size, MIME, network, server) per error class.
- Submit button label dynamically reflects state: "Publikovať" / "Publikovať (čaká na upload)" / spinner + "Publikujem…" / disabled until all files done OR explicitly cleared.
- "Uložiť návrh" ghost button bypasses upload completion — saves what's currently uploaded as a draft document, even with failed files removed. Prevents users from losing metadata work after a flaky upload session.
- Push notification toggle defaults to ON because the dominant case is "newly published, residents should know". The toggle label preview ("32 príjemcov" in súhrn rail) updates live based on selected audience.
- Audience radio-cards use real building counts (Všetci rezidenti 32 / Iba vlastníci 24 / Iba správcovia 3); these come from the building roster API at page load. Don't hardcode.
- Description textarea is intentionally optional — many documents are self-describing by category + title. Don't make it required.
- Drag-over state uses `.over` modifier on the dropzone — must hook to `dragenter` and `dragleave` events at the **window level** to handle drag-from-anywhere, not just from over the dropzone.
- 4 locales: SK / CS / DE / EN.

## Agent Log

<!-- newest entries on top -->

- 2026-07-15 — agent: wired direct-to-S3 upload (gap-84-1) — added `createUploadUrl` + `uploadDocumentDirect` bindings and `useUploadDocumentDirect` hook to @ppt/api-client; switched `DocumentUpload` to the direct hook (upload-url → presigned PUT → register). Frontend-only; backend endpoint landed in #2309.
- 2026-05-09 — agent: design analyzed (pages/ppt-upload-document.html — 3 artboards: loaded-4-files-mixed / empty-drag-over / success-toast); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (10 sections), 6 states, design-specific notes (HLAVNÝ auto-detect + per-file error specificity + audience-driven notification count + draft-bypass-upload-completion); declared 8 sharedComponents; added 1 relatedScreen (document-detail sibling)
- 2026-05-08 — init: created from scan (source: sitemap)
