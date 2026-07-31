# pm-frontend — 2026-07-31

_Frontend/mobile delivery lens. Daily PM rotation (rotation index 2 → 3). Static, read-only analysis. Last run 2026-06-10 (52d stale)._

## Summary

The 2026-07-30 → 2026-07-31 window was dominated by **frontend security hardening** — 3 untrusted-cast / XSS fixes landed the same day across ppt-web + reality-web (#2596 AML decision cast, #2600 tenant JSON XSS, #2603 viewsource cast), plus admin-web mobile-config no-op Save paths (#2601) and pre-existing test failures unblocking verify-gate (#2602). The two remaining `partial` MVP stories (84-1 direct-to-S3 wire, 84-2 signer page) are unchanged, but **84-1 is now unblocked** (its dependency #2573 was closed by PR #2597 landing the DELETE-by-file-key ref-check guard).

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Claim 84-1 next dispatcher round: wire ppt-web direct-to-S3 upload via POST /documents/upload-url (api-client binding + UploadDocument integration + regression test) — dependency #2573 cleared | none | 84-1 flips from partial → done in coverage.json; screen-map ppt/upload-document buildStatus stays shipped with apiStatus complete; regression test present |
| high | Cross-cutting frontend security pattern: add lint/codemod for untrusted-to-union casts and SSR string interpolation into `<script>` — #2596/#2600/#2603 same-window findings prove this is systemic | pm-tech-lead pick mechanism | Biome rule or codemod PR landed; sweep across ppt-web + reality-web reports zero remaining call sites |
| medium | Package a scoped implementer brief for 84-2 (signer document-sign page) — 3 prior attempts failed no-PR; include screen-map location, hooked API, i18n keys, exact green-test target | none | Retry chain gap-84-2-*-retry4 spawns with a scope note that reads implementable |
| medium | Frontend verify-gate hygiene: pnpm test failures should be first-class dev-push signals (post-#2602 pre-existing failures accumulated silently for a window) | pm-devops | frontend.yml on-push test job OR verify-gate escalation to routine signal is decided + shipped |
| medium | Audit reality-web SSR surface for remaining string-interpolation-into-HTML patterns (post-#2600) and untrusted URL-search-params casts (post-#2603) | none | reality-web SSR audit report produced; residual patterns fixed or triaged |
| low | Follow-through on the MobileConfigPage.tsx churn hotspot (post-#2601 Save-path fix): extract Save-mutation hooks into a shared platform-settings pattern | none | Shared hook used by ≥2 platform-settings pages; no-op Save regressions prevented at pattern level |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Repeated XSS/cast findings across ppt-web + reality-web in one window (#2596 + #2600 + #2603) — systemic pattern gap | high | high | Add lint/codemod (queued as pm-frontend-cross-cutting-frontend-security-pattern-2026-07-31); sweep reality-web SSR surface |
| Frontend test suite silently broken for at least one window before #2602 caught it — verify-gate feedback loop was quiet | medium | medium | Add on-push CI test job or escalate verify-gate to routine signal (queued as dx-frontend-verify-gate-testrunner-guard-2026-07-31) |
| 84-1 has been top mvp partial for weeks — dependency cleared (#2597) but consumer wiring still hasn't been picked up; risk of MVP close slipping | medium | medium | Explicit claim next dispatcher window with scoped implementer brief |
| Accounting MVP-loop trio (#2555 / #2558 / #2559) is drafted with frontend consumer surface; reviewer starvation on backend side blocks the frontend wiring push | medium | medium | Frontend audit + note-file so reviewer pass is efficient once the trio unblocks |
| 3 UC-33.x dispute UC links still missing screen-map — coverage.screen_gaps flags them; two are queued, one added this run (UC-33.3) | low | low | gap-uc-33-3 queued; small doc PR closes the last one |

## Open questions

- Should Biome host the new untrusted-cast rule, or should we codemod the existing sites and rely on code review? A Biome custom rule needs a plugin surface not currently used in the frontend workspace.
- Does the frontend workspace already have a shared `escapeInlineJson()` helper (or similar) that #2600 could have used? If yes, `<script>` inline patterns should reference it in a shared component to make the mistake harder.
- Is the drift gate on reality-api-client (#2607) meant to include ppt-web consumers, or only reality-web? ppt-web currently duplicates some listing/search types.

## Decisions needed

- **NEW (2026-07-31):** Mechanism for the frontend security pattern lint — Biome custom rule vs codemod sweep vs Ecosystem policy note. Owner: pm-frontend + pm-tech-lead.
- **NEW (2026-07-31):** Frontend verify-gate escalation — on-push pnpm test job OR routine-signal from verify-gate failures. Owner: pm-devops + pm-tech-lead.
- Carried: Retention policy scope for support_tooling_events (blocking data audit expansion) — owner: pm-security + pm-data.
