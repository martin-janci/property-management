# Multi-stage Dockerfile for ppt-web (React SPA with Vite)
# Produces a static build served by Nginx

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
FROM node:20-alpine AS deps

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy all workspace package.json files. Every workspace package referenced
# via `workspace:*` in any consumed package.json must have its manifest
# copied here; otherwise `pnpm install` fails with "in the dependencies
# field, no project of name X found".
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
# `worktree` mode means the dev panel uses whatever VITE_API_URL was baked in
# at build (typically `/api` in deployed bundles) instead of falling through to
# the misleading `local` label. See #454.
ARG VITE_API_DEFAULT=worktree

ENV VITE_API_URL=${VITE_API_URL}
ENV VITE_WS_URL=${VITE_WS_URL}
ENV VITE_API_DEFAULT=${VITE_API_DEFAULT}

# Bake the version identity into the bundle: any module that reads
# `import.meta.env.VITE_APP_VERSION` / `VITE_GIT_SHA` / `VITE_BUILT_AT` gets the
# same values the image labels and the `/version` endpoint report.
ARG APP_VERSION
ARG GIT_SHA
ARG BUILD_DATE
ENV VITE_APP_VERSION=${APP_VERSION}
ENV VITE_GIT_SHA=${GIT_SHA}
ENV VITE_BUILT_AT=${BUILD_DATE}

RUN pnpm --filter @ppt/web build

# =============================================================================
# Stage 3: Production - Nginx
# =============================================================================
FROM nginx:alpine AS production

# `gettext` brings `envsubst`, used by /docker-entrypoint.d/10-render-template.sh
# at container startup to substitute `${BG_TARGET}` and `${BG_COLOR}` into the
# /api and /ws proxy upstreams, so this ppt-web instance proxies to the same
# blue/green color of api-server in the same Docker network as itself.
RUN apk add --no-cache gettext

# Ship the template (not the final config) — rendered at startup.
COPY docker/nginx/ppt-web.nginx.conf.template /etc/nginx/conf.d/default.conf.template

# Shared, `include`d partials (relative redirects, gzip, common security
# headers). Kept OUTSIDE /etc/nginx/conf.d so the base image's
# `include /etc/nginx/conf.d/*.conf` does not try to load them as standalone
# server configs; the template pulls them in by absolute path. No envsubst
# needed — they carry no `${BG_*}` placeholders.
COPY docker/nginx/partials/ /etc/nginx/partials/

# nginx:alpine's stock entrypoint runs every script in /docker-entrypoint.d/*.sh
# before exec'ing nginx, so dropping the renderer here gives us templating
# without a custom ENTRYPOINT line.
COPY docker/nginx/render-template.sh /docker-entrypoint.d/10-render-template.sh
RUN chmod +x /docker-entrypoint.d/10-render-template.sh

COPY --from=builder /app/apps/ppt-web/dist /usr/share/nginx/html

RUN addgroup -g 1001 -S ppt && \
    adduser -S -D -H -u 1001 -h /var/cache/nginx -s /sbin/nologin -G ppt -g ppt ppt && \
    chown -R ppt:ppt /var/cache/nginx /var/run /run /var/log/nginx /usr/share/nginx/html /etc/nginx/conf.d

USER ppt
EXPOSE 8080

# OCI version identity (T6). Until now `git grep -c LABEL -- docker/` was 0 and
# the only labels came from metadata-action, whose `image.version` is the branch
# name on a branch build. Declared last in the stage so a new commit SHA
# invalidates nothing but this metadata layer.
#
# ENV as well as ARG, because /docker-entrypoint.d/10-render-template.sh
# envsubst's these three into the `/version` endpoint at container start.
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

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:8080/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
