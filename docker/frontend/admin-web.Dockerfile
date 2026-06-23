# Multi-stage Dockerfile for admin-web (Super-admin Control Plane, React SPA).
# Produces a static build served by Nginx.

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM node:20-alpine AS deps
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Workspace package.json files for every package admin-web transitively
# depends on (workspace:* deps need the manifest present at install time).
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
COPY frontend/apps/admin-web/package.json ./apps/admin-web/

RUN pnpm install

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:20-alpine AS builder
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

COPY --from=deps /app/ ./
COPY frontend/ ./

RUN pnpm --filter @ppt/admin-web build

# =============================================================================
# Stage 3: Production - Nginx
# =============================================================================
FROM nginx:alpine AS production

# envsubst (from gettext) renders ${BG_TARGET}/${BG_COLOR} into the
# /api/* upstream at container start.
RUN apk add --no-cache gettext

COPY docker/nginx/admin-web.nginx.conf.template /etc/nginx/conf.d/default.conf.template
# Shared, `include`d partials (relative redirects, gzip, common security
# headers). Kept OUTSIDE /etc/nginx/conf.d so the base image's
# `include /etc/nginx/conf.d/*.conf` does not load them as standalone configs;
# the template pulls them in by absolute path. No envsubst needed.
COPY docker/nginx/partials/ /etc/nginx/partials/
COPY docker/nginx/render-template.sh /docker-entrypoint.d/10-render-template.sh
RUN chmod +x /docker-entrypoint.d/10-render-template.sh

COPY --from=builder /app/apps/admin-web/dist /usr/share/nginx/html

RUN addgroup -g 1001 -S ppt && \
    adduser -S -D -H -u 1001 -h /var/cache/nginx -s /sbin/nologin -G ppt -g ppt ppt && \
    chown -R ppt:ppt /var/cache/nginx /var/run /run /var/log/nginx /usr/share/nginx/html /etc/nginx/conf.d

USER ppt
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:8080/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
