// backend/servers/deploy-server/src/infra/git.rs
use crate::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct GitFetcher {
    repo_url: String,
    worktree_dir: PathBuf,
    deploy_key_path: PathBuf,
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl GitFetcher {
    pub fn new(
        repo_url: impl Into<String>,
        worktree_dir: impl Into<PathBuf>,
        deploy_key_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repo_url: repo_url.into(),
            worktree_dir: worktree_dir.into(),
            deploy_key_path: deploy_key_path.into(),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Fetch a branch into `<worktree_dir>/<sanitized_branch>/`.
    /// Per-branch lock prevents two concurrent calls for the same branch racing.
    pub async fn fetch_branch(&self, branch: &str) -> Result<PathBuf> {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(branch.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;

        let dest = self.worktree_dir.join(sanitize(branch));
        let ssh_cmd = format!(
            "ssh -i {} -o StrictHostKeyChecking=accept-new",
            self.deploy_key_path.display()
        );

        if dest.join(".git").exists() {
            self.run_git(&dest, &ssh_cmd, &["fetch", "origin", branch])
                .await?;
            self.run_git(
                &dest,
                &ssh_cmd,
                &["reset", "--hard", &format!("origin/{branch}")],
            )
            .await?;
        } else {
            tokio::fs::create_dir_all(&self.worktree_dir).await?;
            self.run_git_in(
                &self.worktree_dir,
                &ssh_cmd,
                &[
                    "clone",
                    "--branch",
                    branch,
                    "--depth",
                    "1",
                    &self.repo_url,
                    dest.to_str().unwrap(),
                ],
            )
            .await?;
        }
        Ok(dest)
    }

    async fn run_git(&self, cwd: &Path, ssh_cmd: &str, args: &[&str]) -> Result<()> {
        self.run_git_in(cwd, ssh_cmd, args).await
    }

    async fn run_git_in(&self, cwd: &Path, ssh_cmd: &str, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_SSH_COMMAND", ssh_cmd)
            .output()
            .await?;
        if !output.status.success() {
            return Err(crate::DeployError::Internal(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}

pub fn sanitize(branch: &str) -> String {
    branch
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

/// Strict validator for inputs that flow into shell-out commands or SQL.
/// Allows: alphanumeric, `-`, `_`, `.`, `/`. Rejects leading `-` (git option flag).
/// Returns the input unchanged if valid; Err if not.
pub fn validate_branch_strict(branch: &str) -> crate::Result<&str> {
    if branch.is_empty() || branch.starts_with('-') || branch.starts_with('.') {
        return Err(crate::DeployError::BadRequest(format!(
            "invalid branch name: {branch:?}"
        )));
    }
    if !branch
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
    {
        return Err(crate::DeployError::BadRequest(format!(
            "branch contains disallowed characters: {branch:?}"
        )));
    }
    Ok(branch)
}

/// Strict validator for `alias` and database names.
/// Allows: alphanumeric, `-`, `_`. No path separators, no leading dash.
pub fn validate_alias_strict(alias: &str) -> crate::Result<&str> {
    if alias.is_empty() || alias.starts_with('-') {
        return Err(crate::DeployError::BadRequest(format!(
            "invalid alias: {alias:?}"
        )));
    }
    if alias.len() > 30 {
        return Err(crate::DeployError::BadRequest(
            "alias too long (max 30 chars)".into(),
        ));
    }
    if !alias
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(crate::DeployError::BadRequest(format!(
            "alias contains disallowed characters: {alias:?}"
        )));
    }
    Ok(alias)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_branch_to_subdomain() {
        assert_eq!(sanitize("feature/UC-14"), "feature-uc-14");
        assert_eq!(sanitize("hotfix/Critical Fix"), "hotfix-critical-fix");
        assert_eq!(sanitize("///---bad---///"), "bad");
    }

    #[tokio::test]
    async fn fetch_with_local_repo_fixture() {
        // Create a tiny local bare repo + clone, simulating origin.
        let tmp = tempfile::tempdir().unwrap();
        let bare = tmp.path().join("origin.git");
        let work = tmp.path().join("seed");
        std::fs::create_dir_all(&bare).unwrap();
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .arg(&bare)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .arg(&work)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", work.to_str().unwrap(), "config", "user.email", "t@t"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", work.to_str().unwrap(), "config", "user.name", "t"])
            .status()
            .unwrap();
        std::fs::write(work.join("README.md"), "hi").unwrap();
        std::process::Command::new("git")
            .args(["-C", work.to_str().unwrap(), "add", "."])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", work.to_str().unwrap(), "commit", "-m", "init"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", work.to_str().unwrap(), "branch", "-M", "feature-x"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args(["-C", work.to_str().unwrap(), "remote", "add", "origin"])
            .arg(&bare)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-C",
                work.to_str().unwrap(),
                "push",
                "-u",
                "origin",
                "feature-x",
            ])
            .status()
            .unwrap();

        let dest_root = tmp.path().join("worktrees");
        let fetcher = GitFetcher::new(
            bare.to_string_lossy().to_string(),
            dest_root.clone(),
            "/dev/null",
        );
        let dest = fetcher.fetch_branch("feature-x").await.unwrap();
        assert!(dest.join("README.md").exists());
    }
}

#[cfg(test)]
mod strict_tests {
    use super::*;

    #[test]
    fn alias_strict_accepts_valid() {
        assert!(validate_alias_strict("feature-uc14").is_ok());
        assert!(validate_alias_strict("uc14").is_ok());
        assert!(validate_alias_strict("a_b-c").is_ok());
    }

    #[test]
    fn alias_strict_rejects_sql_injection() {
        assert!(validate_alias_strict("a\";DROP DATABASE \"x\";--").is_err());
        assert!(validate_alias_strict("a/b").is_err());
        assert!(validate_alias_strict("").is_err());
        assert!(validate_alias_strict("-foo").is_err());
        assert!(validate_alias_strict(&"a".repeat(31)).is_err());
    }

    #[test]
    fn branch_strict_accepts_valid() {
        assert!(validate_branch_strict("main").is_ok());
        assert!(validate_branch_strict("feature/UC-14").is_ok());
        assert!(validate_branch_strict("hotfix/v1.2.3").is_ok());
    }

    #[test]
    fn branch_strict_rejects_injection() {
        assert!(validate_branch_strict("--upload-pack=evil").is_err());
        assert!(validate_branch_strict("-foo").is_err());
        assert!(validate_branch_strict(".hidden").is_err());
        assert!(validate_branch_strict("a;rm -rf /").is_err());
        assert!(validate_branch_strict("").is_err());
    }
}
