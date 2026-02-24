# Multi-stage Dockerfile for ppt-web (React SPA with Vite)
# Produces a static build served by Nginx

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM node:20-alpine AS deps

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy all workspace package.json files
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
COPY frontend/packages/shared/package.json ./packages/shared/
COPY frontend/packages/ui-kit/package.json ./packages/ui-kit/
COPY frontend/packages/api-client/package.json ./packages/api-client/
COPY frontend/packages/reality-api-client/package.json ./packages/reality-api-client/
COPY frontend/packages/sitemap/package.json ./packages/sitemap/
COPY frontend/apps/ppt-web/package.json ./apps/ppt-web/

RUN pnpm install

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy all deps at once
COPY --from=deps /app/ ./

# Copy source
COPY frontend/ ./

ARG VITE_API_URL=http://localhost:8080
ARG VITE_WS_URL=ws://localhost:8080

ENV VITE_API_URL=${VITE_API_URL}
ENV VITE_WS_URL=${VITE_WS_URL}

RUN pnpm --filter @ppt/web build

# =============================================================================
# Stage 3: Production - Nginx
# =============================================================================
FROM nginx:alpine AS production

COPY docker/nginx/ppt-web.nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/apps/ppt-web/dist /usr/share/nginx/html

RUN addgroup -g 1001 -S ppt && \
    adduser -S -D -H -u 1001 -h /var/cache/nginx -s /sbin/nologin -G ppt -g ppt ppt && \
    chown -R ppt:ppt /var/cache/nginx /var/run /run /var/log/nginx /usr/share/nginx/html

USER ppt
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:8080/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
