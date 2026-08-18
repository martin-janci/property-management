---
id: ppt/announcements
name: Announcements
product: ppt
sitemapRefs:
  mobile: mobile-announcements
implementations:
  ppt-web:
    component: AnnouncementsPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    component: AnnouncementsScreen + AnnouncementDetailScreen
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: complete
endpoints:
  - announcements_list
  - announcements_get
  - announcements_mark_read
  - announcements_acknowledge
  - announcements_get_acknowledgments
  - announcements_comments_list
  - announcements_comments_create
  - announcements_comments_delete
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
    file: guest-registration-v2-design-system/project/pages/ppt-announcements.html
    frame: loaded-2-selected-1-pinned / empty / loading-8-skel / error-503
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/ppt-web/announcement-detail.html
    frame: announcement-detail
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobAnnouncementsScreen + MobAnnouncementDetailScreen
useCases:
  - UC-02
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header (web — pattern from detail view)
- [ ] [w] PPT manager header (60px, sticky), Announcements tab active

### List view (web — directly designed)
- [ ] [w] Page header: H1 "Oznamy" + count chip "32 publikovaných · 4 koncepty · 2 plánované"
- [ ] [w] Right toolbar: "Exportovať PDF" secondary + "+ Nový oznam" primary
- [ ] [w] **Filter sidebar** (220px) — Stav 5-state machine (Koncept / Plánované / Publikované / Archivované / Pripnuté) + Kategória + Publikum + Čítaná miera (range-slider 0–100% per `forms/range-slider.html`)
- [ ] [w] Toolbar: search + segmented category chips + sort dropdown (Najnovšie / Najmenej prečítané / Pripnuté najskôr / Plánované najskôr)
- [ ] [w] Card list (NOT table — denser): pin glyph left + H3 title + 2-line excerpt + meta row (status + audience + category + author + relative time) + read-rate progress bar + "<n>/<total> potvrdili (NN%)" muted right + 3-dot menu
- [ ] [w] Bulk-action bar (≥1 selected): Pripnúť · Zrušiť pripnutie · Archivovať · Plánovať na… · Exportovať vybrané PDF
- [ ] [w] Pagination footer per `forms/pagination.html`

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

- **Empty (list)**: megaphone-icon tile + "Žiadne oznamy" + body + primary "+ Nový oznam" + secondary "Importovať z Faults" link. **Implemented** (PR #1033): `AnnouncementList` shows "No announcements found." when not loading, not error, and the list is empty.
- **Loading (list)**: 8 skeleton cards (pin skel + 2 line skels + meta skels + progress-bar skel). Code currently renders a single spinner while `isLoading` (skeleton-card treatment still TBD).
- **Error 503 (list)**: danger tile + retry; toolbar + sidebar interactive. **Implemented** (PR #1033): `AnnouncementList` renders an inline `role="alert"` danger tile (red-50 bg / red-200 border) with `announcements.failedToLoad` ("Failed to load announcements") + a `common.retry` button calling `onRetry` (route wrapper `refetch()`). Threaded `AnnouncementsPageRoute → AnnouncementsPage → AnnouncementList` via `isError`/`onRetry` props; mutually exclusive with loading/empty/loaded.
- **Loaded (list)**: 8 cards covering 5 states; 2 selected with bulk bar; 1 pinned at top
- **Detail (existing)**: as designed — published with delivery + ack stats

## Notes

### Broader context

UC-02 announcements — manager-published, resident-acknowledged messages. The detail view exposes the **read-rate** as a primary KPI, making it the manager's accountability surface (did the message land?). Pin + audience-targeting are the differentiated affordances over plain email.

### Specific (recent)

- **Drift note**: Sitemap doesn't include a `ppt-announcements` route; bumped `ppt-web.buildStatus` from `n/a` → `planned` to track. Add route at implementation.
- **List view now designed** (Batch D delivery): full filter sidebar + card list + bulk-action bar pattern matches `ppt-documents.html`. List pattern uses cards (not table) because of the read-rate progress bar — denser visual.
- **Read-rate filter** (range-slider 0–100%) is unique to this surface — useful for finding low-engagement announcements.
- Pinned indicator uses Unicode 📌 emoji in the design — per SKILL.md, must migrate to Lucide `pin` (or inline SVG with the same stroke). Don't ship 📌.
- Callout block styling uses `--warning-50` and `--warning-500` (with fallbacks `#fffbeb` / `#f59e0b`) — token-driven; ink `#78350f` is hard-coded; surface a `--warning-ink-strong` token for this.
- Read-rate progress bar uses `--success-500` fill — only because high read-rate is good. If read-rate is low, no design treatment for that — flag at implementation: should we color-shift below a threshold (e.g. <50% gets warning bg)?
- Ack list `mask-image: linear-gradient(#000 75%, transparent)` for fade-out — works but Safari needs `-webkit-mask-image` prefix; respect `prefers-reduced-motion` (no animated reveal).
- Audience meta currently free text ("All residents") — must become a tokenized chip set (All residents / Owners only / By unit / By role) tied to the audience-selector when composing.
- Multi-language announcements (`Slovak · English` in details kv) — implementation must support per-language body editing + per-language read receipts; v1 may ship single-language only.
- Mobile bottom-nav must drop the legacy 📢 emoji per SKILL.md non-negotiable.
- **List error/retry wired (PR #1033):** the designed `error-503` artboard now has a code counterpart — `AnnouncementList` owns the loading/error/empty triad and the error tile carries a retry button (`onRetry → refetch()`, i18n `announcements.failedToLoad` + shared `common.retry`). Skeleton-card loading treatment (8 cards per design) is still TBD; code shows a single spinner.
- **Mobile comments out-of-scope for Story 6.3**: Story 6.3's task breakdown (`_bmad-output/.../stories/6-3-announcement-comments-discussion.md`) scopes the comment feature to **backend + ppt-web only** — there is no mobile task; ppt-web Task 5 was the sole UI surface (originally "future", now shipped as `AnnouncementComments`). Mobile RN intentionally renders only the comment **count** (`AnnouncementsScreen` badge + `AnnouncementDetailScreen` stat, both derived from `comment_count`); the full read/post/reply thread is deliberately NOT built on mobile for this story and belongs to a future dedicated mobile story. So the "no mobile comments UI" observation is by-design for 6.3, not an open 6.3 gap. This screen's `mobile.buildStatus` stays `in-progress` for the broader mobile detail surface (6-2/6-4 + skeleton), independent of the comments decision.
- **Comment-threading depth (Story 6.3)**: `AnnouncementComments` renders the discussion thread under the detail view (only when `commentsEnabled`). Threads are **capped at depth 3** for the reply affordance — the `Reply` button is suppressed once `depth >= 3`, and indentation (`marginLeft`) is clamped at `min(depth, 3) * 24px`. Deeper replies still render (so a server that returns depth-4+ data won't lose comments) but can't be replied to from the web UI; new replies fold into the depth-3 ancestor. Soft-deleted comments collapse to a `[This comment has been deleted]` tombstone (no content, no actions). Delete affordance shows for the comment owner OR a manager (`technical_manager` / `property_manager`). Any change to the max depth must stay in sync with the backend's comment-nesting limit and the mobile detail screen.
- **Test follow-up (PR #1418 / #1349):** mobile `AnnouncementsScreen.test.ts` — the "beyond a week" `formatRelativeDate` case previously compared the output against a live `toLocaleDateString('en-US', …)` call (tautological / self-referential — it could not fail if the formatter changed); PR #1418 replaced it with a fixed `'May 1'` expected value so the test actually pins the formatter's output. Test-quality fix only — no change to the `formatRelativeDate` helper, mobile screen behavior, API wiring, or status (build/api unchanged).

## Agent Log

<!-- newest entries on top -->

- 2026-07-04 — agent: screen-map-drift-pr-1418-ppt — reconcile the remaining PR #1418 (BIT-21) drift: the PR also touched the mobile `AnnouncementsScreen.test.ts` (`formatRelativeDate` suite), replacing a tautological formatter-vs-formatter assertion with a fixed `'May 1'` expected value (issue #1349). Test-quality fix only — no change to the `formatRelativeDate` helper, screen behavior, API wiring, or status. (The faults half of PR #1418 was reconciled in `faults-list.md` on 2026-06-22.) Added Notes > Specific entry; frontmatter unchanged; docs-only.

- 2026-07-02 — agent: verify-6-3-announcement-comments-discussion (closeout) — Re-verified Story 6.3 end-to-end on dev; all three surfaces confirmed and traceability reconciled. Backend: comments router registered (`routes/announcements/mod.rs` `mod comments; .merge(comments::router())`), list/create/delete handlers in `routes/announcements/comments.rs`, RLS repo methods + migration 00016 + threading/RLS regression tests. api-client: `listComments`/`createComment`/`deleteComment` ops + `useAnnouncementComments`/`useCreateAnnouncementComment`/`useDeleteAnnouncementComment` hooks (announcements/api.ts + hooks.ts, contract tests in api.test.ts). ppt-web: `AnnouncementComments` wired via `routes/groups/announcements.tsx` → `ViewAnnouncementPage` (`AnnouncementComments.test.tsx`, 14 cases). sprint-status already `6-3: done` (reconciled 2026-06-25). Recorded the explicit **out-of-scope decision** for mobile comments (Notes > Specific): Story 6.3 has no mobile task, so mobile-count-only is by-design, not a 6.3 gap — this closes the stale coverage "No mobile comments UI observed" flag. Screen `buildStatus` intentionally left `in-progress` (aggregates 6-2/6-4 + skeleton-card loading TBD). Docs/screen-map closeout only — no code change.

- 2026-06-25 — agent: verify-6-2-announcement-viewing-acknowledgment — Coverage 6-2 finish-to-done: verified ppt-web viewing + acknowledgment slice live (ViewAnnouncementPage renders readCount/acknowledgedCount + AcknowledgmentStatsPanel + Acknowledge/Mark-as-Read; auto-fire POST /{id}/read on load + POST /{id}/acknowledge; api-client useMarkReadAnnouncement/useAcknowledgeAnnouncement/useAnnouncementAcknowledgmentStats route through shared authenticatedFetchJson — no direct getToken). Story-gate #486 confirmed CLOSED/completed (2026-05-26). Flipped ppt-web buildStatus in-progress→shipped; sprint-status 6-2 ready-for-dev→done. Docs/status reconciliation only — no code change.

- 2026-06-25 — agent: verify-6-4-pinned-announcements — Story 6.4 (pinned announcements) verified functionally complete on dev and reconciled to `done` in sprint-status. Backend: PATCH/POST /api/v1/announcements/{id}/pin (lifecycle.rs pin_announcement) — manager-only, max-3-per-org cap enforced atomically via pin_with_cap_rls (SELECT FOR UPDATE vs TOCTOU race #518), auto-unpin scheduler (services/scheduler.rs auto_unpin_expired_announcements, 30-day cutoff), composite index idx_announcements_org_pinned_published (migration 00158). Repository: pin_with_cap_rls/unpin_rls/count_pinned_rls. Regression: announcement_tests.rs test_pin_announcement + test_auto_unpin_expired. Frontend ppt-web: PinnedAnnouncementsBand (features/announcements/components) — separate usePinnedAnnouncements query (pinned=true, staleTime 5 min, immune to list filters), AnnouncementCard Pin/Unpin button calling usePinAnnouncement mutation, AnnouncementList optimistic pinned-first sort. Tests: PinnedAnnouncementsBand.test.tsx (4 cases), AnnouncementList.test.tsx pin/unpin UI cases, announcements.list.route.test.tsx case (e); 838 tests pass. This screen stays buildStatus=in-progress: aggregates 6-2 viewing + skeleton-card loading still TBD; NOT flipped to shipped.

- 2026-06-25 — agent: verify-6-3-announcement-comments-discussion — Story 6.3 (announcement comments & discussion) verified functionally complete on dev and reconciled to `done` in sprint-status (epic-6 stories_completed 2→3). Backend comment CRUD (GET/POST /api/v1/announcements/{id}/comments + DELETE /{comment_id} in routes/announcements/comments.rs; RLS repo methods create_comment_rls/get_comments_rls/get_comment_replies_rls/delete_comment_rls; migration 00016; threading + soft-delete + author/manager moderation; threading + RLS regression tests) and ppt-web comment thread UI (AnnouncementComments wired into ViewAnnouncementPage with depth-3 reply cap, tombstones, optimistic add, owner/manager delete; @ppt/api-client list/create/delete ops + hooks; AnnouncementComments.test.tsx 14 cases) are all real, wired, and tested. This list/detail screen stays `buildStatus: in-progress` on purpose: mobile RN renders only the comment COUNT (AnnouncementsScreen badge + AnnouncementDetailScreen stat) — the full comment-thread read/post UI is NOT yet on mobile — and the screen also still aggregates 6-2/6-4 viewing/pinned items + skeleton-card loading (per States > Loading). The missing mobile comments thread is a separate follow-up, not part of this verify reconciliation.

- 2026-06-25 — agent: verify-6-1-announcement-creation-targeting — Story 6.1 (create + audience targeting) verified functionally complete on dev and reconciled to `done` in sprint-status. Backend create path (POST /api/v1/announcements, target_type all/building/units/roles, target_ids org-membership validation, RLS create_rls + targeting-visibility predicate, migrations 00015/00176, targeting-visibility tests) and ppt-web create/edit UI (CreateAnnouncementPage/EditAnnouncementPage → AnnouncementForm + TargetSelector, useCreateAnnouncement/useUpdateAnnouncement mutations) are all real and tested. The dedicated creation screen-maps (announcement-new.md / announcement-edit.md) are already `buildStatus: shipped`. This list/detail screen stays `in-progress` on purpose: it aggregates 6-2/6-3/6-4 viewing/comments/pinned work and still has open items (skeleton-card loading state TBD per States > Loading), so it is NOT flipped to shipped here.

- 2026-06-09 — agent: feat-6-3-announce-comments-web-ui — Coverage 6-3: added AnnouncementComments.test.tsx (14 cases) covering threading/replies, depth-3 reply cap, deleted-comment tombstone, add/reply wiring (parentId), owner/manager delete affordance, disabled + empty + loading states, Ctrl+Enter submit. Feature (component + api-client hooks + page wiring) already shipped under gap-6-3; this closes the test-coverage gap (mirrors #1190/#1196 pattern). Clarified comment-threading depth in Notes > Specific. ppt-web test/typecheck/biome clean.

- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `ViewAnnouncementPageRoute` in `frontend/apps/ppt-web/src/routes/groups/announcements.tsx` now renders its missing-param fallback via `t('errors.announcementNotFound', 'Announcement not found')` (was hardcoded English); `errors.announcementNotFound` key added to all locale bundles. No route or component change.

- 2026-06-09 — agent: Coverage 6-2 — announcement web viewing + acknowledgment verified live; apiStatus flipped to reflect wired backend; route-wiring tests added.

- 2026-06-05 — agent: test-gap-screen-map-drift-pr-1033-ppt — screen-map sync for PR #1033: AnnouncementsPageRoute now threads `isError`/`onRetry` (from `useAnnouncements` error + `refetch`) through AnnouncementsPage → AnnouncementList; AnnouncementList renders the designed error-503 tile as a `role="alert"` inline error + retry button (i18n `announcements.failedToLoad` + `common.retry`), mutually exclusive with loading/empty/loaded; added AnnouncementsPage.test.tsx regression (gap-79-1). Updated States (Empty/Loading/Error → Implemented) + Notes; docs-only, no code change here

- 2026-05-27 — agent: gap-6-2-announcement-read-receipt-retry — Story 6.2 retry: added AnnouncementDetailScreen (mobile) with auto-fire POST /read on mount + acknowledge button; wired AnnouncementDetail case in mobile App.tsx; added auto-mark-read useEffect in ppt-web ViewAnnouncementPageInner (fire-and-forget on announcement load); ppt-web & mobile typecheck + biome clean; mobile component: AnnouncementsScreen + AnnouncementDetailScreen

- 2026-05-25 — agent: gap-6-4-pinned-announcements-ui — Story 6.4: added PinnedAnnouncementsBand component + CSS; wired usePinnedAnnouncements (pinned=true) query in AnnouncementsPageRoute (App.tsx); propagated pinnedAnnouncements prop through AnnouncementsPage → AnnouncementList; mobile AnnouncementsScreen adds pinned-band with separate /api/v1/announcements?pinned=true query

- 2026-05-25 — agent: gap-6-3 review fixes — added i18n to AnnouncementComments.tsx (useTranslation + 12 keys); registered 18 toast keys + comments sub-object in en.json; fixed isManager to include technical_manager + property_manager; added announcements_comments_list/create/delete to sitemap + screen-map

- 2026-05-24 — agent: gap-6-3-comments-web-ui — Story 6.3: added AnnouncementComments component + CSS; added useAnnouncementComments/useCreateAnnouncementComment/useDeleteAnnouncementComment standalone hooks to @ppt/api-client; wired comment hooks in ViewAnnouncementPageInner with manager-role delete affordance; ppt-web buildStatus planned→in-progress

- 2026-05-24 — agent: gap-6-2-announcement-web-ui — Story 6.2: added useAnnouncement/useMarkReadAnnouncement/useAcknowledgeAnnouncement/useAnnouncementAcknowledgmentStats standalone hooks; replaced ViewAnnouncementPageRoute mock stub with real wiring (ViewAnnouncementPageInner); added acknowledgmentStats prop + AcknowledgmentStatsPanel; apiStatus stub→partial

- 2026-05-24 — agent: gap-79-1 — wired AnnouncementsPage to useAnnouncements+useDeleteAnnouncement+usePublishAnnouncement+useArchiveAnnouncement+usePinAnnouncement hooks; ppt-web.apiStatus stub→partial; auth header fix applied (Authorization: Bearer from getToken())

- 2026-05-09 (later) — agent: integrated Batch D (pages/ppt-announcements.html — list now designed: 4 artboards loaded-2-selected-1-pinned/empty/loading-8/error); replaced design list-as-TBD with real list specs; updated states; attached new pages/ppt-announcements.html as primary designSource; mobile reference updated to MobAnnouncementsScreen + MobAnnouncementDetailScreen
- 2026-05-09 — agent: design analyzed (ui_kits/ppt-web/announcement-detail.html — DETAIL only; list pending + ui_kits/mobile/screens.jsx for mobile list); flipped ppt-web from n/a → planned + redesignStatus in-progress (drift: route not in sitemap); flipped mobile redesignStatus → in-progress; attached 2 designSources (with note on detail-only coverage); populated functionality checklist (5 sections + 5-state pill set), states, design-specific notes; declared 4 sharedComponents; added 1 relatedScreen
- 2026-05-08 — init: created from scan (source: sitemap)
