# Multi-stage Dockerfile for Rust backend services (api-server, reality-server)
# Supports: linux/amd64, linux/arm64

# =============================================================================
# Stage 1: Chef - Prepare recipe for dependency caching
# =============================================================================
FROM --platform=$BUILDPLATFORM rust:slim-bookworm AS chef

RUN cargo install cargo-chef --locked
WORKDIR /app

# =============================================================================
# Stage 2: Planner - Create dependency recipe
# =============================================================================
FROM chef AS planner

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/crates ./crates
COPY backend/servers ./servers
# Sitemap JSON referenced by common crate via include_str!
COPY frontend/packages/sitemap/src/json/sitemap.json /frontend/packages/sitemap/src/json/sitemap.json

RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: Builder - Build dependencies then application
# =============================================================================
FROM rust:slim-bookworm AS builder

ARG TARGETPLATFORM
ARG TARGETARCH

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    mold \
    clang \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Install cargo-chef
RUN cargo install cargo-chef --locked

# Set up cross-compilation for arm64 if needed
RUN if [ "$TARGETARCH" = "arm64" ]; then \
    apt-get update && apt-get install -y --no-install-recommends \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    libssl-dev:arm64 \
    mold \
    clang \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add aarch64-unknown-linux-gnu; \
    fi

# Set target triple based on platform
ENV CARGO_TARGET_DIR=/app/target
RUN if [ "$TARGETARCH" = "arm64" ]; then \
    echo "aarch64-unknown-linux-gnu" > /tmp/rust-target; \
    else \
    echo "x86_64-unknown-linux-gnu" > /tmp/rust-target; \
    fi

WORKDIR /app

# Copy and build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json

RUN RUST_TARGET=$(cat /tmp/rust-target) && \
    if [ "$TARGETARCH" = "arm64" ]; then \
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc && \
    export PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu && \
    cargo chef cook --release --target $RUST_TARGET --recipe-path recipe.json; \
    else \
    cargo chef cook --release --recipe-path recipe.json; \
    fi

# Copy source code
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/crates ./crates
COPY backend/servers ./servers
# Sitemap JSON referenced by common crate via include_str!
COPY frontend/packages/sitemap/src/json/sitemap.json /frontend/packages/sitemap/src/json/sitemap.json

# Build the applications
# Build server binaries plus the api-server-side helper bins (`ppt-seed`
# bootstraps OAuth clients + admin user; `ppt-features` toggles feature
# flags) in a single cargo invocation so the matrix workflow can share a
# single GHA cache scope (see .github/workflows/docker-build.yml). The
# helper bins are needed at runtime — operators run them via
# `docker exec <api-container> ppt-seed ...` post-deploy — so they must
# ship in the api-server image, not just exist in the build stage.
ENV SQLX_OFFLINE=true
RUN RUST_TARGET=$(cat /tmp/rust-target) && \
    if [ "$TARGETARCH" = "arm64" ]; then \
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc && \
    export PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu && \
    cargo build --release --target $RUST_TARGET \
        --bin api-server --bin reality-server \
        --bin ppt-seed --bin ppt-features && \
    mkdir -p target/release && \
    cp target/$RUST_TARGET/release/api-server target/release/api-server && \
    cp target/$RUST_TARGET/release/reality-server target/release/reality-server && \
    cp target/$RUST_TARGET/release/ppt-seed target/release/ppt-seed && \
    cp target/$RUST_TARGET/release/ppt-features target/release/ppt-features; \
    else \
    cargo build --release \
        --bin api-server --bin reality-server \
        --bin ppt-seed --bin ppt-features; \
    fi

# =============================================================================
# Stage 4: Runtime Base - Minimal production image base
# =============================================================================
FROM debian:bookworm-slim AS runtime-base

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r ppt && useradd -r -g ppt ppt

WORKDIR /app

# =============================================================================
# API Server - Production target
# =============================================================================
FROM runtime-base AS api-server

COPY --from=builder /app/target/release/api-server /app/api-server
# Helper bins shipped alongside `api-server` so operators can run them via
# `docker exec <container> ppt-seed --reality-portal-secret "$SECRET"`
# (bootstraps OAuth clients + admin) and `ppt-features` (toggles feature
# flags). They share the same image so they read the same DATABASE_URL +
# secrets the running api-server has.
COPY --from=builder /app/target/release/ppt-seed /usr/local/bin/ppt-seed
COPY --from=builder /app/target/release/ppt-features /usr/local/bin/ppt-features
RUN chown -R ppt:ppt /app
USER ppt

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/api-server"]

# =============================================================================
# Reality Server - Production target
# =============================================================================
FROM runtime-base AS reality-server

COPY --from=builder /app/target/release/reality-server /app/reality-server
RUN chown -R ppt:ppt /app
USER ppt

EXPOSE 8081

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

CMD ["/app/reality-server"]
