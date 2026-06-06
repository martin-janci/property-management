// backend/servers/deploy-server/src/infra/blue_green.rs
use crate::infra::{CaddyClient, DockerClient};
use crate::Result;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Service names participating in blue/green deploys.
/// Used by the deployer itself and by lifecycle code (gc, etc.) that needs to
/// enumerate per-target containers without hardcoding the list.
///
/// `admin-web` was added in Phase 5 (2026-05) when the super-admin control
/// plane moved out of `ppt-web` into its own SPA served at `admin.<reality_apex>`
/// (`admin.rlt.sk` in prod, `admin.staging.rlt.sk` in staging). It joins the
/// existing four services as a 5th atomic-flip member so `pmctl promote` and
/// `pmctl rollback` cover the admin app too.
pub const BG_SERVICES: &[&str] = &["api", "reality", "ppt", "reality-web", "admin-web"];

pub struct BlueGreenDeployer {
    pub docker: Arc<DockerClient>,
    pub caddy: Arc<CaddyClient>,
}

#[derive(Debug, Clone)]
pub struct BlueGreenSpec {
    pub tag: String,
    pub api_image: String,
    pub reality_image: String,
    pub ppt_web_image: String,
    pub reality_web_image: String,
    /// Super-admin SPA image (`ghcr.io/martin-janci/ppt-admin-web:<tag>`).
    /// Phase 5 control plane served at `admin.<reality_apex>`.
    ///
    /// Optional for backwards compatibility: Release rows persisted BEFORE
    /// admin-web was added (registered through `build_staging_images` < #268)
    /// don't carry an `admin-web` image. Rollback via `pmctl rollback prod`
    /// builds the prev_color spec from one of those older Release rows; if
    /// this field were required, rollback would 400 with "missing image for
    /// service 'admin-web'" and leave the operator without a recovery path.
    ///
    /// When `None`, `deploy()` skips the admin-web pull/run/wait/register
    /// steps entirely (no admin-web container is created for that color and
    /// no `admin.<apex>` route is registered). The other four services are
    /// still deployed atomically.
    pub admin_web_image: Option<String>,
    /// Reality Portal apex (e.g. `rlt.sk`, `staging.rlt.sk`). reality-web is
    /// served at this exact host; reality-server at `api.<reality_apex>`.
    pub reality_apex: String,
    /// Property Management apex (e.g. `ppt.rlt.sk`, `staging.ppt.rlt.sk`).
    /// ppt-web is served at this exact host; api-server at `api.<ppt_apex>`.
    pub ppt_apex: String,
    pub target_name: String,
    /// Per-service environment variables to inject into each container,
    /// keyed by short service name (`api`, `reality`, `ppt`, `reality-web`).
    /// Each value is a list of `KEY=VALUE` strings passed straight to
    /// Docker. Without this, backend containers panic at startup because
    /// they can't read `DATABASE_URL` / `JWT_SECRET`. Constructed in the
    /// HTTP handlers (deploy/promote) where access to secrets and target
    /// config is centralized.
    pub service_envs: std::collections::HashMap<String, Vec<String>>,
}

impl BlueGreenSpec {
    /// Build a deploy spec from a Release row + target config.
    /// Use this instead of inlining the boilerplate in deploy/promote/rollback handlers.
    /// Fails with a clear error if any required service image is missing — better than
    /// running `docker pull` on an empty ref and getting cryptic registry errors.
    pub fn from_release(
        rel: &crate::domain::Release,
        target_name: &str,
        target: &crate::config::Target,
        service_envs: std::collections::HashMap<String, Vec<String>>,
    ) -> crate::Result<Self> {
        fn require(rel: &crate::domain::Release, key: &str) -> crate::Result<String> {
            rel.images.get(key).cloned().ok_or_else(|| {
                crate::DeployError::BadRequest(format!(
                    "release {} is missing image for service '{}'",
                    rel.tag, key
                ))
            })
        }
        Ok(Self {
            tag: rel.tag.clone(),
            target_name: target_name.to_string(),
            api_image: require(rel, "api-server")?,
            reality_image: require(rel, "reality-server")?,
            ppt_web_image: require(rel, "ppt-web")?,
            reality_web_image: require(rel, "reality-web")?,
            // admin-web missing on older Release rows (pre #268) is fine —
            // skip it during deploy() instead of refusing the rollback.
            admin_web_image: rel.images.get("admin-web").cloned(),
            reality_apex: target.reality_apex.clone(),
            ppt_apex: target.ppt_apex.clone(),
            service_envs,
        })
    }
}

/// The env-var KEYS the `api-server` container requires at startup outside
/// `RUST_ENV=development`. Mirrors the panics in `api-server/src/main.rs`:
/// `DATABASE_URL` and `JWT_SECRET` (the core bootstrap), the two encryption
/// keys (`TOTP_ENCRYPTION_KEY`, `INTEGRATION_ENCRYPTION_KEY`), and the
/// e-signature secrets (`ESIGN_TOKEN_SECRET`, `ESIGN_WEBHOOK_SECRET`).
///
/// This is the shared contract between api-server's startup config and the
/// deploy-server's [`build_service_envs`] (issue #951): `build_service_envs`
/// MUST inject every one of these onto the `api` service, or the container
/// crash-loops on deploy (which is exactly what bit the ESIGN rollout). The
/// `api_env_contains_all_required_startup_vars` test guards the contract so a
/// newly-required secret can't silently reach a deploy un-injected — add the
/// var here and the test fails until `build_service_envs` templates it.
pub const API_REQUIRED_STARTUP_ENV: &[&str] = &[
    "DATABASE_URL",
    "JWT_SECRET",
    "TOTP_ENCRYPTION_KEY",
    "INTEGRATION_ENCRYPTION_KEY",
    "ESIGN_TOKEN_SECRET",
    "ESIGN_WEBHOOK_SECRET",
];

/// Build the per-service environment map for a target deploy.
///
/// Reads shared secrets from the deploy-server's own environment
/// (`POSTGRES_PASSWORD`, `PPT_JWT_SECRET`) and templates per-service env
/// strings using the target's apex hostnames. Each backend service gets a
/// dedicated `DATABASE_URL` pointing at `ppt_<target_name>` on the shared
/// `ppt-postgres` container; both backends share the same JWT secret so
/// SSO between them works. The Next.js `reality-web` gets the public API
/// URLs that its `/env.js` route serves to the browser. `ppt-web` is
/// Vite-built static and currently has nothing useful to inject (its API
/// URL is baked at build time as the relative path `/api`).
///
/// Errors out with a clear `Config` error if the deploy-server's own env
/// is missing the required secrets — better than starting containers
/// that crash-loop with empty values.
pub fn build_service_envs(
    target_name: &str,
    target: &crate::config::Target,
) -> crate::Result<std::collections::HashMap<String, Vec<String>>> {
    fn require(name: &str) -> crate::Result<String> {
        std::env::var(name).map_err(|_| {
            crate::DeployError::Config(format!(
                "{name} env var is not set; refusing to start backend containers \
                 without it. Set it in /etc/ppt-deploy/secrets.env."
            ))
        })
    }

    let postgres_password = require("POSTGRES_PASSWORD")?;
    let jwt_secret = require("PPT_JWT_SECRET")?;
    if jwt_secret.len() < 32 {
        return Err(crate::DeployError::Config(
            "PPT_JWT_SECRET must be at least 32 characters".into(),
        ));
    }
    // Both extra encryption keys must be 64 hex chars (32 bytes for AES-256).
    // api-server panics at startup without `TOTP_ENCRYPTION_KEY` and warns
    // (but otherwise stores secrets in plaintext) without
    // `INTEGRATION_ENCRYPTION_KEY`. We treat both as required so the deploy
    // never silently downgrades data-at-rest protection in a target.
    let totp_key = require("PPT_TOTP_ENCRYPTION_KEY")?;
    if totp_key.len() != 64 || !totp_key.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(crate::DeployError::Config(
            "PPT_TOTP_ENCRYPTION_KEY must be exactly 64 hex characters (32 bytes)".into(),
        ));
    }
    let integration_key = require("PPT_INTEGRATION_ENCRYPTION_KEY")?;
    if integration_key.len() != 64 || !integration_key.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(crate::DeployError::Config(
            "PPT_INTEGRATION_ENCRYPTION_KEY must be exactly 64 hex characters (32 bytes)".into(),
        ));
    }
    // OAuth client secret used by reality-server when authenticating to the
    // api-server's OAuth provider (client_id = "reality-portal" by default).
    // The matching record on the api-server side is seeded via app
    // migrations / data fixtures, not the deploy-server.
    let pm_client_secret = require("PPT_PM_CLIENT_SECRET")?;
    if pm_client_secret.len() < 32 {
        return Err(crate::DeployError::Config(
            "PPT_PM_CLIENT_SECRET must be at least 32 characters".into(),
        ));
    }
    // E-signature secrets. api-server refuses to boot outside
    // `RUST_ENV=development` without BOTH of these (see
    // api-server/src/main.rs: `LightweightProvider::from_env()` panics
    // without `ESIGN_TOKEN_SECRET`, and the next check panics without
    // `ESIGN_WEBHOOK_SECRET`). Treat both as required so a deploy never
    // crash-loops the api container on a fresh feature rollout.
    let esign_token_secret = require("PPT_ESIGN_TOKEN_SECRET")?;
    if esign_token_secret.len() < 32 {
        return Err(crate::DeployError::Config(
            "PPT_ESIGN_TOKEN_SECRET must be at least 32 characters".into(),
        ));
    }
    let esign_webhook_secret = require("PPT_ESIGN_WEBHOOK_SECRET")?;
    if esign_webhook_secret.is_empty() {
        return Err(crate::DeployError::Config(
            "PPT_ESIGN_WEBHOOK_SECRET must not be empty".into(),
        ));
    }

    // The `ppt-postgres` container is reachable by name from the
    // `ppt-<target>` bridge network once it's connected to that network
    // (see ops docs). Per-target database name pattern: `ppt_<target>`
    // (e.g. `ppt_prod`, `ppt_staging`) — those databases must exist before
    // first deploy (ops creates them from `ppt_dev_template`).
    //
    // Build the URL via `url::Url::set_password` rather than `format!()` so
    // passwords containing reserved URL characters (`@`, `:`, `/`, `#`,
    // etc.) are correctly percent-encoded. Mirrors `PostgresOps::url_for`'s
    // approach for the same reason.
    let mut url = url::Url::parse("postgres://ppt@ppt-postgres:5432/").map_err(|e| {
        crate::DeployError::Config(format!("internal: bad postgres URL template: {e}"))
    })?;
    url.set_password(Some(&postgres_password)).map_err(|()| {
        crate::DeployError::Config("failed to set postgres password on URL".into())
    })?;
    url.set_path(&format!("/ppt_{target_name}"));
    let database_url: String = url.into();

    // CORS allow-list — both UIs and both APIs from this target's tree, so
    // the browser running on rlt.sk / ppt.rlt.sk can talk to api.rlt.sk
    // and api.ppt.rlt.sk without 'Access-Control-Allow-Origin' rejections.
    let cors = format!(
        "https://{0},https://api.{0},https://{1},https://api.{1}",
        target.reality_apex, target.ppt_apex
    );

    // Shared baseline both backends need. Per-service additions are made
    // below where they differ (api-server has TOTP+INTEGRATION encryption
    // keys; reality-server has the PM OAuth client secret).
    //
    // Several URL-shaped vars (BASE_URL / APP_BASE_URL / API_BASE_URL) are
    // read by individual modules with localhost defaults that break in
    // prod (e.g. signed email links pointing at http://localhost:3000).
    // We inject the right per-target apexes so those modules don't have
    // to know about the deploy topology.
    let app_base_url = format!("https://{}", target.ppt_apex);
    let api_base_url = format!("https://api.{}", target.ppt_apex);
    let mut backend_env = vec![
        format!("DATABASE_URL={database_url}"),
        format!("JWT_SECRET={jwt_secret}"),
        format!("CORS_ALLOWED_ORIGINS={cors}"),
        // `APP_BASE_URL` is consumed by api-server's EmailService for
        // verification links; `BASE_URL` is read by routes/agencies and
        // signatures::DEFAULT_BASE_URL falls back to localhost without it.
        // Setting both to the PM UI's apex covers both code paths.
        format!("APP_BASE_URL={app_base_url}"),
        format!("BASE_URL={app_base_url}"),
        // `API_BASE_URL` is used by integrations callbacks (e.g. webhook
        // URLs we hand out to Adobe Sign / DocuSign) and must point at the
        // public api.<ppt_apex> host the third party can reach.
        format!("API_BASE_URL={api_base_url}"),
        "RUST_LOG=info".into(),
    ];

    // PLATFORM_HOST: comma-separated list of hosts the api-core
    // `host_tenant_middleware` should resolve to `TenantSource::PlatformHost`
    // (global-read context, no specific tenant). Both backends use the same
    // resolver, so this list goes on api-server AND reality-server.
    //
    // Includes both the Reality Portal apex tree (`<reality_apex>`,
    // `www.<reality_apex>`, `api.<reality_apex>`), the Property Management
    // apex tree (`<ppt_apex>`, `api.<ppt_apex>`), and the super-admin
    // control plane (`admin.<reality_apex>`). Without these the api-server
    // rejects every request with 404 "Unknown tenant host", because
    // `reserved_platform_hosts` is consulted by the agency_domains CHECK
    // constraint but NOT by the resolver itself (see api-core
    // middleware/host_tenant.rs `load_platform_hosts`).
    let platform_hosts = format!(
        "{0},www.{0},api.{0},{1},api.{1},admin.{0}",
        target.reality_apex, target.ppt_apex
    );
    let mut api_env = backend_env.clone();
    api_env.push(format!("TOTP_ENCRYPTION_KEY={totp_key}"));
    api_env.push(format!("INTEGRATION_ENCRYPTION_KEY={integration_key}"));
    api_env.push(format!("ESIGN_TOKEN_SECRET={esign_token_secret}"));
    api_env.push(format!("ESIGN_WEBHOOK_SECRET={esign_webhook_secret}"));
    api_env.push(format!("PLATFORM_HOST={platform_hosts}"));

    let mut reality_env = std::mem::take(&mut backend_env);
    reality_env.push(format!("PM_CLIENT_SECRET={pm_client_secret}"));
    // Point reality-server at the api-server inside the same target's tree
    // so the SSO/OAuth handshake reaches the right backend. Variable name
    // is `PM_API_URL` (verified against reality-server/src/state.rs:44 —
    // the value is read into `pm_api_base` and used as the prefix for the
    // OAuth authorize/token/userinfo/introspect URLs). Easy to confuse with
    // `API_BASE_URL` or `PM_API_BASE_URL`; those names are not consulted
    // by reality-server. Hits the public api.<ppt_apex> host so the SSO
    // round-trip flows through Caddy, which already proxies to the active
    // blue/green color — simpler than threading the active-color name into
    // env config.
    reality_env.push(format!("PM_API_URL={api_base_url}"));
    // `SSO_CALLBACK_URL` defaults to `http://localhost:8081/api/v1/sso/callback`
    // — that endpoint is on reality-server itself, exposed publicly at
    // `api.<reality_apex>/api/v1/sso/callback`. The reality apex host only
    // reverse-proxies to reality-web (Next.js :3000), which has no
    // `/api/v1/sso/*` handler; using the bare apex as the callback produces
    // a Next.js 404 after the OAuth redirect instead of a code exchange
    // (fix for #952). The `api.<apex>` subdomain is already provisioned by
    // `BlueGreenDeployer::deploy` → `register_route("api.{reality_apex}", …)`
    // pointing at reality-server :8081, so this is the correct public URL.
    // Without overriding the default, the redirect from PM SSO would point at
    // localhost in the user's browser → broken login.
    reality_env.push(format!(
        "SSO_CALLBACK_URL=https://api.{}/api/v1/sso/callback",
        target.reality_apex
    ));
    reality_env.push(format!("PLATFORM_HOST={platform_hosts}"));

    let mut envs = std::collections::HashMap::new();
    envs.insert("api".into(), api_env);
    envs.insert("reality".into(), reality_env);
    // Next.js reads window.__ENV__ (served by /env.js) at runtime; the
    // env.js route handler reads NEXT_PUBLIC_* from process.env.
    envs.insert(
        "reality-web".into(),
        vec![
            format!("NEXT_PUBLIC_API_URL=https://api.{}", target.reality_apex),
            format!("NEXT_PUBLIC_SITE_URL=https://{}", target.reality_apex),
        ],
    );
    // ppt-web is a Vite-built static bundle: API URL is inlined at build
    // time as the relative path `/api` (see docker-frontend.yml build-args).
    // The cross-origin shim happens at request time inside the container's
    // own nginx — `/api/` and `/ws/` are proxied to the matching color of
    // api-server in the same network. The two BG_* vars that drive that
    // proxy upstream are appended in `BlueGreenDeployer::deploy` because
    // the color is only known after the next-color decision; this entry
    // stays empty here.
    envs.insert("ppt".into(), vec![]);
    // admin-web is the same shape as ppt-web: a static Vite SPA served by
    // nginx. Its build-time `VITE_API_URL=/api` is baked into the bundle,
    // so the SPA fetches `/api/*` against its own origin. The container's
    // nginx then proxies `/api/` to `${BG_TARGET}-api-${BG_COLOR}:8080`
    // via envsubst at startup — `BG_TARGET` / `BG_COLOR` are appended in
    // `BlueGreenDeployer::deploy` once the next color is known.
    envs.insert("admin-web".into(), vec![]);
    Ok(envs)
}

#[cfg(test)]
mod build_service_envs_tests {
    use super::*;

    fn test_target() -> crate::config::Target {
        crate::config::Target {
            docker_socket: "/var/run/docker.sock".into(),
            caddy_url: "http://localhost:2019".into(),
            reality_apex: "staging.rlt.sk".into(),
            ppt_apex: "staging.ppt.rlt.sk".into(),
            idle_timeout: None,
            promote_strategy: None,
            rollback_mode: "manual".into(),
            health_grace: None,
        }
    }

    fn set_all_required_secrets() {
        // Deploy-server-side secrets `build_service_envs` reads, satisfying its
        // length/hex validation.
        std::env::set_var("POSTGRES_PASSWORD", "test-postgres-password");
        std::env::set_var("PPT_JWT_SECRET", "x".repeat(32));
        std::env::set_var("PPT_TOTP_ENCRYPTION_KEY", "a".repeat(64));
        std::env::set_var("PPT_INTEGRATION_ENCRYPTION_KEY", "b".repeat(64));
        std::env::set_var("PPT_PM_CLIENT_SECRET", "c".repeat(32));
        std::env::set_var("PPT_ESIGN_TOKEN_SECRET", "d".repeat(32));
        std::env::set_var("PPT_ESIGN_WEBHOOK_SECRET", "webhook-secret");
    }

    /// Issue #951: every secret the api-server requires at startup
    /// (`API_REQUIRED_STARTUP_ENV`) must be injected onto the `api` container by
    /// `build_service_envs`, or the container crash-loops on deploy — and when
    /// any required deploy-server secret is absent, `build_service_envs` must
    /// fail fast with a `Config` error rather than start empty-valued containers.
    ///
    /// Both halves are in one test because they mutate process-wide env vars and
    /// would race if run as separate (parallel) tests.
    #[test]
    fn build_service_envs_enforces_api_startup_contract() {
        set_all_required_secrets();

        let envs = build_service_envs("staging", &test_target())
            .expect("build_service_envs should succeed with all secrets set");
        let api_env = envs.get("api").expect("`api` service env must exist");

        for key in API_REQUIRED_STARTUP_ENV {
            assert!(
                api_env.iter().any(|kv| kv.starts_with(&format!("{key}="))),
                "the `api` container env is missing `{key}`, which api-server requires at \
                 startup (issue #951). Add it to `build_service_envs` so the deploy doesn't \
                 crash-loop the api container."
            );
        }

        // Missing required secret -> fail fast with a Config error.
        std::env::remove_var("PPT_ESIGN_TOKEN_SECRET");
        let err = build_service_envs("staging", &test_target())
            .expect_err("must error when a required secret is missing");
        assert!(
            matches!(err, crate::DeployError::Config(_)),
            "expected a Config error, got {err:?}"
        );
    }
}

impl BlueGreenDeployer {
    pub async fn deploy(&self, spec: &BlueGreenSpec) -> Result<()> {
        let docker = self.docker.bollard();
        for img in [
            &spec.api_image,
            &spec.reality_image,
            &spec.ppt_web_image,
            &spec.reality_web_image,
        ] {
            self.pull_image(docker, img).await?;
        }
        if let Some(admin_img) = &spec.admin_web_image {
            self.pull_image(docker, admin_img).await?;
        }

        let target_name = &spec.target_name;

        // Make sure the Caddy container shares the `ppt-{target}` Docker
        // network so its Docker-DNS lookup of `<target>-{api,reality,ppt,reality-web}-{color}`
        // upstreams succeeds. ppt-caddy defaults to only the host bridge —
        // without this, every prod/staging request would 502 with
        // "host not found in upstream". Idempotent: no-op if already
        // connected.
        self.docker
            .ensure_network_membership(&format!("ppt-{target_name}"), "ppt-caddy")
            .await?;

        // Check how many of each color is running. Pick the OPPOSITE color of whichever
        // has more services running. If tied (everything down or split), default to "blue".
        let mut blue_count = 0u8;
        let mut green_count = 0u8;
        for service in BG_SERVICES {
            if self
                .docker
                .is_running(&format!("{target_name}-{service}-blue"))
                .await
                .unwrap_or(false)
            {
                blue_count += 1;
            }
            if self
                .docker
                .is_running(&format!("{target_name}-{service}-green"))
                .await
                .unwrap_or(false)
            {
                green_count += 1;
            }
        }

        // Decide next_color: the color that has FEWER (or no) live services.
        // Tie-breaker: "blue" for first deploy AND for mixed state (split-recovery).
        // We log the mixed-state path explicitly because it's an unusual recovery action.
        let next_color = if blue_count > green_count {
            "green"
        } else if green_count > blue_count {
            "blue"
        } else {
            // blue_count == green_count: either both 0 (cold start) or both > 0 (mixed state).
            if blue_count > 0 {
                tracing::warn!(
                    target = %target_name,
                    blue_count,
                    green_count,
                    "blue/green target is in mixed state — both colors have running services; recovering by deploying blue"
                );
            }
            "blue"
        };
        let prev_color = if next_color == "blue" {
            "green"
        } else {
            "blue"
        };

        // Per-service env: if the spec doesn't have an entry for a service
        // we pass an empty Vec rather than panicking. Missing api/reality
        // entries would surface as crash-looping containers (recoverable,
        // but bad UX). ppt-web's `BG_TARGET` / `BG_COLOR` are appended
        // here because the color is only known at this point — the
        // ppt-web image's nginx renders /api and /ws proxy upstreams from
        // them at startup so SPA fetches reach the same-color api-server
        // in the same Docker network.
        let env_for = |s: &str| spec.service_envs.get(s).cloned().unwrap_or_default();
        let mut ppt_env = env_for("ppt");
        ppt_env.push(format!("BG_TARGET={target_name}"));
        ppt_env.push(format!("BG_COLOR={next_color}"));
        // admin-web's nginx renders the /api/* proxy upstream from the same
        // two vars at container start (envsubst). Pattern mirrors ppt-web.
        let mut admin_web_env = env_for("admin-web");
        admin_web_env.push(format!("BG_TARGET={target_name}"));
        admin_web_env.push(format!("BG_COLOR={next_color}"));

        // reality-server's `/health` periodically pings the PM API. Without
        // an override it uses `${PM_API_URL}/health` — which is the public
        // `https://api.<ppt_apex>` host that goes Container → DNS → CF →
        // onyx → Caddy. That CF round-trip produced SSL handshake errors
        // (CF Full mode + onyx cert lifecycle) and pinned reality-server's
        // `/health` to `unhealthy`, keeping wait_until_ready in STARTING
        // and timing the deploy out without registering Caddy routes.
        // Override `PM_API_HEALTH_URL` to hit the api-server container
        // directly over the `ppt-{target}` Docker network. Color-aware
        // because the container name encodes blue/green.
        //
        // PM_API_URL stays at the public host: it's also the prefix for
        // OAuth `/authorize` URLs that the BROWSER is redirected to during
        // SSO, and the browser obviously can't resolve internal Docker
        // names. Only the server-to-server health probe needs the
        // shortcut.
        let mut reality_env = env_for("reality");
        reality_env.push(format!(
            // Hits api-server's /readiness (deep dep check) — reality-server's
            // OWN readiness wants the full picture for its dashboard, not just
            // "is the process up". The Docker HEALTHCHECK still uses /health.
            "PM_API_HEALTH_URL=http://{target_name}-api-{next_color}:8080/readiness"
        ));
        self.run_service(
            &format!("{target_name}-api-{next_color}"),
            &spec.api_image,
            8080,
            target_name,
            env_for("api"),
        )
        .await?;
        self.run_service(
            &format!("{target_name}-reality-{next_color}"),
            &spec.reality_image,
            8081,
            target_name,
            reality_env,
        )
        .await?;
        // ppt-web's nginx listens on 8080 (see docker/frontend/ppt-web.Dockerfile —
        // `EXPOSE 8080` matched by `listen 8080` in the rendered nginx
        // config). Earlier code passed 80 here and in the Caddy upstream
        // below, which would have refused the connection on the bridge
        // network — caught when this PR's nginx changes were first wired
        // up. Aligning to 8080 across run_service / wait_until_ready /
        // register_route.
        self.run_service(
            &format!("{target_name}-ppt-{next_color}"),
            &spec.ppt_web_image,
            8080,
            target_name,
            ppt_env,
        )
        .await?;
        self.run_service(
            &format!("{target_name}-reality-web-{next_color}"),
            &spec.reality_web_image,
            3000,
            target_name,
            env_for("reality-web"),
        )
        .await?;
        // admin-web: same nginx-on-8080 pattern as ppt-web. The Dockerfile
        // EXPOSEs 8080 and renders its /api/* proxy upstream from BG_TARGET
        // and BG_COLOR at start.
        //
        // Optional: missing on rollback specs built from pre-#268 Release rows
        // (see `BlueGreenSpec::admin_web_image` rationale). When absent, we
        // skip the container — the corresponding `admin.<apex>` Caddy route
        // (registered below) is also skipped so requests to that host fall
        // through to onyx / 404 instead of routing to a dead upstream.
        if let Some(admin_img) = spec.admin_web_image.as_deref() {
            self.run_service(
                &format!("{target_name}-admin-web-{next_color}"),
                admin_img,
                8080,
                target_name,
                admin_web_env,
            )
            .await?;
        }

        // Wait for each container to reach a ready state before flipping Caddy upstream.
        // Backends (api / reality) get a longer timeout because they run all DB
        // migrations on first boot of a fresh target DB before binding the
        // listener — 30s wasn't enough head-room on a cold `ppt_prod` /
        // `ppt_staging`. Frontends stay on 30s; they bind sooner.
        self.wait_until_ready(&format!("{target_name}-api-{next_color}"), 8080, 90)
            .await?;
        self.wait_until_ready(&format!("{target_name}-reality-{next_color}"), 8081, 90)
            .await?;
        self.wait_until_ready(&format!("{target_name}-ppt-{next_color}"), 8080, 30)
            .await?;
        self.wait_until_ready(&format!("{target_name}-reality-web-{next_color}"), 3000, 30)
            .await?;
        if spec.admin_web_image.is_some() {
            self.wait_until_ready(&format!("{target_name}-admin-web-{next_color}"), 8080, 30)
                .await?;
        }

        // Per-service Caddy routes use the dual-apex layout:
        //   <reality_apex>           → reality-web   (Reality Portal UI, bare apex)
        //   api.<reality_apex>       → reality-server (Reality public API)
        //   <ppt_apex>               → ppt-web       (Property Management UI)
        //   api.<ppt_apex>           → api-server    (PM API)
        // For prod with reality_apex="rlt.sk" + ppt_apex="ppt.rlt.sk" this gives
        // rlt.sk / api.rlt.sk / ppt.rlt.sk / api.ppt.rlt.sk respectively.
        let reality_apex = &spec.reality_apex;
        let ppt_apex = &spec.ppt_apex;
        self.caddy
            .register_route(
                &format!("api.{ppt_apex}"),
                &format!("{target_name}-api-{next_color}:8080"),
            )
            .await?;
        self.caddy
            .register_route(
                &format!("api.{reality_apex}"),
                &format!("{target_name}-reality-{next_color}:8081"),
            )
            .await?;
        self.caddy
            .register_route(ppt_apex, &format!("{target_name}-ppt-{next_color}:8080"))
            .await?;
        self.caddy
            .register_route(
                reality_apex,
                &format!("{target_name}-reality-web-{next_color}:3000"),
            )
            .await?;
        // `www.` alias for the Reality Portal apex. Slovak users habitually
        // type `www.` even on bare-apex domains, so a missing route means a
        // visitor lands on `www.rlt.sk/...` and gets `ERR_SSL_PROTOCOL_ERROR`
        // (Caddy can't auto-issue a cert for a host with no matching site).
        // For staging that's `www.staging.rlt.sk`; the route is inert until
        // DNS for that host points at onyx, which is fine.
        //
        // No `www.` for the PM apex (`ppt.rlt.sk`) because operator-facing
        // dashboards aren't typed by hand with a www prefix; if a need ever
        // comes up, the same one-liner pattern applies.
        self.caddy
            .register_route(
                &format!("www.{reality_apex}"),
                &format!("{target_name}-reality-web-{next_color}:3000"),
            )
            .await?;
        // admin.<reality_apex> → admin-web. The admin-web nginx itself
        // proxies /api/* to api-server in the same color (same way ppt-web
        // does), so Caddy registers a single upstream per host — no
        // path-based split needed at the edge.
        //
        // Prod: admin.rlt.sk. Staging: admin.staging.rlt.sk. Both rely on
        // Cloudflare Universal SSL (`*.rlt.sk`) for TLS, and on `PLATFORM_HOST`
        // env var injected by `build_service_envs` so api-server's
        // host_tenant_middleware resolves the host as `PlatformHost`.
        // (Migration 00151 adds the host to `reserved_platform_hosts` for
        // the agency_domains CHECK constraint — it prevents an agency from
        // claiming `admin.<apex>` — but that table is NOT consulted by the
        // resolver itself; see api-core middleware/host_tenant.rs
        // `load_platform_hosts`.)
        //
        // Skipped when `admin_web_image` is None (rollback to pre-#268
        // release); no admin-web container exists in that color and an
        // `admin.<apex>` route here would just 502.
        if spec.admin_web_image.is_some() {
            self.caddy
                .register_route(
                    &format!("admin.{reality_apex}"),
                    &format!("{target_name}-admin-web-{next_color}:8080"),
                )
                .await?;
        }

        let prev_containers: Vec<String> = BG_SERVICES
            .iter()
            .map(|s| format!("{target_name}-{s}-{prev_color}"))
            .collect();
        self.docker.cleanup_containers(&prev_containers).await;
        Ok(())
    }

    async fn pull_image(&self, docker: &Docker, image: &str) -> Result<()> {
        let opts = CreateImageOptions {
            from_image: image.to_string(),
            ..Default::default()
        };
        let mut stream = docker.create_image(Some(opts), None, None);
        while let Some(item) = stream.next().await {
            item.map_err(crate::DeployError::Docker)?;
        }
        Ok(())
    }

    async fn run_service(
        &self,
        name: &str,
        image: &str,
        container_port: u16,
        target: &str,
        env: Vec<String>,
    ) -> Result<()> {
        let docker = self.docker.bollard();
        let _ = docker
            .remove_container(
                name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        let mut exposed = HashMap::new();
        exposed.insert(format!("{container_port}/tcp"), HashMap::<(), ()>::new());

        let host_config = HostConfig {
            network_mode: Some(format!("ppt-{target}")),
            restart_policy: Some(bollard::models::RestartPolicy {
                name: Some(bollard::models::RestartPolicyNameEnum::UNLESS_STOPPED),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cfg = Config {
            image: Some(image.to_string()),
            exposed_ports: Some(exposed),
            // Docker MERGES image env (`Dockerfile ENV`) with container env
            // — explicitly verified: `docker run -e FOO=bar node:20-alpine env`
            // still shows the image's `NODE_VERSION` and `PATH`. So setting
            // `Some(env)` doesn't wipe defaults like `NODE_ENV=production`,
            // `PORT=3000`, `HOSTNAME=0.0.0.0` baked into reality-web. We
            // still pass `None` for the empty case as a small clarity win
            // (no allocation of an empty Vec on the wire).
            env: if env.is_empty() { None } else { Some(env) },
            host_config: Some(host_config),
            ..Default::default()
        };

        let create = docker
            .create_container(
                Some(CreateContainerOptions {
                    name: name.to_string(),
                    platform: None,
                }),
                cfg,
            )
            .await
            .map_err(crate::DeployError::Docker)?;
        docker
            .start_container(&create.id, None::<StartContainerOptions<String>>)
            .await
            .map_err(crate::DeployError::Docker)?;
        Ok(())
    }

    /// Poll the container's runtime state until it's "ready enough" to receive traffic.
    ///
    /// The deploy-server host doesn't share the `ppt-{target}` bridge network with the
    /// staging containers, so we can't TCP-connect to `container_name:port` directly.
    /// Instead, we inspect the container and treat it as ready when:
    ///   1. a Docker healthcheck is configured AND has reported HEALTHY, OR
    ///   2. a Docker healthcheck is configured but is still STARTING — keep
    ///      polling until it transitions to HEALTHY/UNHEALTHY (or the overall
    ///      `timeout_secs` elapses), OR
    ///   3. no healthcheck is configured (or status is Empty/None), but the
    ///      container has been in `running` state for at least `grace` seconds
    ///      (typical axum/next.js processes bind their listener within 2-3s).
    ///
    /// Distinguishing STARTING from "no healthcheck" matters when the app does
    /// real startup work — e.g. api-server runs ~100 SQL migrations on first
    /// boot of a fresh target DB, which takes longer than the 3s `grace`. The
    /// previous logic would treat the container as ready on `grace` regardless
    /// of healthcheck state, flipping Caddy upstream BEFORE `/health` could
    /// pass and producing a 502 window. Holding off until HEALTHY (or timeout)
    /// fixes that race.
    async fn wait_until_ready(
        &self,
        container_name: &str,
        _container_port: u16,
        timeout_secs: u64,
    ) -> crate::Result<()> {
        use bollard::models::HealthStatusEnum;
        use std::time::Duration;
        use tokio::time::sleep;

        let docker = self.docker.bollard();
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
        let mut first_running_at: Option<std::time::Instant> = None;
        let grace = Duration::from_secs(3);

        while std::time::Instant::now() < deadline {
            match docker.inspect_container(container_name, None).await {
                Ok(info) => {
                    let state = info.state.as_ref();
                    let running = state.and_then(|s| s.running).unwrap_or(false);
                    let health = state.and_then(|s| s.health.as_ref()).and_then(|h| h.status);

                    if running {
                        match health {
                            Some(HealthStatusEnum::HEALTHY) => return Ok(()),
                            Some(HealthStatusEnum::UNHEALTHY) => {
                                return Err(crate::DeployError::Internal(format!(
                                    "container {container_name} reported UNHEALTHY"
                                )));
                            }
                            // STARTING: a healthcheck is configured and the
                            // app is still warming up (e.g. running migrations
                            // before binding the listener). Keep polling — do
                            // NOT fall back to the grace heuristic, because
                            // that would flip Caddy before `/health` succeeds.
                            Some(HealthStatusEnum::STARTING) => {
                                // intentionally fall through to next poll
                            }
                            // No healthcheck (None) or status Empty: fall back
                            // to the original "running for grace seconds"
                            // heuristic. Keeps support for static images
                            // (e.g. ppt-web nginx) that don't always have a
                            // healthcheck wired up.
                            _ => {
                                let now = std::time::Instant::now();
                                let first = first_running_at.get_or_insert(now);
                                if now.duration_since(*first) >= grace {
                                    return Ok(());
                                }
                            }
                        }
                    } else {
                        first_running_at = None;
                    }
                }
                Err(e) => {
                    tracing::debug!(error = %e, container = %container_name, "inspect during readiness");
                }
            }
            sleep(Duration::from_millis(500)).await;
        }

        Err(crate::DeployError::Internal(format!(
            "container {container_name} not ready within {timeout_secs}s"
        )))
    }
}

// Backward-compat aliases (callers from Phase 2 use the staging names).
pub type StagingDeployer = BlueGreenDeployer;
pub type StagingDeploySpec = BlueGreenSpec;
