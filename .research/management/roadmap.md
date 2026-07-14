# PPT Roadmap — deep scan 2026-07-15

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics)
- Delta vs 2026-07-13 scan (40/9/0): +7 done — 80-2 (07-13 gap was a false positive), 81-1 (#2292/#2295/#2313), 83-2 (#2315/#2194), 83-3 (#2286/#2294), 84-3 (#2285/#2287/#2288), 84-4 (#2290/#2293/#2310), 84-5 (#2291/#2312).
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempt failed.
- Screen coverage: 0 orphan screens · 0 validation errors (post-#2297) · 3 missing UC links (UC-33.x dispute sub-UCs).
- Batch-verifier note: its 12 claimed regressions were disproved against sprint-status ground truth (#2298/#2307 reconciliation is in effect) — no downgrades applied.

## Ranked plan

### mvp
- [medium] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url: api-client binding + UploadDocument integration + screen-map note (backend endpoint landed in #2309) (84-1 S3 Presigned URL Implementation) — owner: pm-frontend — why: mvp partial; finish-what's-started; backend ready
- [medium] Build the signer-facing document-sign page in ppt-web against the shipped signing API; flip screen-map ppt/document-sign buildStatus planned->shipped; verify signature-request email delivery. NOTE: prior attempt (gap-84-2-no-frontend-ui-component-consuming-the) failed with no PR — fresh scoped attempt (84-2 E-Signature Email Integration) — owner: pm-frontend — why: mvp partial; finish-what's-started; API complete, UI planned
- [medium] Link UC-33.1/UC-33.2/UC-33.3 (dispute sub-UCs) to the dispute screen-maps' use-cases frontmatter — owner: pm-frontend — why: screen-map UC coverage; small
- [medium] Reconcile stale statuses: sprint-status 80-2-dispute-filing-flow partial->done (AC-4 verified shipped); screen-map ppt/reports apiStatus partial->complete (#2292/#2295/#2313 filled the last gaps) — owner: pm-backend — why: tracking accuracy; cheap

> Epic 86 gap-analysis doc remains superseded by this map.

Buffer: 4/36 open · project at 47/49 — backlog genuinely converging; treat underflow as success, not starvation
