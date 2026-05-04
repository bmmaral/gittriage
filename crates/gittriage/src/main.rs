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
        /// Fail with a non-zero exit code if GitHub ingestion fails.
        #[arg(long)]
        fail_on_ingest_error: bool,
        /// Fail with a non-zero exit code if local git enrichment fails.
        #[arg(long)]
        fail_on_enrich_error: bool,
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
    let result = gittriage_agent::resolve_target(&plan, &snap, &query);
    match format {
        AgentOutputFormat::Json => match result {
            Ok(out) => println!("{}", serde_json::to_string_pretty(&out)?),
            Err(e) => {
                println!("{}", serde_json::to_string_pretty(&e)?);
                std::process::exit(1);
            }
        },
        AgentOutputFormat::Text => {
            let out = result.map_err(|e| anyhow::anyhow!(e.message))?;
            println!("gittriage resolve — workspace truth (computed from inventory)");
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
        .as_ref()
        .ok()
        .and_then(|r| r.cluster_id.as_ref())
        .and_then(|id| plan.clusters.iter().find(|c| c.cluster.id == *id));

    let provenance = gittriage_agent::Provenance::from_snapshot(&snap);
    let verdict = cp
        .map(|c| gittriage_agent::automation_verdict_for_cluster(c, &snap))
        .unwrap_or_else(|| {
            gittriage_agent::automation_verdict_unresolved(
                &resolved.err().map(|e| e.message).unwrap_or_else(|| {
                    "Target did not resolve to a cluster.".to_string()
                }),
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
                "cluster_id": resolved.ok().and_then(|r| r.cluster_id),
                "verdict": verdict,
            });
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        AgentOutputFormat::Text => {
            println!("gittriage verdict — deterministic automation safety");
            println!("  target: {target}");
            if resolved.is_err() {
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
            fail_on_ingest_error,
            fail_on_enrich_error,
        } => cmd_scan(
            &mut db,
            &bundle,
            roots,
            github_owner,
            github_owner_mode,
            fail_on_ingest_error,
            fail_on_enrich_error,
        ),
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
    fail_on_ingest_error: bool,
    fail_on_enrich_error: bool,
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
            match gittriage_git::enrich_clone(&path, clone) {
                Ok(git_remotes) => {
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
                },
                Err(e) => {
                    eprintln!("warning: failed to enrich local git clone at {}: {e}", clone.path);
                    if fail_on_enrich_error {
                        anyhow::bail!("local enrich failed for {}: {e}", clone.path);
                    }
                }
            }
        }
    }
    let enrich_ms = t_enrich.elapsed().as_millis();

    let gh_owner = github_owner.clone().or_else(|| config.github_owner.clone());
    let t_gh = std::time::Instant::now();
    let mut github_remotes: Vec<gittriage_core::RemoteRecord> = match &gh_owner {
        Some(owner) => {
            match gittriage_github::ingest_owner(owner) {
                Ok(remotes) => remotes,
                Err(e) => {
                    eprintln!("github ingest error: {e}");
                    if fail_on_ingest_error {
                        anyhow::bail!("GitHub ingest failed: {e}");
                    }
                    vec![]
                }
            }
        },
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