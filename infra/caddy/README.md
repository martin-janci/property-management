# Caddy — Phase 3 hosting + on-demand TLS

This directory holds the Caddy reverse-proxy config that fronts both backend
servers and handles TLS issuance for every tenant.

## The architecture in one paragraph

`agency_domains` is the **single source of truth** for two things:

1. **Tenant resolution** — Phase 1's `host_tenant_middleware` (in both
   `api-server` and `reality-server`) reads the inbound `Host` header,
   looks it up in `agency_domains`, and sets the RLS context.
2. **TLS issuance** — Caddy's `on_demand_tls.ask` directive points at
   `api-server`'s `/internal/caddy-ask` endpoint, which queries the same
   table. A host that is not in `agency_domains` (or whose
   `verification_state` is `pending` / `failed`) cannot mint a cert.

This collapses what would otherwise be two divergent allowlists (TLS allow
vs RLS allow) into one row. Adding a tenant is a single `INSERT` into
`agency_domains` (via the platform-admin endpoint from Phase 1); routing
and TLS both come online in the same write.

## Defense: leak #4 (Caddy on-demand TLS as DoS amplifier)

Without the ask-endpoint, an attacker spraying TLS handshakes for random
hostnames would drive Caddy to ask Let's Encrypt for a cert per host until
LE's per-account rate limit fires and locks legitimate cert issuance for
hours. The ask-endpoint pre-filters on `agency_domains`, so unknown hosts
never reach LE.

The api-server's `/internal/caddy-ask` enforces three layers of defense:

1. **Auth gate** — `X-Internal-Token` shared-secret check in non-development
   environments. Missing token in prod = 401 (Caddy refuses to issue).
2. **Per-IP rate limit** — process-local 60/minute. Caddy itself sits on the
   loopback (or a private network), so this should never fire under
   legitimate load.
3. **Database query** — only `verifying` / `verified` rows answer 200. Any
   other state, including `pending` and `failed`, returns 403. Database
   errors return `503` (which Caddy treats as "no" — the ask-endpoint
   contract is "200 means yes; everything else means no").

## Environment variables

| Var                 | Required | Description                                                            |
|---------------------|----------|------------------------------------------------------------------------|
| `PLATFORM_HOST`     | yes      | Apex domain. Used to derive `*.<PLATFORM_HOST>` wildcard cert + the   |
|                     |          | API host `api.<PLATFORM_HOST>`. e.g. `rlt.sk`.                         |
| `API_INTERNAL_URL`  | yes      | Service URL of api-server (Caddy uses it for the ask-endpoint).        |
|                     |          | e.g. `http://api-server:8080` inside the docker network.               |
| `LETSENCRYPT_EMAIL` | yes      | ACME contact email. LE rejects cert orders without one.                |
| `INTERNAL_API_TOKEN`| prod     | Shared secret matched by api-server's `/internal/caddy-ask`.           |
|                     |          | Skipped in `RUST_ENV=development` where the endpoint binds loopback.   |

## Running locally

```bash
# Validate the Caddyfile without starting Caddy:
docker run --rm -v "$PWD/infra/caddy/Caddyfile:/etc/caddy/Caddyfile" \
    caddy:2-alpine caddy validate --config /etc/caddy/Caddyfile

# Start the full Phase 3 stack (Caddy + api + reality + db):
docker compose -f infra/caddy/docker-compose.yml up
```

For a smoke test of the ask-endpoint without Caddy in the loop:

```bash
# Dev mode (no token required):
RUST_ENV=development curl -i 'http://127.0.0.1:8080/internal/caddy-ask?domain=acme.dev.rlt.sk'
# Prod mode (token required):
curl -i -H "X-Internal-Token: $INTERNAL_API_TOKEN" \
    'http://127.0.0.1:8080/internal/caddy-ask?domain=acme.rlt.sk'
```

## What lives here

- `Caddyfile` — the reverse-proxy + TLS config.
- `docker-compose.yml` — Caddy in front of the existing api / reality
  services. Layers on top of the root `docker-compose.yml`.
- `README.md` — this file.

## Future evolution

- **DNS-01 challenge for the wildcard** — currently the `*.<PLATFORM_HOST>`
  block uses on-demand TLS, which requires HTTP-01 (one cert per visited
  subdomain). Switching to DNS-01 with the chosen DNS provider (Cloudflare
  / Route53) lets a single wildcard cert cover every subdomain in one
  ACME order; configure the `dns` block under `tls` once the provider is
  settled.
- **Cert state -> agency_domains** — Caddy can write back the issuance
  result (success / failure + expiry) into a column on `agency_domains`
  via a webhook; the platform-admin UI then surfaces "cert issued" /
  "issuance pending" without scraping Caddy's storage.
- **Edge ACL on `:443` catch-all** — rate-limit the bare `:443` block
  per source IP at the edge so unknown-host scans don't hit the
  api-server at all.
