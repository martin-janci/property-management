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

/// Sanitize a branch ref into a worktree-safe alias.
///
/// Replaces every non-ASCII-alphanumeric character with `-`, **collapses runs
/// of `-`** into a single dash, trims leading/trailing dashes, and lowercases.
/// Collapsing matters because `validate_branch_strict` allows both `/` and `_`,
/// so a branch like `feature/_x` would otherwise sanitize to `feature--x`,
/// which doesn't match the worktree-name shape that the frontend tooling
/// produces.
pub fn sanitize(branch: &str) -> String {
    let mut out = String::with_capacity(branch.len());
    let mut last_was_dash = false;
    for c in branch.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
        // else: skip — collapse run of dashes
    }
    out.trim_matches('-').to_string()
}

/// Strict validator for inputs that flow into shell-out commands or SQL.
///
/// Allowed characters: alphanumeric, `-`, `_`, `.`, `/`.
///
/// Rejected:
/// - Empty string (git fetch with no branch is meaningless and a likely bug).
/// - Leading `-` (would be parsed as a git option flag — option injection).
/// - Leading `.` (git refuses refs starting with `.`; rejecting here gives a
///   clearer error than letting `git fetch` fail mid-flight).
/// - Branches with no ASCII-alphanumeric characters (e.g. `__`, `///`,
///   `_-_`). Such inputs sanitize to an empty string, which would make
///   `worktree_dir.join(sanitize(branch))` collapse to the worktrees root
///   and clone into the wrong place.
///
/// Returns the input unchanged if valid; `BadRequest` if not.
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
    // Defense in depth: reject branches that sanitize to an empty string
    // (i.e. contain no ASCII-alphanumeric characters at all). Without this,
    // `worktree_dir.join(sanitize("__"))` becomes `worktree_dir.join("")` =
    // `worktree_dir` itself, and the subsequent `git clone` would land in
    // the worktrees root rather than a per-branch subdirectory.
    if !branch.chars().any(|c| c.is_ascii_alphanumeric()) {
        return Err(crate::DeployError::BadRequest(format!(
            "branch must contain at least one alphanumeric character: {branch:?}"
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
        // Adjacent separators (e.g. `/_`) are now collapsed to a single dash so
        // the alias matches what the frontend worktree sanitizer produces.
        assert_eq!(sanitize("feature/_x"), "feature-x");
        assert_eq!(sanitize("a//b__c"), "a-b-c");
        assert_eq!(sanitize("--leading-and-trailing--"), "leading-and-trailing");
    }

    #[test]
    fn validate_branch_rejects_empty_sanitized() {
        // All-separator branches sanitize to an empty string. validate_branch_strict
        // must reject them BEFORE GitFetcher::fetch_branch joins the empty alias
        // onto worktree_dir and clones into the root directory.
        assert!(validate_branch_strict("").is_err());
        assert!(validate_branch_strict("__").is_err());
        assert!(validate_branch_strict("///").is_err());
        assert!(validate_branch_strict("_-_").is_err());
        assert!(validate_branch_strict("...").is_err());
        // Leading `-` and `.` are still rejected for the original reasons.
        assert!(validate_branch_strict("-x").is_err());
        assert!(validate_branch_strict(".x").is_err());
        // Healthy branch names still pass.
        assert!(validate_branch_strict("feature/UC-14").is_ok());
        assert!(validate_branch_strict("main").is_ok());
        assert!(validate_branch_strict("hotfix/critical_fix").is_ok());
    }

    #[tokio::test]
    async fn fetch_with_local_repo_fixture() {
        // Create a tiny local bare repo + clone, simulating origin.
        // Each git invocation is checked via `assert_git_ok`: a non-zero exit
        // (rejected commit, missing config, etc.) panics with stderr instead
        // of silently leaving a broken fixture for the actual test to trip
        // over much later.
        let tmp = tempfile::tempdir().unwrap();
        let bare = tmp.path().join("origin.git");
        let work = tmp.path().join("seed");
        std::fs::create_dir_all(&bare).unwrap();
        let work_str = work.to_str().unwrap();
        let bare_str = bare.to_str().unwrap();
        assert_git_ok(&["init", "--bare", bare_str]);
        assert_git_ok(&["init", work_str]);
        assert_git_ok(&["-C", work_str, "config", "user.email", "t@t"]);
        assert_git_ok(&["-C", work_str, "config", "user.name", "t"]);
        std::fs::write(work.join("README.md"), "hi").unwrap();
        assert_git_ok(&["-C", work_str, "add", "."]);
        assert_git_ok(&["-C", work_str, "commit", "-m", "init"]);
        assert_git_ok(&["-C", work_str, "branch", "-M", "feature-x"]);
        assert_git_ok(&["-C", work_str, "remote", "add", "origin", bare_str]);
        assert_git_ok(&["-C", work_str, "push", "-u", "origin", "feature-x"]);

        let dest_root = tmp.path().join("worktrees");
        let fetcher = GitFetcher::new(
            bare.to_string_lossy().to_string(),
            dest_root.clone(),
            "/dev/null",
        );
        let dest = fetcher.fetch_branch("feature-x").await.unwrap();
        assert!(dest.join("README.md").exists());
    }

    fn assert_git_ok(args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn git {args:?}: {e}"));
        if !output.status.success() {
            panic!(
                "git {args:?} failed with status {:?}\n--- stderr ---\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr),
            );
        }
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
