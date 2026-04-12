use chrono::Utc;
use gittriage_core::{CloneRecord, CloneRemoteLink, RemoteRecord, RunRecord};
use gittriage_db::Database;
use tempfile::tempdir;

#[test]
fn replace_clone_remote_links_replaces_existing_rows() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("state.db");
    let mut db = Database::open(&db_path).expect("open db");

    let run = RunRecord {
        id: "run-1".into(),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
        roots: vec!["/tmp/root".into()],
        github_owner: None,
        version: "0.1.1".into(),
        stats: None,
    };
    db.save_run(&run).expect("save run");

    let clone = CloneRecord {
        id: "clone-1".into(),
        path: "/tmp/clone-1".into(),
        display_name: "clone-1".into(),
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
    };
    db.save_clones(&run.id, &[clone]).expect("save clone");

    let remote = RemoteRecord {
        id: "remote-1".into(),
        provider: "local-git".into(),
        owner: None,
        name: Some("origin".into()),
        full_name: None,
        url: "https://example.com/acme/demo.git".into(),
        normalized_url: "example.com/acme/demo".into(),
        default_branch: None,
        is_fork: false,
        is_archived: false,
        is_private: false,
        pushed_at: None,
    };
    db.save_remotes(&[remote]).expect("save remote");

    let links_1 = vec![CloneRemoteLink {
        clone_id: "clone-1".into(),
        remote_id: "remote-1".into(),
        relationship: "origin".into(),
    }];

    db.replace_clone_remote_links(&links_1)
        .expect("replace links 1");
    let snap_1 = db.load_inventory().expect("load inventory 1");
    assert_eq!(snap_1.links.len(), 1);
    assert_eq!(snap_1.links[0], links_1[0]);

    let links_2 = vec![CloneRemoteLink {
        clone_id: "clone-1".into(),
        remote_id: "remote-1".into(),
        relationship: "origin2".into(),
    }];
    db.replace_clone_remote_links(&links_2)
        .expect("replace links 2");
    let snap_2 = db.load_inventory().expect("load inventory 2");
    assert_eq!(snap_2.links.len(), 1);
    assert_eq!(snap_2.links[0], links_2[0]);
}
