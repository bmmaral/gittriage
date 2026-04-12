//! Read-only HTTP contract tests for `gittriage serve` (same router as production).

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use gittriage_api::{router, AppState};
use gittriage_config::ConfigBundle;
use gittriage_db::Database;
use std::sync::Arc;
use tower::ServiceExt;

fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("state.db");
    Database::open(&db_path).expect("open db");
    let bundle = ConfigBundle {
        config: gittriage_config::GitTriageConfig::default(),
        source_path: None,
        effective_db_path: db_path.clone(),
    };
    let state = Arc::new(AppState { db_path, bundle });
    (dir, state)
}

#[tokio::test]
async fn get_health_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["service"], "gittriage-api");
    assert!(v["version"].as_str().is_some());
}

#[tokio::test]
async fn get_inventory_counts_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v1/inventory")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["clones"], 0);
    assert_eq!(v["remotes"], 0);
    assert_eq!(v["links"], 0);
}

#[tokio::test]
async fn get_plan_has_expected_top_level_keys() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v1/plan")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v.get("schema_version").is_some());
    assert!(v.get("scoring_rules_version").is_some());
    assert!(v.get("clusters").is_some());
    assert!(v["clusters"].is_array());
}

#[tokio::test]
async fn unknown_route_404() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v1/no-such-route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
