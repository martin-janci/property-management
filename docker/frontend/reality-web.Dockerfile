# Multi-stage Dockerfile for reality-web (Next.js SSR)
# Supports standalone output mode for optimized production deployment

# =============================================================================
# Version identity (T6) — declared before the first FROM so every stage can
# pick the value up with a bare `ARG` re-declaration. CI passes all three from
# .github/workflows/docker-frontend-images.yml, where APP_VERSION is the
# release version (never a branch name) and GIT_SHA is github.sha. The
# defaults apply to local builds only.
# =============================================================================
ARG APP_VERSION=0.0.0-dev
ARG GIT_SHA=unknown
ARG BUILD_DATE=unknown

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
# Node 22, not 20 (T7). `frontend/package.json` declares
# `engines.node: ^20.19.0 || >=22.12.0`, and the workspace has since picked up
# packages that exclude 20 outright — `rollup-plugin-visualizer@7.1.1`
# (`engines.node: >=22`, imported at module scope by
# `frontend/apps/ppt-web/vite.config.ts`, so it loads on every `vite build`) and
# `commander@15.0.0` (`>=22.12.0`). 22-alpine satisfies every `engines` range in
# `frontend/pnpm-lock.yaml`; 20 does not.
#
# NOT claimed to be the diagnosed cause of the 20 red runs. No `.npmrc` in this
# repo sets `engine-strict`, so pnpm only WARNS on a violated `engines` range,
# and `frontend.yml` builds ppt-web green on node-version '20' against this exact
# lockfile. Node 22 is the correct base because it is the one the declared ranges
# ask for — the actual failing step of those runs is still unread (the GitHub API
# was rate-limited), and the remaining hypotheses are in
# docs/runbooks/frontend-image-required-check.md.
FROM node:22-alpine AS deps

WORKDIR /app

# Copy the manifest of every workspace package this image's app depends on,
# transitively. A `workspace:*` dependency whose manifest is absent makes
# `pnpm install` fail with "in the dependencies field, no project of name X
# found", and because the list is enumerated by hand it used to rot silently
# whenever a package gained a new workspace dep.
#
# `scripts/check-frontend-docker-manifests.sh` now recomputes the transitive
# closure from the real package.json files and fails CI when a Dockerfile's
# COPY list no longer covers it, so the rot is loud. Run it locally before
# touching this list.
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
COPY frontend/packages/shared/package.json ./packages/shared/
COPY frontend/packages/ui-kit/package.json ./packages/ui-kit/
COPY frontend/packages/api-client/package.json ./packages/api-client/
COPY frontend/packages/reality-api-client/package.json ./packages/reality-api-client/
COPY frontend/packages/sitemap/package.json ./packages/sitemap/
COPY frontend/packages/dev-panel/package.json ./packages/dev-panel/
COPY frontend/packages/vite-plugin-ppt-worktree/package.json ./packages/vite-plugin-ppt-worktree/
COPY frontend/packages/admin-ui/package.json ./packages/admin-ui/
COPY frontend/packages/e2e/package.json ./packages/e2e/
COPY frontend/apps/reality-web/package.json ./apps/reality-web/

# pnpm is pinned in exactly ONE place: `frontend/package.json`'s
# `packageManager` field (pnpm@8.14.0, the version that wrote
# `frontend/pnpm-lock.yaml`, which is still `lockfileVersion: '6.0'`, and the
# version `pnpm/action-setup` installs in frontend.yml). `corepack install`
# reads that field, so CI and this image can no longer run different pnpm
# majors. The previous `corepack prepare pnpm@9.15.0 --activate` was worse than
# wrong, it was INERT: with corepack enabled, the `packageManager` field of the
# package.json in the working directory always wins, so the image silently ran
# 8.14.0 anyway while claiming 9.15.0. Verified locally:
# `corepack pnpm --version` in a tree carrying this package.json prints 8.14.0.
#
# It must come AFTER the manifests are copied — corepack resolves
# `packageManager` relative to the current working directory.
RUN corepack enable && corepack install

# `--frozen-lockfile`: without it the image resolved `^`-ranged specs afresh on
# every build, so the bundle CI green-lit and the bundle that shipped could
# contain different dependency versions, and a lockfile that no longer matched
# the workspace failed silently instead of loudly. Verified locally against
# this lockfile with the exact manifest set copied above (pnpm 8.14.0:
# "Scope: all 11 workspace projects / Done", lockfile byte-identical
# afterwards) and negative-controlled by bumping one spec, which then fails
# with ERR_PNPM_OUTDATED_LOCKFILE.
RUN pnpm install --frozen-lockfile

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:22-alpine AS builder

WORKDIR /app

# Copy dependencies from deps stage. Every workspace package whose source
# is imported (transitively) by the reality-web build needs its own
# `node_modules` here — pnpm's isolated/symlink layout gives each package
# directory its own resolution. Missing `packages/dev-panel/node_modules`
# was the root of the docker-frontend.yml CI break since the dev-panel
# landed: reality-web's `[locale]/layout.tsx` imports `<DevPanelMount />`,
# which pulls in `@ppt/dev-panel`, whose source imports `react` — and
# `next build`'s type-check phase failed with
# `Cannot find module 'react'` because dev-panel's @types/react wasn't
# materialized in its package directory.
#
# Adding `dev-panel` (the new culprit) plus `sitemap` (next-on-the-list
# transitive workspace dep) to the explicit copy list. ppt-web's
# Dockerfile sidesteps this by `COPY --from=deps /app/ ./` (whole tree);
# reality-web keeps the selective list because next.js's standalone
# output expects this exact layout for the runtime image.
#
# 2026-07-29: added `api-client` — reality-web pulls in `@ppt/api-client`
# transitively (`@ppt/shared`'s src/index.ts re-exports it, and shared is
# consumed at source), whose package.json declares `@tanstack/react-query`
# and `@tanstack/query-core` as direct deps. Under pnpm's isolated layout
# those materialize in `packages/api-client/node_modules`, which was never
# copied here, so `next build`'s Turbopack type-check failed with 38
# `Cannot find module '@tanstack/react-query'` errors, all in
# `packages/api-client/src/**`. See #2560.

# 2026-09-26: added `e2e`. `tsconfig.json` includes `**/*.ts`, which sweeps in
# `apps/reality-web/e2e/**/*.spec.ts`; those import `@ppt/e2e`, whose own
# devDependencies (`@playwright/test`, `@ppt/sitemap`) materialize in
# `packages/e2e/node_modules`. That directory was never copied, so `next build`
# died in type-check with `Cannot find module '@playwright/test'` in
# `packages/e2e/src/**` plus the TS7031 implicit-any fallout in every spec.
# Proven by the docker-frontend.yml run on PR #2971:
# https://github.com/martin-janci/property-management/actions/runs/35759371277
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules
COPY --from=deps /app/packages/ui-kit/node_modules ./packages/ui-kit/node_modules
COPY --from=deps /app/packages/api-client/node_modules ./packages/api-client/node_modules
COPY --from=deps /app/packages/reality-api-client/node_modules ./packages/reality-api-client/node_modules
COPY --from=deps /app/packages/dev-panel/node_modules ./packages/dev-panel/node_modules
COPY --from=deps /app/packages/sitemap/node_modules ./packages/sitemap/node_modules
COPY --from=deps /app/apps/reality-web/node_modules ./apps/reality-web/node_modules
COPY --from=deps /app/packages/e2e/node_modules ./packages/e2e/node_modules
# `vite-plugin-ppt-worktree` (package name `@ppt/vite-plugin-worktree`) is
# required(...) at the top of next.config.js. It resolves today only because it
# declares no runtime `dependencies` — the source arrives via `COPY frontend/`
# and needs nothing from its own node_modules. One `dependencies` entry would
# break the build; copy it so the list matches the declared closure.
COPY --from=deps /app/packages/vite-plugin-ppt-worktree/node_modules ./packages/vite-plugin-ppt-worktree/node_modules

# Copy source
COPY frontend/ ./

# Same single pnpm pin as the deps stage; `packageManager` arrives with the
# package.json copied above.
RUN corepack enable && corepack install

ARG NEXT_PUBLIC_API_URL=http://localhost:8081
ARG NEXT_PUBLIC_SITE_URL=http://localhost:3001

ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ENV NEXT_PUBLIC_SITE_URL=${NEXT_PUBLIC_SITE_URL}
ENV NEXT_TELEMETRY_DISABLED=1

# Bake the version identity into the bundle (NEXT_PUBLIC_* is inlined at build
# time) — the same values the image labels carry and `/api/version` serves.
ARG APP_VERSION
ARG GIT_SHA
ARG BUILD_DATE
ENV NEXT_PUBLIC_APP_VERSION=${APP_VERSION}
ENV NEXT_PUBLIC_GIT_SHA=${GIT_SHA}
ENV NEXT_PUBLIC_BUILT_AT=${BUILD_DATE}

RUN pnpm --filter @ppt/reality-web build

# =============================================================================
# Stage 3: Production Runner
# =============================================================================
FROM node:22-alpine AS production

WORKDIR /app

RUN addgroup -g 1001 -S nextjs && \
    adduser -S -u 1001 nextjs

COPY --from=builder --chown=nextjs:nextjs /app/apps/reality-web/.next/standalone ./
COPY --from=builder --chown=nextjs:nextjs /app/apps/reality-web/.next/static ./apps/reality-web/.next/static
COPY --from=builder --chown=nextjs:nextjs /app/apps/reality-web/public* ./apps/reality-web/public/

ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

# OCI version identity (T6). reality-web has no nginx in front of it, so the
# runtime surface is a Next route handler rather than an nginx `location`. It
# answers on `/version` — the same path ppt-web and admin-web serve from
# docker/nginx/*.nginx.conf.template, so one `curl https://<host>/version`
# works against all five service images — with `/api/version` kept as an alias.
# Both read the env vars below; see frontend/apps/reality-web/src/lib/
# version-payload.ts and .../src/app/version/route.ts. `/version` is excluded
# from the next-intl middleware matcher so it is not locale-rewritten.
ARG APP_VERSION
ARG GIT_SHA
ARG BUILD_DATE
LABEL org.opencontainers.image.version="${APP_VERSION}" \
      org.opencontainers.image.revision="${GIT_SHA}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.source="https://github.com/martin-janci/property-management"
ENV APP_VERSION=${APP_VERSION} \
    GIT_SHA=${GIT_SHA} \
    BUILD_DATE=${BUILD_DATE}

USER nextjs
EXPOSE 3000

# `127.0.0.1` not `localhost`: Alpine's BusyBox wget tries IPv6 first per
# RFC 6724 ordering, and `localhost` resolves to BOTH `127.0.0.1` and
# `::1`. Next.js standalone with `HOSTNAME=0.0.0.0` binds IPv4 only, so the
# IPv6 connect refused and the healthcheck flapped to UNHEALTHY despite
# the server actually serving on `127.0.0.1:3000`. The deploy-server's
# wait_until_ready (post-#218) treats UNHEALTHY as a hard fail and bailed
# every blue/green flip without registering Caddy routes for the new color.
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://127.0.0.1:3000/api/health || exit 1

CMD ["node", "apps/reality-web/server.js"]
