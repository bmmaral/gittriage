use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Environment variable pointing at a `gittriage.toml` file. Highest precedence after explicit CLI `--config`.
pub const ENV_GITTRIAGE_CONFIG: &str = "GITTRIAGE_CONFIG";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanMode {
    /// Only directories with `.git` are considered project roots (default).
    #[default]
    GitOnly,
    /// Directories with `.git` or common manifest files.
    ProjectRoots,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    pub respect_gitignore: bool,
    pub max_readme_bytes: usize,
    pub max_hash_files: usize,
    pub scan_mode: ScanMode,
    pub max_depth: Option<usize>,
    /// Discover `.git` directories nested inside another scanned git root (off by default).
    pub include_nested_git: bool,
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

/// How `github_owner` / `--github-owner` combines with a local scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitHubOwnerMode {
    /// Only keep GitHub repos whose normalized URL matches a local-git remote from this scan.
    #[default]
    Augment,
    /// Ingest the full `gh repo list` catalog (can add many remote-only clusters).
    FullCatalog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitTriageConfig {
    pub db_path: PathBuf,
    pub default_roots: Vec<String>,
    pub github_owner: Option<String>,
    /// `augment` (default): GitHub ingest is filtered to URLs already seen from local clones.
    /// `full_catalog`: full `gh repo list` for the owner (legacy / portfolio mode).
    pub github_owner_mode: GitHubOwnerMode,
    pub include_hidden: bool,
    pub scan: ScanConfig,
    pub planner: PlannerConfig,
    pub ai: AiConfig,
    /// When set, `gittriage tui` `o` export writes here; otherwise a timestamped file in cwd.
    pub tui_export_path: Option<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            max_readme_bytes: 16 * 1024,
            max_hash_files: 64,
            scan_mode: ScanMode::default(),
            max_depth: None,
            include_nested_git: false,
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

impl Default for GitTriageConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(".gittriage/state.db"),
            default_roots: vec!["~/Projects".into()],
            github_owner: None,
            github_owner_mode: GitHubOwnerMode::default(),
            include_hidden: false,
            scan: ScanConfig::default(),
            planner: PlannerConfig::default(),
            ai: AiConfig::default(),
            tui_export_path: None,
        }
    }
}

/// Resolved configuration and where it came from.
#[derive(Debug, Clone)]
pub struct ConfigBundle {
    pub config: GitTriageConfig,
    /// TOML file that was loaded, if any.
    pub source_path: Option<PathBuf>,
    /// Absolute path used for SQLite. Relative `db_path` is resolved from the config file's
    /// parent directory when a config file exists, otherwise from the process cwd.
    pub effective_db_path: PathBuf,
}

impl ConfigBundle {
    /// Load config using precedence:
    /// 1. `explicit` path from `--config` (must exist)
    /// 2. `GITTRIAGE_CONFIG` env (must exist when set)
    /// 3. `./gittriage.toml` under the current working directory
    /// 4. XDG config dir `gittriage.toml` (`ProjectDirs::config_dir`)
    /// 5. Built-in defaults (no file)
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let (config, source_path) = load_layered(explicit)?;
        let base = source_path
            .as_deref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let effective_db_path = resolve_db_path(&config.db_path, &base);
        Ok(Self {
            config,
            source_path,
            effective_db_path,
        })
    }
}

fn load_layered(explicit: Option<&Path>) -> Result<(GitTriageConfig, Option<PathBuf>)> {
    if let Some(path) = explicit {
        let path = path.to_path_buf();
        let cfg = read_config_file(&path)?;
        return Ok((cfg, Some(path)));
    }

    if let Ok(from_env) = env::var(ENV_GITTRIAGE_CONFIG) {
        let path = PathBuf::from(&from_env);
        ensure_config_exists(&path)?;
        let cfg = read_config_file(&path)?;
        return Ok((cfg, Some(path)));
    }

    let cwd = env::current_dir().context("failed to resolve current directory")?;
    let local = cwd.join("gittriage.toml");
    if local.exists() {
        let cfg = read_config_file(&local)?;
        return Ok((cfg, Some(local)));
    }

    if let Some(dirs) = ProjectDirs::from("org", "gittriage", "gittriage") {
        let xdg = dirs.config_dir().join("gittriage.toml");
        if xdg.exists() {
            let cfg = read_config_file(&xdg)?;
            return Ok((cfg, Some(xdg)));
        }
    }

    Ok((GitTriageConfig::default(), None))
}

fn read_config_file(path: &Path) -> Result<GitTriageConfig> {
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
            ENV_GITTRIAGE_CONFIG,
            path.display()
        );
    }
}

fn resolve_db_path(db_path: &Path, base: &Path) -> PathBuf {
    if db_path.is_absolute() {
        db_path.to_path_buf()
    } else {
        let expanded = expand_tilde(db_path);
        if expanded.is_absolute() {
            expanded
        } else {
            base.join(expanded)
        }
    }
}

fn expand_tilde(p: &Path) -> PathBuf {
    if let Ok(s) = p.strip_prefix("~") {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(s);
        }
        if let Some(dirs) = ProjectDirs::from("org", "gittriage", "gittriage") {
            if let Some(home) = dirs.data_dir().parent().and_then(|p| p.parent()) {
                return home.to_path_buf().join(s);
            }
        }
    }
    p.to_path_buf()
}

pub fn default_config_path() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("org", "gittriage", "gittriage") {
        dirs.config_dir().join("gittriage.toml")
    } else {
        PathBuf::from("gittriage.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn partial_toml_fills_defaults() {
        let raw = r#"
db_path = "/tmp/gittriage.db"
default_roots = []
"#;
        let cfg: GitTriageConfig = toml::from_str(raw).expect("parse");
        assert_eq!(cfg.db_path, PathBuf::from("/tmp/gittriage.db"));
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
        let cfg: GitTriageConfig = toml::from_str(raw).expect("parse");
        assert_eq!(cfg.scan.max_hash_files, 1);
        assert!(cfg.scan.respect_gitignore);
        assert_eq!(cfg.scan.max_readme_bytes, 16 * 1024);
    }

    #[test]
    fn resolve_db_path_absolute_unchanged() {
        let base = Path::new("/tmp/wd");
        let p = Path::new("/var/db.sqlite");
        assert_eq!(resolve_db_path(p, base), PathBuf::from("/var/db.sqlite"));
    }

    #[test]
    fn resolve_db_path_relative_joins_base() {
        let base = Path::new("/home/user/proj");
        let p = Path::new(".gittriage/state.db");
        assert_eq!(
            resolve_db_path(p, base),
            PathBuf::from("/home/user/proj/.gittriage/state.db")
        );
    }

    #[test]
    fn expand_tilde_resolves_home() {
        let p = Path::new("~/data/gittriage.db");
        let expanded = expand_tilde(p);
        assert!(
            expanded.is_absolute() || std::env::var("HOME").is_err(),
            "tilde should expand to an absolute path when HOME is set"
        );
    }
}
