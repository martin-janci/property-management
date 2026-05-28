# Specialist: react-web

React + Vite + TanStack Query implementer for `frontend/apps/ppt-web` and
`frontend/apps/admin-web`.

## You own
- `frontend/apps/ppt-web/src/` — manager-facing SPA (buildings, faults, announcements, messages, documents, …)
- `frontend/apps/admin-web/src/` — platform/super-admin UI
- API client hooks in `frontend/packages/api-client/` when adding new endpoints
- Screen-map updates in `docs/screens/ppt/<id>.md` per CLAUDE.md "Screen-Map Self-Management Protocol"

## Project layout cheatsheet
```
frontend/
  apps/
    ppt-web/src/
      features/<area>/   — pages, components, hooks per feature
      lib/api.ts         — axios client with interceptors
      lib/queryKeys.ts   — TanStack queryKeys factory
      providers/         — Auth, Toast, WebSocket, Router
    admin-web/src/       — similar layout
  packages/
    api-client/          — @ppt/api-client — generated + hand-rolled hooks
```

## Conventions
- TanStack Query: `useQuery({queryKey: queryKeys.foo.list(orgId), queryFn: () => api.foo.list(orgId)})`.
- Mutations invalidate the relevant `queryKey` on success.
- Use Toast (`useToast()`) for success/error feedback — never `alert()`.
- Form state: `react-hook-form` + `zod` (already a dependency).
- Routes live in `App.tsx` (ppt-web) — protected via `<ProtectedRoute>`.
- After adding/removing a route: update `docs/screens/ppt/<id>.md` frontmatter (`buildStatus`, `apiStatus`) and add an Agent Log entry.

## Step-by-step
1. If task adds a new API call: add the hook in `packages/api-client/src/<area>/hooks.ts` first (or regenerate if it's a TypeSpec endpoint — defer to `typespec` specialist).
2. Add/modify the page/component under `apps/<app>/src/features/<area>/`.
3. Wire it into the route table in `App.tsx`.
4. Update the screen-map markdown (see CLAUDE.md Section "Screen-Map").
5. Promote screen `apiStatus: stub` → `partial` / `complete` once the call is real.

## Verify (MANDATORY)
```bash
pnpm -F ppt-web typecheck     # or -F admin-web
pnpm -F ppt-web lint
```
Quote both exit codes. If you touched a generated API type, also:
```bash
pnpm -F @ppt/api-client build
```

## Common pitfalls
- Forgetting to invalidate the list query after a mutation → stale UI.
- Hard-coding org/user IDs → use `useAuth()` / `useOrg()` selectors.
- Skipping the screen-map update → drift; `/screens validate` will fail.
- Adding a fetch outside a hook → bypasses interceptors (auth, error parsing).

## Return-line examples
- `pr=513 status=done specialist=react-web note=wired TwoFactorAuthPage to useMfa hooks; typecheck+lint clean`
- `pr=none status=blocked specialist=react-web note=needs backend route /api/v1/auth/mfa/setup first`
