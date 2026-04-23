use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use gittriage_core::{normalize_remote_url, CloneRecord, UpstreamTracking};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
    pub normalized_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct GitMetadata {
    pub head_oid: Option<String>,
    pub active_branch: Option<String>,
    pub default_branch: Option<String>,
    pub is_dirty: bool,
    pub last_commit_at: Option<DateTime<Utc>>,
    pub remotes: Vec<GitRemote>,
    pub upstream_tracking: Option<UpstreamTracking>,
}

pub fn enrich_clone(path: &Path, clone: &mut CloneRecord) -> Result<Vec<GitRemote>> {
    if !clone.is_git {
        return Ok(Vec::new());
    }

    let meta = read_git_metadata(path)?;
    clone.head_oid = meta.head_oid;
    clone.active_branch = meta.active_branch;
    clone.default_branch = meta.default_branch;
    clone.is_dirty = meta.is_dirty;
    clone.last_commit_at = meta.last_commit_at;
    clone.upstream_tracking = meta.upstream_tracking;
    Ok(meta.remotes)
}

pub fn read_git_metadata(path: &Path) -> Result<GitMetadata> {
    if !path.join(".git").exists() {
        return Err(anyhow!("not a git repo: {}", path.display()));
    }

    let head_oid = run_git(path, ["rev-parse", "HEAD"]).ok();
    let active_branch = run_git(path, ["branch", "--show-current"]).ok();
    let is_dirty = !run_git(path, ["status", "--porcelain"])
        .unwrap_or_default()
        .trim()
        .is_empty();

    let last_commit_at = run_git(path, ["log", "-1", "--format=%cI"])
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let default_branch = run_git(path, ["symbolic-ref", "refs/remotes/origin/HEAD"])
        .ok()
        .and_then(|s| s.rsplit('/').next().map(|v| v.trim().to_string()));

    let upstream_tracking = read_upstream_tracking(path).ok();
    let remotes = parse_remotes(path)?;

    Ok(GitMetadata {
        head_oid,
        active_branch,
        default_branch,
        is_dirty,
        last_commit_at,
        remotes,
        upstream_tracking,
    })
}

pub fn read_upstream_tracking(path: &Path) -> Result<UpstreamTracking> {
    if !path.join(".git").exists() {
        return Err(anyhow!("not a git repo: {}", path.display()));
    }

    let mut active_branch = run_git(path, ["branch", "--show-current"]).unwrap_or_default();
    if active_branch.trim().is_empty() {
        active_branch = run_git(path, ["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    }
    if active_branch.trim() == "HEAD" || active_branch.trim().is_empty() {
        return Ok(UpstreamTracking {
            upstream_branch: None,
            ahead_count: 0,
            behind_count: 0,
            no_upstream_configured: true,
            upstream_resolution_error: None,
        });
    }

    let branch_remote = run_git(
        path,
        ["config", "--get", &format!("branch.{active_branch}.remote")],
    )
    .ok();
    let branch_merge = run_git(
        path,
        ["config", "--get", &format!("branch.{active_branch}.merge")],
    )
    .ok();
    let Some(remote) = branch_remote else {
        return Ok(UpstreamTracking {
            upstream_branch: None,
            ahead_count: 0,
            behind_count: 0,
            no_upstream_configured: true,
            upstream_resolution_error: None,
        });
    };
    let Some(merge_ref) = branch_merge else {
        return Ok(UpstreamTracking {
            upstream_branch: None,
            ahead_count: 0,
            behind_count: 0,
            no_upstream_configured: true,
            upstream_resolution_error: None,
        });
    };
    let merge_short = merge_ref
        .trim()
        .strip_prefix("refs/heads/")
        .unwrap_or(merge_ref.trim());
    let fallback_branch = format!("{}/{}", remote.trim(), merge_short);
    let resolved_upstream = run_git(
        path,
        ["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .ok()
    .filter(|s| !s.is_empty());
    let upstream_branch = resolved_upstream
        .clone()
        .or_else(|| Some(fallback_branch.clone()));

    let mut ahead_count = 0;
    let mut behind_count = 0;
    let mut upstream_resolution_error = None;
    if let Some(resolved_ref) = resolved_upstream {
        let revspec = format!("{resolved_ref}...HEAD");
        match run_git(path, ["rev-list", "--left-right", "--count", &revspec]) {
            Ok(counts) => {
                let mut parts = counts.split_whitespace();
                behind_count = parts
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);
                ahead_count = parts
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);
            }
            Err(err) => {
                upstream_resolution_error = Some(err.to_string());
            }
        }
    } else {
        upstream_resolution_error =
            Some("upstream is configured but could not be resolved via `@{u}`".to_string());
    }

    Ok(UpstreamTracking {
        upstream_branch,
        ahead_count,
        behind_count,
        no_upstream_configured: false,
        upstream_resolution_error,
    })
}

fn parse_remotes(path: &Path) -> Result<Vec<GitRemote>> {
    let output = run_git(path, ["remote", "-v"])?;
    let mut remotes = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let name = parts[0].to_string();
        let url = parts[1].to_string();

        if remotes
            .iter()
            .any(|r: &GitRemote| r.name == name && r.url == url)
        {
            continue;
        }

        remotes.push(GitRemote {
            name,
            normalized_url: normalize_remote_url(&url),
            url,
        });
    }

    Ok(remotes)
}

#[derive(Debug, Clone)]
pub struct MergeBaseHint {
    pub other_head: String,
    /// The other repo's HEAD exists as a commit object in `repo_a`.
    pub objects_shared: bool,
    pub merge_base_oid: Option<String>,
    pub detail: String,
}

/// Best-effort merge-base between two **local** clones.  
/// Computes `git merge-base HEAD b` inside `repo_a` when `b`'s `HEAD` is present in `repo_a`'s object database.
pub fn merge_base_between_local_clones(repo_a: &Path, repo_b: &Path) -> Result<MergeBaseHint> {
    if !repo_a.join(".git").exists() || !repo_b.join(".git").exists() {
        anyhow::bail!("both paths must be git repositories");
    }

    let other_head = run_git(repo_b, ["rev-parse", "HEAD"])?;
    let spec = format!("{other_head}^{{commit}}");

    let in_a = Command::new("git")
        .arg("-C")
        .arg(repo_a)
        .args(["cat-file", "-e", &spec])
        .output()
        .context("git cat-file")?
        .status
        .success();

    if !in_a {
        return Ok(MergeBaseHint {
            other_head,
            objects_shared: false,
            merge_base_oid: None,
            detail: format!(
                "HEAD of {} is not in object database of {}; merge-base skipped",
                repo_b.display(),
                repo_a.display()
            ),
        });
    }

    let out = Command::new("git")
        .arg("-C")
        .arg(repo_a)
        .args(["merge-base", "HEAD", &other_head])
        .output()
        .context("git merge-base")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Ok(MergeBaseHint {
            other_head,
            objects_shared: true,
            merge_base_oid: None,
            detail: format!("objects overlap but merge-base failed: {}", stderr.trim()),
        });
    }

    let mb = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let detail = format!(
        "merge-base between {} and HEAD of {} ({}) is {}",
        repo_a.display(),
        repo_b.display(),
        other_head,
        mb
    );
    Ok(MergeBaseHint {
        other_head,
        objects_shared: true,
        merge_base_oid: Some(mb),
        detail,
    })
}

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git in {}", path.display()))?;

    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::{enrich_clone, read_upstream_tracking};
    use anyhow::{Context, Result};
    use gittriage_core::CloneRecord;
    use std::path::Path;
    use std::process::Command;

    fn run(path: &Path, args: &[&str]) -> Result<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .context("git command failed to spawn")?;
        if !out.status.success() {
            anyhow::bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn init_repo(path: &Path) -> Result<()> {
        run(path, &["init", "-b", "main"])?;
        set_identity(path)?;
        Ok(())
    }

    fn set_identity(path: &Path) -> Result<()> {
        run(path, &["config", "user.email", "dev@example.com"])?;
        run(path, &["config", "user.name", "Dev"])?;
        Ok(())
    }

    fn commit_file(path: &Path, rel: &str, content: &str, msg: &str) -> Result<()> {
        std::fs::write(path.join(rel), content)?;
        run(path, &["add", rel])?;
        run(path, &["commit", "-m", msg])?;
        Ok(())
    }

    #[test]
    fn upstream_tracking_with_upstream_reports_ahead_behind() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let remote = tmp.path().join("remote.git");
        let origin_seed = tmp.path().join("origin-seed");
        let local = tmp.path().join("local");

        run(
            tmp.path(),
            &["init", "--bare", remote.to_str().expect("utf8 path")],
        )?;

        std::fs::create_dir_all(&origin_seed)?;
        init_repo(&origin_seed)?;
        commit_file(&origin_seed, "seed.txt", "seed", "seed")?;
        run(
            &origin_seed,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("utf8 path"),
            ],
        )?;
        run(&origin_seed, &["push", "-u", "origin", "main"])?;
        run(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"])?;

        run(
            tmp.path(),
            &[
                "clone",
                remote.to_str().expect("utf8 path"),
                local.to_str().expect("utf8 path"),
            ],
        )?;
        set_identity(&local)?;
        commit_file(&local, "local.txt", "local", "local change")?;

        commit_file(&origin_seed, "remote.txt", "remote", "remote change")?;
        run(&origin_seed, &["push"])?;
        run(&local, &["fetch", "origin"])?;

        let tracking = read_upstream_tracking(&local)?;
        assert_eq!(tracking.upstream_branch.as_deref(), Some("origin/main"));
        assert_eq!(tracking.ahead_count, 1);
        assert_eq!(tracking.behind_count, 1);
        assert!(!tracking.no_upstream_configured);
        assert_eq!(tracking.upstream_resolution_error, None);
        Ok(())
    }

    #[test]
    fn enrich_clone_populates_upstream_tracking() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let remote = tmp.path().join("remote.git");
        let origin_seed = tmp.path().join("origin-seed");
        let local = tmp.path().join("local");

        run(
            tmp.path(),
            &["init", "--bare", remote.to_str().expect("utf8 path")],
        )?;
        std::fs::create_dir_all(&origin_seed)?;
        init_repo(&origin_seed)?;
        commit_file(&origin_seed, "seed.txt", "seed", "seed")?;
        run(
            &origin_seed,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("utf8 path"),
            ],
        )?;
        run(&origin_seed, &["push", "-u", "origin", "main"])?;
        run(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"])?;

        run(
            tmp.path(),
            &[
                "clone",
                remote.to_str().expect("utf8 path"),
                local.to_str().expect("utf8 path"),
            ],
        )?;
        let mut clone = CloneRecord {
            id: "clone-1".into(),
            path: local.to_string_lossy().to_string(),
            display_name: "local".into(),
            is_git: true,
            head_oid: None,
            active_branch: None,
            default_branch: None,
            is_dirty: false,
            last_commit_at: None,
            upstream_tracking: None,
            size_bytes: None,
            manifest_kind: None,
            readme_title: None,
            license_spdx: None,
            fingerprint: None,
            has_lockfile: false,
            has_ci: false,
            has_tests_dir: false,
        };

        let remotes = enrich_clone(&local, &mut clone)?;
        assert!(!remotes.is_empty());
        let tracking = clone
            .upstream_tracking
            .as_ref()
            .expect("upstream tracking populated");
        assert_eq!(tracking.upstream_branch.as_deref(), Some("origin/main"));
        assert_eq!(tracking.ahead_count, 0);
        assert_eq!(tracking.behind_count, 0);
        Ok(())
    }

    #[test]
    fn upstream_tracking_without_upstream_is_non_fatal() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        init_repo(tmp.path())?;
        commit_file(tmp.path(), "a.txt", "a", "init")?;

        let tracking = read_upstream_tracking(tmp.path())?;
        assert!(tracking.no_upstream_configured);
        assert_eq!(tracking.upstream_branch, None);
        assert_eq!(tracking.ahead_count, 0);
        assert_eq!(tracking.behind_count, 0);
        assert_eq!(tracking.upstream_resolution_error, None);
        Ok(())
    }

    #[test]
    fn upstream_tracking_detached_head_without_upstream() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        init_repo(tmp.path())?;
        commit_file(tmp.path(), "a.txt", "a", "init")?;
        let head = run(tmp.path(), &["rev-parse", "HEAD"])?;
        run(tmp.path(), &["checkout", "--detach", &head])?;

        let tracking = read_upstream_tracking(tmp.path())?;
        assert!(tracking.no_upstream_configured);
        assert_eq!(tracking.upstream_branch, None);
        assert_eq!(tracking.upstream_resolution_error, None);
        Ok(())
    }

    #[test]
    fn upstream_tracking_reports_broken_tracking_ref() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let remote = tmp.path().join("remote.git");
        let origin_seed = tmp.path().join("origin-seed");
        let local = tmp.path().join("local");

        run(
            tmp.path(),
            &["init", "--bare", remote.to_str().expect("utf8 path")],
        )?;
        std::fs::create_dir_all(&origin_seed)?;
        init_repo(&origin_seed)?;
        commit_file(&origin_seed, "seed.txt", "seed", "seed")?;
        run(
            &origin_seed,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("utf8 path"),
            ],
        )?;
        run(&origin_seed, &["push", "-u", "origin", "main"])?;
        run(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
        run(
            tmp.path(),
            &[
                "clone",
                remote.to_str().expect("utf8 path"),
                local.to_str().expect("utf8 path"),
            ],
        )?;

        run(&local, &["update-ref", "-d", "refs/remotes/origin/main"])?;
        let tracking = read_upstream_tracking(&local)?;
        assert!(!tracking.no_upstream_configured);
        assert_eq!(tracking.upstream_branch.as_deref(), Some("origin/main"));
        assert_eq!(tracking.ahead_count, 0);
        assert_eq!(tracking.behind_count, 0);
        assert!(tracking.upstream_resolution_error.is_some());
        Ok(())
    }

    #[test]
    fn upstream_tracking_with_local_branch_upstream_uses_resolved_ref() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        init_repo(tmp.path())?;
        commit_file(tmp.path(), "seed.txt", "seed", "seed")?;
        run(tmp.path(), &["checkout", "-b", "base"])?;
        commit_file(tmp.path(), "base.txt", "base", "base commit")?;
        run(tmp.path(), &["checkout", "-b", "feature"])?;
        run(tmp.path(), &["branch", "--set-upstream-to=base", "feature"])?;
        commit_file(tmp.path(), "feature.txt", "feature", "feature commit")?;

        let tracking = read_upstream_tracking(tmp.path())?;
        assert_eq!(tracking.upstream_branch.as_deref(), Some("base"));
        assert_eq!(tracking.ahead_count, 1);
        assert_eq!(tracking.behind_count, 0);
        assert!(!tracking.no_upstream_configured);
        assert_eq!(tracking.upstream_resolution_error, None);
        Ok(())
    }

    #[test]
    fn upstream_tracking_unborn_head_reads_branch_config() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        run(tmp.path(), &["init", "-b", "main"])?;
        run(tmp.path(), &["config", "branch.main.remote", "origin"])?;
        run(
            tmp.path(),
            &["config", "branch.main.merge", "refs/heads/main"],
        )?;

        let tracking = read_upstream_tracking(tmp.path())?;
        assert!(!tracking.no_upstream_configured);
        assert_eq!(tracking.upstream_branch.as_deref(), Some("origin/main"));
        assert_eq!(tracking.ahead_count, 0);
        assert_eq!(tracking.behind_count, 0);
        assert!(tracking.upstream_resolution_error.is_some());
        Ok(())
    }
}
