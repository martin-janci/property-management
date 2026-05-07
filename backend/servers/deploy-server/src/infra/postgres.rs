// backend/servers/deploy-server/src/infra/postgres.rs
use crate::Result;
use std::path::Path;
use tokio::process::Command;

#[derive(Clone)]
pub struct PostgresOps {
    pub admin_url: String,   // postgres://user:pass@host:5432/postgres (admin DB)
    pub template_db: String, // "ppt_dev_template"
    pub user_db_prefix: String, // "ppt_wt_"
}

impl PostgresOps {
    /// Build a connection URL pointing at `db_name` while preserving the original
    /// admin URL's user/host/port/query-string. Replaces only the path component to
    /// avoid the failure mode where `replace("/postgres", ...)` would mangle a URL
    /// whose username or host happened to contain the literal `/postgres`.
    fn url_for(&self, db_name: &str) -> Result<String> {
        let mut url = url::Url::parse(&self.admin_url)
            .map_err(|e| crate::DeployError::Config(format!("bad postgres admin_url: {e}")))?;
        url.set_path(&format!("/{db_name}"));
        Ok(url.into())
    }

    pub async fn create_from_template(&self, db_name: &str) -> Result<()> {
        run_psql(
            &self.admin_url,
            &format!(
                "CREATE DATABASE \"{db_name}\" TEMPLATE \"{}\"",
                self.template_db
            ),
        )
        .await
    }

    pub async fn drop_db(&self, db_name: &str) -> Result<()> {
        let _ = run_psql(
            &self.admin_url,
            &format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db_name}'"
            ),
        )
        .await;
        run_psql(
            &self.admin_url,
            &format!("DROP DATABASE IF EXISTS \"{db_name}\""),
        )
        .await
    }

    pub async fn dump(&self, db_name: &str, out: &Path) -> Result<()> {
        let url = self.url_for(db_name)?;
        let status = Command::new("pg_dump")
            .args(["-Fc", "-f", out.to_str().unwrap(), "--no-owner", "--no-acl"])
            .arg(&url)
            .status()
            .await?;
        if !status.success() {
            return Err(crate::DeployError::Internal(format!(
                "pg_dump failed for {db_name}"
            )));
        }
        Ok(())
    }

    pub async fn restore(&self, db_name: &str, dump: &Path) -> Result<()> {
        // Drop any partial state from a previous failed restore (#12). Best-effort —
        // ignore errors (DB may not exist).
        let _ = self.drop_db(db_name).await;
        self.create_from_template(db_name).await?;
        let url = self.url_for(db_name)?;
        let status = Command::new("pg_restore")
            .args([
                "-d",
                &url,
                "--no-owner",
                "--no-acl",
                "--clean",
                "--if-exists",
            ])
            .arg(dump)
            .status()
            .await?;
        if !status.success() {
            return Err(crate::DeployError::Internal(format!(
                "pg_restore failed for {db_name}"
            )));
        }
        Ok(())
    }
}

async fn run_psql(url: &str, sql: &str) -> Result<()> {
    let status = Command::new("psql").args(["-c", sql, url]).status().await?;
    if !status.success() {
        return Err(crate::DeployError::Internal(format!("psql failed: {sql}")));
    }
    Ok(())
}
