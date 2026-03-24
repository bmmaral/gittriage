use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use nexus_core::{normalize_remote_url, RemoteRecord};
use serde::Deserialize;
use std::process::Command;
use which::which;

#[derive(Debug, Deserialize)]
struct GhRepo {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    url: String,
    #[serde(rename = "isArchived")]
    is_archived: bool,
    #[serde(rename = "isFork")]
    is_fork: bool,
    #[serde(rename = "isPrivate")]
    is_private: bool,
    #[serde(rename = "pushedAt")]
    pushed_at: Option<String>,
    #[serde(rename = "defaultBranchRef")]
    default_branch_ref: Option<GhBranch>,
}

#[derive(Debug, Deserialize)]
struct GhBranch {
    name: String,
}

pub fn ensure_gh_installed() -> Result<()> {
    which("gh").context("gh CLI not found in PATH")?;
    Ok(())
}

pub fn ingest_owner(owner: &str) -> Result<Vec<RemoteRecord>> {
    ensure_gh_installed()?;

    let output = Command::new("gh")
        .args([
            "repo",
            "list",
            owner,
            "--limit",
            "500",
            "--json",
            "nameWithOwner,url,isArchived,isFork,isPrivate,pushedAt,defaultBranchRef",
        ])
        .output()
        .context("failed to execute gh repo list")?;

    if !output.status.success() {
        return Err(anyhow!(
            "gh repo list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let rows: Vec<GhRepo> =
        serde_json::from_slice(&output.stdout).context("failed to parse gh repo list JSON")?;

    let remotes = rows
        .into_iter()
        .map(|repo| {
            let parts: Vec<&str> = repo.name_with_owner.split('/').collect();
            let owner = parts.first().map(|v| (*v).to_string());
            let name = parts.get(1).map(|v| (*v).to_string());

            RemoteRecord {
                id: format!("remote-{}", uuid::Uuid::new_v4()),
                provider: "github".into(),
                owner,
                name,
                full_name: Some(repo.name_with_owner.clone()),
                normalized_url: normalize_remote_url(&repo.url),
                url: repo.url,
                default_branch: repo.default_branch_ref.map(|b| b.name),
                is_fork: repo.is_fork,
                is_archived: repo.is_archived,
                is_private: repo.is_private,
                pushed_at: repo
                    .pushed_at
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            }
        })
        .collect();

    Ok(remotes)
}
