# Epic 6 Announcement Web UI — PR Sequencing Plan

_Created: 2026-05-25 by pm-frontend-sequence-epic6-announcement-drafts task_

## Context

Epic 2B notification pipeline (PR #463) merged 2026-05-24T17:15:04Z.
All three announcement web UI PRs are now unblocked.

## PR Status Summary

| PR | Branch | Title | Status | Verdict | CI | Action |
|----|--------|-------|--------|---------|-------|--------|
| #474 | `auto-impl/gap-6-2-announcement-web-ui` | Story 6.2 viewing/ack wiring | Open, **was draft → now ready** | approve | all green | **Merge first** |
| #475 | `auto-impl/gap-6-3-comments-web-ui` | Story 6.3 comments UI | Open, draft | changes-requested | all green | Fix required changes, then close as superseded by #474 or fix separately |
| #479 | `auto-impl/gap-6-4-pin-web-ui` | Story 6.4 pin/unpin wiring | Open, draft | approve | all green | Needs rebase after #474 merges — significant overlap |

## Conflict Analysis

### PR #474 vs PR #479 — Shared file conflicts

Both PRs modify the same three files:

1. `frontend/packages/api-client/src/announcements/hooks.ts`
   - #474 adds: `useAnnouncement`, `useMarkReadAnnouncement`, `useAcknowledgeAnnouncement`, `useAnnouncementAcknowledgmentStats`, comment hooks
   - #479 adds: `useGetAnnouncement`, `useMarkReadAnnouncement`, `useAcknowledgeAnnouncement` (duplicate/conflicting)
   - Resolution: #474's hooks are more complete (includes ack stats + all comment hooks). After #474 merges, #479 must be rebased and the duplicate hook additions removed.

2. `frontend/apps/ppt-web/src/App.tsx`
   - Both rewrite `ViewAnnouncementPageRoute` — direct conflict.
   - #474's version is more complete (Inner+Route pattern, comments wired, ack stats).
   - #479's pin wiring is a subset of #474's implementation.
   - Resolution: After #474 merges, #479's `App.tsx` changes are entirely superseded. PR #479 can potentially be closed as its pin wiring is already present in #474.

3. `docs/screens/ppt/announcements.md`
   - #474 sets `apiStatus: partial`; #479 sets `apiStatus: complete` and `buildStatus: shipped`.
   - Resolution: #479's screen-map update (marking complete) is the desired end state. Must be applied after #474 merges, either by rebasing #479 or by a direct commit to dev.

### PR #474 vs PR #475 — Overlap assessment

PR #474 already includes the Story 6.3 comment wiring (hooks, AnnouncementComments component, App.tsx wiring) as part of its implementation. PR #475 covers the same ground.

After #474 merges, PR #475 becomes partially superseded. The remaining value in #475 is:
- **Required (blocking):** i18n keys for comment strings (PR #474 review noted ~10 hardcoded strings as SHOULD-fix)
- **Required:** `isManager` role check should include `technical_manager` and `property_manager`
- **Required:** Comment endpoints missing from sitemap (`announcements_comments_list/create/delete`)

These items from #475's review findings should be addressed as follow-up issues, not merged via #475.

## Recommended Sequence

### Step 1: Merge PR #474 (immediate — already promoted from draft)
- Verdict: approve
- CI: all green (6/6 checks pass)
- Status: promoted from draft (2026-05-25)
- Unblocks: Stories 6.2 + 6.3 frontend

### Step 2: Close PR #479 (after #474 merges) or rebase
- Option A (recommended): Close #479 as superseded by #474.
  Pin wiring (`usePinAnnouncement` + `handlePin`) is already present in #474's `ViewAnnouncementPageInner`.
  The screen-map update (`buildStatus: shipped, apiStatus: complete`) should be applied as a direct follow-up commit.
- Option B: Rebase #479 against dev post-#474-merge, remove conflicting hook additions, keep only the screen-map `apiStatus: complete` update.

### Step 3: Handle PR #475 required changes as follow-up issues
Do NOT merge #475 as-is — it is superseded by #474 for the core implementation.
Open follow-up issues for the blocking items identified in the #475 review:
1. i18n: wrap all hardcoded strings in `AnnouncementComments.tsx` with `t()` keys under `announcements.comments.*`
2. `isManager` role check: add `technical_manager` and `property_manager`
3. Sitemap: register `announcements_comments_list`, `announcements_comments_create`, `announcements_comments_delete`

### Step 4: Update sprint-status.yaml
After #474 merges:
- `6-2-announcement-viewing-acknowledgment: done`
- `6-3-announcement-comments-discussion: review` (pending i18n follow-up)
- `6-4-pinned-announcements: done` (pin wiring is in #474)
- `epics.epic-6.stories_completed: 4` (was 1)

## Follow-up Issues to Create

| # | Title | Severity | Blocking |
|---|-------|----------|---------|
| TBD | i18n: AnnouncementComments.tsx hardcoded strings | medium | no (defaultValue fallbacks work) |
| TBD | isManager role check missing technical_manager + property_manager | high | yes for moderation correctness |
| TBD | Sitemap: missing comment endpoints (list/create/delete) | medium | no |
| TBD | useAcknowledgeAnnouncement: add acknowledgments(id) invalidation | low | no |

## Notes

- PR #463 (Epic 2B notification pipeline) confirmed merged 2026-05-24T17:15:04Z.
- PR #474 promoted from draft 2026-05-25 by this sequencing task.
- PR #479 was also approved but conflicts with #474 on all 3 shared files — must be resolved before merge.
- PR #475 changes-requested; its core implementation is superseded by #474; remaining items should become follow-up issues.
