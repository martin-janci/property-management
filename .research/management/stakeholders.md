# Stakeholder map — PPT delivery

> Static reference. The roles below map 1:1 to the `pm-*` agents in `.claude/agents/`.
> Edited rarely; the routine reads this but does not regenerate it.

| Role (agent) | Responsibility | Required inputs | Expected outputs | Decision authority |
|---|---|---|---|---|
| Scrum Master (`pm-scrum-master`) | Organize the work; maintain delivery state | sprint-status.yaml, merged PRs, research backlog | project-state.md, action-list.json, decisions.md | Sprint scope & sequencing |
| Tech lead / architect (`pm-tech-lead`) | Architecture coherence, cross-cutting decisions | epics, architecture.md, churn | architecture risks, key decisions | Technical direction |
| Backend (`pm-backend`) | APIs, data model, business logic | stories, backend churn | backend task list, API/data risks | Backend implementation |
| Frontend / mobile (`pm-frontend`) | Screens, flows, state, API consumption | stories, UX notes, OpenAPI client | frontend task list, UX/API risks | Frontend implementation |
| QA / test (`pm-qa`) | Test strategy, acceptance, regression | stories, acceptance criteria | test matrix, release recommendation | Release readiness (quality) |
| DevOps / infra (`pm-devops`) | Environments, CI/CD, observability | workflows, deploy config | infra task list, deploy risks | Deploy & environments |
| Security (`pm-security`) | Threat model, authz, data protection | auth code, RLS, deps | security risks, release blockers | Security release gate |
| Data / analytics (`pm-data`) | KPIs, event tracking, reporting | features, db schema | event plan, data risks | Analytics definitions |
| Integration / API owners (`pm-integration`) | External/internal API contracts | integration code, OpenAPI | contract checklist, integration risks | API contracts |
