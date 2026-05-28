# pm-tech-lead — 2026-05-23

**Summary:** The active sprint stacks three cross-cutting platform capabilities — OAuth provider (10A), notification infra (implied by 6 & 8A), and WebSocket real-time (8A.3) — but 10A is entirely `ready-for-dev` and the notification/WS substrate (architecture ADR-008, Epic 2B) is not yet built, so Epics 6 and 8A are shipping around a missing foundation. The dead/duplicate handler modules (AuthHandler/BuildingHandler) and three churn-hot route files (integrations/organizations/documents.rs, all ~4k lines) are accumulating maintainability and security-divergence risk on the busiest auth/multitenancy paths.

## Next actions
| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Decide build order: land Epic 2B notification infra before Epic 6 publish + 8A dispatch, or formally defer those story slices | pm-scrum-master | Documented sequencing decision in decisions.md |
| high | Remove dead duplicate AuthHandler/BuildingHandler modules so security fixes can't diverge between handler/route copies | none | `pub mod auth/buildings` handler structs deleted or wired; single live auth path |
| medium | Split churn-hot route modules (integrations.rs 4.3k, organizations.rs 4.0k, documents.rs 3.5k) by surface (install/oauth/sync/webhook) | none | Each file < ~1.5k lines; submodules per surface |
| medium | Define OAuth provider (10A) token/storage/rotation design before 10a-1 pickup to avoid rework | none | ADR/design note for authorization-server + token mgmt |
| low | Confirm WebSocket infra ownership for 8A.3 sync (ADR-008) is scheduled, not implicitly assumed | pm-scrum-master | WS infra epic referenced in sprint plan |

## Risks
| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Epics 6 & 8A built against a notification/WS foundation (Epic 2B, ADR-008) that does not yet exist — rework when it lands | high | high | Sequence 2B first or freeze dependent story slices behind feature flags |
| Duplicate AuthHandler/BuildingHandler modules: a security/correctness fix to the live route copy won't propagate to the dead handler copy, misleading reviewers | medium | high | Delete dead modules; enforce single implementation per auth/building op |
| OAuth provider (10A) started with no agreed token/rotation/storage design — divergent implementation across 3 stories | medium | high | Lock ADR for 10A before pickup |
| Three ~4k-line route files (integrations/organizations/documents.rs) on hot multitenancy paths — review fatigue, RLS-predicate omission risk | medium | medium | Module split; RLS-predicate checklist on these files |

## Open questions
- Is Epic 2B notification infrastructure (and the ADR-008 WebSocket server) in scope this sprint, or assumed pre-existing?
- Is there an agreed ADR for the OAuth 2.0 provider token store/rotation backing Epic 10A?

## Decisions needed
- Build-order: Epic 2B notification/WebSocket infra before vs. after the dependent Epic 6/8A slices — owner: pm-tech-lead
- Whether to delete the dead AuthHandler/BuildingHandler modules now vs. wire them as the canonical path — owner: pm-tech-lead
