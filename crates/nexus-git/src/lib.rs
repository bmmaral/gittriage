use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use nexus_core::{normalize_remote_url, CloneRecord};
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

    let remotes = parse_remotes(path)?;

    Ok(GitMetadata {
        head_oid,
        active_branch,
        default_branch,
        is_dirty,
        last_commit_at,
        remotes,
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
    Ok(MergeBaseHint {
        other_head,
        objects_shared: true,
        merge_base_oid: Some(mb.clone()),
        detail: format!(
            "merge-base between {} and HEAD of {} ({}) is {}",
            repo_a.display(),
            repo_b.display(),
            other_head,
            mb
        ),
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
