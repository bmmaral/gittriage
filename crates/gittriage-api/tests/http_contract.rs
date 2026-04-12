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

/// `/v2/agent/*` — stable machine-facing surface (empty inventory still returns JSON).
#[tokio::test]
async fn v2_agent_resolve_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/resolve?query=foo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["kind"], "gittriage_resolve");
    assert_eq!(v["query"], "foo");
    assert!(v.get("generated_at").is_some());
    assert!(v.get("freshness").is_some());
    assert!(v.get("unsafe_for_automation").is_some());
}

#[tokio::test]
async fn v2_agent_verdict_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/verdict?target=bar")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["kind"], "gittriage_verdict");
    assert_eq!(v["target"], "bar");
    assert!(v.get("verdict").is_some());
    assert!(v["verdict"].get("unsafe_for_automation").is_some());
}

#[tokio::test]
async fn v2_agent_preflight_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/preflight?target=baz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["kind"], "gittriage_preflight");
    // `AutomationVerdict` is flattened into the preflight document.
    assert!(v.get("unsafe_for_automation").is_some());
    assert!(v.get("automation_verdict").is_some());
}

#[tokio::test]
async fn v2_agent_check_path_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/check-path?path=/tmp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["kind"], "gittriage_check_path");
    assert!(v.get("disposition").is_some());
}

#[tokio::test]
async fn v2_agent_summary_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["kind"], "gittriage_agent_summary");
}

#[tokio::test]
async fn v2_agent_duplicate_groups_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/duplicate-groups")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["kind"], "gittriage_duplicate_groups");
    assert!(v["groups"].is_array());
}

#[tokio::test]
async fn v2_agent_unsafe_targets_ok_shape() {
    let (_dir, state) = test_state();
    let app = router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/v2/agent/unsafe-targets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["kind"], "gittriage_unsafe_targets");
    assert!(v["targets"].is_array());
}
