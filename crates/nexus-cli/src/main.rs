mod explain;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use nexus_config::ConfigBundle;
use nexus_core::{CloneRemoteLink, InventorySnapshot, RunRecord};
use nexus_db::Database;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "nexus")]
#[command(
    about = "Local-first repo fleet triage — inventory, cluster, score, plan.",
    long_about = "Nexus inventories your local git clones, ingests GitHub metadata, groups \
    everything into clusters, scores them, and writes a deterministic plan — \
    without touching your working trees.\n\n\
    Golden path: scan → score → plan → report",
    version,
    after_help = "Docs: https://github.com/bmmaral/nexus/tree/main/docs"
)]
struct Cli {
    /// Path to nexus.toml (default: ./nexus.toml or $NEXUS_CONFIG).
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Discover local repos and optionally ingest GitHub metadata.
    Scan {
        /// Directories to scan for git repositories.
        #[arg(value_name = "ROOT")]
        roots: Vec<PathBuf>,
        /// GitHub user or org to ingest (requires `gh` on PATH).
        #[arg(long)]
        github_owner: Option<String>,
    },
    /// Compute cluster scores and evidence from the current inventory.
    Score {
        /// Output format.
        #[arg(long, default_value = "text")]
        format: ScoreFormat,
        /// Skip pairwise merge-base evidence between git clones.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional external scanners on canonical clones.
        #[arg(long)]
        external: bool,
        /// Scoring profile: default, publish, open_source, security, ai_handoff.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Resolve clusters, score, attach actions, and write a JSON plan.
    Plan {
        /// Where to write the plan JSON file.
        #[arg(long, default_value = "nexus-plan.json")]
        write: PathBuf,
        /// Skip pairwise merge-base evidence between git clones.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional external scanners on canonical clones.
        #[arg(long)]
        external: bool,
        /// Scoring profile: default, publish, open_source, security, ai_handoff.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Render a human-readable report (Markdown or JSON) from inventory.
    Report {
        /// Output format.
        #[arg(long, default_value = "md")]
        format: ReportFormat,
        /// Scoring profile: default, publish, open_source, security, ai_handoff.
        #[arg(long)]
        profile: Option<String>,
    },
    /// Check environment, config, database, and tool availability.
    Doctor {
        /// Output format.
        #[arg(long, default_value = "text")]
        format: DoctorFormat,
    },
    /// Preview proposed actions without applying them (v1 is read-only).
    Apply {
        /// Required in v1 — mutating apply is not yet implemented.
        #[arg(long)]
        dry_run: bool,
        /// Output format.
        #[arg(long, default_value = "text")]
        format: ApplyFormat,
    },
    /// [experimental] Read-only JSON API over local SQLite.
    Serve {
        /// Port to listen on.
        #[arg(long, default_value_t = 3030)]
        port: u16,
        /// Listen address (default: 127.0.0.1; use 0.0.0.0 for network access).
        #[arg(long, default_value = "127.0.0.1")]
        listen: std::net::IpAddr,
    },
    /// Show which optional external scanners are on PATH.
    Tools {
        /// Output format.
        #[arg(long, default_value = "text")]
        format: ToolsFormat,
    },
    /// Export inventory as JSON (optionally with an embedded plan).
    Export {
        /// Write to file instead of stdout.
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// Include a freshly computed plan in the export.
        #[arg(long)]
        with_plan: bool,
        /// Skip pairwise merge-base evidence.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional external scanners.
        #[arg(long)]
        external: bool,
    },
    /// Restore inventory from a `nexus export` JSON file.
    Import {
        /// Path to the export JSON file.
        #[arg(value_name = "FILE")]
        path: PathBuf,
        /// Confirm replacement (clears existing inventory and persisted plan).
        #[arg(long)]
        force: bool,
    },
    /// Interactive terminal UI for browsing clusters, scores, and evidence.
    Tui {
        /// Skip pairwise merge-base evidence.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional external scanners.
        #[arg(long)]
        external: bool,
    },
    /// Deep-dive into one cluster: scores, evidence, actions.
    Explain {
        /// Skip pairwise merge-base evidence.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional external scanners.
        #[arg(long)]
        external: bool,
        /// Output format.
        #[arg(long, default_value = "text")]
        format: explain::ExplainFormat,
        /// Append an AI-generated narrative (requires ai.enabled + API key).
        #[arg(long)]
        ai: bool,
        /// Scoring profile: default, publish, open_source, security, ai_handoff.
        #[arg(long)]
        profile: Option<String>,
        #[command(subcommand)]
        target: explain::ExplainTarget,
    },
    /// [experimental] AI-generated executive summary of the full plan.
    AiSummary {
        /// Skip pairwise merge-base evidence.
        #[arg(long)]
        no_merge_base: bool,
        /// Run optional external scanners.
        #[arg(long)]
        external: bool,
    },
    /// Generate shell completions for bash, zsh, fish, elvish, or powershell.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum ReportFormat {
    Md,
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum DoctorFormat {
    Text,
    /// Stable JSON for scripts (`kind: "nexus_doctor"`).
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum ToolsFormat {
    Text,
    /// JSON map of tool binary name → on PATH (`kind: "nexus_tools"`).
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum ApplyFormat {
    Text,
    /// JSON summary when used with `--dry-run` (`kind: "nexus_apply_dry_run"`).
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum ScoreFormat {
    /// One block per cluster: headline scores and evidence count.
    Text,
    /// JSON document with `clusters` (same `ClusterRecord` shape as inside `plan.json`, without actions).
    Json,
}

fn parse_scoring_profile(raw: &Option<String>) -> nexus_plan::ScoringProfile {
    let Some(s) = raw.as_deref().map(str::trim).filter(|x| !x.is_empty()) else {
        return nexus_plan::ScoringProfile::Default;
    };
    let x = s.to_ascii_lowercase().replace('-', "_");
    match x.as_str() {
        "default" => nexus_plan::ScoringProfile::Default,
        "publish" | "publish_readiness" => nexus_plan::ScoringProfile::PublishReadiness,
        "open_source" | "open_source_readiness" | "oss" => {
            nexus_plan::ScoringProfile::OpenSourceReadiness
        }
        "security" | "security_supply_chain" | "supply_chain" => {
            nexus_plan::ScoringProfile::SecuritySupplyChain
        }
        "ai_handoff" | "ai" => nexus_plan::ScoringProfile::AiHandoff,
        other => {
            tracing::warn!(profile = %other, "unknown planner.scoring_profile; using default");
            nexus_plan::ScoringProfile::Default
        }
    }
}

fn plan_build_opts(bundle: &ConfigBundle, merge_base: bool) -> nexus_plan::PlanBuildOpts {
    let p = &bundle.config.planner;
    nexus_plan::PlanBuildOpts {
        merge_base,
        ambiguous_cluster_threshold_pct: p.ambiguous_cluster_threshold.clamp(1, 99),
        oss_candidate_threshold: p.oss_candidate_threshold.min(100),
        archive_duplicate_canonical_min: p.archive_duplicate_threshold.min(100),
        user_intent: nexus_plan::PlanUserIntent {
            pin_canonical_clone_ids: p.canonical_pins.iter().cloned().collect::<HashSet<_>>(),
            ignored_cluster_keys: p
                .ignored_cluster_keys
                .iter()
                .cloned()
                .collect::<HashSet<_>>(),
            archive_hint_cluster_keys: p
                .archive_hint_cluster_keys
                .iter()
                .cloned()
                .collect::<HashSet<_>>(),
            scoring_profile: parse_scoring_profile(&p.scoring_profile),
        },
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let bundle = ConfigBundle::load(cli.config.as_deref())?;
    let mut db = Database::open(&bundle.effective_db_path)?;

    match cli.command {
        Commands::Scan {
            roots,
            github_owner,
        } => cmd_scan(&mut db, &bundle, roots, github_owner),
        Commands::Score {
            format,
            no_merge_base,
            external,
            profile,
        } => cmd_score(&db, &bundle, format, no_merge_base, external, profile),
        Commands::Plan {
            write,
            no_merge_base,
            external,
            profile,
        } => cmd_plan(&mut db, &bundle, &write, no_merge_base, external, profile),
        Commands::Report { format, profile } => cmd_report(&db, &bundle, format, profile),
        Commands::Doctor { format } => cmd_doctor(&bundle, format),
        Commands::Apply { dry_run, format } => cmd_apply(&db, &bundle, dry_run, format),
        Commands::Serve { port, listen } => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("tokio runtime")?;
            rt.block_on(nexus_api::serve(
                bundle.effective_db_path.clone(),
                port,
                listen,
                bundle.clone(),
            ))?;
            Ok(())
        }
        Commands::Tools { format } => cmd_tools(format),
        Commands::Export {
            output,
            with_plan,
            no_merge_base,
            external,
        } => cmd_export(&db, &bundle, with_plan, no_merge_base, external, output),
        Commands::Import { path, force } => cmd_import(&mut db, &path, force),
        Commands::Tui {
            no_merge_base,
            external,
        } => cmd_tui(&db, &bundle, no_merge_base, external),
        Commands::Explain {
            no_merge_base,
            external,
            format,
            ai,
            profile,
            target,
        } => cmd_explain(
            &db,
            &bundle,
            target,
            format,
            no_merge_base,
            external,
            ai,
            profile,
        ),
        Commands::AiSummary {
            no_merge_base,
            external,
        } => cmd_ai_summary(&db, &bundle, no_merge_base, external),
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "nexus", &mut std::io::stdout());
            Ok(())
        }
    }
}

fn cmd_scan(
    db: &mut Database,
    bundle: &ConfigBundle,
    roots: Vec<PathBuf>,
    github_owner: Option<String>,
) -> Result<()> {
    let t0 = std::time::Instant::now();
    let config = &bundle.config;
    let resolved_roots = if roots.is_empty() {
        config
            .default_roots
            .iter()
            .map(|s| expand_tilde(s.as_str()))
            .collect::<Vec<PathBuf>>()
    } else {
        roots
    };

    let scan_mode = match config.scan.scan_mode {
        nexus_config::ScanMode::GitOnly => nexus_scan::ScanMode::GitOnly,
        nexus_config::ScanMode::ProjectRoots => nexus_scan::ScanMode::ProjectRoots,
    };

    let t_scan = std::time::Instant::now();
    let mut clones = nexus_scan::scan_roots(
        &resolved_roots,
        &nexus_scan::ScanOptions {
            respect_gitignore: config.scan.respect_gitignore,
            include_hidden: config.include_hidden,
            max_readme_bytes: config.scan.max_readme_bytes,
            max_hash_files: config.scan.max_hash_files,
            scan_mode,
            max_depth: config.scan.max_depth,
        },
    )?;
    let scan_ms = t_scan.elapsed().as_millis();

    let t_enrich = std::time::Instant::now();
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
    let enrich_ms = t_enrich.elapsed().as_millis();

    let gh_owner = github_owner.clone().or_else(|| config.github_owner.clone());
    let t_gh = std::time::Instant::now();
    let github_remotes: Vec<nexus_core::RemoteRecord> = match &gh_owner {
        Some(owner) => nexus_github::ingest_owner(owner).unwrap_or_default(),
        None => vec![],
    };
    let gh_ms = t_gh.elapsed().as_millis();
    let n_github = github_remotes.len();

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
        github_owner: gh_owner.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
    };

    let t_db = std::time::Instant::now();
    db.save_run(&run)?;
    db.save_clones(&run.id, &clones)?;
    db.save_remotes(&remotes)?;
    db.replace_clone_remote_links(&links)?;
    let db_ms = t_db.elapsed().as_millis();

    let n_git = clones.iter().filter(|c| c.is_git).count();
    let total_size: u64 = clones.iter().filter_map(|c| c.size_bytes).sum();

    println!("scan complete ({:.1}s)", t0.elapsed().as_secs_f64());
    println!();
    println!(
        "  clones         {:>5}  ({} git repos)",
        clones.len(),
        n_git
    );
    println!(
        "  remotes        {:>5}  ({} local-git, {} github)",
        remotes.len(),
        remotes.iter().filter(|r| r.provider == "local-git").count(),
        n_github,
    );
    println!("  links          {:>5}", links.len());
    if total_size > 0 {
        println!("  total size     {:>5}", format_bytes(total_size));
    }
    println!();
    println!("  scan     {:>6}ms", scan_ms);
    println!("  enrich   {:>6}ms", enrich_ms);
    if gh_owner.is_some() {
        println!("  github   {:>6}ms", gh_ms);
    }
    println!("  persist  {:>6}ms", db_ms);
    println!();
    println!("  db: {}", bundle.effective_db_path.display());
    Ok(())
}

fn parse_inventory_import(bytes: &[u8]) -> Result<InventorySnapshot> {
    let v: serde_json::Value = serde_json::from_slice(bytes).context(
        "parse JSON (expected export envelope with `inventory` or a raw inventory object)",
    )?;
    if let Some(inv) = v.get("inventory") {
        serde_json::from_value(inv.clone()).context("deserialize `inventory` field")
    } else {
        serde_json::from_value(v).context("deserialize inventory snapshot")
    }
}

fn cmd_export(
    db: &Database,
    bundle: &ConfigBundle,
    with_plan: bool,
    no_merge_base: bool,
    external: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    let snapshot = db.load_inventory()?;
    let plan_json = if with_plan {
        let opts = plan_build_opts(bundle, !no_merge_base);
        let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
        if external {
            nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
        }
        Some(plan)
    } else {
        None
    };

    let mut doc = serde_json::json!({
        "schema_version": 1,
        "kind": "nexus_inventory_export_v1",
        "exported_at": Utc::now().to_rfc3339(),
        "generated_by": format!("nexus {}", env!("CARGO_PKG_VERSION")),
        "inventory": snapshot,
    });
    if let Some(p) = plan_json {
        doc["plan"] = serde_json::to_value(&p)?;
    }

    let json = serde_json::to_string_pretty(&doc)?;
    match output {
        Some(path) => {
            fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?
        }
        None => println!("{json}"),
    }
    Ok(())
}

fn cmd_import(db: &mut Database, path: &Path, force: bool) -> Result<()> {
    anyhow::ensure!(
        force,
        "import replaces all inventory and clears the persisted plan; pass `--force` to confirm"
    );
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let snapshot = parse_inventory_import(&bytes)?;
    db.replace_inventory_snapshot(&snapshot, env!("CARGO_PKG_VERSION"))
        .context("replace inventory")?;
    println!(
        "imported {} clones, {} remotes, {} links",
        snapshot.clones.len(),
        snapshot.remotes.len(),
        snapshot.links.len()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_explain(
    db: &Database,
    bundle: &ConfigBundle,
    target: explain::ExplainTarget,
    format: explain::ExplainFormat,
    no_merge_base: bool,
    external: bool,
    ai: bool,
    profile: Option<String>,
) -> Result<()> {
    let snapshot = db.load_inventory()?;
    let mut local_bundle = bundle.clone();
    if let Some(ref p) = profile {
        local_bundle.config.planner.scoring_profile = Some(p.clone());
    }
    let opts = plan_build_opts(&local_bundle, !no_merge_base);
    let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    if external {
        nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    explain::run_explain(&snapshot, &plan, target, format)?;

    if ai {
        let ai_cfg = nexus_ai::AiConfig {
            enabled: bundle.config.ai.enabled,
            api_base: bundle.config.ai.api_base.clone(),
            model: bundle.config.ai.model.clone(),
            max_tokens: bundle.config.ai.max_tokens,
            temperature: bundle.config.ai.temperature,
        };
        let cp = plan
            .clusters
            .first()
            .ok_or_else(|| anyhow::anyhow!("no clusters in plan"))?;
        let rt = tokio::runtime::Runtime::new()?;
        let narrative = rt.block_on(nexus_ai::explain_cluster(&ai_cfg, cp))?;
        println!("\n── AI explanation (model-generated, not deterministic) ──\n");
        println!("{narrative}");
    }
    Ok(())
}

fn cmd_ai_summary(
    db: &Database,
    bundle: &ConfigBundle,
    no_merge_base: bool,
    external: bool,
) -> Result<()> {
    let ai_cfg = nexus_ai::AiConfig {
        enabled: bundle.config.ai.enabled,
        api_base: bundle.config.ai.api_base.clone(),
        model: bundle.config.ai.model.clone(),
        max_tokens: bundle.config.ai.max_tokens,
        temperature: bundle.config.ai.temperature,
    };
    ai_cfg.validate()?;

    let snapshot = db.load_inventory()?;
    let opts = plan_build_opts(bundle, !no_merge_base);
    let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    if external {
        nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }

    let rt = tokio::runtime::Runtime::new()?;
    let summary = rt.block_on(nexus_ai::summarize_plan(&ai_cfg, &plan))?;
    println!("── AI plan summary (model-generated, not deterministic) ──\n");
    println!("{summary}");
    Ok(())
}

fn cmd_score(
    db: &Database,
    bundle: &ConfigBundle,
    format: ScoreFormat,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    let t0 = std::time::Instant::now();
    let snapshot = db.load_inventory()?;
    let mut local_bundle = bundle.clone();
    if let Some(ref p) = profile {
        local_bundle.config.planner.scoring_profile = Some(p.clone());
    }
    let opts = plan_build_opts(&local_bundle, !no_merge_base);
    let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    if external {
        nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    match format {
        ScoreFormat::Text => {
            use nexus_core::ClusterStatus;
            let n_amb = plan
                .clusters
                .iter()
                .filter(|cp| matches!(cp.cluster.status, ClusterStatus::Ambiguous))
                .count();
            let n_actions: usize = plan.clusters.iter().map(|cp| cp.actions.len()).sum();
            println!(
                "nexus score  {} clusters, {} actions, {} ambiguous  (rules v{}, {:.1}s)",
                plan.clusters.len(),
                n_actions,
                n_amb,
                plan.scoring_rules_version,
                t0.elapsed().as_secs_f64(),
            );
            println!();
            println!(
                "{:<26} {:>5} {:>6} {:>5} {:>5} {:>5}  {:>4} {:>4}  STATUS",
                "LABEL", "CANON", "HEALTH", "RECV", "PUB", "RISK", "EV", "ACT"
            );
            println!("{}", "─".repeat(88));
            for cp in &plan.clusters {
                let c = &cp.cluster;
                let st = match c.status {
                    ClusterStatus::Resolved => "OK",
                    ClusterStatus::Ambiguous => "AMB",
                    ClusterStatus::ManualReview => "REV",
                };
                let label = if c.label.len() > 25 {
                    format!("{}…", &c.label[..24])
                } else {
                    c.label.clone()
                };
                println!(
                    "{:<26} {:>5.0} {:>6.0} {:>5.0} {:>5.0} {:>5.0}  {:>4} {:>4}  {}",
                    label,
                    c.scores.canonical,
                    c.scores.usability,
                    c.scores.recoverability,
                    c.scores.oss_readiness,
                    c.scores.risk,
                    c.evidence.len(),
                    cp.actions.len(),
                    st,
                );
            }
            if plan.clusters.is_empty() {
                println!("(no clusters — run `nexus scan` first)");
            }
        }
        ScoreFormat::Json => {
            let clusters: Vec<serde_json::Value> = plan
                .clusters
                .iter()
                .map(|cp| serde_json::to_value(&cp.cluster))
                .collect::<Result<_, _>>()?;
            let doc = serde_json::json!({
                "schema_version": 1,
                "scoring_rules_version": plan.scoring_rules_version,
                "kind": "nexus_scores",
                "generated_at": Utc::now().to_rfc3339(),
                "generated_by": format!("nexus {}", env!("CARGO_PKG_VERSION")),
                "clusters": clusters,
            });
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
    }
    Ok(())
}

fn cmd_plan(
    db: &mut Database,
    bundle: &ConfigBundle,
    write: &Path,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    let t0 = std::time::Instant::now();
    let snapshot = db.load_inventory()?;
    let mut local_bundle = bundle.clone();
    if let Some(ref p) = profile {
        local_bundle.config.planner.scoring_profile = Some(p.clone());
    }
    let opts = plan_build_opts(&local_bundle, !no_merge_base);
    let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    if external {
        nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    db.persist_plan(&plan)?;
    let json = serde_json::to_string_pretty(&plan)?;
    fs::write(write, json).with_context(|| format!("failed to write plan {}", write.display()))?;

    let n_clusters = plan.clusters.len();
    let n_actions: usize = plan.clusters.iter().map(|cp| cp.actions.len()).sum();
    let n_high = plan
        .clusters
        .iter()
        .flat_map(|cp| &cp.actions)
        .filter(|a| matches!(a.priority, nexus_core::Priority::High))
        .count();
    let n_evidence: usize = plan
        .clusters
        .iter()
        .map(|cp| cp.cluster.evidence.len())
        .sum();

    println!("plan written ({:.1}s)", t0.elapsed().as_secs_f64());
    println!();
    println!(
        "  clusters  {:>5}   evidence  {:>5}",
        n_clusters, n_evidence
    );
    println!("  actions   {:>5}   high-pri  {:>5}", n_actions, n_high);
    println!();
    println!("  json: {}", write.display());
    println!("  db:   {}", bundle.effective_db_path.display());
    Ok(())
}

fn cmd_report(
    db: &Database,
    bundle: &ConfigBundle,
    format: ReportFormat,
    profile: Option<String>,
) -> Result<()> {
    let snapshot: InventorySnapshot = db.load_inventory()?;
    let mut local_bundle = bundle.clone();
    if let Some(ref p) = profile {
        local_bundle.config.planner.scoring_profile = Some(p.clone());
    }
    let opts = plan_build_opts(&local_bundle, true);
    let plan = nexus_plan::build_plan_with(&snapshot, opts)?;
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

fn cmd_tui(
    db: &Database,
    bundle: &ConfigBundle,
    no_merge_base: bool,
    external: bool,
) -> Result<()> {
    let snapshot = db.load_inventory()?;
    let opts = plan_build_opts(bundle, !no_merge_base);
    let mut plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    if external {
        nexus_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    let config_pins = bundle
        .config
        .planner
        .canonical_pins
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    nexus_tui::run(plan, nexus_tui::TuiConfig { config_pins })
}

fn cmd_tools(format: ToolsFormat) -> Result<()> {
    let probes = nexus_adapters::probe_all();
    match format {
        ToolsFormat::Text => {
            println!("Optional external tools (on PATH):");
            for (tool, ok) in &probes {
                println!(
                    "  {:10} {}",
                    tool.bin_name(),
                    if *ok { "yes" } else { "no" }
                );
            }
        }
        ToolsFormat::Json => {
            let tools: serde_json::Map<String, serde_json::Value> = probes
                .into_iter()
                .map(|(t, ok)| (t.bin_name().to_string(), serde_json::json!(ok)))
                .collect();
            let doc = serde_json::json!({
                "schema_version": 1,
                "kind": "nexus_tools",
                "generated_at": Utc::now().to_rfc3339(),
                "generated_by": format!("nexus {}", env!("CARGO_PKG_VERSION")),
                "tools": tools,
            });
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
    }
    Ok(())
}

fn cmd_apply(
    db: &Database,
    bundle: &ConfigBundle,
    dry_run: bool,
    format: ApplyFormat,
) -> Result<()> {
    anyhow::ensure!(
        dry_run,
        "v1 is plan-only: use `nexus apply --dry-run` (mutating apply is disabled)"
    );
    let snapshot = db.load_inventory()?;
    let opts = plan_build_opts(bundle, true);
    let plan = nexus_plan::build_plan_with(&snapshot, opts)?;
    let actions: usize = plan.clusters.iter().map(|cp| cp.actions.len()).sum();
    let n_clusters = plan.clusters.len();
    match format {
        ApplyFormat::Text => {
            println!(
                "apply --dry-run: {n_clusters} clusters, {actions} proposed actions (no changes made)",
            );
        }
        ApplyFormat::Json => {
            let doc = serde_json::json!({
                "schema_version": 1,
                "kind": "nexus_apply_dry_run",
                "dry_run": true,
                "cluster_count": n_clusters,
                "action_count": actions,
                "scoring_rules_version": plan.scoring_rules_version,
            });
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
    }
    Ok(())
}

fn cmd_doctor(bundle: &ConfigBundle, format: DoctorFormat) -> Result<()> {
    let config = &bundle.config;
    let config_source = bundle.source_path.as_ref().map(|p| p.display().to_string());

    let (db_open_ok, sqlite_version, sqlite_query_error, db_open_error) =
        match Database::open(&bundle.effective_db_path) {
            Ok(db) => match db.sqlite_version() {
                Ok(version) => (true, Some(version), None, None),
                Err(e) => (true, None, Some(format!("{e:#}")), None),
            },
            Err(e) => (false, None, None, Some(format!("{e:#}"))),
        };

    let gh_ok = which::which("gh").is_ok();
    let git_ok = which::which("git").is_ok();
    let cc_ok = matches!(
        std::process::Command::new("cc").arg("--version").output(),
        Ok(out) if out.status.success()
    );

    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let optional_scanners: serde_json::Map<String, serde_json::Value> = nexus_adapters::probe_all()
        .into_iter()
        .map(|(t, ok)| (t.bin_name().to_string(), serde_json::json!(ok)))
        .collect();

    match format {
        DoctorFormat::Json => {
            let doc = serde_json::json!({
                "schema_version": 1,
                "kind": "nexus_doctor",
                "generated_at": Utc::now().to_rfc3339(),
                "generated_by": format!("nexus {}", env!("CARGO_PKG_VERSION")),
                "config_source": config_source,
                "db_path_config": config.db_path.display().to_string(),
                "db_path_effective": bundle.effective_db_path.display().to_string(),
                "default_roots": config.default_roots,
                "database": {
                    "open_ok": db_open_ok,
                    "sqlite_version": sqlite_version,
                    "sqlite_query_error": sqlite_query_error,
                    "open_error": db_open_error,
                },
                "path_tools": {
                    "git": git_ok,
                    "gh": gh_ok,
                    "cc": cc_ok,
                },
                "rustc_version": rustc_version,
                "optional_scanners_on_path": optional_scanners,
            });
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        DoctorFormat::Text => {
            println!(
                "config file: {}",
                config_source
                    .as_deref()
                    .unwrap_or("(defaults — no nexus.toml found)")
            );
            if bundle.source_path.is_none() {
                println!(
                    "  → tip: copy `nexus.toml.example` to `./nexus.toml` or set `{}`",
                    nexus_config::ENV_NEXUS_CONFIG
                );
            }
            println!("db_path (config): {}", config.db_path.display());
            println!(
                "db_path (effective): {}",
                bundle.effective_db_path.display()
            );

            if db_open_ok {
                if let Some(ref v) = sqlite_version {
                    println!("sqlite: {v}");
                } else if let Some(ref e) = sqlite_query_error {
                    println!("sqlite version: unavailable ({e})");
                }
                println!("db open: ok");
            } else if let Some(ref e) = db_open_error {
                println!("db open: FAILED ({e})");
                println!(
                    "  → fix: ensure the parent directory exists and is writable; check `db_path` in config"
                );
                println!("  → see: docs/CONFIG.md");
            }

            println!("default roots: {:?}", config.default_roots);
            if config.default_roots.is_empty() {
                println!(
                    "  → tip: set `default_roots` in nexus.toml or pass paths to `nexus scan <path> ...`"
                );
            }

            println!("gh in PATH: {gh_ok}");
            if !gh_ok {
                println!(
                    "  → optional: install GitHub CLI for `scan --github-owner` (docs/EXTERNAL_TOOLS.md)"
                );
            }
            println!("git in PATH: {git_ok}");
            if !git_ok {
                println!(
                    "  → fix: install git; Nexus uses it for clone metadata and merge-base evidence"
                );
            }
            if cc_ok {
                println!("cc (C linker): ok");
            } else {
                println!("cc (C linker): missing or not functional");
                #[cfg(target_os = "macos")]
                println!("  → fix: install Xcode CLT (`xcode-select --install`) so `cargo` / rusqlite can link");
                #[cfg(not(target_os = "macos"))]
                println!(
                    "  → fix: install a C toolchain (e.g. build-essential, clang) for rusqlite"
                );
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
            if scanners.is_empty() {
                println!(
                    "  → optional: install tools listed in `nexus tools` for `plan --external`"
                );
            }
            if let Some(v) = rustc_version {
                println!("rustc: {v}");
            }
        }
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn expand_tilde(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(input)
}
