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
    && rm -rf /var/lib/apt/lists/*

# Install cargo-chef
RUN cargo install cargo-chef --locked

# Set up cross-compilation for arm64 if needed
RUN if [ "$TARGETARCH" = "arm64" ]; then \
    apt-get update && apt-get install -y --no-install-recommends \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    libssl-dev:arm64 \
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
ENV SQLX_OFFLINE=true
RUN RUST_TARGET=$(cat /tmp/rust-target) && \
    if [ "$TARGETARCH" = "arm64" ]; then \
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc && \
    export PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu && \
    cargo build --release --target $RUST_TARGET -p api-server -p reality-server && \
    mkdir -p target/release && \
    cp target/$RUST_TARGET/release/api-server target/release/api-server && \
    cp target/$RUST_TARGET/release/reality-server target/release/reality-server; \
    else \
    cargo build --release -p api-server -p reality-server; \
    fi

# Strip debug symbols for smaller binaries
RUN strip target/release/api-server target/release/reality-server || true

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
