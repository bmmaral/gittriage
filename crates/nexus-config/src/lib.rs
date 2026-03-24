use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable pointing at a `nexus.toml` file. Highest precedence after explicit CLI `--config`.
pub const ENV_NEXUS_CONFIG: &str = "NEXUS_CONFIG";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub respect_gitignore: bool,
    pub max_readme_bytes: usize,
    pub max_hash_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    pub archive_duplicate_threshold: u8,
    pub oss_candidate_threshold: u8,
    pub ambiguous_cluster_threshold: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusConfig {
    pub db_path: PathBuf,
    pub default_roots: Vec<String>,
    pub github_owner: Option<String>,
    pub include_hidden: bool,
    pub scan: ScanConfig,
    pub planner: PlannerConfig,
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(".nexus/state.db"),
            default_roots: vec!["~/Projects".into()],
            github_owner: None,
            include_hidden: false,
            scan: ScanConfig {
                respect_gitignore: true,
                max_readme_bytes: 16 * 1024,
                max_hash_files: 64,
            },
            planner: PlannerConfig {
                archive_duplicate_threshold: 80,
                oss_candidate_threshold: 70,
                ambiguous_cluster_threshold: 55,
            },
        }
    }
}

/// Resolved configuration and where it came from.
#[derive(Debug, Clone)]
pub struct ConfigBundle {
    pub config: NexusConfig,
    /// TOML file that was loaded, if any.
    pub source_path: Option<PathBuf>,
    /// Absolute path used for SQLite (relative `db_path` entries are resolved from the process cwd).
    pub effective_db_path: PathBuf,
}

impl ConfigBundle {
    /// Load config using precedence:
    /// 1. `explicit` path from `--config` (must exist)
    /// 2. `NEXUS_CONFIG` env (must exist when set)
    /// 3. `./nexus.toml` under the current working directory
    /// 4. XDG config dir `nexus.toml` (`ProjectDirs::config_dir`)
    /// 5. Built-in defaults (no file)
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let (config, source_path) = load_layered(explicit)?;
        let effective_db_path = resolve_db_path(&config.db_path);
        Ok(Self {
            config,
            source_path,
            effective_db_path,
        })
    }
}

fn load_layered(explicit: Option<&Path>) -> Result<(NexusConfig, Option<PathBuf>)> {
    if let Some(path) = explicit {
        let path = path.to_path_buf();
        let cfg = read_config_file(&path)?;
        return Ok((cfg, Some(path)));
    }

    if let Ok(from_env) = env::var(ENV_NEXUS_CONFIG) {
        let path = PathBuf::from(&from_env);
        ensure_config_exists(&path)?;
        let cfg = read_config_file(&path)?;
        return Ok((cfg, Some(path)));
    }

    let cwd = env::current_dir().context("failed to resolve current directory")?;
    let local = cwd.join("nexus.toml");
    if local.exists() {
        let cfg = read_config_file(&local)?;
        return Ok((cfg, Some(local)));
    }

    if let Some(dirs) = ProjectDirs::from("org", "nexus", "nexus") {
        let xdg = dirs.config_dir().join("nexus.toml");
        if xdg.exists() {
            let cfg = read_config_file(&xdg)?;
            return Ok((cfg, Some(xdg)));
        }
    }

    Ok((NexusConfig::default(), None))
}

fn read_config_file(path: &Path) -> Result<NexusConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse TOML from {}", path.display()))
}

fn ensure_config_exists(path: &Path) -> Result<()> {
    if path.exists() {
        Ok(())
    } else {
        anyhow::bail!(
            "{} is set but file does not exist: {}",
            ENV_NEXUS_CONFIG,
            path.display()
        );
    }
}

fn resolve_db_path(db_path: &Path) -> PathBuf {
    if db_path.is_absolute() {
        db_path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(db_path)
    }
}

pub fn default_config_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("org", "nexus", "nexus") {
        dirs.config_dir().join("nexus.toml")
    } else {
        PathBuf::from("nexus.toml")
    }
}
