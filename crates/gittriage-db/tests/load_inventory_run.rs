use chrono::{Duration, Utc};
use gittriage_core::{CloneRecord, RunRecord, RunScanStats};
use gittriage_db::Database;
use tempfile::tempdir;

#[test]
fn load_inventory_includes_latest_run() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("runs.db");
    let mut db = Database::open(&db_path).expect("open");

    let run = RunRecord {
        id: "run-1".into(),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
        roots: vec!["/tmp".into()],
        github_owner: None,
        version: "0.1.1".into(),
        stats: Some(RunScanStats {
            skipped_nested_git: vec!["/tmp/p/nested/.git".into()],
        }),
    };
    db.save_run(&run).expect("save_run");
    db.save_clones(
        &run.id,
        &[CloneRecord {
            id: "c1".into(),
            path: "/tmp/p".into(),
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
            has_lockfile: false,
            has_ci: false,
            has_tests_dir: false,
        }],
    )
    .expect("save_clones");

    let inv = db.load_inventory().expect("load");
    let r = inv.run.expect("run");
    assert_eq!(r.id, "run-1");
    assert_eq!(r.roots, vec!["/tmp".to_string()]);
    let st = r.stats.expect("stats");
    assert_eq!(st.skipped_nested_git, vec!["/tmp/p/nested/.git"]);
}

#[test]
fn load_inventory_latest_run_is_most_recent_started_at() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("runs2.db");
    let db = Database::open(&db_path).expect("open");

    let older = RunRecord {
        id: "run-old".into(),
        started_at: Utc::now() - Duration::hours(3),
        finished_at: None,
        roots: vec!["old".into()],
        github_owner: None,
        version: "a".into(),
        stats: None,
    };
    let newer = RunRecord {
        id: "run-new".into(),
        started_at: Utc::now(),
        finished_at: None,
        roots: vec!["new".into()],
        github_owner: None,
        version: "b".into(),
        stats: None,
    };
    db.save_run(&older).expect("save older");
    db.save_run(&newer).expect("save newer");

    let inv = db.load_inventory().expect("load");
    assert_eq!(inv.run.map(|r| r.id), Some("run-new".into()));
}
