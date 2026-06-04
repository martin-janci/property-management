# Docker Infrastructure

This directory contains Docker configuration for the Property Management System (PPT).

## Quick Start

### Local Development

```bash
# Start all services with hot-reload
docker compose -f docker-compose.dev.yml up

# Start specific services
docker compose -f docker-compose.dev.yml up postgres redis api-server

# Stop all services
docker compose -f docker-compose.dev.yml down

# Stop and remove volumes (clean slate)
docker compose -f docker-compose.dev.yml down -v
```

### Production Deployment

```bash
# Copy and configure environment
cp docker/.env.example .env
# Edit .env with your production values

# Start production stack
docker compose up -d

# View logs
docker compose logs -f

# Scale services
docker compose up -d --scale api-server=2
```

### Synology NAS Deployment

See [Synology Deployment Guide](#synology-deployment-guide) below.

## Directory Structure

```
docker/
├── README.md                    # This file
├── .env.example                 # Environment template
├── .env.synology.example        # Synology-specific template
├── backend/
│   ├── Dockerfile              # Multi-stage production build
│   └── Dockerfile.dev          # Development with hot-reload
├── frontend/
│   ├── ppt-web.Dockerfile      # Property Management SPA
│   └── reality-web.Dockerfile  # Reality Portal SSR
├── nginx/
│   ├── ppt-web.nginx.conf.template   # Nginx config template for the ppt-web SPA
│   │                                 # (rendered at container startup —
│   │                                 # /api and /ws proxy upstreams are
│   │                                 # filled from BG_TARGET + BG_COLOR
│   │                                 # so the SPA fetches reach the
│   │                                 # api-server container in the same
│   │                                 # blue/green color)
│   ├── admin-web.nginx.conf.template # Same, for the admin-web SPA
│   ├── partials/                     # Shared `include`d snippets, copied to
│   │   │                             # /etc/nginx/partials/ (NOT conf.d, so the
│   │   │                             # base image's conf.d/*.conf glob ignores
│   │   │                             # them); both templates pull them in
│   │   ├── server-common.conf            # relative redirects + gzip
│   │   └── security-headers-common.conf  # headers identical across both SPAs
│   └── render-template.sh            # envsubst entrypoint hook
└── scripts/
    ├── docker-build.sh         # Multi-arch build script
    └── synology-setup.sh       # Synology setup helper
```

## Compose Files

| File | Purpose | Use Case |
|------|---------|----------|
| `docker-compose.dev.yml` | Local development | Development with hot-reload |
| `docker-compose.yml` | Production | General production deployment |
| `docker-compose.synology.yml` | Synology NAS | Optimized for NAS constraints |

## Services

### Backend Services

| Service | Port | Description |
|---------|------|-------------|
| `api-server` | 8080 | Property Management API |
| `reality-server` | 8081 | Reality Portal API |

### Frontend Services

| Service | Port | Description |
|---------|------|-------------|
| `ppt-web` | 3000 (dev), 80 (prod) | Property Management SPA |
| `reality-web` | 3001 (dev), 3000 (prod) | Reality Portal SSR |

### Infrastructure

| Service | Port | Description |
|---------|------|-------------|
| `postgres` | 5432 | PostgreSQL 16 database |
| `redis` | 6379 | Session/cache store |
| `minio` | 9000/9001 | S3-compatible storage (dev only) |

## Environment Variables

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `POSTGRES_PASSWORD` | Database password | `secure-password` |
| `JWT_SECRET` | JWT signing key (min 32 chars) | `your-secret-key` |

### Optional Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `POSTGRES_USER` | `ppt` | Database username |
| `POSTGRES_DB` | `ppt` | Database name |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug) |
| `TZ` | `Europe/Bratislava` | Timezone |

### S3 Configuration

| Variable | Description |
|----------|-------------|
| `S3_ENDPOINT` | S3-compatible endpoint URL |
| `S3_ACCESS_KEY` | Access key ID |
| `S3_SECRET_KEY` | Secret access key |
| `S3_BUCKET` | Bucket name |
| `S3_REGION` | AWS region |

## Building Images

### Local Build (Single Architecture)

```bash
# Build all services
docker compose build

# Build specific service
docker compose build api-server
```

### Multi-Architecture Build

```bash
# Build for amd64 and arm64
./docker/scripts/docker-build.sh

# Build and push to registry
./docker/scripts/docker-build.sh --push

# Build specific service
./docker/scripts/docker-build.sh --service api-server

# Custom registry
./docker/scripts/docker-build.sh --registry your-registry.com --push
```

## Development Workflow

### Hot-Reload

The development compose file mounts source code as volumes, enabling hot-reload:

- **Backend**: Uses `cargo-watch` for automatic recompilation
- **Frontend**: Uses Vite HMR (ppt-web) and Next.js HMR (reality-web)

### Database Access

```bash
# Connect to PostgreSQL
docker compose -f docker-compose.dev.yml exec postgres psql -U ppt -d ppt

# Run migrations (if applicable)
docker compose -f docker-compose.dev.yml exec api-server sqlx migrate run
```

### Viewing Logs

```bash
# All services
docker compose -f docker-compose.dev.yml logs -f

# Specific service
docker compose -f docker-compose.dev.yml logs -f api-server

# Last 100 lines
docker compose -f docker-compose.dev.yml logs --tail=100 api-server
```

## Synology Deployment Guide

### Prerequisites

- Synology NAS with Container Manager installed
- SSH access enabled
- Docker Compose v2 support (Container Manager 20.10+)

### Supported Models

| Model | Architecture | RAM | Notes |
|-------|--------------|-----|-------|
| DS920+ | x86_64 | 4GB+ | Recommended |
| DS923+ | x86_64 | 4GB+ | Recommended |
| DS223 | ARM64 | 2GB+ | Minimum viable |
| DS423 | ARM64 | 2GB+ | Minimum viable |

### Setup Steps

1. **SSH into your NAS:**
   ```bash
   ssh admin@your-nas-ip
   ```

2. **Run the setup script:**
   ```bash
   sudo ./docker/scripts/synology-setup.sh /volume1/docker/ppt
   ```

3. **Configure environment:**
   ```bash
   sudo nano /volume1/docker/ppt/.env
   ```

4. **Start the stack:**
   ```bash
   cd /volume1/docker/ppt
   sudo docker compose up -d
   ```

### Memory Allocation

Total recommended memory for Synology deployment:

| Service | Min | Max | Notes |
|---------|-----|-----|-------|
| PostgreSQL | 128MB | 384MB | Tuned for NAS |
| Redis | 32MB | 96MB | With LRU eviction |
| api-server | 64MB | 256MB | Rust is memory-efficient |
| reality-server | 64MB | 256MB | Rust is memory-efficient |
| ppt-web | 16MB | 64MB | Static files via nginx |
| reality-web | 32MB | 128MB | Next.js SSR |
| **Total** | **336MB** | **1184MB** | |

### Reverse Proxy Setup

Configure Synology's built-in reverse proxy:

1. Open **Control Panel** > **Login Portal** > **Advanced**
2. Click **Reverse Proxy** > **Create**
3. Configure entries:

| Source | Destination | Description |
|--------|-------------|-------------|
| `ppt.nas.local:443` | `localhost:3000` | Property Management |
| `api.nas.local:443` | `localhost:8080` | API Server |
| `reality.nas.local:443` | `localhost:3001` | Reality Portal |

### Backups

A backup script is included at `/volume1/docker/ppt/backup.sh`:

```bash
# Manual backup
sudo /volume1/docker/ppt/backup.sh

# View backups
ls -la /volume1/docker/ppt/backups/
```

To automate, add a Scheduled Task:
1. **Control Panel** > **Task Scheduler**
2. Create a **Scheduled Task** > **User-defined script**
3. Set schedule (e.g., daily at 3 AM)
4. Script: `/volume1/docker/ppt/backup.sh`

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker compose logs api-server

# Check health
docker compose ps

# Inspect container
docker inspect ppt-api-server
```

### Database Connection Issues

```bash
# Test database connectivity
docker compose exec api-server curl -v telnet://postgres:5432

# Check PostgreSQL logs
docker compose logs postgres
```

### Permission Issues on Synology

```bash
# Reset permissions
sudo chown -R 1000:1000 /volume1/docker/ppt
sudo chmod 700 /volume1/docker/ppt/postgres
```

### Memory Issues

If services are OOM-killed on NAS:

1. Reduce `shared_buffers` in PostgreSQL config
2. Lower `maxmemory` in Redis config
3. Use `RUST_LOG=warn` to reduce memory usage
4. Consider running fewer services

### Slow Startup

Rust services may take 30-60 seconds on first start (compiling/loading). Subsequent restarts are faster.

## Performance Tuning

### PostgreSQL

For NAS deployment, these settings are applied:
- `shared_buffers=128MB`
- `effective_cache_size=256MB`
- `max_connections=50`

### Redis

- `maxmemory=64mb` (Synology) / `128mb` (production)
- `maxmemory-policy=allkeys-lru`

### Nginx

- Gzip compression enabled
- Static asset caching (1 year)
- Connection keep-alive

## Health Checks

All services expose health check endpoints:

| Service | Endpoint | Interval |
|---------|----------|----------|
| api-server | `GET /health` | 30s |
| reality-server | `GET /health` | 30s |
| ppt-web | `GET /health` | 30s |
| reality-web | `GET /api/health` | 30s |
| postgres | `pg_isready` | 30s |
| redis | `redis-cli ping` | 30s |

## Security Considerations

1. **Secrets**: Never commit `.env` files. Use secrets management in production.
2. **Network Isolation**: Internal services use private networks.
3. **Non-root Users**: All containers run as non-root users.
4. **HTTPS**: Use reverse proxy for SSL/TLS termination.
5. **Updates**: Regularly update base images for security patches.
