mod explain;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use gittriage_config::ConfigBundle;
use gittriage_core::{CloneRemoteLink, ClusterScopeFilter, InventorySnapshot, RunRecord};
use gittriage_db::Database;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "gittriage")]
#[command(
    about = "Deterministic workspace-truth and preflight safety for coding agents.",
    long_about = "GitTriage inventories local git checkouts, groups duplicates into clusters, picks a \
    canonical path, and emits **automation verdicts** — so agents know which repo is real and when \
    to stop for human review.\n\n\
    Agent path: `preflight` / `resolve` / `check-path` / `verdict` → hand JSON to your agent.\n\
    Human path: `scan` → `plan` → `report` / `tui`.",
    version,
    after_help = "Docs: https://github.com/bmmaral/gittriage/tree/main/docs"
)]
struct Cli {
    /// Path to gittriage.toml (default: ./gittriage.toml or $GITTRIAGE_CONFIG).
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Resolve a label, filesystem path, or remote URL to the canonical checkout (stable JSON for agents).
    Resolve {
        #[arg(value_name = "QUERY")]
        query: String,
        #[arg(long, default_value = "text")]
        format: AgentOutputFormat,
        #[arg(long)]
        no_merge_base: bool,
        #[arg(long)]
        external: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Automation safety verdict for a cluster (label / id / path / URL query).
    Verdict {
        #[arg(value_name = "TARGET")]
        target: String,
        #[arg(long, default_value = "text")]
        format: AgentOutputFormat,
        #[arg(long)]
        no_merge_base: bool,
        #[arg(long)]
        external: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Compact preflight manifest for a coding agent (canonical path, alternates, verdict, warnings).
    Preflight {
        #[arg(value_name = "TARGET")]
        target: String,
        #[arg(long, default_value = "text")]
        format: AgentOutputFormat,
        #[arg(long)]
        no_merge_base: bool,
        #[arg(long)]
        external: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Check if a path is the canonical clone or a wrong duplicate checkout.
    CheckPath {
        #[arg(value_name = "PATH")]
        path: PathBuf,
        #[arg(long, default_value = "text")]
        format: AgentOutputFormat,
        #[arg(long)]
        no_merge_base: bool,
        #[arg(long)]
        external: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Token-light workspace summary for agents (`--agent`); duplicates, unsafe clusters, dirty canon.
    Summary {
        #[arg(long)]
        agent: bool,
        #[arg(value_name = "DIR")]
        workspace: Vec<PathBuf>,
        #[arg(long, default_value = "text")]
        format: AgentOutputFormat,
        #[arg(long)]
        no_merge_base: bool,
        #[arg(long)]
        external: bool,
        #[arg(long)]
        profile: Option<String>,
    },
    /// Discover local repos and optionally ingest GitHub metadata.
    Scan {
        /// Directories to scan for git repositories.
        #[arg(value_name = "ROOT")]
        roots: Vec<PathBuf>,
        /// GitHub user or org to ingest (requires `gh` on PATH).
        #[arg(long)]
        github_owner: Option<String>,
        /// Override `github_owner_mode` for this run (`augment` = only repos matching local remotes).
        #[arg(long, value_enum)]
        github_owner_mode: Option<GitHubOwnerModeCli>,
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
        #[arg(long, default_value = "gittriage-plan.json")]
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
    ///
    /// The plan body is always recomputed from the current inventory (same engine as `plan` / `score`), not read from the last `plan --write` file. The markdown header still compares timing against any SQLite-persisted plan rows.
    Report {
        /// Output format.
        #[arg(long, default_value = "md")]
        format: ReportFormat,
        /// Scoring profile: default, publish, open_source, security, ai_handoff.
        #[arg(long)]
        profile: Option<String>,
        /// Only include clusters with this member scope (Markdown header still summarizes full counts when set).
        #[arg(long, value_enum)]
        scope: Option<ReportScopeCli>,
        /// After rendering, persist the freshly built plan to SQLite (like `gittriage plan` without writing JSON).
        #[arg(long)]
        persist_plan: bool,
        /// No-op: documents that report always recomputes from inventory (for scripts and CI).
        #[arg(long, hide = true)]
        recompute: bool,
    },
    /// Check environment, config, database, and tool availability.
    Doctor {
        /// Output format.
        #[arg(long, default_value = "text")]
        format: DoctorFormat,
    },
    /// Preview-only: summarize clusters and proposed actions. v1 has no mutating apply — omitting `--dry-run` exits with an error on purpose.
    #[command(
        visible_alias = "preview",
        long_about = "Preview proposed plan actions without mutating any repository.\n\n\
v1 does not implement a mutating apply path; use `gittriage plan --write` and follow actions manually, or consume JSON from `export` / `serve`. \
A future release may add guarded filesystem or git operations behind explicit opt-in flags."
    )]
    Apply {
        /// Required in v1 — there is no mutating apply yet; this flag selects the preview path.
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
    /// Restore inventory from a `gittriage export` JSON file.
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
        /// Only list clusters in this member-scope bucket (local-only, mixed, remote-only).
        #[arg(long, value_enum)]
        scope: Option<ReportScopeCli>,
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
        #[arg(long, global = true)]
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
    /// Check AI settings: enabled flag, API key presence, optional API reachability.
    AiDoctor {
        /// GET the configured OpenAI-compatible `…/models` URL (short timeout).
        #[arg(long)]
        probe_network: bool,
    },
    /// Generate shell completions for bash, zsh, fish, elvish, or powershell.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GitHubOwnerModeCli {
    Augment,
    FullCatalog,
}

impl From<GitHubOwnerModeCli> for gittriage_config::GitHubOwnerMode {
    fn from(v: GitHubOwnerModeCli) -> Self {
        match v {
            GitHubOwnerModeCli::Augment => Self::Augment,
            GitHubOwnerModeCli::FullCatalog => Self::FullCatalog,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum ReportFormat {
    Md,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReportScopeCli {
    LocalOnly,
    Mixed,
    RemoteOnly,
}

impl From<ReportScopeCli> for ClusterScopeFilter {
    fn from(v: ReportScopeCli) -> Self {
        match v {
            ReportScopeCli::LocalOnly => ClusterScopeFilter::LocalOnly,
            ReportScopeCli::Mixed => ClusterScopeFilter::Mixed,
            ReportScopeCli::RemoteOnly => ClusterScopeFilter::RemoteOnly,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum DoctorFormat {
    Text,
    /// Stable JSON for scripts (`kind: "gittriage_doctor"`).
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum ToolsFormat {
    Text,
    /// JSON map of tool binary name → on PATH (`kind: "gittriage_tools"`).
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum ApplyFormat {
    Text,
    /// JSON summary when used with `--dry-run` (`kind: "gittriage_apply_dry_run"`).
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
enum ScoreFormat {
    /// One block per cluster: headline scores and evidence count.
    Text,
    /// JSON document with `clusters` (same `ClusterRecord` shape as inside `plan.json`, without actions).
    Json,
}

fn parse_scoring_profile(raw: &Option<String>) -> gittriage_plan::ScoringProfile {
    let Some(s) = raw.as_deref().map(str::trim).filter(|x| !x.is_empty()) else {
        return gittriage_plan::ScoringProfile::Default;
    };
    let x = s.to_ascii_lowercase().replace('-', "_");
    match x.as_str() {
        "default" => gittriage_plan::ScoringProfile::Default,
        "publish" | "publish_readiness" => gittriage_plan::ScoringProfile::PublishReadiness,
        "open_source" | "open_source_readiness" | "oss" => {
            gittriage_plan::ScoringProfile::OpenSourceReadiness
        }
        "security" | "security_supply_chain" | "supply_chain" => {
            gittriage_plan::ScoringProfile::SecuritySupplyChain
        }
        "ai_handoff" | "ai" => gittriage_plan::ScoringProfile::AiHandoff,
        other => {
            tracing::warn!(profile = %other, "unknown planner.scoring_profile; using default");
            gittriage_plan::ScoringProfile::Default
        }
    }
}

fn plan_build_opts(bundle: &ConfigBundle, merge_base: bool) -> gittriage_plan::PlanBuildOpts {
    let p = &bundle.config.planner;
    gittriage_plan::PlanBuildOpts {
        merge_base,
        ambiguous_cluster_threshold_pct: p.ambiguous_cluster_threshold.clamp(1, 99),
        oss_candidate_threshold: p.oss_candidate_threshold.min(100),
        archive_duplicate_canonical_min: p.archive_duplicate_threshold.min(100),
        user_intent: gittriage_plan::PlanUserIntent {
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

fn build_fresh_plan(
    db: &Database,
    bundle: &ConfigBundle,
    no_merge_base: bool,
    external: bool,
    profile: &Option<String>,
) -> Result<(InventorySnapshot, gittriage_core::PlanDocument)> {
    let snapshot = db.load_inventory()?;
    let mut local_bundle = bundle.clone();
    if let Some(ref p) = profile {
        local_bundle.config.planner.scoring_profile = Some(p.clone());
    }
    let opts = plan_build_opts(&local_bundle, !no_merge_base);
    let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if external {
        gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    Ok((snapshot, plan))
}

fn cmd_resolve(
    db: &Database,
    bundle: &ConfigBundle,
    query: String,
    format: AgentOutputFormat,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    let (snap, plan) = build_fresh_plan(db, bundle, no_merge_base, external, &profile)?;
    let out = gittriage_agent::resolve_target(&plan, &snap, &query);
    match format {
        AgentOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&out)?),
        AgentOutputFormat::Text => {
            println!("gittriage resolve — workspace truth (computed from inventory)");
            if let Some(ref e) = out.error {
                println!("  error: {e}");
                return Ok(());
            }
            println!("  query: {}", out.query);
            println!(
                "  canonical_path: {}",
                out.canonical_path.as_deref().unwrap_or("(none)")
            );
            println!("  cluster_id: {}", out.cluster_id.as_deref().unwrap_or("—"));
            println!("  label: {}", out.cluster_label.as_deref().unwrap_or("—"));
            println!(
                "  confidence: {}",
                out.confidence
                    .map(|c| format!("{c:.2}"))
                    .unwrap_or_else(|| "—".into())
            );
            println!("  automation_verdict: {:?}", out.automation_verdict);
            println!("  unsafe_for_automation: {}", out.unsafe_for_automation);
            if !out.alternates.is_empty() {
                println!("  alternates (do not edit for automation):");
                for a in &out.alternates {
                    println!("    - {a}");
                }
            }
            if !out.blocking_reasons.is_empty() {
                println!("  blocking_reasons:");
                for b in &out.blocking_reasons {
                    println!("    - {b}");
                }
            }
            if !out.why_canonical.is_empty() {
                println!("  why_canonical:");
                for w in &out.why_canonical {
                    println!("    - {w}");
                }
            }
        }
    }
    Ok(())
}

fn cmd_verdict(
    db: &Database,
    bundle: &ConfigBundle,
    target: String,
    format: AgentOutputFormat,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    let (snap, plan) = build_fresh_plan(db, bundle, no_merge_base, external, &profile)?;
    let resolved = gittriage_agent::resolve_target(&plan, &snap, &target);
    let cp = resolved
        .cluster_id
        .as_ref()
        .and_then(|id| plan.clusters.iter().find(|c| c.cluster.id == *id));
    let provenance = gittriage_agent::Provenance::from_snapshot(&snap);
    let verdict = cp
        .map(|c| gittriage_agent::automation_verdict_for_cluster(c, &snap))
        .unwrap_or_else(|| {
            gittriage_agent::automation_verdict_unresolved(
                resolved
                    .error
                    .as_deref()
                    .unwrap_or("Target did not resolve to a cluster."),
            )
        });
    match format {
        AgentOutputFormat::Json => {
            let v = serde_json::json!({
                "schema_version": 1u32,
                "kind": "gittriage_verdict",
                "generated_at": provenance.generated_at.to_rfc3339(),
                "inventory_run_id": provenance.inventory_run_id,
                "scope": provenance.scope,
                "freshness": provenance.freshness,
                "data_sources": provenance.data_sources,
                "target": target,
                "cluster_id": resolved.cluster_id,
                "verdict": verdict,
            });
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        AgentOutputFormat::Text => {
            println!("gittriage verdict — deterministic automation safety");
            println!("  target: {target}");
            if resolved.error.is_some() {
                println!("  (could not resolve cluster — verdict is conservative block)");
            }
            println!("  safe_to_read: {}", verdict.safe_to_read);
            println!("  safe_to_index: {}", verdict.safe_to_index);
            println!("  safe_to_modify: {}", verdict.safe_to_modify);
            println!("  safe_to_commit: {}", verdict.safe_to_commit);
            println!("  safe_to_archive: {}", verdict.safe_to_archive);
            println!("  human_review_required: {}", verdict.human_review_required);
            println!(
                "  unsafe_for_automation: {}  ({:?})",
                verdict.unsafe_for_automation, verdict.automation_verdict
            );
            for b in &verdict.blocking_reasons {
                println!("  - {b}");
            }
        }
    }
    Ok(())
}

fn cmd_preflight(
    db: &Database,
    bundle: &ConfigBundle,
    target: String,
    format: AgentOutputFormat,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    let (snap, plan) = build_fresh_plan(db, bundle, no_merge_base, external, &profile)?;
    let out = gittriage_agent::preflight(&plan, &snap, &target);
    match format {
        AgentOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&out)?),
        AgentOutputFormat::Text => {
            println!("gittriage preflight — agent manifest");
            println!("  target: {}", out.target);
            println!(
                "  canonical_path: {}",
                out.canonical_path.as_deref().unwrap_or("(none)")
            );
            println!(
                "  unsafe_for_automation: {}",
                out.verdict.unsafe_for_automation
            );
            println!("  recommended_next_action: {}", out.recommended_next_action);
            if !out.blocked_paths.is_empty() {
                println!("  blocked_paths / alternates:");
                for p in &out.blocked_paths {
                    println!("    - {p}");
                }
            }
            if !out.warnings.is_empty() {
                println!("  warnings:");
                for w in &out.warnings {
                    println!("    - {w}");
                }
            }
        }
    }
    Ok(())
}

fn cmd_check_path(
    db: &Database,
    bundle: &ConfigBundle,
    path: PathBuf,
    format: AgentOutputFormat,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    let (snap, plan) = build_fresh_plan(db, bundle, no_merge_base, external, &profile)?;
    let out = gittriage_agent::check_path(&plan, &snap, &path);
    match format {
        AgentOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&out)?),
        AgentOutputFormat::Text => {
            println!("gittriage check-path — wrong-clone detection");
            println!("  path: {}", out.path);
            println!("  disposition: {:?}", out.disposition);
            println!("  is_wrong_clone: {}", out.is_wrong_clone);
            println!(
                "  canonical_path: {}",
                out.canonical_path.as_deref().unwrap_or("(none)")
            );
            println!("  {}", out.guidance);
            println!(
                "  unsafe_for_automation: {}",
                out.verdict.unsafe_for_automation
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_summary_agent(
    db: &Database,
    bundle: &ConfigBundle,
    agent: bool,
    workspace: Vec<PathBuf>,
    format: AgentOutputFormat,
    no_merge_base: bool,
    external: bool,
    profile: Option<String>,
) -> Result<()> {
    anyhow::ensure!(
        agent,
        "`gittriage summary` requires `--agent` (compact deterministic rollup for coding agents)"
    );
    let (snap, plan) = build_fresh_plan(db, bundle, no_merge_base, external, &profile)?;
    let out = gittriage_agent::agent_summary(&plan, &snap, &workspace);
    match format {
        AgentOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&out)?),
        AgentOutputFormat::Text => {
            println!("gittriage summary --agent");
            println!(
                "  clusters_considered: {}  unsafe_for_automation: {}",
                out.total_clusters_considered, out.total_unsafe_for_automation
            );
            println!("  canonical_paths: {}", out.canonical_paths.len());
            println!("  duplicate_groups: {}", out.duplicate_groups.len());
            if !out.nested_repo_warnings.is_empty() {
                println!("  nested_repo_warnings: {}", out.nested_repo_warnings.len());
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let bundle = ConfigBundle::load(cli.config.as_deref())?;
    let mut db = Database::open(&bundle.effective_db_path)?;

    match cli.command {
        Commands::Resolve {
            query,
            format,
            no_merge_base,
            external,
            profile,
        } => cmd_resolve(
            &db,
            &bundle,
            query,
            format,
            no_merge_base,
            external,
            profile,
        ),
        Commands::Verdict {
            target,
            format,
            no_merge_base,
            external,
            profile,
        } => cmd_verdict(
            &db,
            &bundle,
            target,
            format,
            no_merge_base,
            external,
            profile,
        ),
        Commands::Preflight {
            target,
            format,
            no_merge_base,
            external,
            profile,
        } => cmd_preflight(
            &db,
            &bundle,
            target,
            format,
            no_merge_base,
            external,
            profile,
        ),
        Commands::CheckPath {
            path,
            format,
            no_merge_base,
            external,
            profile,
        } => cmd_check_path(&db, &bundle, path, format, no_merge_base, external, profile),
        Commands::Summary {
            agent,
            workspace,
            format,
            no_merge_base,
            external,
            profile,
        } => cmd_summary_agent(
            &db,
            &bundle,
            agent,
            workspace,
            format,
            no_merge_base,
            external,
            profile,
        ),
        Commands::Scan {
            roots,
            github_owner,
            github_owner_mode,
        } => cmd_scan(&mut db, &bundle, roots, github_owner, github_owner_mode),
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
        Commands::Report {
            format,
            profile,
            scope,
            persist_plan,
            recompute,
        } => cmd_report(
            &mut db,
            &bundle,
            format,
            profile,
            scope,
            persist_plan,
            recompute,
        ),
        Commands::Doctor { format } => cmd_doctor(&bundle, format),
        Commands::Apply { dry_run, format } => cmd_apply(&db, &bundle, dry_run, format),
        Commands::Serve { port, listen } => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("tokio runtime")?;
            rt.block_on(gittriage_api::serve(
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
            scope,
        } => cmd_tui(&db, &bundle, no_merge_base, external, scope),
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
        Commands::AiDoctor { probe_network } => cmd_ai_doctor(&bundle, probe_network),
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "gittriage",
                &mut std::io::stdout(),
            );
            Ok(())
        }
    }
}

fn cmd_scan(
    db: &mut Database,
    bundle: &ConfigBundle,
    roots: Vec<PathBuf>,
    github_owner: Option<String>,
    github_owner_mode_cli: Option<GitHubOwnerModeCli>,
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
        gittriage_config::ScanMode::GitOnly => gittriage_scan::ScanMode::GitOnly,
        gittriage_config::ScanMode::ProjectRoots => gittriage_scan::ScanMode::ProjectRoots,
    };

    let t_scan = std::time::Instant::now();
    let owner_mode = github_owner_mode_cli
        .map(Into::into)
        .unwrap_or(bundle.config.github_owner_mode);

    let scan_outcome = gittriage_scan::scan_roots(
        &resolved_roots,
        &gittriage_scan::ScanOptions {
            respect_gitignore: config.scan.respect_gitignore,
            include_hidden: config.include_hidden,
            max_readme_bytes: config.scan.max_readme_bytes,
            max_hash_files: config.scan.max_hash_files,
            scan_mode,
            max_depth: config.scan.max_depth,
            include_nested_git: config.scan.include_nested_git,
        },
    )?;
    let mut clones = scan_outcome.clones;
    let scan_ms = t_scan.elapsed().as_millis();

    let t_enrich = std::time::Instant::now();
    let mut remotes = Vec::new();
    let mut links = Vec::new();

    for clone in &mut clones {
        let path = PathBuf::from(&clone.path);
        if path.join(".git").exists() {
            if let Ok(git_remotes) = gittriage_git::enrich_clone(&path, clone) {
                for remote in git_remotes {
                    let rid = format!("remote-local-{}", uuid::Uuid::new_v4());
                    remotes.push(gittriage_core::RemoteRecord {
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
    let mut github_remotes: Vec<gittriage_core::RemoteRecord> = match &gh_owner {
        Some(owner) => gittriage_github::ingest_owner(owner).unwrap_or_default(),
        None => vec![],
    };

    if gh_owner.is_some() && owner_mode == gittriage_config::GitHubOwnerMode::Augment {
        let local_urls: std::collections::HashSet<String> = remotes
            .iter()
            .filter(|r| r.provider == "local-git")
            .map(|r| r.normalized_url.clone())
            .collect();
        github_remotes.retain(|r| local_urls.contains(&r.normalized_url));
    }

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

    let skipped_nested: Vec<String> = scan_outcome
        .skipped_nested_git
        .iter()
        .map(|p| p.display().to_string())
        .collect();
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
        stats: if skipped_nested.is_empty() {
            None
        } else {
            Some(gittriage_core::RunScanStats {
                skipped_nested_git: skipped_nested,
            })
        },
    };

    let t_db = std::time::Instant::now();
    db.prepare_for_scan_persist()
        .context("prepare database for scan (clear prior inventory + plan)")?;
    db.save_run(&run)?;
    db.save_clones(&run.id, &clones)?;
    db.save_remotes(&remotes)?;
    db.replace_clone_remote_links(&links)?;
    let db_ms = t_db.elapsed().as_millis();

    let n_git = clones.iter().filter(|c| c.is_git).count();
    let total_size: u64 = clones.iter().filter_map(|c| c.size_bytes).sum();

    println!("scan complete ({:.1}s)", t0.elapsed().as_secs_f64());
    println!();
    if matches!(
        config.scan.scan_mode,
        gittriage_config::ScanMode::ProjectRoots
    ) {
        println!(
            "  project roots  {:>5}  ({} git checkouts, {} manifest-only)",
            clones.len(),
            n_git,
            clones.len().saturating_sub(n_git)
        );
    } else {
        println!("  git checkouts  {:>5}  (scan_mode git_only)", clones.len());
    }
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
    if !scan_outcome.skipped_nested_git.is_empty() {
        eprintln!();
        eprintln!(
            "  warning: skipped nested git repos under another root (set scan.include_nested_git = true to scan them):"
        );
        for p in &scan_outcome.skipped_nested_git {
            eprintln!("    - {}", p.display());
        }
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
        let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
        if external {
            gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
        }
        Some(plan)
    } else {
        None
    };

    let mut doc = serde_json::json!({
        "schema_version": 1,
        "kind": "gittriage_inventory_export_v1",
        "exported_at": Utc::now().to_rfc3339(),
        "generated_by": format!("gittriage {}", env!("CARGO_PKG_VERSION")),
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
    let mut snapshot = parse_inventory_import(&bytes)?;
    snapshot.refresh_semantics();
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
    let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if external {
        gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    explain::run_explain(&snapshot, &plan, &target, format)?;

    if ai {
        if !bundle.config.ai.enabled {
            eprintln!(
                "Note: skipped AI narrative (ai.enabled is false in config). Deterministic output above is complete."
            );
            return Ok(());
        }
        if gittriage_ai::resolve_api_key().is_none() {
            eprintln!(
                "Note: skipped AI narrative (set GITTRIAGE_AI_API_KEY or OPENAI_API_KEY). Deterministic output above is complete."
            );
            return Ok(());
        }
        let ai_cfg = gittriage_ai::AiConfig {
            enabled: bundle.config.ai.enabled,
            api_base: bundle.config.ai.api_base.clone(),
            model: bundle.config.ai.model.clone(),
            max_tokens: bundle.config.ai.max_tokens,
            temperature: bundle.config.ai.temperature,
        };
        let cp = explain::cluster_plan_for_target(&plan, &target)?;
        let rt = tokio::runtime::Runtime::new()?;
        let narrative = rt.block_on(gittriage_ai::explain_cluster(&ai_cfg, cp))?;
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
    if !bundle.config.ai.enabled {
        eprintln!("Note: skipped AI plan summary (ai.enabled is false in config).");
        return Ok(());
    }
    if gittriage_ai::resolve_api_key().is_none() {
        eprintln!("Note: skipped AI plan summary (no GITTRIAGE_AI_API_KEY / OPENAI_API_KEY).");
        return Ok(());
    }

    let ai_cfg = gittriage_ai::AiConfig {
        enabled: bundle.config.ai.enabled,
        api_base: bundle.config.ai.api_base.clone(),
        model: bundle.config.ai.model.clone(),
        max_tokens: bundle.config.ai.max_tokens,
        temperature: bundle.config.ai.temperature,
    };

    let snapshot = db.load_inventory()?;
    let opts = plan_build_opts(bundle, !no_merge_base);
    let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if external {
        gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }

    let rt = tokio::runtime::Runtime::new()?;
    let summary = rt.block_on(gittriage_ai::summarize_plan(&ai_cfg, &plan))?;
    println!("── AI plan summary (model-generated, not deterministic) ──\n");
    println!("{summary}");
    Ok(())
}

fn cmd_ai_doctor(bundle: &ConfigBundle, probe_network: bool) -> Result<()> {
    println!("AI configuration (gittriage ai-doctor)");
    println!();
    println!("  ai.enabled: {}", bundle.config.ai.enabled);
    println!(
        "  API key: {}",
        if gittriage_ai::resolve_api_key().is_some() {
            "set (GITTRIAGE_AI_API_KEY or OPENAI_API_KEY)"
        } else {
            "missing"
        }
    );
    println!("  api_base: {}", bundle.config.ai.api_base);
    println!("  model: {}", bundle.config.ai.model);
    println!();
    if probe_network {
        if !bundle.config.ai.enabled {
            println!("  network probe: skipped (ai.enabled is false)");
        } else {
            let base = bundle.config.ai.api_base.trim_end_matches('/');
            let url = format!("{base}/models");
            println!("  network probe: GET {url}");
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .context("build HTTP client for ai-doctor")?;
            let mut req = client.get(&url);
            if let Some(ref k) = gittriage_ai::resolve_api_key() {
                req = req.bearer_auth(k);
            }
            match req.send() {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        println!("  network probe: OK ({status})");
                    } else {
                        println!(
                            "  network probe: reachable but error status {status} (check api_base, model, and auth)"
                        );
                    }
                }
                Err(e) => println!("  network probe: FAILED ({e:#})"),
            }
        }
    } else {
        println!(
            "  (pass `--probe-network` for a short GET to `{}/models`)",
            bundle.config.ai.api_base.trim_end_matches('/')
        );
    }
    println!();
    println!("Use `gittriage explain --ai …` or `gittriage ai-summary` after enabling AI and setting a key.");
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
    let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if external {
        gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    match format {
        ScoreFormat::Text => {
            use gittriage_core::ClusterStatus;
            let n_amb = plan
                .clusters
                .iter()
                .filter(|cp| matches!(cp.cluster.status, ClusterStatus::Ambiguous))
                .count();
            let n_actions: usize = plan.clusters.iter().map(|cp| cp.actions.len()).sum();
            println!(
                "gittriage score  {} clusters, {} actions, {} ambiguous  (rules v{}, {:.1}s)",
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
                println!("(no clusters — run `gittriage scan` first)");
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
                "kind": "gittriage_scores",
                "generated_at": Utc::now().to_rfc3339(),
                "generated_by": format!("gittriage {}", env!("CARGO_PKG_VERSION")),
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
    let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if external {
        gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
        let probes = gittriage_adapters::probe_all();
        let on_path = probes.iter().filter(|(_, ok)| *ok).count();
        let ev_rows = gittriage_adapters::count_adapter_evidence(&plan);
        println!(
            "  external   {:>5} / {} tools on PATH; {} adapter evidence rows",
            on_path,
            probes.len(),
            ev_rows
        );
        if let Some(ref meta) = plan.external_adapter_run {
            println!(
                "  adapter_run tools_on_path={} canonical_roots={} spawn_attempts={} evidence_attached={} skipped_not_dir={} timeout_or_nonzero_exit={}",
                meta.tools_on_path,
                meta.canonical_clone_roots_considered,
                meta.tool_spawn_attempts,
                meta.evidence_items_attached,
                meta.skipped_clone_path_not_directory,
                meta.runs_timeout_or_nonzero_exit
            );
        }
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
        .filter(|a| matches!(a.priority, gittriage_core::Priority::High))
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
    db: &mut Database,
    bundle: &ConfigBundle,
    format: ReportFormat,
    profile: Option<String>,
    scope: Option<ReportScopeCli>,
    persist_plan: bool,
    recompute: bool,
) -> Result<()> {
    if recompute {
        eprintln!(
            "note: `gittriage report` always recomputes the plan from the current inventory; the markdown header compares timing against any SQLite-persisted snapshot."
        );
    }
    let snapshot: InventorySnapshot = db.load_inventory()?;
    let mut local_bundle = bundle.clone();
    if let Some(ref p) = profile {
        local_bundle.config.planner.scoring_profile = Some(p.clone());
    }
    let opts = plan_build_opts(&local_bundle, true);
    let full_plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if persist_plan {
        db.persist_plan(&full_plan)
            .context("persist plan from `gittriage report --persist-plan`")?;
    }
    let (n_persist, persist_at) = db.persisted_plan_cluster_stats()?;
    let scope_filter = scope.map(Into::into);
    let display_plan = gittriage_report::filter_plan_by_scope(&full_plan, scope_filter);
    let (l, m, r, e) = gittriage_report::scope_breakdown(&full_plan);
    let skipped_nested = snapshot
        .run
        .as_ref()
        .and_then(|run| run.stats.as_ref())
        .map(|s| s.skipped_nested_git.clone())
        .unwrap_or_default();
    match format {
        ReportFormat::Md => {
            let agent_headings = scope_filter.is_none();
            let md = gittriage_report::render_markdown_with(
                &display_plan,
                gittriage_report::ReportExtras {
                    scope_filter,
                    unfiltered_cluster_count: scope_filter.map(|_| full_plan.clusters.len()),
                    local_only_count: l,
                    mixed_count: m,
                    remote_only_count: r,
                    empty_count: e,
                    prefer_local_involved_sections: scope_filter.is_none(),
                    latest_scan_started_at: snapshot.run.as_ref().map(|r| r.started_at),
                    sqlite_persisted: Some((n_persist, persist_at)),
                    skipped_nested_git_paths: skipped_nested,
                    inventory_snapshot: agent_headings.then(|| snapshot.clone()),
                    agent_section_plan: agent_headings.then(|| full_plan.clone()),
                    agent_preflight_headings: agent_headings,
                },
            )?;
            println!("{md}");
        }
        ReportFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&display_plan)?);
        }
    }
    Ok(())
}

fn cmd_tui(
    db: &Database,
    bundle: &ConfigBundle,
    no_merge_base: bool,
    external: bool,
    scope: Option<ReportScopeCli>,
) -> Result<()> {
    let snapshot = db.load_inventory()?;
    let opts = plan_build_opts(bundle, !no_merge_base);
    let mut plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
    if external {
        gittriage_adapters::attach_external_evidence(&mut plan, &snapshot)?;
    }
    let config_pins = bundle
        .config
        .planner
        .canonical_pins
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let export_path = bundle.config.tui_export_path.clone().unwrap_or_else(|| {
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ");
        PathBuf::from(format!("gittriage-plan-tui-export-{ts}.json"))
    });
    let skipped_nested = snapshot
        .run
        .as_ref()
        .and_then(|run| run.stats.as_ref())
        .map(|s| s.skipped_nested_git.clone())
        .unwrap_or_default();
    gittriage_tui::run(
        plan,
        snapshot,
        gittriage_tui::TuiConfig {
            config_pins,
            export_path,
            scope_filter: scope.map(Into::into),
            skipped_nested_git_paths: skipped_nested,
        },
    )
}

fn cmd_tools(format: ToolsFormat) -> Result<()> {
    let probes = gittriage_adapters::probe_all();
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
                "kind": "gittriage_tools",
                "generated_at": Utc::now().to_rfc3339(),
                "generated_by": format!("gittriage {}", env!("CARGO_PKG_VERSION")),
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
        "v1 is plan-only: use `gittriage apply --dry-run` (mutating apply is disabled)"
    );
    let snapshot = db.load_inventory()?;
    let opts = plan_build_opts(bundle, true);
    let plan = gittriage_plan::build_plan_with(&snapshot, opts)?;
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
                "kind": "gittriage_apply_dry_run",
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

    let optional_scanners: serde_json::Map<String, serde_json::Value> =
        gittriage_adapters::probe_all()
            .into_iter()
            .map(|(t, ok)| (t.bin_name().to_string(), serde_json::json!(ok)))
            .collect();

    match format {
        DoctorFormat::Json => {
            let doc = serde_json::json!({
                "schema_version": 1,
                "kind": "gittriage_doctor",
                "generated_at": Utc::now().to_rfc3339(),
                "generated_by": format!("gittriage {}", env!("CARGO_PKG_VERSION")),
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
                    .unwrap_or("(defaults — no gittriage.toml found)")
            );
            if bundle.source_path.is_none() {
                println!(
                    "  → tip: copy `gittriage.toml.example` to `./gittriage.toml` or set `{}`",
                    gittriage_config::ENV_GITTRIAGE_CONFIG
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
                    "  → tip: set `default_roots` in gittriage.toml or pass paths to `gittriage scan <path> ...`"
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
                    "  → fix: install git; GitTriage uses it for clone metadata and merge-base evidence"
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
            let scanners: Vec<_> = gittriage_adapters::probe_all()
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
                    "  → optional: install tools listed in `gittriage tools` for `plan --external`"
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
