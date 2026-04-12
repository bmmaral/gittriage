//! Rescanning after `persist_plan` must not hit SQLite FK errors (see P0-2 audit).
use chrono::Utc;
use gittriage_core::{
    CloneRecord, ClusterPlan, ClusterRecord, ClusterStatus, InventorySnapshot, PlanDocument,
    RunRecord, ScoreBundle,
};
use gittriage_db::Database;
use tempfile::tempdir;

#[test]
fn scan_after_plan_replace_does_not_hit_foreign_keys() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("scan.db");
    let mut db = Database::open(&db_path).expect("open");

    let snap1 = InventorySnapshot {
        run: None,
        clones: vec![CloneRecord {
            id: "clone-a".into(),
            path: "/tmp/gittriage-fk-test/a".into(),
            display_name: "a".into(),
            is_git: true,
            head_oid: None,
            active_branch: None,
            default_branch: None,
            is_dirty: false,
            last_commit_at: None,
            size_bytes: None,
            manifest_kind: None,
            readme_title: None,
            license_spdx: None,
            fingerprint: None,
            has_lockfile: false,
            has_ci: false,
            has_tests_dir: false,
        }],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    db.prepare_for_scan_persist().expect("prepare 1");
    let run1 = RunRecord {
        id: "run-1".into(),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
        roots: vec!["/tmp".into()],
        github_owner: None,
        version: "test".into(),
        stats: None,
    };
    db.save_run(&run1).expect("save_run 1");
    db.save_clones(&run1.id, &snap1.clones)
        .expect("save_clones 1");
    db.save_remotes(&[]).expect("save_remotes 1");
    db.replace_clone_remote_links(&[]).expect("links 1");

    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 1,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        external_adapter_run: None,
        clusters: vec![ClusterPlan {
            cluster: ClusterRecord {
                id: "cluster-1".into(),
                cluster_key: "name:a".into(),
                label: "a".into(),
                status: ClusterStatus::Resolved,
                confidence: 0.9,
                canonical_clone_id: Some("clone-a".into()),
                canonical_remote_id: None,
                members: vec![],
                evidence: vec![],
                scores: ScoreBundle::default(),
            },
            actions: vec![],
        }],
    };
    db.persist_plan(&plan).expect("persist_plan");

    let snap2 = InventorySnapshot {
        run: None,
        clones: vec![CloneRecord {
            id: "clone-b".into(),
            path: "/tmp/gittriage-fk-test/a".into(),
            display_name: "a".into(),
            is_git: true,
            head_oid: None,
            active_branch: None,
            default_branch: None,
            is_dirty: false,
            last_commit_at: None,
            size_bytes: None,
            manifest_kind: None,
            readme_title: None,
            license_spdx: None,
            fingerprint: None,
            has_lockfile: false,
            has_ci: false,
            has_tests_dir: false,
        }],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    db.prepare_for_scan_persist().expect("prepare 2");
    let run2 = RunRecord {
        id: "run-2".into(),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
        roots: vec!["/tmp".into()],
        github_owner: None,
        version: "test".into(),
        stats: None,
    };
    db.save_run(&run2).expect("save_run 2");
    db.save_clones(&run2.id, &snap2.clones)
        .expect("save_clones 2 must succeed (same path, new clone id)");
}
