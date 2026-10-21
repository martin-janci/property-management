# PPT Delivery Roadmap

_Generated: 2026-10-21T18:50:00Z — ranker mode: default (balanced rubric: priority × dependency-foundationality × role-capacity-spread × recency boost for new-this-run items). Coverage 49/49 stories done → sprint delivery is complete at coverage level; the roadmap is now the dispatcher merge-pipeline + follow-up queue, not epic progression._

## Top 10 ranked next steps

| Rank | Action | Owner | Priority | Reason / dependency |
|------|--------|-------|----------|--------------------|
| 1 | **Resolve #3131** — dispatcher-bot identity split (new PAT or GH App) so dispatcher PRs can be APPROVED by a different actor than their author; alt: amend `ppt-pr-merge` to accept documented bot-approvals-of-self. | pm-tech-lead | high | **Single structural blocker on everything downstream.** 16 approved PRs frozen; all 5 stalled drafts (#3028/3070/3072/3074/3075) share this root cause. Must be first. |
| 2 | **Human-approve the top-5 stalled drafts** (#3028 add-to-compare TOCTOU; #3070 ViolationRepository IDOR; #3072 violations role-authz; #3074 record_payment atomic; #3075 violations number-count-race) so the IDOR-adjacent tier lands even while #3131 is in flight. | pm-scrum-master | high | Short-term workaround while #3131 is in flight; unblocks ~5 security-tier PRs in-hand. |
| 3 | **Address #3132 PRICE_MAP_CACHE DoS** — replace O(n log n) sort-on-every-insert with LRU; constrain attacker-controlled cache keys (whitelist + length cap). Both dimensions in one PR. | pm-backend | high | Two-attribute vulnerability; cache-thrash converts into DB-load DoS. |
| 4 | **Address #3136 migration-upload typed storage-error + happy-path test** — match on typed `StorageError::InvalidContentType` variant rather than `Display` string; add happy-path persistence test. | pm-backend | medium | Prevents regression on migration.rs churn hotspot (2nd follow-up in 36h). |
| 5 | **Unblock #2966 swagger-ui egress** — carried from prior runs; still gates backend cloud builds and (together with #3131) is the two-key lock on the IDOR tier. | pm-devops | high | Carried blocker; combined with #3131 resolution unblocks #2946/2944/2945 IDOR retries. |
| 6 | **Close #3006** — collapse @ppt/api-client triplicated auth-header + MFA-retry logic into one shared `authenticatedFetchJson`; add Biome rule banning new fetch primitives. | pm-tech-lead | medium | Carried risk pm-tech-lead-auth-header-triplicate-drift-2026-10-01 — classic security-overtones refactor; three copies will drift. |
| 7 | **Typed-response rotation post-#3129** — identify next 3 untyped `serde_json::Value` list endpoints in api-server routes and open one plan this cycle. | pm-tech-lead | medium | Schema stability for generated OpenAPI/TypeSpec; prevents client-break-via-regen. |
| 8 | **Unblock #2652 mobile-native/KMP cloud builds** — carried; 5+ open backlog items structurally unclaimable in cloud. | pm-devops | high | Carried structural blocker on mobile-native work. |
| 9 | **pm-integration-document-sso-transient-401-contract** — document the reality-server SSO consumer transient-401 contract in docs/api/README.md so client/server treat transient vs permanent 401 symmetrically (follow-up to PR #3121 cushion). | pm-integration | low | Carried item from 2026-10-19 run. |
| 10 | **RUSTSEC exception ADR** — author policy for when a version-bump path is unavailable (version-bump-vs-exception tradeoff). Reference from `backend/deny.toml`. | pm-security | low | Pre-emptive; 4 advisories this cycle were straight bumps but next may not have a clean upstream. |

## Spread by owner (current open items in action-list)

- **pm-tech-lead**: 10 (dispatcher split, api-client collapse, typed-response, 2 accounting stalls, #3098/3104/3109/3116/3132/3136 placeholders)
- **pm-backend**: 13 (mobile-native-kmp cluster, violations trio, add-to-compare TOCTOU, enumeration/replay, totp-rng-panic, formatprice-locale, impersonation-xff, migration-export-stub)
- **pm-devops**: 1 (mobile-native cloud builds) — carried
- **pm-integration**: 3 (openapi-drift audit, rustsec-exception-policy, sso-transient-401 doc)
- **pm-qa**: 2 (ssoservice-untested, screen-map-drift PR 2994)
- **pm-security**: 1 (RUSTSEC-exception ADR — new this cycle)
- **pm-scrum-master**: 1 (stalled-prs human-approve workaround)

## Health signals

- **Merge pipeline**: 🔴 DEADLOCKED (16 PRs blocked on #3131); workaround available (human-approve) but structural fix required.
- **Delivery velocity**: 🟢 5 PRs merged in 36h (prior window: 2 in 72h); dispatcher is producing fixes faster than the merge gate accepts them.
- **Buffer**: 🟡 claimable=12/72 (buffer-low trigger); below 30% refill target but up from 9/72 last cycle.
- **Churn hotspots**: `backend/deny.toml` (2 PRs this run), `backend/servers/api-server/src/routes/migration.rs` (1 PR + 1 follow-up), `backend/servers/api-server/src/routes/aml_dsa/aml.rs` (typed-response pattern).
- **Code review (Phase 1.5)**: 3 new findings expected into backlog this run.
