use chrono::Utc;
use nexus_core::{
    CloneRecord, ClusterPlan, ClusterRecord, ClusterStatus, InventorySnapshot, PlanDocument,
    ScoreBundle,
};
use nexus_db::Database;
use tempfile::tempdir;

#[test]
fn replace_inventory_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("inv.db");
    let mut db = Database::open(&db_path).expect("open");

    let snap = InventorySnapshot {
        run: None,
        clones: vec![CloneRecord {
            id: "c1".into(),
            path: "/tmp/a".into(),
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
        }],
        remotes: vec![],
        links: vec![],
    };

    db.replace_inventory_snapshot(&snap, "test")
        .expect("replace");
    let loaded = db.load_inventory().expect("load");
    assert!(
        loaded.run.is_some(),
        "import/replace inserts a synthetic run"
    );
    assert_eq!(loaded.clones.len(), 1);
    assert_eq!(loaded.clones[0].id, "c1");
    assert_eq!(loaded.clones[0].path, "/tmp/a");
}

#[test]
fn replace_inventory_clears_persisted_plan() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("plan.db");
    let mut db = Database::open(&db_path).expect("open");

    let snap = InventorySnapshot {
        run: None,
        clones: vec![CloneRecord {
            id: "c1".into(),
            path: "/p".into(),
            display_name: "p".into(),
            is_git: false,
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
        }],
        remotes: vec![],
        links: vec![],
    };
    db.replace_inventory_snapshot(&snap, "test").expect("seed");

    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        clusters: vec![ClusterPlan {
            cluster: ClusterRecord {
                id: "cl-1".into(),
                cluster_key: "key".into(),
                label: "Label".into(),
                status: ClusterStatus::Resolved,
                confidence: 0.9,
                canonical_clone_id: None,
                canonical_remote_id: None,
                members: vec![],
                evidence: vec![],
                scores: ScoreBundle::default(),
            },
            actions: vec![],
        }],
    };
    db.persist_plan(&plan).expect("persist");
    assert_eq!(db.cluster_count().expect("count"), 1);

    db.replace_inventory_snapshot(&snap, "test")
        .expect("replace again");
    assert_eq!(db.cluster_count().expect("count after"), 0);
}
