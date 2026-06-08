# Documentation Deep Dive (Duplication / Contradictions / Alignment Plan)

This document audits the **existing documentation set** and highlights:
- **duplication** (same knowledge repeated in multiple places),
- **contradictions** (docs disagree with each other),
- **alignment gaps** (docs vs specs vs code),
and recommends a **clear “source of truth” model**.

## Document inventory (what each doc is for)

- **Use case catalog**: `docs/use-cases.md`
  - High-level actor + feature catalog (UC-xx / UC-xx.y)
  - Should answer **“what the system must do”**, not define API or data models.

- **Functional Requirements**: `docs/functional-requirements.md`
  - Defines **FR-xx.y** functions with inputs/outputs + **business rules (BR-xx.y.z)**.
  - Also defines **API endpoints** per function (this is important for duplication risk).

- **Validation pack**:
  - `docs/validation/checklist.md`: stakeholder validation + implementation readiness checklist.
  - `docs/validation/edge-cases.md`: edge cases + error paths + cross-cutting concerns.

- **Architecture/code review**: `docs/ARCHITECTURE_REVIEW.md`
  - Maps repo structure and identifies engineering gaps/risks (tenant isolation, auth, contract pipeline, maturity).

## Primary duplication patterns (what’s repeated)

### 1) “Business rules” are duplicated between FR and edge-cases

Examples:
- **Account lockout** appears as BR in FR and as a login edge-case rule.
- **Password reset security** (“don’t reveal if email exists”) appears in both.
- **Tenant isolation** appears in edge-cases as SQL guidance; FR repeats tenant-related BRs inside feature sections.

**Recommendation**
- Keep **business rules canonical in `docs/functional-requirements.md`** (BR-*).
- Keep `docs/validation/edge-cases.md` for **non-happy-path scenarios**, and when it repeats a BR, replace with a short reference: “See BR-14.2.1”.

### 2) API endpoint definitions are duplicated between FR and TypeSpec

FR defines a very broad API surface (notifications, announcements, messages, forms, etc.).
TypeSpec currently defines only a subset of domains (auth/organizations/buildings/units/faults/voting/documents/rentals/listings/compliance).

**Recommendation**
- Choose one as the **API contract source of truth**:
  - **Prefer** TypeSpec/OpenAPI as the contract (as your docs already claim).
  - Treat FR endpoint lines as **“intended mapping”** and keep them as references only, or remove them once TypeSpec is authoritative.

## Hard contradictions (docs disagree with each other)

### 1) Use-case counts disagree across docs

- `docs/use-cases.md` ends with **TOTAL = 479**.
- `docs/functional-requirements.md` states: “decomposes all **493** use cases”.
- `docs/CLAUDE.md` and other docs refer to “**407** use cases” in multiple places.

**Recommendation**
- Pick **one canonical count** and update the others.
- Best option: make `docs/use-cases.md` canonical and update:
  - `docs/CLAUDE.md`
  - `docs/functional-requirements.md`
  - any tables in `docs/validation/checklist.md`

### 2) Fault status model conflicts (FR vs edge-cases vs TypeSpec)

- `docs/validation/edge-cases.md` shows a transition diagram using states like **New / In Progress / Resolved / Closed / Escalated / Reopened**.
- `docs/functional-requirements.md` BR examples also use **new → in_progress → resolved → closed**.
- TypeSpec `docs/api/typespec/domains/faults.tsp` defines statuses like:
  - `reported`, `acknowledged`, `in_progress`, `on_hold`, `resolved`, `closed`, `rejected`.

This is not just wording—this changes workflow and UI filtering semantics.

**Recommendation**
- Define the **canonical `FaultStatus` enum and transitions** in exactly one place:
  - Prefer **TypeSpec** for enum values (because SDKs are generated from it).
  - Reference it from FR and edge-cases.

### 3) Endpoint shapes conflict (FR vs TypeSpec)

Examples:
- **Faults**
  - FR: many endpoints are nested under buildings (e.g. `POST /api/v1/buildings/{buildingId}/faults`)
  - TypeSpec: `POST /api/v1/faults` with `buildingId` in body; list uses `buildingId` query param
- **Voting**
  - FR: uses building-scoped endpoints and search endpoints (e.g. `/buildings/{id}/votes/search`)
  - TypeSpec: canonical base is `@route("/api/v1/votes")`
- **User profile**
  - FR: `PUT /api/v1/users/profile`
  - TypeSpec: profile is under `/api/v1/auth/me` (GET/PATCH)

**Recommendation**
- Decide routing conventions once:
  - “flat + query/body references” (TypeSpec current direction), or
  - “nested resources” (FR current direction)
- Then update the other doc layer to match.

### 4) Organization type / tier enums conflict

- FR organization types: `housing_coop | management_company`
- TypeSpec includes: `housing_cooperative | property_management | real_estate_agency | individual`

**Recommendation**
- Canonicalize enum values in TypeSpec; treat FR’s values as outdated aliases or update them to match.

## Alignment gaps (docs vs code)

### 1) Backend router coverage (corrected)

> **Corrected 2026-06-08 (PAP-25).** This section previously claimed the
> `api-server` mounts **only 8 routers** (`/auth, /organizations, /buildings,
> /faults, /voting, /rentals, /listings, /integrations`). That was **~9× stale** —
> it captured only the first MVP slice.

Reality (audited at HEAD `a8a65b0fe` in
[`docs/EPIC_STORY_STATUS.md`](./EPIC_STORY_STATUS.md)): the `api-server` mounts
**71 `/api/v1/*` route groups**, and every documented epic has backing routes +
migrations. Notifications / announcements / messages and the rest are mounted.

The *residual* gap is no longer "missing backend routers" but **UI-surface
routing**: a few already-built `ppt-web` feature dirs (voting, meters, leases)
are not yet wired into `AppRoutes.tsx`. See `EPIC_STORY_STATUS.md` §5 (FR
coverage) and §7 (follow-ups).

**Recommendation**
- Track implementation via the layered sources of truth:
  - TypeSpec endpoints (contract)
  - Rust routes/handlers (implementation)
  - `EPIC_STORY_STATUS.md` (delivery status vs codebase)
  - Validation checklist (readiness)

### 2) Contract generation pipeline mismatch (“by-service” specs)

Many scripts and docs refer to `docs/api/generated/by-service/*.yaml`, but current TypeSpec config emits only `docs/api/generated/openapi.yaml`.

**Recommendation**
- Standardize either:
  - single consolidated spec for all SDK generation, or
  - actually generate by-service specs.

## Proposed “source of truth” model (recommended)

- **Use Cases (`docs/use-cases.md`)**: product catalog, actors, scope.
- **Functional Requirements (`docs/functional-requirements.md`)**: BRs + acceptance-level IO definitions.
  - Avoid duplicating the API contract; reference TypeSpec.
- **API Contract (`docs/api/typespec/**`)**: canonical API paths, request/response models, enums.
- **Validation (`docs/validation/**`)**: stakeholder sign-off + edge cases + test readiness.
- **Architecture Review (`docs/ARCHITECTURE_REVIEW.md`)**: engineering risks, implementation maturity, structure.

## Concrete next doc edits (high value)

1) **Fix UC count drift**
   - Update "407/493" references to match the canonical catalog.

2) **Canonicalize FaultStatus + transitions**
   - Define transitions (or reference them) once; align FR + edge-cases to TypeSpec.

3) **Resolve API routing convention**
   - Flatten vs nested resources; update FR endpoint lines or TypeSpec accordingly.

4) **Remove/clarify duplication**
   - Replace repeated BR text in edge-cases with BR references.
   - Add cross-links: each FR section should point to relevant validation edge cases.

---

## Resolution Status

**Decision: Option A - TypeSpec/OpenAPI is the canonical API contract**

### Fixed Issues

#### 1. UC Count Drift - RESOLVED
- **Actual count:** 508 use cases across 51 categories
- **Updated docs:**
  - `docs/use-cases.md` - Fixed total from 493 to 508
  - `docs/CLAUDE.md` - Updated all references to 508
  - `docs/testability-and-implementation.md` - Updated summary

#### 2. Canonical Enums - RESOLVED (TypeSpec is source of truth)

**FaultStatus** (from `docs/api/typespec/domains/faults.tsp`):
```
reported → acknowledged → in_progress → on_hold → resolved → closed
                                    └─→ rejected
```
Values: `reported`, `acknowledged`, `in_progress`, `on_hold`, `resolved`, `closed`, `rejected`

**OrganizationType** (from `docs/api/typespec/domains/organizations.tsp`):
- `housing_cooperative`
- `property_management`
- `real_estate_agency`
- `individual`

**FaultCategory**: `plumbing`, `electrical`, `hvac`, `structural`, `elevator`, `security`, `cleaning`, `landscaping`, `other`

**FaultPriority**: `low`, `medium`, `high`, `critical`

#### 3. API Routing Convention - RESOLVED
- **Decision:** Flat routes with query/body references (TypeSpec pattern)
- **Pattern:** `/api/v1/{resource}` with `buildingId` as query param or body field
- **FR endpoint lines:** Treated as "intended mapping", TypeSpec is authoritative

### Source of Truth Model

| Layer | Document | Purpose |
|-------|----------|---------|
| Product | `docs/use-cases.md` | What the system must do (508 UCs) |
| Requirements | `docs/functional-requirements.md` | Business rules (BR-*), acceptance criteria |
| API Contract | `docs/api/typespec/**` | Canonical endpoints, DTOs, enums |
| Validation | `docs/validation/**` | Edge cases, error paths (reference BRs) |
| Architecture | `docs/architecture.md` | System design, ADRs |
| Implementation | `docs/technical-design.md` | DTOs, state machines (derived from TypeSpec) |

### Cross-Reference Convention

When FR or edge-cases need to reference TypeSpec enums:
```markdown
**Status:** See `FaultStatus` enum in `docs/api/typespec/domains/faults.tsp`
```

When edge-cases need to reference business rules:
```markdown
**Rule:** See BR-14.2.1 in `docs/functional-requirements.md`
```



