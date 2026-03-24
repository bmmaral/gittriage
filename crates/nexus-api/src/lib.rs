use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use nexus_db::Database;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

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
        _ => nexus_plan::ScoringProfile::Default,
    }
}

fn plan_build_opts_from_bundle(bundle: &nexus_config::ConfigBundle) -> nexus_plan::PlanBuildOpts {
    let p = &bundle.config.planner;
    nexus_plan::PlanBuildOpts {
        merge_base: true,
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

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/plan", get(plan_json))
        .route("/v1/inventory", get(inventory_summary))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "service": "nexus-api" }))
}

async fn plan_json(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let path = state.db_path.clone();
    let value = tokio::task::spawn_blocking(move || -> anyhow::Result<serde_json::Value> {
        let bundle = nexus_config::ConfigBundle::load(None)?;
        let db = Database::open(&path)?;
        let snap = db.load_inventory()?;
        let opts = plan_build_opts_from_bundle(&bundle);
        let plan = nexus_plan::build_plan_with(&snap, opts)?;
        Ok(serde_json::to_value(&plan)?)
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

pub async fn serve(db_path: PathBuf, port: u16) -> anyhow::Result<()> {
    let state = Arc::new(AppState { db_path });
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!(%port, "nexus API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
