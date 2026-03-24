use anyhow::Context;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use nexus_db::Database;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

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
        let db = Database::open(&path)?;
        let snap = db.load_inventory()?;
        let plan = nexus_plan::build_plan(&snap)?;
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
