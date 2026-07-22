# Property Management Project - Unified Task Runner
#
# Usage: just <recipe>
# Install just: https://github.com/casey/just
#
# List all recipes: just --list

set shell := ["bash", "-cu"]

# Default recipe - show help
default:
    @just --list

# =============================================================================
# SETUP & BOOTSTRAP
# =============================================================================

# First-time setup for new developers
setup:
    @echo "🚀 Setting up Property Management development environment..."
    ./scripts/setup.sh

# Install git hooks (version bump, formatting checks)
hooks:
    @echo "🔗 Installing git hooks..."
    ./scripts/install-hooks.sh

# =============================================================================
# DEVELOPMENT COMMANDS
# =============================================================================

# Run backend api-server (port 8080)
api:
    cd backend && cargo run -p api-server

# Run backend reality-server (port 8081)
reality-server:
    cd backend && cargo run -p reality-server

# Run frontend Property Management web (dev mode)
web:
    cd frontend && pnpm dev:ppt

# Run frontend Reality Portal web (dev mode)
reality-web:
    cd frontend && pnpm dev:reality

# Run mobile app (React Native Expo)
mobile:
    cd frontend && pnpm dev:mobile

# Run all services in parallel (requires tmux or separate terminals)
dev:
    @echo "Starting all services..."
    @echo "Terminal 1: just api"
    @echo "Terminal 2: just web"
    @echo "Consider using 'tmux' or 'mprocs' for multiple services"

# =============================================================================
# BUILD COMMANDS
# =============================================================================

# Build all projects
build: build-backend build-frontend build-mobile-native

# Build backend (release)
build-backend:
    @echo "🦀 Building Rust backend..."
    cd backend && cargo build --release --workspace

# Build frontend (all apps)
build-frontend:
    @echo "📦 Building frontend..."
    cd frontend && pnpm build

# Build mobile-native (Kotlin Multiplatform)
build-mobile-native:
    @echo "📱 Building mobile-native..."
    cd mobile-native && ./gradlew build

# =============================================================================
# CODE QUALITY
# =============================================================================

# Deterministic impact-scoped verify gate (see scripts/verify-impact.sh + .claude/skills/_verify-rules.md)
verify *ARGS:
    ./scripts/verify-impact.sh {{ARGS}}

# Print the verify plan without running it (quote VERIFY-PLAN block in PR bodies)
verify-plan:
    ./scripts/verify-impact.sh --plan-only

# Run all quality checks
check: check-backend check-frontend

# Check backend (format + clippy)
check-backend:
    @echo "🦀 Checking Rust code..."
    cd backend && cargo fmt --all -- --check
    cd backend && cargo clippy --workspace -- -D warnings

# Check frontend (Biome lint + format)
check-frontend:
    @echo "📦 Checking frontend code..."
    cd frontend && pnpm check

# Fix all code quality issues
fix: fix-backend fix-frontend

# Fix backend formatting
fix-backend:
    @echo "🦀 Fixing Rust formatting..."
    cd backend && cargo fmt --all

# Fix frontend formatting and lint
fix-frontend:
    @echo "📦 Fixing frontend code..."
    cd frontend && pnpm check:fix

# =============================================================================
# TESTING
# =============================================================================

# Run all tests
test: test-backend test-frontend

# Run backend tests
test-backend:
    @echo "🦀 Running Rust tests..."
    cd backend && cargo test --workspace

# Run frontend tests
test-frontend:
    @echo "📦 Running frontend tests..."
    cd frontend && pnpm test

# Run backend tests with database (integration tests)
test-integration:
    @echo "🦀 Running integration tests..."
    cd backend && cargo test --workspace --test '*' -- --ignored

# =============================================================================
# API CLIENT GENERATION
# =============================================================================

# Generate all API clients
generate-api: generate-api-client generate-reality-api-client

# Generate Property Management API client
generate-api-client:
    @echo "📝 Generating API client..."
    cd frontend && pnpm generate-api

# Generate Reality Portal API client
generate-reality-api-client:
    @echo "📝 Generating Reality API client..."
    cd frontend && pnpm generate-reality-api

# =============================================================================
# DATABASE
# =============================================================================

# Run database migrations
db-migrate:
    @echo "🗄️  Running database migrations..."
    cd backend/crates/db && sqlx migrate run

# Create a new migration
db-migration name:
    @echo "🗄️  Creating migration: {{name}}"
    cd backend/crates/db && sqlx migrate add {{name}}

# Prepare SQLx offline data
db-prepare:
    @echo "🗄️  Preparing SQLx offline data..."
    cd backend && cargo sqlx prepare --workspace

# =============================================================================
# VERSION MANAGEMENT
# =============================================================================

# Show current version
version:
    @cat VERSION

# Bump patch version (0.1.0 -> 0.1.1)
bump-patch:
    ./scripts/bump-version.sh patch

# Bump minor version (0.1.0 -> 0.2.0)
bump-minor:
    ./scripts/bump-version.sh minor

# Bump major version (0.1.0 -> 1.0.0)
bump-major:
    ./scripts/bump-version.sh major

# Sync version across all projects
sync-version:
    ./scripts/update-version.sh

# =============================================================================
# MAINTENANCE
# =============================================================================

# Clean all build artifacts
clean:
    @echo "🧹 Cleaning all build artifacts..."
    ./scripts/clean.sh

# Check project health (dependencies, tools)
health:
    @echo "🏥 Running health checks..."
    ./scripts/health-check.sh

# Update dependencies (all platforms)
update-deps:
    @echo "📦 Updating dependencies..."
    cd backend && cargo update
    cd frontend && pnpm update
    cd mobile-native && ./gradlew dependencyUpdates

# Check for outdated dependencies
outdated:
    @echo "📦 Checking for outdated dependencies..."
    cd backend && cargo outdated || true
    cd frontend && pnpm outdated || true
    cd mobile-native && ./gradlew dependencyUpdates || true

# Audit dependencies for security issues
audit:
    @echo "🔒 Auditing dependencies..."
    cd backend && cargo audit || true
    cd frontend && pnpm audit || true

# =============================================================================
# GIT HELPERS
# =============================================================================

# Create a feature branch (branch model: PRs target dev)
feature name:
    git checkout dev
    git pull origin dev
    git checkout -b feature/{{name}}

# Create a bugfix branch (branch model: PRs target dev)
bugfix name:
    git checkout dev
    git pull origin dev
    git checkout -b bugfix/{{name}}

# Sync current branch with dev
sync:
    git fetch origin dev
    git rebase origin/dev

# =============================================================================
# CI/CD HELPERS
# =============================================================================

# Run the same checks as CI
ci: check test build
    @echo "✅ All CI checks passed!"

# Prepare for PR (check + test + commit)
pr-ready: check test
    @echo "✅ Ready for PR!"

# =============================================================================
# RESEARCH ROUTINE HELPERS
# =============================================================================

# Scaffold a new plan from .research/plan-template.md.
# Refuses if the target already exists. Substitutes <slug> into the H1 line.
# Slug must match [a-z0-9][a-z0-9-]* — anything else is rejected so the sed
# substitution (and the surrounding shell) cannot be confused by delimiters
# or metacharacters in the input.
new-plan slug:
    @SLUG='{{slug}}'; \
    if [ -z "$SLUG" ]; then echo "usage: just new-plan <slug>"; exit 2; fi; \
    if ! printf '%s' "$SLUG" | grep -Eq '^[a-z0-9][a-z0-9-]*$'; then \
        echo "invalid slug: '$SLUG' (must match [a-z0-9][a-z0-9-]*)"; exit 2; \
    fi; \
    target=".research/plans/$SLUG.md"; \
    if [ -e "$target" ]; then echo "refusing to overwrite $target"; exit 1; fi; \
    if [ ! -f .research/plan-template.md ]; then echo "missing .research/plan-template.md"; exit 1; fi; \
    mkdir -p .research/plans; \
    sed "1s|^# <slug>\$|# $SLUG|" .research/plan-template.md > "$target"; \
    echo "wrote $target — fill placeholders, then commit on a fresh branch."
