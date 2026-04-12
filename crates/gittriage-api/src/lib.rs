use anyhow::Context;
use axum::extract::Query;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use gittriage_db::Database;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

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
        _ => gittriage_plan::ScoringProfile::Default,
    }
}

fn plan_build_opts_from_bundle(
    bundle: &gittriage_config::ConfigBundle,
) -> gittriage_plan::PlanBuildOpts {
    let p = &bundle.config.planner;
    gittriage_plan::PlanBuildOpts {
        merge_base: true,
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

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub bundle: gittriage_config::ConfigBundle,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/plan", get(plan_json))
        .route("/v1/inventory", get(inventory_summary))
        .route("/v2/agent/resolve", get(agent_resolve))
        .route("/v2/agent/verdict", get(agent_verdict))
        .route("/v2/agent/preflight", get(agent_preflight))
        .route("/v2/agent/check-path", get(agent_check_path))
        .route("/v2/agent/summary", get(agent_summary))
        .route("/v2/agent/duplicate-groups", get(agent_duplicate_groups))
        .route("/v2/agent/unsafe-targets", get(agent_unsafe_targets))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "service": "gittriage-api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn plan_json(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let db = Database::open(&path)?;
        let snap = db.load_inventory()?;
        let opts = plan_build_opts_from_bundle(&bundle);
        let plan = gittriage_plan::build_plan_with(&snap, opts)?;
        Ok(serde_json::to_value(&plan)?)
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

fn load_plan_blocking(
    path: PathBuf,
    bundle: gittriage_config::ConfigBundle,
) -> anyhow::Result<(
    gittriage_core::InventorySnapshot,
    gittriage_core::PlanDocument,
)> {
    let db = Database::open(&path)?;
    let snap = db.load_inventory()?;
    let opts = plan_build_opts_from_bundle(&bundle);
    let plan = gittriage_plan::build_plan_with(&snap, opts)?;
    Ok((snap, plan))
}

#[derive(Debug, Deserialize)]
pub struct AgentQueryParam {
    pub query: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentTargetParam {
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentPathParam {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentSummaryParams {
    #[serde(default)]
    pub workspace: Vec<String>,
}

async fn agent_resolve(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentQueryParam>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
        let out = gittriage_agent::resolve_target(&plan, &snap, &q.query);
        Ok(serde_json::to_value(&out)?)
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn agent_verdict(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentTargetParam>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let target = q.target.clone();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
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
        Ok(serde_json::json!({
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
        }))
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn agent_preflight(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentTargetParam>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let target = q.target.clone();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
        let out = gittriage_agent::preflight(&plan, &snap, &target);
        Ok(serde_json::to_value(&out)?)
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn agent_check_path(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentPathParam>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let p = PathBuf::from(q.path.clone());
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
        let out = gittriage_agent::check_path(&plan, &snap, &p);
        Ok(serde_json::to_value(&out)?)
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn agent_summary(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentSummaryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let roots: Vec<PathBuf> = q.workspace.iter().map(PathBuf::from).collect();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
        let out = gittriage_agent::agent_summary(&plan, &snap, &roots);
        Ok(serde_json::to_value(&out)?)
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn agent_duplicate_groups(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentSummaryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let roots: Vec<PathBuf> = q.workspace.iter().map(PathBuf::from).collect();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
        let list = gittriage_agent::list_duplicate_groups(&plan, &snap, &roots);
        Ok(serde_json::json!({
            "schema_version": 1u32,
            "kind": "gittriage_duplicate_groups",
            "groups": list,
        }))
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn agent_unsafe_targets(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AgentSummaryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let bundle = state.bundle.clone();
    let roots: Vec<PathBuf> = q.workspace.iter().map(PathBuf::from).collect();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let (snap, plan) = load_plan_blocking(path, bundle)?;
        let list = gittriage_agent::list_unsafe_targets(&plan, &snap, &roots);
        Ok(serde_json::json!({
            "schema_version": 1u32,
            "kind": "gittriage_unsafe_targets",
            "targets": list,
        }))
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

async fn inventory_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let db = Database::open(&path)?;
        let snap = db.load_inventory()?;
        Ok(serde_json::json!({
            "clones": snap.clones.len(),
            "remotes": snap.remotes.len(),
            "links": snap.links.len(),
        }))
    })
    .await
    .context("join")??;
    Ok(Json(value))
}

pub struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = %self.0, "api error");
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", self.0)).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(e.into())
    }
}

pub async fn serve(
    db_path: PathBuf,
    port: u16,
    listen: std::net::IpAddr,
    bundle: gittriage_config::ConfigBundle,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState { db_path, bundle });
    let app = router(state);
    let listener = tokio::net::TcpListener::bind((listen, port)).await?;
    tracing::info!(%listen, %port, "gittriage API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
