# Role: pm-frontend — 2026-07-31

> Frontend/mobile delivery lens. Rotating slot (rotation idx 2). Last run 2026-06-10 (51 days stale).
> Static, read-only analysis.

## Summary

Frontend delivery is back on the two long-standing partial stories (84-1 direct-to-S3, 84-2 sign page): both blockers cleared this window (#2573 DELETE-guard closed by PR #2597; #2574 Android SSO mint() wired by PR #2593). Meanwhile this window shipped a healthy stream of small frontend hardening (`admin-web` Save no-op fix #2601, ppt-web WebSocket re-auth on token rotation #2578, WS console gating #2592, AML review-decision validation #2596, reality-web inline JSON XSS escape #2600 + untrusted `?source=` validation #2603) — all code-review autofixes rather than net-new feature slices.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Wire ppt-web direct-to-S3 upload (84-1) via POST /api/v1/documents/upload-url — api-client binding + `UploadDocument` integration + regression test | none (#2573 cleared by #2597) | 84-1 flips partial→done; screen-map upload-document remains `shipped`; regression covers register→PUT→confirm happy path + orphan cleanup expiry |
| high | Build signer-facing document-sign page in ppt-web (84-2) against shipped signing API | none | ppt/document-sign screen-map `buildStatus: planned → shipped`; signature-request email E2E verified; retry-3/2 pool depletion — needs specialist attention |
| medium | Backfill screen-map frontmatter `epics:` field across `docs/screens/**` so coverage-scan orphan detector stops manufacturing false orphans (all epic-7a stories still show `screen_refs: []` despite shipped UI) | none | 5 epic-7a stories gain `screen_refs`; systemic finding from 2026-06-23 deep scan closed |
| medium | Link UC-33.1, UC-33.2, UC-33.3 to dispute screen-maps — 3 UCs referenced in dispute story files but absent from any `docs/screens/ppt/disputes*.md` `use-cases:` frontmatter | none | 3 UCs no longer in `screen_gaps.missing_use_cases`; coverage `screen_gaps` shrinks |
| medium | Follow-through on #2464 LayoutEditorPage style extraction — apply the extracted style pattern to remaining layout-editor components (MobileConfigPage refactor in this window is the template) | none | Inline-style churn on `frontend/apps/admin-web/src/features/layout-editor/**` drops below 100 LOC/window |
| low | Audit reality-web `?source=` + inline tenant-JSON escape parity (post PR #2600/#2603) — verify no other SSR pages inline untrusted JSON without escape | pm-security | Grep of `dangerouslySetInnerHTML` in reality-web returns only escaped call sites; short report added to `docs/security/reality-web-ssr-escaping.md` |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| 84-2 signer page retry chain (retry_3/2 pool exhausted) — third failed implementer attempt would strand the story indefinitely | medium | high | Spawn a dedicated frontend-specialist subagent with tight scope: shipped API contract + screen-map first, page component second; consider splitting page-shell vs signing-flow into two smaller tasks if retry-3 fails |
| Coverage `screen_refs: []` for all 5 epic-7a stories is a screen-map frontmatter-drift artifact, not a real UI gap — ranker inflates "backend without UI" score by +1 per story on shipped work | high | low | Backfill `epics:` frontmatter (docs-screen-map-frontmatter-epics task) is now the highest-leverage single edit — one PR closes 5 spurious signals + future scans |
| Voice-webhook security cluster (Alexa signature never verified, HMAC default-secret, cross-tenant auth bypass) has no frontend surface — but any future voice-first UX in ppt-web would inherit the trust boundary. Blocks any voice-UI feature planning until pm-security lands the fix. | low | high | Frontend team should not scope any voice-UI story until the 3 findings are closed |
| Accounting MVP-loop trio (#2555/#2558/#2559) is backend-only right now but each ships an endpoint the ppt-web accounting module will need to consume — no frontend counterpart drafted yet | medium | medium | Draft ppt-web accounting-page skeleton + `@ppt/api-client` bindings in parallel so the moment the trio merges, integration work is not sequential |

## Open questions

- Does the accounting MVP-loop trio (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY-by-square QR) have a frontend consumer scoped yet, or is the ppt-web accounting module still pre-plan?
- Post PR #2565 (reality-web Docker build fix): is the frontend Docker image now publishing on every dev merge, or is there still a manual trigger gap?
- Should the layout-editor refactor follow-through (item #5 above) be split per-component into micro-tasks so it fits the dispatcher's small-slot buffer?

## Decisions needed

- 84-2 signer-page implementation strategy — split into shell + flow, or single-shot with a hardened specialist prompt — owner: pm-tech-lead / pm-frontend.
- Screen-map frontmatter epics: backfill — batch PR (all products, one commit) vs per-product incremental — owner: pm-frontend.
