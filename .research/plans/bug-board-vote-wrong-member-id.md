# bug-board-vote-wrong-member-id

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review api-handlers 2026-07-23 (tier1d)
**Confidence:** high

## Hypothesis
`cast_vote` (POST `/motions/{id}/vote`) writes every board-meeting vote using `let board_member_id = motion_id;` — a placeholder that stores the motion's own UUID as the voting member's identity. The correct member id must be resolved from the authenticated `user.user_id` against the `board_members` table for the motion's building/org. Left as-is, every vote on a motion is attributed to the same bogus id, silently corrupting audit/quorum tallies on a governance path.

## Evidence
- `backend/servers/api-server/src/routes/board_meetings.rs:740-745` — inline comment admits `"In production, we'd look up the board_member_id from user.user_id ... For now, we'll use a placeholder - this should be retrieved from the board_members table"`; the placeholder `board_member_id = motion_id` is then persisted via `board_meeting_repo.cast_vote(...)`.
- `backend/servers/api-server/src/routes/board_meetings.rs:124` — route `POST /motions/{id}/vote` is registered against `cast_vote` (production wiring, not gated).
- Consequences: (1) all votes on a given motion share the same bogus `board_member_id` (= motion id); (2) per-member uniqueness constraints/tallies collapse or collide; (3) audit/quorum computations built on `MotionVote.board_member_id` become unreliable.

## Files
- `backend/servers/api-server/src/routes/board_meetings.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Seed an org with a building, a board meeting, one motion `M`, and two distinct board members `Alice` and `Bob` (each with `board_members.user_id` linked to a real user).
2. As Alice, POST `/motions/{M}/vote` with a vote payload. As Bob, POST `/motions/{M}/vote` with a different payload.
3. Expected: two rows in `motion_votes` with `board_member_id = alice.id` and `bob.id`. Actual: both rows have `board_member_id = M` (the motion id) — the second insert either violates a per-(motion, member) uniqueness constraint or silently overwrites the first.

## Suggested approach
1. Add a repository lookup `board_members_repo.find_by_user_and_meeting(user_id, meeting_id) -> Option<BoardMember>` (or reuse the existing one if present; grep `board_members` in `backend/crates/db/src/repositories/`).
2. In `cast_vote` at `board_meetings.rs:735-747`: after loading the motion (which already carries the meeting/building/org), resolve `board_member = board_members_repo.find_by_user_and_meeting(user.user_id, motion.meeting_id)`.
3. Reject with 403 `not_a_board_member` if `None` (RLS-scoped RLS conn is already present).
4. Pass `board_member.id` (not `motion_id`) into `cast_vote(&mut **rls.conn(), board_member.id, input)`.
5. Delete the placeholder comment.
6. Add integration coverage: two distinct board members voting on the same motion each land a distinct `motion_votes.board_member_id`; a non-board-member user gets 403.
7. Run `cargo test -p api-server board_meetings::cast_vote --nocapture` and `cargo clippy -p api-server -- -D warnings`.

## Alternatives considered
- **Look the board_member up during the SQL insert with a `SELECT ... FROM board_members WHERE user_id=$1 AND meeting_id=$2` sub-query** — rejected because it hides the 403 case behind a NOT-NULL FK failure with a confusing error surface; a discrete Rust-side lookup returns a clean 403 and produces a testable code path.
- **Take `board_member_id` from the request body** — rejected because it re-introduces the same IDOR pattern already fixed on the disputes / marketplace / ai/ocr surfaces; the caller's identity must come from the auth session.

## Root-cause trace
1. Symptom: `motion_votes.board_member_id` equals the motion id for every vote; per-member tallies collapse.
2. ← `backend/servers/api-server/src/routes/board_meetings.rs:741` — `let board_member_id = motion_id;` explicit placeholder.
3. ← `backend/servers/api-server/src/routes/board_meetings.rs:743` — `board_meeting_repo.cast_vote(&mut **rls.conn(), board_member_id, input)` persists the placeholder.
4. Origin: initial `cast_vote` handler landed under the board-meetings epic with the placeholder + comment; the follow-up lookup work never landed.

## Test plan
- [ ] `cargo test -p api-server board_meetings::cast_vote_uses_real_board_member_id` — two distinct users → two distinct `board_member_id` in `motion_votes` for the same motion.
- [ ] `cargo test -p api-server board_meetings::cast_vote_rejects_non_board_member` — user with no `board_members` row for the meeting ⇒ 403.
- [ ] `cargo test -p api-server board_meetings::cast_vote_regression_tally` — after two votes, the meeting-level tally reflects both members (no collapse).
- [ ] `cargo test -p api-server -- board_meetings::cast_vote --nocapture`

## Out of scope
- Redesign of the board-member roster / invitation flow.
- Historical repair of already-corrupted `motion_votes` rows (call out in the PR body; back-fill needs a separate data-migration plan).
- Any change to the motion or meeting lifecycle endpoints.

## After-merge
- Move this file to `plans/_archive/bug-board-vote-wrong-member-id.md`
- Mark the matching `backlog.json` row as `status: "done"`
