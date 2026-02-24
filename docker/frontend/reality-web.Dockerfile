# Multi-stage Dockerfile for reality-web (Next.js SSR)
# Supports standalone output mode for optimized production deployment

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM node:20-alpine AS deps

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy ALL workspace package.json files (avoid transitive dep errors)
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
COPY frontend/packages/shared/package.json ./packages/shared/
COPY frontend/packages/ui-kit/package.json ./packages/ui-kit/
COPY frontend/packages/api-client/package.json ./packages/api-client/
COPY frontend/packages/reality-api-client/package.json ./packages/reality-api-client/
COPY frontend/packages/sitemap/package.json ./packages/sitemap/
COPY frontend/apps/reality-web/package.json ./apps/reality-web/

RUN pnpm install

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy dependencies from deps stage
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules
COPY --from=deps /app/packages/ui-kit/node_modules ./packages/ui-kit/node_modules
COPY --from=deps /app/packages/reality-api-client/node_modules ./packages/reality-api-client/node_modules
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

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:3000/api/health || exit 1

CMD ["node", "apps/reality-web/server.js"]
