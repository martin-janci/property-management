# pm-frontend — 2026-08-01 rotation slot

**Rotation:** idx 2 of 8 · role_last_run: 2026-06-10 (52 days stale — longest rotation gap on the wheel this cycle)
**Scope this run:** active-sprint UI stories + this-window frontend PRs (#2616, #2618) + the two remaining `partial` frontend slices (84-1, 84-2).

## summary

The two `partial` frontend slices are the only thing between the project and 49/49, but both are stuck for different reasons — 84-1 is backend-blocked (gh-issue-2573 unclaimed for 3 days), 84-2 has failed 3x as a single squash. This window's PR #2616 (admin-web ui-kit consolidation) is a directly transferable pattern for ppt-web's duplicated primitives, and PR #2618 cleaned the last reality-web screen-map drift on our books.

## next_actions

1. **Split 84-2 signer sign page into 3 mergeable slices** — (1) `/sign/:token` route + fetch signer manifest, (2) canvas signature capture + PATCH signature-request, (3) verification & delivery-confirmation UI. Priority: **high**. Dependency: none. Definition of done: three merged PRs, each flipping its own screen-map subsection; overall `ppt/document-sign` buildStatus planned -> shipped.
2. **Re-open 84-1 wiring the moment gh-issue-2573 lands** — swap XHR-to-proxy for POST /api/v1/documents/upload-url + PUT presigned URL, cover building_id + folder_id metadata, add EvidenceUploader-style progress test. Priority: **high**. Dependency: pm-backend (gh-issue-2573). Definition of done: 84-1 flips coverage status -> done with a merged PR.
3. **Extend #2616's ui-kit consolidation pattern to ppt-web** — audit top-10 duplicated Spinner/EmptyState/Button variants (per code-review-ppt-web-ui-duplicated-spinner-markup) and land as one deprecation PR. Priority: **medium**. Dependency: none. Definition of done: PR merged; code-review-ppt-web-ui-duplicated-spinner-markup marked resolved.
4. **Close the UC-33.x wave** — link UC-33.3 to a dispute screen-map (UC-33.1 and UC-33.2 already queued 2026-07-30). Priority: **medium**. Dependency: none. Definition of done: `docs/screens/ppt/disputes*.md` frontmatter includes all three UC codes; `screen_gaps.missing_use_cases` empty at next scan.
5. **Follow-through on layout-editor style extraction** (in-progress carry-over from 2026-07-23) — apply the #2464 pattern to remaining LayoutEditor components to reduce inline-style churn on LayoutEditorPage.tsx (900-line hotspot). Priority: **low**. Dependency: none. Definition of done: inline-style count on LayoutEditorPage.tsx drops >=50%.

## risks

1. **84-2 retry3 single-squash pattern will fail again** — three attempts have burned without a PR; each retry rebases against a moving ppt-web routing surface. probability: **high**, impact: medium. mitigation: split into 3 mergeable slices per next_action #1.
2. **84-1 blocked-chain drift** — 84-1 blocked on backend #2573 which has had zero churn for 3 days; the frontend fix is trivial but can't ship. probability: medium, impact: **high**. mitigation: pm-scrum-master to explicitly hand #2573 to a backend owner this window.
3. **ppt-web primitive drift compounds** — the same duplicated Spinner/EmptyState pattern that #2616 fixed for admin-web is still fragmenting in ppt-web; every new dispute/document PR adds another variant. probability: medium, impact: medium. mitigation: land next_action #3 as a single sweep before more feature PRs land.

## open_questions

- Should 84-2 slice-1 (route + manifest fetch) ship behind a feature flag while slices 2-3 land, or land dark under `/sign/:token`?
- For the ppt-web ui-kit sweep, do we deprecate old exports immediately or keep a 1-sprint transitional re-export?

## decisions_needed

- **Slice-vs-squash policy for retry-3+ stories** — when an implementer attempt fails 2+ times as a single squash, should the routine automatically re-plan as a 3-slice sequence? Owner: pm-tech-lead + pm-scrum-master. (raised 2026-08-01 by pm-frontend, prompted by 84-2 retry3.)
