use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use nexus_config::ConfigBundle;
use nexus_core::{CloneRemoteLink, InventorySnapshot, RunRecord};
use nexus_db::Database;
use std::fs;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "nexus")]
#[command(about = "Deterministic repo fleet intelligence engine", version)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Scan {
        #[arg(value_name = "ROOT")]
        roots: Vec<PathBuf>,

        #[arg(long)]
        github_owner: Option<String>,
    },
    Plan {
        #[arg(long, default_value = "nexus-plan.json")]
        write: PathBuf,
        /// Skip pairwise merge-base evidence between git clones.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional scanners (gitleaks, semgrep, jscpd, syft) on canonical clones when installed.
        #[arg(long)]
        external: bool,
    },
    Report {
        #[arg(long, default_value = "md")]
        format: ReportFormat,
    },
    Doctor,
    /// v1 placeholder: pass `--dry-run` (mutating apply is not implemented).
    Apply {
        #[arg(long)]
        dry_run: bool,
    },
    /// Read-only JSON API over loaded SQLite state (`GET /health`, `/v1/plan`, `/v1/inventory`).
    Serve {
        #[arg(long, default_value_t = 3030)]
        port: u16,
    },
    /// Show which optional external tools are on PATH (Phase 10 adapters).
    Tools,
}

#[derive(Debug, Clone, ValueEnum)]
enum ReportFormat {
    Md,
    Json,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let bundle = ConfigBundle::load(cli.config.as_deref())?;
    let db = Database::open(&bundle.effective_db_path)?;

    match cli.command {
        Commands::Scan {
            roots,
            github_owner,
        } => cmd_scan(&db, &bundle, roots, github_owner),
        Commands::Plan {
            write,
            no_merge_base,
            external,
        } => cmd_plan(&db, &write, no_merge_base, external),
        Commands::Report { format } => cmd_report(&db, format),
        Commands::Doctor => cmd_doctor(&bundle),
        Commands::Apply { dry_run } => cmd_apply(&db, dry_run),
        Commands::Serve { port } => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("tokio runtime")?;
            rt.block_on(nexus_api::serve(bundle.effective_db_path.clone(), port))?;
            Ok(())
        }
        Commands::Tools => {
            cmd_tools();
            Ok(())
        }
    }
}

fn cmd_scan(
    db: &Database,
    bundle: &ConfigBundle,
    roots: Vec<PathBuf>,
    github_owner: Option<String>,
) -> Result<()> {
    let config = &bundle.config;
    let resolved_roots = if roots.is_empty() {
        config
            .default_roots
            .iter()
            .map(expand_tilde)
            .collect::<Vec<PathBuf>>()
    } else {
        roots
    };

    let mut clones = nexus_scan::scan_roots(
        &resolved_roots,
        &nexus_scan::ScanOptions {
            respect_gitignore: config.scan.respect_gitignore,
            include_hidden: config.include_hidden,
            max_readme_bytes: config.scan.max_readme_bytes,
            max_hash_files: config.scan.max_hash_files,
        },
    )?;

    let mut remotes = Vec::new();
    let mut links = Vec::new();

    for clone in &mut clones {
        let path = PathBuf::from(&clone.path);
        if path.join(".git").exists() {
            if let Ok(git_remotes) = nexus_git::enrich_clone(&path, clone) {
                for remote in git_remotes {
                    let rid = format!("remote-local-{}", uuid::Uuid::new_v4());
                    remotes.push(nexus_core::RemoteRecord {
                        id: rid.clone(),
                        provider: "local-git".into(),
                        owner: None,
                        name: Some(remote.name.clone()),
                        full_name: None,
                        url: remote.url,
                        normalized_url: remote.normalized_url,
                        default_branch: clone.default_branch.clone(),
                        is_fork: false,
                        is_archived: false,
                        is_private: false,
                        pushed_at: clone.last_commit_at,
                    });
                    links.push(CloneRemoteLink {
                        clone_id: clone.id.clone(),
                        remote_id: rid,
                        relationship: remote.name,
                    });
                }
            }
        }
    }

    let gh_owner = github_owner.clone().or_else(|| config.github_owner.clone());
    let github_remotes: Vec<nexus_core::RemoteRecord> = match &gh_owner {
        Some(owner) => nexus_github::ingest_owner(owner).unwrap_or_default(),
        None => vec![],
    };

    let github_by_url: std::collections::HashMap<String, String> = github_remotes
        .iter()
        .map(|r| (r.normalized_url.clone(), r.id.clone()))
        .collect();

    remotes.extend(github_remotes);

    let mut seen_pairs = std::collections::HashSet::<(String, String)>::new();
    let mut extra_links = Vec::new();
    for link in &links {
        if let Some(local) = remotes.iter().find(|r| r.id == link.remote_id) {
            if local.provider != "local-git" {
                continue;
            }
            if let Some(gh_id) = github_by_url.get(&local.normalized_url) {
                let key = (link.clone_id.clone(), gh_id.clone());
                if seen_pairs.insert(key) {
                    extra_links.push(CloneRemoteLink {
                        clone_id: link.clone_id.clone(),
                        remote_id: gh_id.clone(),
                        relationship: format!("{}→github", link.relationship),
                    });
                }
            }
        }
    }
    links.extend(extra_links);

    let run = RunRecord {
        id: format!("run-{}", uuid::Uuid::new_v4()),
        started_at: chrono::Utc::now(),
        finished_at: Some(chrono::Utc::now()),
        roots: resolved_roots
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        github_owner: gh_owner,
        version: env!("CARGO_PKG_VERSION").into(),
    };

    db.save_run(&run)?;
    db.save_clones(&run.id, &clones)?;
    db.save_remotes(&remotes)?;
    db.replace_clone_remote_links(&links)?;

    println!("scan complete");
    println!("clones: {}", clones.len());
    println!("remotes: {}", remotes.len());
    println!("clone↔remote links: {}", links.len());
    println!("db: {}", bundle.effective_db_path.display());
    Ok(())
}

fn cmd_plan(db: &Database, write: &Path, no_merge_base: bool, external: bool) -> Result<()> {
    let snapshot = db.load_inventory()?;
    let opts = nexus_plan::PlanBuildOpts {
        merge_base: !no_merge_base,
    };
    let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    if external {
        nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    db.persist_plan(&plan)?;
    let json = serde_json::to_string_pretty(&plan)?;
    fs::write(write, json).with_context(|| format!("failed to write plan {}", write.display()))?;
    println!("wrote {}", write.display());
    println!("persisted clusters/actions to sqlite");
    Ok(())
}

fn cmd_report(db: &Database, format: ReportFormat) -> Result<()> {
    let snapshot: InventorySnapshot = db.load_inventory()?;
    let plan = nexus_plan::build_plan(&snapshot)?;
    match format {
        ReportFormat::Md => {
            let md = nexus_report::render_markdown(&plan)?;
            println!("{md}");
        }
        ReportFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
    }
    Ok(())
}

fn cmd_tools() {
    println!("Optional external tools (on PATH):");
    for (tool, ok) in nexus_adapters::probe_all() {
        println!("  {:10} {}", tool.bin_name(), if ok { "yes" } else { "no" });
    }
}

fn cmd_apply(db: &Database, dry_run: bool) -> Result<()> {
    anyhow::ensure!(
        dry_run,
        "v1 is plan-only: use `nexus apply --dry-run` (mutating apply is disabled)"
    );
    let snapshot = db.load_inventory()?;
    let plan = nexus_plan::build_plan(&snapshot)?;
    let actions: usize = plan.clusters.iter().map(|cp| cp.actions.len()).sum();
    println!(
        "apply --dry-run: {} clusters, {actions} proposed actions (no changes made)",
        plan.clusters.len()
    );
    Ok(())
}

fn cmd_doctor(bundle: &ConfigBundle) -> Result<()> {
    let config = &bundle.config;
    println!(
        "config file: {}",
        bundle
            .source_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(defaults — no nexus.toml found)".into())
    );
    println!("db_path (config): {}", config.db_path.display());
    println!(
        "db_path (effective): {}",
        bundle.effective_db_path.display()
    );

    match Database::open(&bundle.effective_db_path) {
        Ok(db) => {
            match db.sqlite_version() {
                Ok(version) => println!("sqlite: {version}"),
                Err(e) => println!("sqlite version: unavailable ({e:#})"),
            }
            println!("db open: ok");
        }
        Err(e) => {
            println!("db open: FAILED ({e:#})");
        }
    }

    println!("default roots: {:?}", config.default_roots);
    println!("gh in PATH: {}", which::which("gh").is_ok());
    println!("git in PATH: {}", which::which("git").is_ok());
    match std::process::Command::new("cc").arg("--version").output() {
        Ok(out) if out.status.success() => println!("cc (C linker): ok"),
        _ => {
            println!("cc (C linker): missing or not functional");
            #[cfg(target_os = "macos")]
            println!("  fix: install Xcode CLT (`xcode-select --install`) so `cargo` can link");
            #[cfg(not(target_os = "macos"))]
            println!("  fix: install a C toolchain (e.g. build-essential) so `cargo` can link");
        }
    }
    let scanners: Vec<_> = nexus_adapters::probe_all()
        .into_iter()
        .filter(|(_, ok)| *ok)
        .map(|(t, _)| t.bin_name())
        .collect();
    let scanner_line = if scanners.is_empty() {
        "(none)".to_string()
    } else {
        scanners.join(", ")
    };
    println!("external scanners on PATH: {scanner_line}");
    if let Ok(ver) = std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        if ver.status.success() {
            println!("rustc: {}", String::from_utf8_lossy(&ver.stdout).trim_end());
        }
    }
    Ok(())
}

fn expand_tilde(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(input)
}
