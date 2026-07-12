# security-sso-client-role-escalation

**Vector:** security
**Score:** 3
**Source:** Issue #2249 | reality-server SSO handler
**Confidence:** high

## Hypothesis
`exchange_pm_token` in `backend/servers/reality-server/src/routes/sso.rs` uses `extract_roles_from_scope(...).or_else(|| request.roles.clone())` at lines 923-924. When the introspected PM token's scope carries no recognizable roles, the code trusts the **client-supplied** `request.roles` verbatim and maps the highest-privilege value via `role_mapping::map_pm_role_to_portal`. Any holder of an active-but-role-less PM token can therefore self-elevate to `AGENT` / `PROPERTY_OWNER` / `VERIFIED_USER` by sending `roles: ["agent"]` in the exchange request body. The `.or_else` fallback turns a field intended to *narrow* roles into a *source* of them.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:923-924` — `let pm_roles = extract_roles_from_scope(token_info.scope.as_deref()).or_else(|| request.roles.clone()).unwrap_or_default();`
- `backend/servers/reality-server/src/routes/sso.rs:849-855` — `pub roles: Option<Vec<String>>` in `ExchangeTokenRequest`, documented "Optional: specific PM roles to include (for role filtering)"
- `backend/servers/reality-server/src/routes/sso.rs:904-910` — introspection only asserts `token_info.active`; never cross-checks requested roles against the token subject
- `backend/servers/reality-server/src/routes/sso.rs:809-841` — `map_pm_role_to_portal` + `get_portal_permissions`; `AGENT` receives listing/inquiry write permissions
- Issue #2249 — filed by dispatcher's Tier-1d review 2026-07-11

## Files
- `backend/servers/reality-server/src/routes/sso.rs:923`
- `backend/servers/reality-server/src/routes/sso.rs:849`

## Dependencies
<none>

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Obtain a PM access token whose introspection returns `active: true` but whose scope carries no recognized PM role (`extract_roles_from_scope` returns `None`) — e.g. a token minted with only base scopes.
2. POST `/api/v1/sso/exchange` with body:
   ```json
   { "pm_access_token": "<token>", "roles": ["real_estate_agent"] }
   ```
3. Expected: response rejected, or portal role clamped to `USER` (the token's true role).
4. Actual: response includes `portal_role = AGENT` and the `AGENT` permission set (listings + inquiries write), and a portal session cookie is minted for that elevated role.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/sso.rs` around line 921, replace the `.or_else` fallback with an *intersect-only* strategy. Concretely:
   - Compute `scope_roles = extract_roles_from_scope(...).unwrap_or_default();`
   - If the client supplied `request.roles`, filter it to values also present in `scope_roles` (case-insensitive after normalization).
   - If `scope_roles` is empty, treat the request as role-less: proceed with `USER`, never with client-supplied values.
2. When scope carries no recognizable roles AND the introspection lacks a role claim, prefer calling the PM `/userinfo` (already reachable via `get_user_info`, see sso.rs:912) to fetch the authoritative role list before falling back to `USER`. This preserves the "narrow" intent for well-scoped tokens without opening the escalation surface.
3. Log a `warn!` line whenever `request.roles` contains values not in `scope_roles` (or `userinfo` roles) — that pattern indicates an attempted escalation.
4. Update the `ExchangeTokenRequest.roles` doc string (sso.rs:852) to make the narrowing-only semantic explicit.
5. Add a handler-level regression test (Files below).

## Alternatives considered
- **Reject the request outright when scope-derived roles are empty and `request.roles` is present** — rejected because it changes the API contract for legitimate no-scope-role token holders (they'd start getting 401s where they used to get `USER`). Silent-narrow-to-USER preserves the current behaviour for honest callers while closing the escalation.
- **Introspect + trust the raw scope string without any role parsing** — rejected because it hardcodes the PM-provider scope shape into the reality-server and doesn't handle the legitimate case where the token was minted for a subject that later gained an elevated PM role (the `userinfo` refresh path handles that).

## Root-cause trace
1. Symptom: portal caller with a role-less PM token receives `AGENT` portal role after supplying `roles: ["agent"]` in the SSO exchange body.
2. ← `pm_roles` at `backend/servers/reality-server/src/routes/sso.rs:921-924` reads the client-supplied `request.roles` because `extract_roles_from_scope` returned `None`.
3. ← `ExchangeTokenRequest.roles` at `backend/servers/reality-server/src/routes/sso.rs:849-855` was designed as a *narrowing* filter but wired as a *source* by the `.or_else` clause.
4. Origin: the `.or_else` fallback was added to keep the flow functional for legacy tokens without scope-embedded roles (see the surrounding "Get PM roles from token scope or fetch from PM API" comment at sso.rs:920). The `or_else(client_roles)` shortcut short-circuited the intended "fetch from PM API" step and turned filter-input into role-source.

## Test plan
- [ ] `backend/servers/reality-server/tests/sso_exchange_role_intersect_tests.rs` — new file. Test cases:
  1. Token with scope-derived `["tenant"]` + `request.roles == None` → `portal_role == USER` (baseline).
  2. Token with scope-derived `["real_estate_agent"]` + `request.roles == ["real_estate_agent"]` → `portal_role == AGENT` (legitimate narrowing).
  3. Token with scope-derived `["real_estate_agent"]` + `request.roles == ["tenant"]` → `portal_role == USER` (client narrows down).
  4. **Regression case**: token with scope-derived `[]` + `request.roles == ["real_estate_agent"]` → `portal_role == USER` (must NOT elevate).
  5. Token with scope-derived `[]` + `request.roles == None` → `portal_role == USER`.
- [ ] `backend/servers/reality-server/tests/sso_exchange_role_intersect_tests.rs::attempt_escalation_logs_warn` — assert the `warn!` fires when client roles exceed scope roles.
- [ ] Run locally: `cargo test -p reality-server --test sso_exchange_role_intersect_tests`.
- [ ] Full reality-server suite: `cargo test -p reality-server`.

## Out of scope
- Refactoring `role_mapping::pm_roles` / `portal_roles` constants (only the exchange handler is in scope).
- Changing the `/api/v1/sso/roles` GET handler (`get_mapped_roles`) — different endpoint, separate audit.
- Rewriting `extract_roles_from_scope` — the parser is fine; the caller is the bug.

## After-merge
- Move this file to `plans/_archive/security-sso-client-role-escalation.md`
- Mark the matching `backlog.json` row as `status: "done"`
