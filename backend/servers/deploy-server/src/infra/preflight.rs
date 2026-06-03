// backend/servers/deploy-server/src/infra/preflight.rs
//! Pre-deploy environment validator.
//!
//! Runs **before** `BlueGreenDeployer::deploy` so an operator gets one clear
//! 400 with every problem listed up-front instead of a half-completed deploy
//! that leaves the target in a wedged state. Each prior production incident
//! that took >10 min to diagnose maps to one of these checks:
//!
//! - Missing `POSTGRES_PASSWORD`/`PPT_JWT_SECRET`/etc. → backend crash-loop with
//!   no obvious entry in deploy-server logs (only the container's stderr).
//! - `ppt-postgres` not running → migration step blocks for 90s then times out.
//! - `ppt_<target>` DB doesn't exist → sqlx connects, migration `CREATE TABLE`
//!   succeeds, but every subsequent INSERT collides with another target's data.
//! - `ppt-<target>` Docker network missing → `create_container` errors with
//!   `network ppt-prod not found`, which is the FIRST docker call so half the
//!   blue/green logic doesn't even run; cleanup state is still consistent but
//!   the operator sees a confusing 500.
//! - `ppt-caddy` not running → route registration succeeds against the admin
//!   API of a dead Caddy (admin survives a process restart cycle if the
//!   container was just paused), routes accumulate, no traffic ever flows.
//!
//! Checks run in parallel where possible and ALL errors are collected before
//! returning — partial results would just trade one whack-a-mole loop for
//! another.

use crate::config::Target;
use crate::infra::{CaddyClient, DockerClient};
use std::fmt;

/// One concrete failure surfaced by the preflight validator.
///
/// `check` is the short tag for grouping/logging (`env`, `postgres`, `db`,
/// `network`, `caddy`). `detail` is the observed error. `remedy` is the
/// minimum fix the operator can apply on the box. The intent is that the
/// operator can copy/paste `remedy` lines into a shell and re-trigger the
/// deploy without further investigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightError {
    pub check: String,
    pub detail: String,
    pub remedy: String,
}

impl fmt::Display for PreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} — fix: {}", self.check, self.detail, self.remedy)
    }
}

/// Inputs for `validate_target`.
///
/// Everything is borrowed; the validator owns no state. The `postgres_admin_url`
/// is the same URL the deploy-server uses for worktree DB ops (see
/// `Config::postgres_admin_url`). The `target_db` is computed by the caller
/// (typically `format!("ppt_{target_name}")`) so target-name conventions stay
/// in one place.
pub struct PreflightContext<'a> {
    pub target_name: &'a str,
    pub target: &'a Target,
    pub docker: &'a DockerClient,
    pub caddy: &'a CaddyClient,
    pub postgres_admin_url: &'a str,
    pub target_db: &'a str,
    /// Container name of the shared Postgres instance (typically `ppt-postgres`).
    pub postgres_container: &'a str,
    /// Container name of the Caddy instance (typically `ppt-caddy`).
    pub caddy_container: &'a str,
    /// Docker network the target's app containers live on (typically
    /// `ppt-<target_name>`). Validated for existence; membership of
    /// `ppt-caddy`/`ppt-postgres` is auto-fixed by the deployer so we don't
    /// fail on it here.
    pub target_network: &'a str,
}

/// Run all preflight checks for `ctx`. Returns an empty vec on full success;
/// otherwise contains every failure observed (no short-circuit).
///
/// Network/postgres checks are I/O-bound, so they're issued concurrently. Env
/// checks are synchronous and run inline.
pub async fn validate_target(ctx: &PreflightContext<'_>) -> Vec<PreflightError> {
    let mut errors = Vec::new();

    // ──── 1. Required env vars ─────────────────────────────────────────────
    errors.extend(check_env_vars());

    // ──── 2-6. I/O checks, in parallel ─────────────────────────────────────
    let (pg_running, pg_db_exists, net_exists, caddy_running) = tokio::join!(
        check_container_running(ctx.docker, ctx.postgres_container),
        check_database_exists(ctx.postgres_admin_url, ctx.target_db),
        check_network_exists(ctx.docker, ctx.target_network),
        check_container_running(ctx.docker, ctx.caddy_container),
    );

    if let Err(e) = pg_running {
        errors.push(PreflightError {
            check: "postgres".into(),
            detail: format!("container '{}' not running: {}", ctx.postgres_container, e),
            remedy: format!(
                "docker start {} (or `pmctl bootstrap-target {}`)",
                ctx.postgres_container, ctx.target_name
            ),
        });
    }

    if let Err(e) = pg_db_exists {
        errors.push(PreflightError {
            check: "db".into(),
            detail: format!("database '{}' not reachable: {}", ctx.target_db, e),
            remedy: format!(
                "docker exec {} psql -U ppt -c 'CREATE DATABASE \"{}\" TEMPLATE \"ppt_dev_template\"' \
                 (or `pmctl bootstrap-target {}`)",
                ctx.postgres_container, ctx.target_db, ctx.target_name
            ),
        });
    }

    if let Err(e) = net_exists {
        errors.push(PreflightError {
            check: "network".into(),
            detail: format!("docker network '{}' missing: {}", ctx.target_network, e),
            remedy: format!(
                "docker network create {} (or `pmctl bootstrap-target {}`)",
                ctx.target_network, ctx.target_name
            ),
        });
    }

    if let Err(ref e) = caddy_running {
        errors.push(PreflightError {
            check: "caddy".into(),
            detail: format!("container '{}' not running: {}", ctx.caddy_container, e),
            remedy: format!("docker start {}", ctx.caddy_container),
        });
    }

    // Caddy admin reachability — only worth checking if the container itself
    // is up (otherwise the failure is redundant noise). Done sequentially
    // because skipping requires the result of the prior check.
    if caddy_running.is_ok() {
        if let Err(e) = check_caddy_admin(ctx.caddy).await {
            errors.push(PreflightError {
                check: "caddy".into(),
                detail: format!("admin API unreachable: {e}"),
                remedy: "verify caddy admin endpoint (default 127.0.0.1:2019) and \
                         restart container if needed"
                    .into(),
            });
        }
    }

    errors
}

/// Required deploy-server env vars. Mirrors the contract enforced by
/// `build_service_envs`; surfacing failure here lets the operator fix
/// `/etc/ppt-deploy/secrets.env` and re-trigger the deploy without first
/// hitting a backend container crash-loop.
const REQUIRED_ENV: &[(&str, EnvCheck)] = &[
    ("POSTGRES_PASSWORD", EnvCheck::NonEmpty),
    ("PPT_JWT_SECRET", EnvCheck::MinLen(32)),
    ("PPT_TOTP_ENCRYPTION_KEY", EnvCheck::HexLen(64)),
    ("PPT_INTEGRATION_ENCRYPTION_KEY", EnvCheck::HexLen(64)),
    // Match `build_service_envs`: it rejects secrets shorter than 32 chars.
    // Earlier we used `NonEmpty` which let preflight pass on a too-short
    // value, only for the deploy itself to fail later with a config error
    // — exactly the up-front-failure contract this validator exists to
    // uphold.
    ("PPT_PM_CLIENT_SECRET", EnvCheck::MinLen(32)),
    // E-signature secrets: `build_service_envs` requires both (token >= 32
    // chars, webhook non-empty) and panics the api container without them.
    // Validate up-front so a missing ESIGN secret fails preflight instead of
    // crash-looping the deploy (issue #951 follow-up).
    ("PPT_ESIGN_TOKEN_SECRET", EnvCheck::MinLen(32)),
    ("PPT_ESIGN_WEBHOOK_SECRET", EnvCheck::NonEmpty),
];

#[derive(Copy, Clone)]
enum EnvCheck {
    NonEmpty,
    MinLen(usize),
    /// Exactly N hex characters (used for AES-256 keys: 64 hex = 32 bytes).
    HexLen(usize),
}

fn check_env_vars() -> Vec<PreflightError> {
    REQUIRED_ENV
        .iter()
        .filter_map(|(name, kind)| validate_one_env(name, *kind))
        .collect()
}

fn validate_one_env(name: &str, kind: EnvCheck) -> Option<PreflightError> {
    let val = match std::env::var(name) {
        Ok(v) => v,
        Err(_) => {
            return Some(PreflightError {
                check: "env".into(),
                detail: format!("{name} not set"),
                remedy: format!("add {name}=... to /etc/ppt-deploy/secrets.env and reload service"),
            });
        }
    };
    match kind {
        EnvCheck::NonEmpty => {
            if val.is_empty() {
                return Some(PreflightError {
                    check: "env".into(),
                    detail: format!("{name} is empty"),
                    remedy: format!("set a non-empty value for {name} in secrets.env"),
                });
            }
        }
        EnvCheck::MinLen(min) => {
            if val.len() < min {
                return Some(PreflightError {
                    check: "env".into(),
                    detail: format!("{name} length {} < required {}", val.len(), min),
                    remedy: format!(
                        "regenerate {name} with `openssl rand -hex {}`",
                        min.div_ceil(2)
                    ),
                });
            }
        }
        EnvCheck::HexLen(want) => {
            let is_hex = val.chars().all(|c| c.is_ascii_hexdigit());
            if val.len() != want || !is_hex {
                return Some(PreflightError {
                    check: "env".into(),
                    detail: format!(
                        "{name} must be exactly {want} hex characters (got len={}, hex={})",
                        val.len(),
                        is_hex
                    ),
                    remedy: format!("regenerate with `openssl rand -hex {}`", want / 2),
                });
            }
        }
    }
    None
}

async fn check_container_running(docker: &DockerClient, name: &str) -> Result<(), String> {
    match docker.is_running(name).await {
        Ok(true) => Ok(()),
        Ok(false) => Err("not running".into()),
        Err(e) => Err(e.to_string()),
    }
}

async fn check_network_exists(docker: &DockerClient, name: &str) -> Result<(), String> {
    use bollard::network::InspectNetworkOptions;
    match docker
        .bollard()
        .inspect_network(name, None::<InspectNetworkOptions<&str>>)
        .await
    {
        Ok(_) => Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Err("not found".into()),
        Err(e) => Err(e.to_string()),
    }
}

/// Verify the target DB exists by running `psql -tAc "SELECT 1 FROM
/// pg_database WHERE datname='<db>'"` against the admin URL.
///
/// `psql` is preferred over a sqlx-postgres dep because:
///   - deploy-server's `sqlx` is compiled with the `sqlite` feature only;
///   - the rest of `infra::postgres` already shells out to `psql` for
///     consistency (the operator can rerun the same command by hand).
///
/// A missing DB returns `Ok(())` from psql with empty stdout, so we check
/// stdout for `1` rather than relying on exit code.
async fn check_database_exists(admin_url: &str, db_name: &str) -> Result<(), String> {
    let escaped = db_name.replace('\'', "''");
    let sql = format!("SELECT 1 FROM pg_database WHERE datname = '{}'", escaped);
    let out = tokio::process::Command::new("psql")
        .args(["-tAc", &sql, admin_url])
        .output()
        .await
        .map_err(|e| format!("psql exec failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "psql exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim() == "1" {
        Ok(())
    } else {
        Err(format!("database '{db_name}' does not exist"))
    }
}

async fn check_caddy_admin(caddy: &CaddyClient) -> Result<(), String> {
    caddy.ping().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `std::env` is process-global, so env-mutating tests must be serialized.
    /// Holding `ENV_LOCK` is necessary but not sufficient — a panic mid-test
    /// would otherwise leave the process env permanently mangled (or worse:
    /// any env var the test runner injected upstream would be silently
    /// dropped). `EnvGuard` snapshots the required vars on construction and
    /// restores their exact prior state on drop, so each test is hermetic
    /// regardless of starting conditions or panic behaviour.
    use std::sync::{Mutex, MutexGuard};
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// RAII snapshot/restore for the `REQUIRED_ENV` keys. The mutex guard
    /// is held by this struct for the same lifetime, so two tests can't
    /// race on the env even if one panics.
    struct EnvGuard {
        _lock: MutexGuard<'static, ()>,
        snapshot: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            // PoisonError on a previous-test panic doesn't poison the env
            // itself — recover the guard and proceed; we'll still restore
            // state on drop.
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let snapshot = REQUIRED_ENV
                .iter()
                .map(|(name, _)| (*name, std::env::var(*name).ok()))
                .collect();
            // Start each test from a clean slate — but the snapshot above
            // remembers the originals so Drop puts them back.
            for (name, _) in REQUIRED_ENV {
                std::env::remove_var(name);
            }
            Self {
                _lock: lock,
                snapshot,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (name, prior) in &self.snapshot {
                match prior {
                    Some(v) => std::env::set_var(name, v),
                    None => std::env::remove_var(name),
                }
            }
        }
    }

    #[test]
    fn env_check_reports_every_missing_var() {
        let _g = EnvGuard::new();
        let errors = check_env_vars();
        assert_eq!(errors.len(), REQUIRED_ENV.len());
        for e in &errors {
            assert_eq!(e.check, "env");
            assert!(e.detail.contains("not set"), "got {}", e.detail);
        }
    }

    #[test]
    fn env_check_passes_with_valid_values() {
        let _g = EnvGuard::new();
        std::env::set_var("POSTGRES_PASSWORD", "x");
        std::env::set_var("PPT_JWT_SECRET", "a".repeat(32));
        std::env::set_var("PPT_TOTP_ENCRYPTION_KEY", "0".repeat(64));
        std::env::set_var("PPT_INTEGRATION_ENCRYPTION_KEY", "f".repeat(64));
        std::env::set_var("PPT_ESIGN_TOKEN_SECRET", "e".repeat(32));
        std::env::set_var("PPT_ESIGN_WEBHOOK_SECRET", "w");
        std::env::set_var("PPT_PM_CLIENT_SECRET", "z".repeat(32));
        let errors = check_env_vars();
        assert!(errors.is_empty(), "expected no errors, got {errors:?}");
    }

    #[test]
    fn env_check_rejects_short_jwt_secret() {
        let _g = EnvGuard::new();
        std::env::set_var("POSTGRES_PASSWORD", "x");
        std::env::set_var("PPT_JWT_SECRET", "tooshort");
        std::env::set_var("PPT_TOTP_ENCRYPTION_KEY", "0".repeat(64));
        std::env::set_var("PPT_INTEGRATION_ENCRYPTION_KEY", "f".repeat(64));
        std::env::set_var("PPT_ESIGN_TOKEN_SECRET", "e".repeat(32));
        std::env::set_var("PPT_ESIGN_WEBHOOK_SECRET", "w");
        std::env::set_var("PPT_PM_CLIENT_SECRET", "z".repeat(32));
        let errors = check_env_vars();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].detail.contains("PPT_JWT_SECRET"));
    }

    #[test]
    fn env_check_rejects_non_hex_encryption_key() {
        let _g = EnvGuard::new();
        std::env::set_var("POSTGRES_PASSWORD", "x");
        std::env::set_var("PPT_JWT_SECRET", "a".repeat(32));
        // 64 chars but not hex
        std::env::set_var("PPT_TOTP_ENCRYPTION_KEY", "z".repeat(64));
        std::env::set_var("PPT_INTEGRATION_ENCRYPTION_KEY", "f".repeat(64));
        std::env::set_var("PPT_ESIGN_TOKEN_SECRET", "e".repeat(32));
        std::env::set_var("PPT_ESIGN_WEBHOOK_SECRET", "w");
        std::env::set_var("PPT_PM_CLIENT_SECRET", "z".repeat(32));
        let errors = check_env_vars();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].detail.contains("PPT_TOTP_ENCRYPTION_KEY"));
    }

    #[test]
    fn env_check_rejects_wrong_length_encryption_key() {
        let _g = EnvGuard::new();
        std::env::set_var("POSTGRES_PASSWORD", "x");
        std::env::set_var("PPT_JWT_SECRET", "a".repeat(32));
        std::env::set_var("PPT_TOTP_ENCRYPTION_KEY", "0".repeat(32)); // half the required length
        std::env::set_var("PPT_INTEGRATION_ENCRYPTION_KEY", "f".repeat(64));
        std::env::set_var("PPT_ESIGN_TOKEN_SECRET", "e".repeat(32));
        std::env::set_var("PPT_ESIGN_WEBHOOK_SECRET", "w");
        std::env::set_var("PPT_PM_CLIENT_SECRET", "z".repeat(32));
        let errors = check_env_vars();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].detail.contains("PPT_TOTP_ENCRYPTION_KEY"));
    }

    #[test]
    fn env_check_rejects_short_pm_client_secret() {
        // PPT_PM_CLIENT_SECRET upgraded from NonEmpty to MinLen(32) so the
        // preflight contract matches `build_service_envs`. Pin it.
        let _g = EnvGuard::new();
        std::env::set_var("POSTGRES_PASSWORD", "x");
        std::env::set_var("PPT_JWT_SECRET", "a".repeat(32));
        std::env::set_var("PPT_TOTP_ENCRYPTION_KEY", "0".repeat(64));
        std::env::set_var("PPT_INTEGRATION_ENCRYPTION_KEY", "f".repeat(64));
        std::env::set_var("PPT_ESIGN_TOKEN_SECRET", "e".repeat(32));
        std::env::set_var("PPT_ESIGN_WEBHOOK_SECRET", "w");
        std::env::set_var("PPT_PM_CLIENT_SECRET", "tooshort");
        let errors = check_env_vars();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].detail.contains("PPT_PM_CLIENT_SECRET"));
    }
}
