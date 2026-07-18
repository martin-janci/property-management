# Roadmap — 2026-07-18

<sub>Generated 2026-07-18T12:00:00Z by ppt-project-management (default mode / rotating role). Deep scans are LOCAL ONLY.</sub>

## State of the project
- Stories: **47 done / 2 partial / 0 not-started** across 13 epics.
  - backend: 32 done · 2 partial · 0 not-started
  - frontend: 37 done · 2 partial · 0 not-started
  - mobile: 10 done · 0 partial · 0 not-started
- **Biggest gaps:**
  - `84-1-s3-presigned-urls` (partial, MVP) — ppt-web upload flow not wired to POST /documents/upload-url
  - `84-2-esignature-email` (partial, MVP) — signer-facing page shipped but story still open pending end-to-end email verification
  - Dispatcher buffer at 0/72 claimable — pipeline idle regardless of ready work

- **Screen coverage:** 0 stories without screen-map · 0 orphan epics · 0 orphan screens · 0 missing UC links (per coverage.json)

## Ranked plan

### MVP (this sprint)
- [high] Rebuild action-list.json/backlog.json from the 10 ready .research/plans/*.md and coverage.json partials — the buffer is at 0/72 claimable and starving the implementer — owner: researcher — why: action-list.json has >=5 claimable rows sourced from plans+coverage gaps; dispatcher buffer >0
- [high] Seed security-llm-doc-idor (routes/ai.rs publish_description/list_listing_descriptions/get_photo_enhancement discard principal — state-mutating cross-tenant IDOR, score 3) into action-list — owner: none — why: task assigned, principal bound + org predicate added to llm_document.rs queries, regression test add
- [high] Seed security-realtors-mark-inquiry-read-idor (reality-server routes/realtors.rs:250 mark_inquiry_read has no realtor predicate — cross-account write IDOR, score 3) into action-list — owner: none — why: handler binds principal, calls mark_inquiry_read_for_realtor, 404-on-no-match test added

### Follow-ups (post-sprint)
- [medium] Close 84-1-s3-presigned-urls gap: wire ppt-web upload flow to POST /documents/upload-url (endpoint shipped in PR #2309, no frontend consumer yet) — owner: none — why: api-client binding + UploadDocument uses direct-to-S3 PUT; screen ppt/upload-document apiStatus part
- [medium] Close 84-2-esignature-email gap: build the ppt/document-sign signer-facing page in ppt-web (backend + email complete, no UI component yet; prior attempt gap-84-2 landed no PR) — owner: none — why: document-sign page renders + signs via existing API; screen buildStatus planned->shipped
- [medium] Manually re-scope gh-issue-2360 (quarantined: fix_rounds=3 exhausted, verdict still 'changes' — Ok(None) dup-collapse + clippy red + migration 00219 collision) rather than re-queue for another automated round — owner: none — why: issue re-scoped/split or explicitly deferred with a human-reviewed plan

⚠ Buffer below half — consider running scan to refresh coverage. Buffer: 13/36 open · candidates unqueued: not-computed (rotating mode; scan mode required for deep rank)
