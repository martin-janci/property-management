// backend/servers/deploy-server/src/config.rs
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind: String,
    pub state_dir: String,
    pub worktree_dir: String,
    pub snapshot_dir: String,
    #[serde(default = "default_ttl")]
    pub default_ttl_seconds: i64,
    pub idle_pause_seconds: i64,
    pub idle_stop_seconds: i64,
    pub git_repo_url: String,
    #[serde(default = "default_postgres_admin_url")]
    pub postgres_admin_url: String,
    #[serde(default = "default_postgres_template_db")]
    pub postgres_template_db: String,
    #[serde(default = "default_postgres_user_db_prefix")]
    pub postgres_user_db_prefix: String,
    #[serde(default = "default_backend_image_prefix")]
    pub backend_image_prefix: String,
    #[serde(default = "default_gh_repo")]
    pub gh_repo: String,
}

fn default_ttl() -> i64 {
    172_800
}

fn default_postgres_admin_url() -> String {
    "postgres://ppt:ppt_dev_password@localhost:5432/postgres".into()
}

fn default_postgres_template_db() -> String {
    "ppt_dev_template".into()
}

fn default_postgres_user_db_prefix() -> String {
    "ppt_wt_".into()
}

fn default_backend_image_prefix() -> String {
    "ghcr.io/martin-janci".into()
}

fn default_gh_repo() -> String {
    "martin-janci/property-management".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct TargetsConfig {
    pub targets: HashMap<String, Target>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub docker_socket: String,
    pub caddy_url: String,
    /// Apex hostname for the Reality Portal subtree at this target. The Caddy
    /// route for `reality-web` (Next.js UI) is registered at this exact host;
    /// `reality-server` (the Reality API) at `api.<reality_apex>`.
    /// Examples: "rlt.sk" (prod), "staging.rlt.sk" (staging).
    pub reality_apex: String,
    /// Apex hostname for the Property Management subtree at this target. The
    /// Caddy route for `ppt-web` (Vite/React UI) is registered at this exact
    /// host; `api-server` (the PM API) at `api.<ppt_apex>`.
    /// Examples: "ppt.rlt.sk" (prod), "staging.ppt.rlt.sk" (staging).
    pub ppt_apex: String,
    #[serde(default)]
    pub idle_timeout: Option<String>,
    #[serde(default)]
    pub promote_strategy: Option<String>,
    #[serde(default = "default_rollback_mode")]
    pub rollback_mode: String,
    #[serde(default)]
    pub health_grace: Option<String>,
}

fn default_rollback_mode() -> String {
    "manual".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub api_keys: Vec<ApiKey>,
    pub oidc: OidcConfig,
    pub webhook_secret: String,
    pub gh_api_token: String,
    pub gh_deploy_key_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKey {
    pub name: String,
    pub hash: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub jwks_url: String,
    pub audience: String,
    pub allowed_repos: Vec<String>,
    pub allowed_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DnsConfig {
    pub provider: String,
    #[serde(flatten)]
    pub providers: serde_yaml::Value,
}

pub fn load_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> crate::Result<T> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| crate::DeployError::Config(format!("read {}: {e}", path.display())))?;
    let expanded = shellexpand::env(&raw)
        .map_err(|e| crate::DeployError::Config(format!("env expand {}: {e}", path.display())))?;
    serde_yaml::from_str(&expanded)
        .map_err(|e| crate::DeployError::Config(format!("parse {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn loads_config_with_env_substitution() {
        std::env::set_var("PPT_TEST_REPO", "git@github.com:test/repo.git");
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "bind: 0.0.0.0:8443").unwrap();
        writeln!(f, "state_dir: /var/lib/ppt-deploy").unwrap();
        writeln!(f, "worktree_dir: /var/lib/ppt-deploy/worktrees").unwrap();
        writeln!(f, "snapshot_dir: /var/lib/ppt-deploy/snapshots").unwrap();
        writeln!(f, "idle_pause_seconds: 1800").unwrap();
        writeln!(f, "idle_stop_seconds: 86400").unwrap();
        writeln!(f, "git_repo_url: ${{PPT_TEST_REPO}}").unwrap();
        let cfg: Config = load_yaml(&path).unwrap();
        assert_eq!(cfg.bind, "0.0.0.0:8443");
        assert_eq!(cfg.git_repo_url, "git@github.com:test/repo.git");
        assert_eq!(cfg.default_ttl_seconds, 172_800);
    }
}
