use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Converts to lowercase, replaces non-alphanumeric chars (except `-`) with hyphens,
/// and trims leading/trailing hyphens.
pub fn sanitize_branch_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Creates worktree at `{project_base_path}/.worktrees/{sanitized_name}`
/// with branch `wt/{sanitized_name}` from the repo's current HEAD.
pub async fn create_worktree(
    repo_path: &Path,
    worktree_name: &str,
    project_base_path: &Path,
) -> Result<PathBuf> {
    let sanitized = sanitize_branch_name(worktree_name);
    let branch_name = format!("wt/{}", sanitized);
    let worktree_dir = project_base_path.join(".worktrees").join(&sanitized);

    debug!(
        repo = %repo_path.display(),
        worktree = %worktree_dir.display(),
        branch = %branch_name,
        "Creating git worktree"
    );

    tokio::fs::create_dir_all(project_base_path.join(".worktrees"))
        .await
        .map_err(|e| anyhow!("Failed to create .worktrees directory: {}", e))?;

    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            &worktree_dir.to_string_lossy(),
            "-b",
            &branch_name,
        ])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| anyhow!("Failed to run git worktree add: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("git worktree add failed: {}", stderr.trim()));
    }

    debug!(worktree = %worktree_dir.display(), "Worktree created successfully");
    Ok(worktree_dir)
}

/// Force-removes a worktree and prunes stale entries. Tolerates already-removed worktrees.
pub async fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    debug!(
        repo = %repo_path.display(),
        worktree = %worktree_path.display(),
        "Removing git worktree"
    );

    let output = Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| anyhow!("Failed to run git worktree remove: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("is not a working tree") && !stderr.contains("No such file") {
            return Err(anyhow!("git worktree remove failed: {}", stderr.trim()));
        }
        debug!("Worktree already removed or not found, continuing");
    }

    let _ = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(repo_path)
        .output()
        .await;

    debug!("Worktree removed and pruned");
    Ok(())
}

/// Force-deletes a branch. Tolerates branches that don't exist.
pub async fn delete_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    debug!(
        repo = %repo_path.display(),
        branch = %branch_name,
        "Deleting git branch"
    );

    let output = Command::new("git")
        .args(["branch", "-D", branch_name])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| anyhow!("Failed to run git branch -D: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("not found") {
            return Err(anyhow!("git branch -D failed: {}", stderr.trim()));
        }
        debug!("Branch not found, already deleted");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .await
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .await
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .await
            .unwrap();

        tokio::fs::write(path.join("README.md"), "# Test")
            .await
            .unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .await
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(path)
            .output()
            .await
            .unwrap();

        dir
    }

    #[test]
    fn test_is_git_repo_true() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert!(is_git_repo(dir.path()));
    }

    #[test]
    fn test_is_git_repo_false() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(!is_git_repo(dir.path()));
    }

    #[test]
    fn test_sanitize_branch_name_spaces() {
        assert_eq!(sanitize_branch_name("my topic name"), "my-topic-name");
    }

    #[test]
    fn test_sanitize_branch_name_uppercase() {
        assert_eq!(sanitize_branch_name("MyTopic"), "mytopic");
    }

    #[test]
    fn test_sanitize_branch_name_special_chars() {
        assert_eq!(sanitize_branch_name("feat/my.topic@v2"), "feat-my-topic-v2");
    }

    #[test]
    fn test_sanitize_branch_name_leading_trailing() {
        assert_eq!(sanitize_branch_name("--my-topic--"), "my-topic");
    }

    #[tokio::test]
    async fn test_create_worktree_success() {
        let repo_dir = create_test_repo().await;
        let base_dir = TempDir::new().unwrap();

        let result = create_worktree(repo_dir.path(), "test-topic", base_dir.path()).await;
        assert!(result.is_ok());

        let wt_path = result.unwrap();
        assert!(wt_path.exists());
        assert!(wt_path.join("README.md").exists());
    }

    #[tokio::test]
    async fn test_create_worktree_branch_name() {
        let repo_dir = create_test_repo().await;
        let base_dir = TempDir::new().unwrap();

        create_worktree(repo_dir.path(), "My Topic", base_dir.path())
            .await
            .unwrap();

        let output = Command::new("git")
            .args(["branch", "--list", "wt/my-topic"])
            .current_dir(repo_dir.path())
            .output()
            .await
            .unwrap();
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(branches.contains("wt/my-topic"));
    }

    #[tokio::test]
    async fn test_create_worktree_duplicate_branch_fails() {
        let repo_dir = create_test_repo().await;
        let base_dir = TempDir::new().unwrap();

        create_worktree(repo_dir.path(), "dup-topic", base_dir.path())
            .await
            .unwrap();

        let result = create_worktree(repo_dir.path(), "dup-topic", base_dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_worktree_no_commits_fails() {
        let dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .await
            .unwrap();

        let base_dir = TempDir::new().unwrap();
        let result = create_worktree(dir.path(), "test", base_dir.path()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_worktree_success() {
        let repo_dir = create_test_repo().await;
        let base_dir = TempDir::new().unwrap();

        let wt_path = create_worktree(repo_dir.path(), "rm-topic", base_dir.path())
            .await
            .unwrap();
        assert!(wt_path.exists());

        let result = remove_worktree(repo_dir.path(), &wt_path).await;
        assert!(result.is_ok());
        assert!(!wt_path.exists());
    }

    #[tokio::test]
    async fn test_remove_worktree_already_deleted() {
        let repo_dir = create_test_repo().await;
        let base_dir = TempDir::new().unwrap();

        let wt_path = create_worktree(repo_dir.path(), "gone-topic", base_dir.path())
            .await
            .unwrap();

        tokio::fs::remove_dir_all(&wt_path).await.unwrap();

        let result = remove_worktree(repo_dir.path(), &wt_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_branch_success() {
        let repo_dir = create_test_repo().await;
        let base_dir = TempDir::new().unwrap();

        let wt_path = create_worktree(repo_dir.path(), "del-branch", base_dir.path())
            .await
            .unwrap();
        remove_worktree(repo_dir.path(), &wt_path).await.unwrap();

        let result = delete_branch(repo_dir.path(), "wt/del-branch").await;
        assert!(result.is_ok());

        let output = Command::new("git")
            .args(["branch", "--list", "wt/del-branch"])
            .current_dir(repo_dir.path())
            .output()
            .await
            .unwrap();
        let branches = String::from_utf8_lossy(&output.stdout);
        assert!(!branches.contains("wt/del-branch"));
    }
}
