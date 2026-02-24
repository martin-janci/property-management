# Multi-stage Dockerfile for ppt-web (React SPA with Vite)
# Produces a static build served by Nginx

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM node:20-alpine AS deps

WORKDIR /app

# Install pnpm (pinned version for reproducible builds)
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy package files
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
COPY frontend/packages/shared/package.json ./packages/shared/
COPY frontend/packages/ui-kit/package.json ./packages/ui-kit/
COPY frontend/packages/api-client/package.json ./packages/api-client/
COPY frontend/packages/sitemap/package.json ./packages/sitemap/
COPY frontend/apps/ppt-web/package.json ./apps/ppt-web/

# Install dependencies
RUN pnpm install 

# =============================================================================
# Stage 2: Builder
# =============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

# Install pnpm (pinned version for reproducible builds)
RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

# Copy dependencies
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/shared/node_modules ./packages/shared/node_modules
COPY --from=deps /app/packages/ui-kit/node_modules ./packages/ui-kit/node_modules
COPY --from=deps /app/packages/api-client/node_modules ./packages/api-client/node_modules
COPY --from=deps /app/packages/sitemap/node_modules ./packages/sitemap/node_modules
COPY --from=deps /app/apps/ppt-web/node_modules ./apps/ppt-web/node_modules

# Copy source
COPY frontend/ ./

# Build arguments for environment
ARG VITE_API_URL=http://localhost:8080
ARG VITE_WS_URL=ws://localhost:8080

ENV VITE_API_URL=${VITE_API_URL}
ENV VITE_WS_URL=${VITE_WS_URL}

# Build the application
RUN pnpm --filter @ppt/web build

# =============================================================================
# Stage 3: Production - Nginx
# =============================================================================
FROM nginx:alpine AS production

# Copy custom nginx config
COPY docker/nginx/ppt-web.nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets
COPY --from=builder /app/apps/ppt-web/dist /usr/share/nginx/html

# Create non-root user
RUN addgroup -g 1001 -S ppt && \
    adduser -S -D -H -u 1001 -h /var/cache/nginx -s /sbin/nologin -G ppt -g ppt ppt && \
    chown -R ppt:ppt /var/cache/nginx /var/run /var/log/nginx /usr/share/nginx/html

# Switch to non-root user
USER ppt

# Expose port
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:80/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
