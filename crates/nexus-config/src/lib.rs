use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable pointing at a `nexus.toml` file. Highest precedence after explicit CLI `--config`.
pub const ENV_NEXUS_CONFIG: &str = "NEXUS_CONFIG";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    pub respect_gitignore: bool,
    pub max_readme_bytes: usize,
    pub max_hash_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlannerConfig {
    /// Minimum canonical score (0–100) before suggesting `ArchiveLocalDuplicate` for non-canonical clones.
    pub archive_duplicate_threshold: u8,
    /// Emit license/CI/security publish actions when `oss_readiness` is below this (0–100).
    pub oss_candidate_threshold: u8,
    /// Cluster is `Ambiguous` when planner confidence is strictly below this percent (1–99).
    pub ambiguous_cluster_threshold: u8,
    /// Clone IDs to treat as canonical when present in a cluster (`scores`/`plan.json` → `canonical_clone_id`).
    pub canonical_pins: Vec<String>,
    /// Exact `cluster_key` values (e.g. `name:my-repo`, `url:github.com/o/r`) — no plan actions; evidence only.
    pub ignored_cluster_keys: Vec<String>,
    /// Exact `cluster_key` values — adds `user_archive_hint` evidence only (no automation).
    pub archive_hint_cluster_keys: Vec<String>,
    /// Optional scoring profile: `default`, `publish`, `open_source`, `security`, `ai_handoff` (see `docs/SCORING_PROFILES.md`).
    pub scoring_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub enabled: bool,
    pub api_base: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            max_tokens: 1024,
            temperature: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NexusConfig {
    pub db_path: PathBuf,
    pub default_roots: Vec<String>,
    pub github_owner: Option<String>,
    pub include_hidden: bool,
    pub scan: ScanConfig,
    pub planner: PlannerConfig,
    pub ai: AiConfig,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            max_readme_bytes: 16 * 1024,
            max_hash_files: 64,
        }
    }
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            archive_duplicate_threshold: 80,
            oss_candidate_threshold: 70,
            // Matches historical engine behavior (`confidence < 0.6` → ambiguous).
            ambiguous_cluster_threshold: 60,
            canonical_pins: Vec::new(),
            ignored_cluster_keys: Vec::new(),
            archive_hint_cluster_keys: Vec::new(),
            scoring_profile: None,
        }
    }
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(".nexus/state.db"),
            default_roots: vec!["~/Projects".into()],
            github_owner: None,
            include_hidden: false,
            scan: ScanConfig::default(),
            planner: PlannerConfig::default(),
            ai: AiConfig::default(),
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
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let effective_db_path = resolve_db_path_with_cwd(&config.db_path, &cwd);
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

fn resolve_db_path_with_cwd(db_path: &Path, cwd: &Path) -> PathBuf {
    if db_path.is_absolute() {
        db_path.to_path_buf()
    } else {
        cwd.join(db_path)
    }
}

pub fn default_config_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("org", "nexus", "nexus") {
        dirs.config_dir().join("nexus.toml")
    } else {
        PathBuf::from("nexus.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn partial_toml_fills_defaults() {
        let raw = r#"
db_path = "/tmp/nexus.db"
default_roots = []
"#;
        let cfg: NexusConfig = toml::from_str(raw).expect("parse");
        assert_eq!(cfg.db_path, PathBuf::from("/tmp/nexus.db"));
        assert!(cfg.default_roots.is_empty());
        assert!(!cfg.include_hidden);
        assert_eq!(cfg.github_owner, None);
        assert!(cfg.scan.respect_gitignore);
        assert_eq!(cfg.scan.max_readme_bytes, 16 * 1024);
        assert_eq!(cfg.planner.archive_duplicate_threshold, 80);
        assert_eq!(cfg.planner.ambiguous_cluster_threshold, 60);
        assert!(cfg.planner.canonical_pins.is_empty());
        assert!(cfg.planner.ignored_cluster_keys.is_empty());
    }

    #[test]
    fn partial_scan_table_fills_scan_defaults() {
        let raw = r#"
db_path = "/x.db"
default_roots = []
[scan]
max_hash_files = 1
"#;
        let cfg: NexusConfig = toml::from_str(raw).expect("parse");
        assert_eq!(cfg.scan.max_hash_files, 1);
        assert!(cfg.scan.respect_gitignore);
        assert_eq!(cfg.scan.max_readme_bytes, 16 * 1024);
    }

    #[test]
    fn resolve_db_path_absolute_unchanged() {
        let cwd = Path::new("/tmp/wd");
        let p = Path::new("/var/db.sqlite");
        assert_eq!(
            resolve_db_path_with_cwd(p, cwd),
            PathBuf::from("/var/db.sqlite")
        );
    }

    #[test]
    fn resolve_db_path_relative_joins_cwd() {
        let cwd = Path::new("/home/user/proj");
        let p = Path::new(".nexus/state.db");
        assert_eq!(
            resolve_db_path_with_cwd(p, cwd),
            PathBuf::from("/home/user/proj/.nexus/state.db")
        );
    }
}
