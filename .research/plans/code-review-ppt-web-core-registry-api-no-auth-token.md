# code-review-ppt-web-core-registry-api-no-auth-token

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review ppt-web-core 2026-10-07 (routine Phase 1.5)
**Confidence:** high

## Hypothesis

The ppt-web Registry feature (Epic 57 — pets/vehicles/parking) constructs its API client at module scope with no auth material: `createRegistryApi({ baseUrl: API_BASE_URL })` is called without `accessToken` or `tenantId`. `createRegistryApi` bakes `Authorization header (Bearer scheme) <token>` and `X-Tenant-ID` into the fetch config *at construction time*, so every pets/vehicles/rules/parking call goes out with no bearer and no tenant header, and the client never rebuilds when the token rotates. The fix mirrors the pattern already used by `useBuildings.ts` — hoist construction into a hook keyed on the current access-token + tenant from `AuthContext`, so the api/hooks instance is re-created when either changes.

## Evidence

- `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx:14-21` — module-scope `const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''`; `const registryApi = createRegistryApi({ baseUrl: API_BASE_URL })` with a TODO comment: "accessToken and tenantId would come from auth context".
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:19` — same pattern, same omission.
- `frontend/packages/api-client/src/registry/api.ts` — `createRegistryApi(config)` reads `config.accessToken` and `config.tenantId` at construction and bakes them into every request header; no runtime token refresh hook.
- Comparator (correct pattern): `frontend/apps/ppt-web/src/features/buildings/hooks/useBuildings.ts:28-33` — memoizes client construction keyed on `getToken()`, so a rotation triggers rebuild.
- Related historical bug class: PR #3004 (`#2982` follow-up) and PR #2979 fixed raw-fetch bypasses; this is the builder-pattern analogue missed by that sweep.

## Files

- `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx:18`
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:19`
- `frontend/packages/api-client/src/registry/api.ts`
- `frontend/apps/ppt-web/src/contexts/AuthContext.tsx`

## Dependencies

<!-- no blocking dependencies -->

## Required capabilities

- [x] C1 — Systematic debugging (bug class — missing header path)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Start ppt-web locally (or point `VITE_API_BASE_URL` at a dev api-server).
2. Log in to the manager role, open DevTools → Network tab.
3. Navigate to `/registry` and click the Pets tab.
4. Observe the outgoing `GET /api/v1/buildings/{id}/registrations/pets` request headers.
5. **Expected:** `Authorization header (Bearer scheme) <token>` and `X-Tenant-ID: <org>` present.
6. **Actual:** neither header present; the server either 401s (feature appears broken) or — on a dev build without server-side auth — returns global-scope data.

## Suggested approach

1. In `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx`, remove the module-scope `createRegistryApi` call. Replace with a hook that reads auth from context:
   ```tsx
   const { getToken, tenantId } = useAuth();
   const registryApi = useMemo(
     () => createRegistryApi({ baseUrl: API_BASE_URL, accessToken: getToken(), tenantId }),
     [getToken(), tenantId]
   );
   const registryHooks = useMemo(() => createRegistryHooks(registryApi), [registryApi]);
   ```
2. Mirror the change in `frontend/apps/ppt-web/src/features/registry/pages/RegistryRulesPage.tsx:19`.
3. (Optional cleanup) Extract the hook into `frontend/apps/ppt-web/src/features/registry/hooks/useRegistryApi.ts` so both pages share one implementation.
4. Delete the misleading comments at `RegistryPage.tsx:17-20` and `RegistryRulesPage.tsx:17-20`.
5. Verify the memo key matches what `useBuildings.ts:28-33` does (whichever auth-rotation primitive the AuthContext exposes — `getToken()` call vs a reactive `accessToken` state — check which triggers a re-render on refresh).
6. Add a vitest in `RegistryPage.test.tsx` using the recording-adapter pattern from `useAiChat.test.tsx` / `useBuildingUnits.test.tsx`: assert the outgoing request carries `Authorization header (Bearer scheme) <mocked-token>` and `X-Tenant-ID: <mocked-tenant>`.
7. Run `pnpm --filter @ppt/web test` and `pnpm --filter @ppt/web typecheck`.

## Alternatives considered

- **Pass accessToken/tenantId as props to RegistryPage / RegistryRulesPage** — rejected because the parent routes don't thread them today, and threading them manually re-creates the problem on the next page that forgets.
- **Patch `createRegistryApi` to read from a global auth store** — rejected because the builder is designed to be auth-agnostic (used by both app contexts in principle); the fix belongs at the construction site, where the auth context is in scope. Mirrors how `useBuildings` already handles it.

## Root-cause trace

1. Symptom: Registry page's pets/vehicles tabs issue API requests without `Authorization header (Bearer scheme)` and `X-Tenant-ID` headers.
2. ← Immediate cause: `createRegistryApi({ baseUrl: API_BASE_URL })` at `RegistryPage.tsx:18` omits `accessToken` and `tenantId`.
3. ← Upstream cause: construction happens at module scope (outside any component), so auth context isn't accessible — the TODO comment on L20 acknowledges this but was never resolved.
4. Origin: Epic 57 initial scaffold — the comment "in production, this would come from a context with auth tokens" was shipped unfixed. Mirrors the raw-fetch class fixed in PR #2979 / #3004 but missed because this bypass hides behind a builder.

## Test plan

- [ ] `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.test.tsx` — IG3 recording-adapter test asserting the pets-list GET carries `Authorization header (Bearer scheme) <t>` and `X-Tenant-ID: <org>`.
- [ ] `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.test.tsx` — token-rotation test: after `getToken()` returns a new value, a subsequent query carries the new bearer.
- [ ] Local verify: `pnpm --filter @ppt/web test -- registry` + `pnpm --filter @ppt/web typecheck`.

## Out of scope

- Changing `createRegistryApi` builder shape (fix lives at the call site, not inside the generated-client package).
- Fixing any other missing-auth call sites surfaced by the review (e.g. `code-review-ppt-web-core-registries-root-logout-leak` has its own plan).
- Rewriting `AuthContext` or `useBuildings`'s memo pattern — reuse what exists.

## After-merge

- Move this file to `plans/_archive/code-review-ppt-web-core-registry-api-no-auth-token.md`
- Mark the matching `backlog.json` row as `status: "done"`
