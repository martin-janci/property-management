---
id: ppt/file-dispute
name: File Dispute (5-step wizard)
product: ppt
sitemapRefs:
  ppt-web: ppt-dispute-new
implementations:
  ppt-web:
    component: FileDisputePage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - disputes_create
relatedScreens:
  - id: ppt/disputes
    rel: parent
  - id: ppt/dispute-detail
    rel: sibling
epics:
  - Epic-77
sharedComponents:
  - wizard
  - stepper
  - radio-cards
  - chip-group
  - address-combobox
  - file-upload
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-file-dispute.html
    frame: step1-category+severity / step3-attachments-uploading / step5-review / submitted-D-2026-0058 / step3-validation-errors
useCases:
  - UC-38
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Wizard chrome
- [ ] [w] Manager chrome + breadcrumb `Hlásenia / Spory / Nový`
- [ ] [w] Stepper: 1 Kategória → 2 Strany → 3 Opis → 4 Riešenie → 5 Súhrn
- [x] [w] Auto-save indicator: "Návrh uložený · pred 14 sekundami" (shipped on the single-page form: `useDraftStorage` + `DraftSavedIndicator`, AC-4)
- [ ] [w] Right toolbar: ghost "Uložiť návrh a zavrieť" + "Zrušiť" link

### Step 1 · Kategória + Závažnosť
- [ ] [w] 6 category radio-cards (Hluk · Poškodenie · Fakturácie · Spoločné priestory · Domáce zvieratá · Iné)
- [ ] [w] 4 severity radio-cards (Nízka / Stredná / Vysoká / Eskalujúce)

### Step 2 · Strany sporu
- [ ] [w] Sťažovateľ pre-filled (combobox per `forms/address-combobox.html`)
- [ ] [w] Druhá strana combobox
- [ ] [w] Optional witnesses/experts repeating add-row

### Step 3 · Opis + prílohy
- [ ] [w] Title (required, 120 char limit)
- [ ] [w] Description textarea (min 30 chars)
- [ ] [w] Date/time of incident
- [ ] [w] Repeating? checkbox + frequency text
- [ ] [w] Attachments (JPG/PNG/PDF/MP3/MP4, max 50 MB/file)

### Step 4 · Preferovaný spôsob riešenia
- [ ] [w] 4 radio-cards: Dohovor s druhou stranou · Mediácia správcom · Formálne hlasovanie · Eskalácia mimo systému
- [ ] [w] Optional consent checkbox-card "Súhlasím že obe strany dostanú prístup k spisu"

### Step 5 · Súhrn (Review)
- [ ] [w] Recap card with 4 sections + per-section "Upraviť →" links
- [ ] [w] Confirm checkbox: "Potvrdzujem že informácie sú pravdivé..."
- [ ] [w] Submit "Otvoriť spor" disabled until checkbox

## States

- **Step 1**: category + severity selected, "Pokračovať" enabled
- **Step 3 · attachments uploading**: 1 done + 2 uploading queue rows
- **Step 5 · review**: final state before submit
- **Submitted · success**: success card "Spor otvorený · D-2026-0058" + 2 actions
- **Step 3 · validation errors**: top banner.err + inline field errors

## Notes

### Broader context

UC-38 dispute creation flow. 5-step wizard balances thoroughness (legal record) with speed (resident must be willing to complete). Auto-save between steps prevents loss.

### Specific (recent)

- AC-4 (draft auto-save) is now SHIPPED on the current single-page form (it remains a checklist item for the 5-step wizard redesign too). New `useDraftStorage<T>(key)` hook (`features/disputes/hooks/useDraftStorage.ts`) is a generic debounced (800 ms) localStorage-backed draft store: synchronous restore-on-mount, `savedAt` epoch, `save()` / `clear()`. `FileDisputePage` seeds its react-hook-form `defaultValues` from the restored draft, persists on every change (skipped while submitting), shows a "Restored your saved draft." notice when a draft was recovered, and renders `DraftSavedIndicator` (`components/DraftSavedIndicator.tsx`, "Draft saved · 14s ago", re-renders on a 10 s tick). The draft is cleared only on a successful submit — `FileDisputePageRoute` now RE-THROWS on filing failure so the page keeps the draft for retry; the page catches that rejection so react-hook-form doesn't bubble an unhandled rejection. Evidence files are deliberately NOT persisted (File objects aren't serialisable). All draft copy is rendered via `t(key, defaultValue)` with English defaults, so the feature is fully functional today; FAST-FOLLOW: add localized `disputes.draft*` keys to the 6 message bundles (en/sk/cs/de/pl/hu) — deferred from this PR to keep the diff source-only. Tests: `useDraftStorage.test.tsx` (7) + 5 new draft tests in `FileDisputePage.test.tsx`; route stub updated to swallow the now-re-thrown rejection.
- Resolution preference option 4 ("Eskalácia mimo systému") is the legally-sensitive choice — disclaimer copy may need legal review.
- Attachments support audio (MP3) and video (MP4) — useful for noise complaints with recordings.
- Auto-save fires on step transition, not on every keystroke (rate-limit consideration).
- Route wrapper extraction (PR #1100): `FileDisputePageRoute` is no longer an inline wrapper inside `App.tsx`. It now lives in its own file `frontend/apps/ppt-web/src/features/disputes/pages/FileDisputePageRoute.tsx` (exported), is mounted by the disputes route group `frontend/apps/ppt-web/src/routes/groups/disputes.tsx` (`<Route path="/disputes/new" element={<FileDisputePageRoute />} />`), and is now covered by a route-level test `FileDisputePageRoute.test.tsx`. The presentational page (`FileDisputePage`) stays pure — all API side-effects (create + sequential evidence upload + nav) live in the route wrapper. This resolved the "route-level orchestration untested" gap noted in the 2026-06-04 test-gap log.
- Partial-fail handling (PR #839, #627): when the dispute is created but some evidence files fail to upload, `FileDisputePageRoute` shows a localized warning toast (`disputes.evidenceUploadErrorsMsg` with `{{count}}` interpolation, present in all 6 locales) and threads the failed `PendingEvidence[]` to `/disputes/:id` via router state (`navigate(..., { state: { failedEvidence } })`). The detail page does not consume that state yet — `TODO(#627)` tracks the evidence-retry UI. A 1-shot retry on `apiUploadEvidence` was deliberately deferred (needs backoff + idempotency design; the upload endpoint is not idempotent today).

## Agent Log

<!-- newest entries on top -->

- 2026-06-13 — agent: feat-dispute-filing-flow-task-checklist-unchecked-frontend. Implemented AC-4 draft auto-save on the shipped single-page filing form (the one remaining unimplemented AC, repeatedly flagged in this log). Added generic `useDraftStorage<T>(key)` hook (debounced 800 ms localStorage; restore-on-mount, `savedAt`, `save`/`clear`, private-mode no-op guard) + `DraftSavedIndicator` pill ("Draft saved · 14s ago", 10 s tick). Wired into `FileDisputePage`: restored draft seeds rhf `defaultValues`, a watch→save effect persists edits (skipped while submitting), restore notice shown, draft cleared on successful submit only. `FileDisputePageRoute` now re-throws on filing failure so the draft survives a failed submit; the page catches it (no unhandled rejection). All draft copy renders via `t(key, defaultValue)` English defaults (feature works today); adding localized `disputes.draft*` keys to the 6 message bundles is a documented fast-follow (kept this diff source-only). Tests: new `useDraftStorage.test.tsx` (7) + 5 draft tests in `FileDisputePage.test.tsx`; updated `FileDisputePageRoute.test.tsx` stub to swallow the re-thrown rejection. Verify: `@ppt/web` typecheck + biome clean, `vitest run src/features/disputes` 82/82 green, `pnpm -F @ppt/web build` exit 0. Checked the wizard-chrome "Auto-save indicator" checklist box (now shipped on the single-page form). buildStatus/redesignStatus/apiStatus unchanged (shipped / in-progress / complete) — the 5-step wizard redesign is still in-progress.
- 2026-06-08 — agent: verify-dispute-filing-flow-ac-coverage. Verified story 80-2 dispute-filing flow against its ACs. AC-1 (6 type radio-cards + required radiogroup), AC-2 (EvidenceUploader — confirmed present + tested; the stale sprint-status note flagging it "missing" was corrected), AC-3 (subject min5/max200 + description min30 + live char counter), AC-5 (submit forwards `{values, evidence}`; disabled + "Filing…" spinner while submitting) are all MET and covered by `FileDisputePage.test.tsx`. Route-level orchestration (create → sequential evidence upload → success/warning toast → `navigate(/disputes/:id, {state})`) is covered by `FileDisputePageRoute.test.tsx` (happy / partial-fail / errored-filtered / create-fail). Added the missing error-path test (e): a non-Error rejection falls back to the localized `auth.unexpectedError` toast message with no navigate. Filing suite now 22 tests, `vitest run` exit 0; `@ppt/web typecheck` + `biome check` exit 0. Story 80-2 STAYS partial (NOT promoted) — AC-4 (draft auto-save / `useDraftStorage`) is unimplemented and the redesigned 5-step wizard remains `in-progress`. Corrected the stale "EvidenceUploader missing / AC-2 not met" note in `_bmad-output/.../sprint-status.yaml` and narrowed the `coverage.json` gap for 80-2 to AC-4 only. No route added/removed; buildStatus/redesignStatus/apiStatus unchanged. The wizard checklist above tracks the redesign target (5-step), not the shipped single-page form, so its boxes intentionally remain unchecked.
- 2026-06-07 — agent: test-gap-screen-map-drift-pr-1100-ppt. Reconciled screen-map with PR #1100, which extracted `FileDisputePageRoute` out of `App.tsx` into its own file (`features/disputes/pages/FileDisputePageRoute.tsx`) mounted by the disputes route group (`routes/groups/disputes.tsx`, `/disputes/new`), and added a route-level test (`FileDisputePageRoute.test.tsx`). Fixed stale frontmatter `component: FileDisputeWizard` → `FileDisputePage` (matches `@ppt/sitemap` `ppt-dispute-new`). Updated Notes > Specific: dropped the "un-exported inline wrapper in App.tsx" description (now false) and documented the extraction + that the 2026-06-04 "route-level orchestration untested" gap is now closed. No route added/removed; sitemap ref `ppt-dispute-new` (`/disputes/new`) unchanged. buildStatus/redesignStatus/apiStatus unchanged (shipped / in-progress / complete). `/screens validate` clean.
- 2026-06-04 — agent: test-gap-80-2-dispute-filing-ac-coverage. Audited story 80-2 AC coverage of the filing surface. AC-1 (type radio-cards), AC-3 (subject/description validation + counter), AC-5 (submit contract) already covered in FileDisputePage.test.tsx; AC-2 (EvidenceUploader) covered in EvidenceUploader.test.tsx. Closed the remaining gaps with 7 new FileDisputePage tests: evidence files forwarded through onSubmit (valid + error-tagged + empty), subject max-200 boundary, evidence section + uploader render, optional other-party (respondent) selector render/hide + value forwarding. AC-4 (draft auto-save / useDraftStorage) remains unimplemented (deferred — no filing-surface code to test). KNOWN GAP: FileDisputePageRoute orchestration in App.tsx (sequential apiUploadEvidence + partial-fail warning toast + navigate(`/disputes/:id`, { state: { failedEvidence } })) is still untested at the route level — it is an un-exported inline wrapper, so a route-level test needs an extraction refactor out of scope for a test-gap task. Build/redesign/api frontmatter unchanged. `/screens validate` clean.
- 2026-06-04 — agent: screen-map drift backfill for PR #839 (#627). FileDisputePageRoute in App.tsx: localized the evidence-upload partial-fail toast (`disputes.evidenceUploadErrorsMsg` switched to `t(key, { count })` with `{{count}}` added to en/sk/cs/de/pl/hu) and now threads the failed `PendingEvidence[]` to `/disputes/:id` via router state (`navigate(..., { state: { failedEvidence } })`); `TODO(#627)` left for the detail-page evidence-retry UI; transient-error retry on `apiUploadEvidence` deferred. No new screen/route; frontmatter unchanged (buildStatus shipped, apiStatus complete). Documented under Notes > Specific. `/screens validate` clean.
- 2026-05-25 — agent: gap-80-2-dispute-filing-ui: added EvidenceUploader.tsx (drag-drop + click, JPG/PNG/WebP/MP3/MP4/PDF, 50 MB/file, 10 files, per-file description + status); rewrote FileDisputePage.tsx with radio-card type selector + inline validation (touched/submitted flags) + evidence upload section (AC-1,2,3 met); App.tsx FileDisputePageRoute wires useCreateDispute then raw apiUploadEvidence sequentially post-creation; navigates to /disputes/:id on success; typecheck+biome+vite-build all clean; apiStatus partial → complete.
- 2026-05-25 — agent: verified story 80-2 AC coverage. FileDisputePage exists at /disputes/new with useCreateDispute mutation (AC-1/3/5 partial). EvidenceUploader.tsx MISSING (AC-2 not met). useDraftStorage.ts MISSING (AC-4 not met). Post-submit nav goes to list not detail. Story 80-2 stays partial; apiStatus stays partial.
- 2026-05-09 — agent: integrated Batch C (pages/ppt-file-dispute.html — 5 artboards: step 1 / step 3 uploading / step 5 review / submitted / step 3 validation errors); flipped redesignStatus → in-progress; attached designSource; populated 6 sections + 5 states + 3 notes; declared 7 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
