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
    apiStatus: complete
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

- 2026-11-04 backend alignment (supersedes older notes above where they conflict): the api-client dispute layer now maps the real backend shapes. `getDispute` → `GET /disputes/{id}` (`DisputeWithDetails` = dispute + `parties[]` with user names/emails + counters), timeline → `GET /{id}/activities` (actor ids only; names resolved from the party roster in `features/disputes/utils/mapDisputeDetail.ts`), evidence/resolutions(+votes via `GET /{id}/resolutions/{rid}`)/action items mapped from snake_case. Status changes are a manager-only select of the backend state machine's allowed transitions (`allowedStatusTransitions` in DisputeDetailPage) via `PATCH /disputes/{id}`; resolving in the mediation workspace is manager-only (`PATCH /{id}/resolve`, notes composed from the form); escalate = `POST /{id}/escalations`; assign mediator = `POST /{id}/parties` role `mediator` (no backend route sets `assigned_to`). REMOVED (no backend route): evidence file upload ("Add Evidence"), ticking off resolution terms, and the workspace Discussion/chat tab (`MediationChatThread`; backend has only a single `mediation_notes` text). Evidence "View" link only renders for http(s) storage URLs.
- Discussion bubbles: 4 distinct speaker types (plaintiff/defendant/mediator/system). Mediator gets violet (event-soft); system messages are neutral muted with icon prefix.
- Audit log writes are append-only — design assumes immutability; flag for engineering.
- Sticky rail uses `position: sticky; top: 84px` matching header height.
- Mediation sessions are now wired in the workspace sidebar (MediationSessionsPanel) with a schedule/reschedule dialog (MediationSessionDialog) and cancel action, backed by `/api/v1/disputes/{id}/sessions*`. Session JSON is snake_case on the wire (backend has no serde rename) — the `MediationSession` api-client type matches that exactly.
- Party submissions are now wired (apiStatus → complete): `/api/v1/disputes/{id}/submissions` (GET list + POST submit). The submission UI lives in MediationWorkspacePage as a "Submissions" tab (MediationSubmissionsPanel) — list view plus a party composer (submission type, content, visible-to-all toggle). `PartySubmission`/`SubmitResponseRequest` api-client types are snake_case on the wire (backend has no serde rename). POST resolves the submitting party from the authed user server-side; the composer only shows for parties (`isParty`).

## Agent Log

<!-- newest entries on top -->

- 2026-11-04 — agent: wired the detail page + mediation workspace to the real dispute routes (detail, activities, evidence, resolutions+votes, action items, status PATCH, resolve PATCH, escalations, parties). Removed affordances with no backend route (evidence upload, resolution-term completion, discussion/chat tab). Role gating aligned with backend: status change + resolve + assign mediator manager-only; mediator can escalate/run sessions; parties can open the workspace, vote and submit. New tests: api-client disputes mappings, DisputeDetailPage.test.tsx, rewritten routes/groups/disputes.test.tsx; i18n keys in all 6 locales. buildStatus/apiStatus unchanged (shipped/complete).
- 2026-07-07 — agent: gap-80-3-disputedetailroute-in-app-tsx-is-stub — verified the reported stub is stale: DisputeDetailRoute (now in `routes/groups/disputes.tsx` after the App.tsx aggregator split, #1108) already renders the real lazy DisputeDetailPage wired to useDispute/useDisputeTimeline/useDisputeEvidence/useUpdateDisputeStatus. Added missing route-level regression tests `routes/groups/disputes.test.tsx` (4 tests: API→UI prop mapping, status-update mutation + toasts, load-failure error page) so the wiring can't regress to a stub. Band A typecheck + biome clean; new suite green; pre-existing FileDisputePage debounce failures (#1364) reproduce on clean dev, unrelated. buildStatus/apiStatus unchanged (shipped/complete).
- 2026-06-25 — agent: 80-3-mediation-resolution — closed the final 80-3 gap: wired party submissions endpoints (`GET/POST /api/v1/disputes/{id}/submissions`, already present on backend). Added @ppt/api-client `listSubmissions`/`submitResponse` + `useDisputeSubmissions`/`useSubmitResponse` + `PartySubmission`/`SubmitResponseRequest` snake_case wire types. New MediationSubmissionsPanel surfaced as a "Submissions" tab in MediationWorkspacePage (list + party composer gated on `isParty`). i18n for all 6 locales; MediationSubmissionsPanel.test.tsx (5 tests). Band A typecheck + biome clean; Band B ppt-web build clean; full dispute suite 152 tests green. Flipped apiStatus partial→complete; sprint-status 80-3 promoted to done.
- 2026-06-03 — agent: gap-80-3-mediation-sessions-ui — built session scheduling UI in MediationWorkspacePage: added MediationSessionsPanel (sidebar list, upcoming-first ordering, reschedule/cancel) + MediationSessionDialog (datetime-local picker, type/duration/location/meeting-url; schedule + reschedule modes). Added session api-client layer (listSessions/scheduleSession/updateSession/cancelSession + useMediationSessions/useScheduleSession/useUpdateSession/useCancelSession + types) wired to `/api/v1/disputes/{id}/sessions*`. i18n keys added for all 6 locales. New Vitest suite (5 tests) for the panel. Band A typecheck + biome clean; Band B ppt-web build clean. apiStatus stays partial (submissions still pending).
- 2026-05-26 — agent: gap-80-3-mediation-ui — built full mediation workspace (MediationWorkspacePage, MediationChatThread, MediationResolutionForm); wired DisputeMediationRoute to use all mediation API hooks (useResolveDispute, useEscalateDispute, useAssignMediator, useMediationNotes, useAddMediationNote); dispute timeline now uses useDisputeTimeline directly; chat thread uses real MediationNote API; resolution form covers all 5 ResolutionType options; Band A typecheck + biome lint clean.
- 2026-05-25 — agent: story 80-3 wired — replaced inline DisputeDetailRoute stub in App.tsx with full DisputeDetailPage integration (all props, hooks, type mapping); added DisputeMediationRoute function; added /disputes/:disputeId/mediation route element; added DisputeDetailPage + MediationPage lazy exports to lazyRoutes.tsx; Band A typecheck + Band B build clean; apiStatus remains partial (sessions/submissions endpoints pending).
- 2026-05-25 — agent: verified story 80-3 AC coverage; DisputeDetailPage.tsx fully implemented (evidence, resolutions with voting/accept/implement, action items with escalation, timeline). MediationPage.tsx fully implemented (sessions, submissions, schedule/complete/submit dialogs). All hooks present (useAssignMediator/useResolveDispute/useEscalateDispute/useAddMediationNote). CRITICAL GAP: DisputeDetailRoute in App.tsx is inline JSX stub not using DisputeDetailPage.tsx; no /disputes/:id/mediation route exists; none of the mediation mutation hooks are wired. Story 80-3 stays partial; apiStatus remains partial. Follow-up wiring task required.
- 2026-05-09 — agent: integrated Batch C (pages/ppt-dispute-detail.html — 3 artboards: V mediácii / Vyriešený read-only / Loading); flipped redesignStatus → in-progress; attached designSource; populated 4 sections + 3 states + 3 notes; declared 6 sharedComponents
- 2026-05-08 — init: created from scan (source: sitemap)
