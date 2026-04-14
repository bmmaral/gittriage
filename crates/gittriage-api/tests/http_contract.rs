//! Read-only HTTP contract tests for `gittriage serve` (same router as production).

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use gittriage_api::{router, AppState};
use gittriage_config::ConfigBundle;
use gittriage_db::Database;
use std::fs;
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

fn test_state_with_inventory() -> (tempfile::TempDir, Arc<AppState>, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("state.db");
    let mut db = Database::open(&db_path).expect("open db");

    let repo_root = dir.path().join("ws").join("canon");
    let alt_root = dir.path().join("ws").join("canon-copy");
    for root in [&repo_root, &alt_root] {
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname='demo'\nversion='0.1.0'\n").unwrap();
        fs::write(root.join("README.md"), "# Demo Repo\n").unwrap();
        fs::write(root.join("LICENSE"), "MIT License\n").unwrap();
    }

    let c1 = gittriage_core::CloneRecord {
        id: "clone-1".into(),
        path: repo_root.to_string_lossy().to_string(),
        display_name: "canon".into(),
        is_git: true,
        head_oid: Some("head-1".into()),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty: false,
        last_commit_at: None,
        size_bytes: Some(1),
        manifest_kind: Some(gittriage_core::ManifestKind::Cargo),
        readme_title: Some("Demo Repo".into()),
        license_spdx: Some("MIT".into()),
        fingerprint: Some("fp-same".into()),
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    };
    let c2 = gittriage_core::CloneRecord {
        id: "clone-2".into(),
        path: alt_root.to_string_lossy().to_string(),
        display_name: "canon-copy".into(),
        is_git: true,
        head_oid: Some("head-2".into()),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty: false,
        last_commit_at: None,
        size_bytes: Some(1),
        manifest_kind: Some(gittriage_core::ManifestKind::Cargo),
        readme_title: Some("Demo Repo".into()),
        license_spdx: Some("MIT".into()),
        fingerprint: Some("fp-same".into()),
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    };

    let snapshot = gittriage_core::InventorySnapshot {
        run: None,
        clones: vec![c1, c2],
        remotes: vec![],
        links: vec![],
        semantics: None,
    };

    db.replace_inventory_snapshot(&snapshot, "tests")
        .expect("persist inventory");

    let bundle = ConfigBundle {
        config: gittriage_config::GitTriageConfig::default(),
        source_path: None,
        effective_db_path: db_path.clone(),
    };
    let state = Arc::new(AppState { db_path, bundle });
    (dir, state, repo_root.to_string_lossy().to_string())
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

#[tokio::test]
async fn api_agent_resolve_preflight_and_verdict_are_contract_compatible() {
    let (_dir, state, repo_root) = test_state_with_inventory();

    let resolve = {
        let app = router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v2/agent/resolve?query={repo_root}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("oneshot");
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()
    };
    assert_eq!(resolve["kind"], "gittriage_resolve");
    assert_eq!(resolve["query"], repo_root);
    assert!(resolve["canonical_path"].is_string() || resolve["canonical_path"].is_null());
    assert!(resolve.get("automation_verdict").is_some());
    assert!(resolve.get("unsafe_for_automation").is_some());

    let preflight = {
        let app = router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v2/agent/preflight?target={repo_root}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("oneshot");
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()
    };
    assert_eq!(preflight["kind"], "gittriage_preflight");
    assert_eq!(preflight["target"], repo_root);
    assert!(preflight.get("automation_verdict").is_some());
    assert!(preflight.get("unsafe_for_automation").is_some());
    assert!(preflight.get("recommended_next_action").is_some());

    let verdict = {
        let app = router(state.clone());
        let res = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v2/agent/verdict?target={repo_root}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("oneshot");
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()
    };
    assert_eq!(verdict["kind"], "gittriage_verdict");
    assert_eq!(verdict["target"], repo_root);
    assert!(verdict["verdict"].get("automation_verdict").is_some());
    assert!(verdict["verdict"].get("unsafe_for_automation").is_some());
    if let Some(pre_label) = preflight.get("automation_verdict") {
        assert_eq!(pre_label, &verdict["verdict"]["automation_verdict"]);
    }
}
