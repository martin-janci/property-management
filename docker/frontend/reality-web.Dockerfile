# Multi-stage Dockerfile for reality-web (Next.js SSR)
# Supports standalone output mode for optimized production deployment

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM node:20-alpine AS deps

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy ALL workspace package.json files (avoid transitive dep errors).
# Every workspace package referenced via `workspace:*` in any consumed
# package.json must have its manifest copied here; otherwise `pnpm install`
# fails with "in the dependencies field, no project of name X found".
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

RUN pnpm install

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

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
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules
COPY --from=deps /app/packages/ui-kit/node_modules ./packages/ui-kit/node_modules
COPY --from=deps /app/packages/api-client/node_modules ./packages/api-client/node_modules
COPY --from=deps /app/packages/reality-api-client/node_modules ./packages/reality-api-client/node_modules
COPY --from=deps /app/packages/dev-panel/node_modules ./packages/dev-panel/node_modules
COPY --from=deps /app/packages/sitemap/node_modules ./packages/sitemap/node_modules
COPY --from=deps /app/apps/reality-web/node_modules ./apps/reality-web/node_modules

# Copy source
COPY frontend/ ./

ARG NEXT_PUBLIC_API_URL=http://localhost:8081
ARG NEXT_PUBLIC_SITE_URL=http://localhost:3001

ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
ENV NEXT_PUBLIC_SITE_URL=${NEXT_PUBLIC_SITE_URL}
ENV NEXT_TELEMETRY_DISABLED=1

RUN pnpm --filter @ppt/reality-web build

# =============================================================================
# Stage 3: Production Runner
# =============================================================================
FROM node:20-alpine AS production

WORKDIR /app

RUN addgroup -g 1001 -S nextjs && \
    adduser -S -u 1001 nextjs

COPY --from=builder --chown=nextjs:nextjs /app/apps/reality-web/.next/standalone ./
COPY --from=builder --chown=nextjs:nextjs /app/apps/reality-web/.next/static ./apps/reality-web/.next/static
COPY --from=builder --chown=nextjs:nextjs /app/apps/reality-web/public* ./apps/reality-web/public/

ENV NODE_ENV=production
ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

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
