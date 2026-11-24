# pm-devops — 2026-11-24

## Summary
Dispatcher throughput at **30 PRs/day** — highest observed single-day cadence. CI pipeline healthy: no failed workflow runs in the merged window, scheduler ran on time (last_run 2026-11-23T04:19:35Z → this run 2026-11-24T04:19:35Z, 24h delta, no drift). The two structural cloud-runner blockers (**#2652** mobile-native KMP build, **#2966** utoipa-swagger-ui egress) remain open; dispatcher stall **#3357** is a workflow-level blocker (pm-tech-lead-owned).

## What moved (infra/CI-adjacent)
- **No CI gate failures** across the 30 merged PRs. `backend.yml`, `frontend.yml`, `api-validation.yml`, `screen-map.yml` all green.
- **#3758** (in-process ratelimit not distributed) — infra hygiene: documents + gates the single-instance ratelimit assumption; owner pm-backend but infra-adjacent.
- **#3760** (voice-webhook RLS release inconsistency) — same conn-pool release contract hygiene.
- **#3744** (SSO pending-session O(n) → amortized eviction) — closes Phase-1-surfaced issue #3738 inside 48h, dispatcher follow-up loop working.
- **#3762** (admin-web capability gates) — RBAC surface hardened on admin-web (no infra change, but gates admin paths).

## Workflows under watch
| Workflow | Signal this run | Note |
|---|---|---|
| `backend.yml` | green | 15+ merged backend PRs, all clean |
| `frontend.yml` | green | 11 i18n PRs clean; push to locale burn-down is working |
| `api-validation.yml` | green | 0 OpenAPI drift PRs this window |
| `mobile-native.yml` | **blocked** | #2652 + #2966 still open; 4 KMP backlog items structurally unclaimable |
| `screen-map.yml` | green | #3752 reality drift reconcile landed clean |
| `quarantine-age-gate.yml` | green but aging | 201 quarantined #[ignore]d rows, due 2027-02-19; burn-down target 0 |

## Next actions
1. **[high]** Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (**#2652, #2966**) — carried since 2026-08. 4 structurally unclaimable KMP backlog items (CreateListingScreen stub, InquiriesResponse contract, Ktor HttpTimeout, SSO error body). DoD: a KMP code-review task is claimable AND one lands green in the cloud dispatcher.
2. **[medium]** Dependabot #3441 (axum-test 21.0.0 → 21.1.0) open since 2026-11-23 — trivial patch, land with normal gate. DoD: merged or explicitly deferred with reason.
3. **[medium]** Define and land a push/* CODEOWNERS scope (pm-backend + pm-qa) so the three open push-subtree PRs (#3771 fcm, #3772 voice-webhook, plus any follow-up to apns) get automatic reviewer attachment. DoD: `.github/CODEOWNERS` entry present, reviewers auto-request on next push PR.
4. **[medium]** Draft a quarantine burn-down cadence proposal (track 2 of active sprint, 201 rows, due 2027-02-19 — **87 days out**). Current progress: 0/201. At 0 rows/day we fail the gate. DoD: a weekly dispatcher-visible metric + a first-wave batch sized by owner (193 pm-backend / 6 pm-devops / 2 pm-security).
5. **[low]** `security-test-gate.yml` has not run in the merged window — verify it's still wired to the right PR path filter after the recent routes split (voice_webhooks/* files moved). DoD: path filter updated or confirmed; one dry-run on a touching PR.

## Risks
- **Mobile-native KMP cloud starvation** (carried, high/medium): #2652 and #2966 still open; 4 backlog items permanently unclaimable in cloud. Mitigation: vendor swagger-ui assets OR allowlist egress OR pre-cache in sandbox base image.
- **Quarantine burn-down trajectory** (medium/high): 201 rows due 2027-02-19, zero progress in two weeks. Risk is gate-failure cliff, not gradual. Mitigation: a per-owner weekly quota with the sprint dashboard.
- **Dispatcher stall #3357 non-mechanical remainder** (medium/high, carried to pm-tech-lead): mechanical resolver (#3718) landed, but reviewer-approved PRs across two merge waves are still blocked.
- **Dependabot thickening** (low/medium): single open Dependabot (#3441) is fine, but the pattern of trivial-but-queued bumps is back; keep the 72h review SLA.

## Open questions
- Does `mobile-native.yml` still fire at all on PR events, or is it silently no-op since the KMP block? (Would explain why no one has been paged on it.)
- Is there a cloud-runner image rebuild cadence we can piggy-back the swagger-ui vendoring onto, or does it need a dedicated PR?

## Decisions needed
- **KMP/swagger-ui unblock path** — vendor assets vs. egress allowlist vs. cached base image. Owner: pm-devops.
- **Quarantine burn-down cadence** — fixed per-week quota vs. sprint-boundary flush vs. per-owner contracts. Owner: pm-devops (with pm-tech-lead).
