# docker/caddy/Dockerfile
# Custom Caddy build with DNS-01 plugins for cloudflare, hetzner, acme-dns.
# Note: bumped from 2.8 -> 2.10 because caddy-dns/hetzner v1.0.0 requires
# caddyserver/caddy/v2 >= v2.10.0.
FROM caddy:2.10-builder AS builder

RUN xcaddy build \
    --with github.com/caddy-dns/cloudflare \
    --with github.com/caddy-dns/hetzner \
    --with github.com/caddy-dns/acmedns

FROM caddy:2.10

COPY --from=builder /usr/bin/caddy /usr/bin/caddy
