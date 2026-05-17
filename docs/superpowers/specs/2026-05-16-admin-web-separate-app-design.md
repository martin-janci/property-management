# Separate super-admin web app (admin.rlt.sk)

**Status:** Draft
**Date:** 2026-05-16
**Authors:** Martin Janci

## Context

PR #260 (Multitenancy phases 2-5.5) introduced a super-admin control plane in
`ppt-web` mounted at `/admin/*`. Pages live in
`frontend/apps/ppt-web/src/features/admin/`, components in
`@ppt/admin-ui` (frontend package), backend routes on `api-server` under
`/api/v1/admin/*` and `/api/v1/platform-admin/*`.

The admin section is a platform-wide control plane: it manages both Reality
Portal and Property Management tenants, capabilities, audit, impersonation,
feature flags, tenant lifecycle. Bundling it inside `ppt-web` mixes a
single-product tenant UI with a cross-product operator UI.

This spec moves the super-admin UI to its own domain (`admin.rlt.sk`) and its
own Vite app (`frontend/apps/admin-web/`), atomically deployed alongside the
existing four services via the blue/green pipeline.

## Goals

- Logical separation: admin is the control plane for both products and lives
  on its own host, distinct from tenant-facing apps.
- Security isolation: tenant users never download admin JS; admin cookies and
  storage are scoped to `admin.rlt.sk` only.
- Independent deploy/rollback: admin-web ships as its own image and its own
  blue/green container, flipped atomically with the rest of the stack.
- Smaller `ppt-web` bundle: no admin pages, no `@ppt/admin-ui` components in
  the tenant bundle.
- Operability: admin traffic is a distinct nginx access stream, easy to filter
  and audit separately from tenant traffic.

## Non-goals

- Per-agency / org-admin UI (managing one's own agency members, branding) —
  stays in `ppt-web`; this spec is only the super-admin control plane.
- Federated IdP / external SSO for admin login — Phase 2 follow-up.
- A standalone Rust `admin-server` binary — admin endpoints stay on
  `api-server`.
- Wildcard `*.reality.rlt.sk` per-agency subdomains — blocked by Cloudflare
  Universal SSL (out of scope here).
- Admin-specific Sentry / RUM — follow-up.

## Architecture

```
                       ┌────────────────────────────────────────┐
                       │   admin.rlt.sk (Cloudflare proxy)      │
                       └─────────────────┬──────────────────────┘
                                         │
                       ┌─────────────────▼──────────────────┐
                       │   ppt-caddy                        │
                       │   site admin.rlt.sk:               │
                       │     /api/* → prod-api-{color}      │
                       │     /*     → prod-admin-web-{color}│
                       └────┬────────────────────────┬──────┘
                            │  ppt-prod net          │
                  ┌─────────▼──────┐      ┌──────────▼─────────┐
                  │ prod-admin-web │      │ prod-api-{color}   │
                  │ -{color}       │      │ Rust, /admin/*     │
                  │ nginx static   │      └────────────────────┘
                  └────────────────┘
```

Untouched: `ppt.rlt.sk`, `www.rlt.sk`, `rlt.sk`, `api.rlt.sk`,
`api.ppt.rlt.sk`, `reality.rlt.sk`. Their containers and routes stay as-is.

### New components

| Component | Purpose |
|---|---|
| `frontend/apps/admin-web/` | Vite SPA, root-mounted admin router |
| `ghcr.io/martin-janci/ppt-admin-web` | Docker image (multi-stage build → nginx-static) |
| `{target}-admin-web-{blue,green}` | Container per blue/green slot, per target |
| Caddy site `admin.rlt.sk` (and `admin.staging.rlt.sk`) | Registered by ppt-deploy at promote time |
| `BlueGreenSpec.admin_web_image` | New field in ppt-deploy Rust spec |

### Untouched

- `api-server` admin routes (no change in PR #260 surface).
- `@ppt/admin-ui` package — reused by `admin-web` exactly as-is.
- `@ppt/api-client` — consumed from `admin-web` exactly as-is.
- DB schema — except a small seed adding `admin.rlt.sk` and
  `admin.staging.rlt.sk` to `reserved_platform_hosts`.

## Data flow

### Login

1. Browser opens `https://admin.rlt.sk/`.
2. Admin SPA: no token in `sessionStorage` → redirect `/login`.
3. `/login` posts `admin.rlt.sk/api/v1/auth/login` (Caddy proxies to
   `api-server`).
4. Response: `access_token` (15m JWT), `refresh_token` (7d httpOnly cookie
   with `Domain=admin.rlt.sk; Secure; SameSite=Strict; Path=/`).
5. SPA stores `access_token` in `sessionStorage` (not `localStorage`: tab
   close = logout for admin, intentional).
6. SPA calls `GET /api/v1/admin/capabilities/me` with `Authorization: Bearer
   <token>`.
7. Any capability-gated call returning `401 mfa_required` triggers
   `MfaChallengeModal` (existing `@ppt/admin-ui` component). Successful
   verify refreshes capabilities.
8. Dashboard renders.

### Cookie and token isolation

- `refresh_token` cookie domain is exactly `admin.rlt.sk` (not `.rlt.sk`).
  `ppt.rlt.sk` JavaScript cannot read it; the browser will not send it on
  `ppt.rlt.sk` requests.
- `access_token` in `sessionStorage` is same-origin-isolated per browser
  policy.
- Logging in on `ppt.rlt.sk` does not log you in on `admin.rlt.sk`. Both apps
  must be authenticated independently.

### API calls

Admin SPA does `fetch('/api/v1/admin/...', { credentials: 'include' })`.
Caddy site `admin.rlt.sk` matches `/api/*`, reverse-proxies to
`prod-api-{color}:8080`, preserving the `Host: admin.rlt.sk` header.

`api-server` `host_tenant_middleware` resolves `admin.rlt.sk` via
`reserved_platform_hosts` → `TenantSource::PlatformHost`. The `/admin/*`
router runs `RequireCapability` and serves the response.

CORS: not applicable. Browser sees same-origin (`admin.rlt.sk` for both UI
and API). `CORS_ALLOWED_ORIGINS` env var on `api-server` does not need
`admin.rlt.sk`.

### Error handling (frontend)

| Server response | Client behavior |
|---|---|
| 401 `unauthenticated` | Clear sessionStorage, redirect `/login` |
| 401 `mfa_required` | `MfaChallengeProvider` opens modal; on verify, retry original request |
| 403 `forbidden_capability` | Toast with capability name; capability-gated UI affordances remain hidden via `useCapability` |
| 5xx | Toast "Server error", retry button, sentry breadcrumb (when Sentry is wired in a follow-up) |
| Network error | TanStack Query auto-retry 2× with exponential backoff, then error UI |

Backend error model: unchanged. `AdminError` enum and JSON error envelope
from PR #260.

## Code organization

### New files (PR-1)

```
frontend/apps/admin-web/
├── package.json                    # name: @ppt/admin-web
├── vite.config.ts                  # base: '/', allowedHosts: ['admin.rlt.sk',
│                                   #   'admin.staging.rlt.sk', 'localhost']
├── tsconfig.json
├── index.html
├── nginx/default.conf              # try_files SPA fallback + /health
├── Dockerfile                      # node build → nginx:alpine static
└── src/
    ├── main.tsx                    # ReactDOM.render + QueryClient + BrowserRouter
    ├── App.tsx                     # Routes: /login, /, /agencies, /users, /audit,
    │                               #   /feature-flags, /platform, /impersonation
    ├── contexts/
    │   ├── AdminAuthContext.tsx    # sessionStorage token, refresh handler,
    │   │                           #   axios interceptor for 401
    │   └── MfaChallengeProvider.tsx  # wraps @ppt/admin-ui MfaChallengeProvider
    ├── pages/
    │   ├── LoginPage.tsx
    │   ├── Dashboard.tsx           # root /, capability cards
    │   ├── agencies.tsx            # moved from ppt-web/features/admin/pages/
    │   ├── users.tsx               # moved
    │   ├── audit.tsx               # moved
    │   ├── feature-flags.tsx       # moved
    │   └── platform.tsx            # moved
    ├── components/
    │   ├── AdminLayout.tsx         # sidebar nav, capability-aware
    │   └── ProtectedRoute.tsx
    └── api/
        └── client.ts               # axios wrapper, baseURL '/api',
                                    #   credentials: 'include', 401 interceptor
```

The `pages/*` files are physically copied from
`frontend/apps/ppt-web/src/features/admin/pages/`. They keep the same
imports from `@ppt/admin-ui` and `@ppt/api-client`. Routing wrapper changes
(no `/admin` prefix; root path is the dashboard).

### Removed from ppt-web (PR-3, after admin.rlt.sk is verified)

- Whole `frontend/apps/ppt-web/src/features/admin/` directory.
- `<Route path="/admin/*">` in `App.tsx`.
- `<Link to="/admin">` in the nav.
- Imports of `AdminRouter`, `ImpersonationWrapper`, `usePrincipalCapabilities`.
- `MfaChallengeProvider` import from `@ppt/admin-ui` stays — it is also used
  for non-admin MFA flows in PM (e.g. settings/two-factor).

### Transition: parallel operation, no code flag

After PR-2 deploys, both URLs work: `https://ppt.rlt.sk/admin` (old, still in
the `ppt-web` build) and `https://admin.rlt.sk/` (new). There is no runtime
feature flag; the two SPAs run in parallel because their code paths are
independent.

If admin.rlt.sk fails verification, PR-3 (the cutover) is postponed and
`pmctl rollback prod` removes the admin-web container. `ppt.rlt.sk/admin`
keeps working. Caddy admin.rlt.sk site can stay or be manually unregistered.

### PR sequence

| PR | Branch | Scope | Verification |
|---|---|---|---|
| PR-1 | `feature/admin-web-app` | New `admin-web` app, copied pages, Dockerfile, CI workflow, unit + integration tests | `pnpm -F @ppt/admin-web build` clean; local docker run reaches dashboard against staging api |
| PR-2 | `feature/admin-web-deploy` | ppt-deploy Rust patch for 5th container, Caddy site templates, CF DNS, `reserved_platform_hosts` migration, runbook update | `pmctl promote prod main` deploys all 5 atomically; `admin.rlt.sk` returns 200 and login + MFA work; rollback works |
| PR-3 | `feature/admin-web-cutover` | Remove `features/admin/` from `ppt-web`, remove `/admin/*` route and nav link, build verify, deploy | `ppt.rlt.sk/admin` returns 404; `admin.rlt.sk` keeps working; `ppt-web` bundle smaller (verified via bundle analyzer) |

Rough size: PR-1 ~800-1200 LOC, PR-2 ~200 LOC Rust + config, PR-3 ~300 LOC
delete.

## Deploy infrastructure

### Cloudflare DNS

```
admin.rlt.sk          A    178.105.92.238   proxied=true
admin.staging.rlt.sk  A    178.105.92.238   proxied=true
```

Universal SSL `*.rlt.sk` covers both. No Advanced Certificate Manager needed.

### Caddy site (templated by ppt-deploy at promote time)

```
admin.{reality_apex} {
  handle /api/* {
    reverse_proxy {target}-api-{color}:8080
  }
  handle {
    reverse_proxy {target}-admin-web-{color}:80
  }
}
```

Concretely: `admin.rlt.sk` for prod (`reality_apex = rlt.sk`),
`admin.staging.rlt.sk` for staging (`reality_apex = staging.rlt.sk`). The
template reads the existing `reality_apex` field in `targets.yaml`; no new
`admin_apex` field is required.

ppt-deploy registers the site via the Caddy admin API at promote time, the
same way it does for the existing four sites. There is no static Caddyfile
entry for admin.

### ppt-deploy Rust patches

Files in `/opt/ppt-deploy-build/servers/deploy-server/`:

| File | Change |
|---|---|
| `crates/deploy-types/src/spec.rs` (or equivalent) | Add `admin_web_image: String` to `BlueGreenSpec` |
| `src/api/blue_green.rs` `run_blue_green_color` | Create 5th container `{target}-admin-web-{color}` from `ghcr.io/martin-janci/ppt-admin-web:{tag}` |
| `src/api/blue_green.rs` health probe | Add admin-web health check (`curl -f http://localhost/health`) |
| `src/api/blue_green.rs` Caddy register | Append `admin.{apex_short}` site definition |
| `src/api/blue_green.rs` Caddy unregister | Remove the same on rollback / teardown |
| `crates/deploy-types/src/targets.rs` | No change — derive admin host from existing `reality_apex` as `admin.{reality_apex}` |
| `bootstrap-target` | No structural change; admin-web container appears at first deploy, network already exists |

Rebuild `ppt-deploy`, deploy via existing install method, restart systemd
unit.

### Docker image

```dockerfile
# frontend/apps/admin-web/Dockerfile
FROM node:20-alpine AS build
WORKDIR /app
COPY pnpm-lock.yaml package.json ./
COPY frontend ./frontend
RUN corepack enable && pnpm install --frozen-lockfile
RUN pnpm -F @ppt/admin-web build

FROM nginx:alpine
COPY --from=build /app/frontend/apps/admin-web/dist /usr/share/nginx/html
COPY frontend/apps/admin-web/nginx/default.conf /etc/nginx/conf.d/default.conf
HEALTHCHECK --interval=30s --timeout=5s CMD wget -qO- http://localhost/health || exit 1
EXPOSE 80
```

`nginx/default.conf`: `try_files $uri /index.html;` for SPA fallback, plus
`location = /health { return 200 'ok'; }`.

### CI workflow

New `.github/workflows/admin-web-build.yml`, triggers on changes in
`frontend/apps/admin-web/`, `frontend/packages/admin-ui/`,
`frontend/packages/api-client/`. Builds and pushes
`ghcr.io/martin-janci/ppt-admin-web:{main, <tag>, <sha>}` consistent with
the existing FE Docker workflows.

### `reserved_platform_hosts` seed

New migration `backend/crates/db/migrations/00147_reserved_admin_hosts.sql`:

```sql
INSERT INTO reserved_platform_hosts (host, reason) VALUES
  ('admin.rlt.sk',         'super-admin control plane (prod)'),
  ('admin.staging.rlt.sk', 'super-admin control plane (staging)')
ON CONFLICT (host) DO NOTHING;
```

## Testing

| Layer | What | Tool |
|---|---|---|
| Unit (admin-web) | `AdminAuthContext` token refresh, capability hook, `ProtectedRoute` redirect | Vitest + React Testing Library |
| Integration (admin-web) | Pages render from mocked api-client; capability-gated UI hidden without capability | Vitest + MSW |
| E2E (smoke) | login → MFA → dashboard → grant capability → audit log shows entry | Playwright against staging after PR-2 |
| Backend | No new tests — admin endpoints already covered by PR #260 | — |
| Deploy | Manual after PR-2: both `admin.rlt.sk` and `ppt.rlt.sk/admin` reachable; `pmctl rollback prod` keeps `ppt.rlt.sk/admin` working | shell |

## Observability

- nginx access log in admin-web container → `docker logs prod-admin-web-{color}`.
- No frontend Sentry / RUM in PR-1 — follow-up.
- Backend `support_activity_log` audit table (PR #260) captures every
  super-admin action — unchanged.

## Acceptance criteria

1. `https://admin.rlt.sk/` returns 200 and redirects to `/login` for
   unauthenticated visitor.
2. Login with a super-admin account followed by TOTP MFA challenge succeeds
   and lands on dashboard.
3. Capability-gated pages (e.g. `/agencies`) visible only to principals with
   the relevant capability.
4. Grant or revoke of a capability via the admin UI appends a row to
   `support_activity_log`.
5. After PR-3: `https://ppt.rlt.sk/` does not contain admin JS in its bundle
   (verified by bundle analyzer or browser network panel).
6. `pmctl promote prod main` flips all 5 containers atomically.
7. `pmctl rollback prod` flips all 5 containers back atomically.
8. A tenant user logged into `ppt.rlt.sk` is not logged into `admin.rlt.sk`
   (verified by cookies/sessionStorage inspection).

## Open questions

None at the time of writing.

## References

- PR #260 — Multitenancy phases 2-5.5
- `docs/multitenancy/ROADMAP.md`
- `docs/multitenancy/operability.md`
- `backend/crates/api-core/src/middleware/host_tenant.rs`
- `frontend/packages/admin-ui/src/index.ts`
