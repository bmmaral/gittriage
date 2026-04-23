use chrono::Utc;
use gittriage_core::{CloneRecord, InventorySnapshot};
use gittriage_plan::{build_plan_with, PlanBuildOpts};
use std::fs;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "gittriage")
        .env("GIT_AUTHOR_EMAIL", "gittriage@example.com")
        .env("GIT_COMMITTER_NAME", "gittriage")
        .env("GIT_COMMITTER_EMAIL", "gittriage@example.com")
        .status()
        .unwrap_or_else(|e| panic!("git {:?} failed: {e}", args));
    assert!(
        status.success(),
        "git {:?} failed in {}",
        args,
        dir.display()
    );
}

fn git_clone(repo_path: &Path, into_path: &Path) {
    let status = Command::new("git")
        .args([
            "clone",
            repo_path.to_str().unwrap(),
            into_path.to_str().unwrap(),
        ])
        .env("GIT_AUTHOR_NAME", "gittriage")
        .env("GIT_AUTHOR_EMAIL", "gittriage@example.com")
        .env("GIT_COMMITTER_NAME", "gittriage")
        .env("GIT_COMMITTER_EMAIL", "gittriage@example.com")
        .status()
        .unwrap_or_else(|e| panic!("git clone failed: {e}"));
    assert!(status.success(), "git clone failed");
}

fn write_commit(dir: &Path, filename: &str, content: &str, msg: &str) {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("write file");
    git(dir, &["add", filename]);
    git(dir, &["commit", "-m", msg]);
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path).expect("create repo dir");
    let status = Command::new("git")
        .args(["init", path.to_str().unwrap()])
        .status()
        .expect("git init");
    assert!(status.success(), "git init failed");
}

#[test]
fn merge_base_evidence_is_attached_best_effort() {
    let base = std::env::temp_dir();
    let repo_a = base.join(format!("gittriage-mb-a-{}", Uuid::new_v4()));
    let repo_b = base.join(format!("gittriage-mb-b-{}", Uuid::new_v4()));

    init_repo(&repo_a);
    write_commit(&repo_a, "a.txt", "v1", "c1");

    git_clone(&repo_a, &repo_b);
    write_commit(&repo_b, "a.txt", "v2", "c2-b");

    // Make sure repo_a has repo_b's HEAD object so merge-base can succeed.
    let file_url = format!("file://{}", repo_b.display());
    git(&repo_a, &["fetch", &file_url, "HEAD"]);

    let clone_a = CloneRecord {
        id: "clone-a".into(),
        path: repo_a.to_string_lossy().to_string(),
        display_name: "proj".into(),
        is_git: true,
        head_oid: None,
        active_branch: None,
        default_branch: None,
        is_dirty: false,
        last_commit_at: Some(Utc::now()),
        upstream_tracking: None,
        size_bytes: None,
        manifest_kind: None,
        readme_title: None,
        license_spdx: None,
        fingerprint: None,
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    };
    let clone_b = CloneRecord {
        id: "clone-b".into(),
        path: repo_b.to_string_lossy().to_string(),
        display_name: "proj".into(),
        is_git: true,
        head_oid: None,
        active_branch: None,
        default_branch: None,
        is_dirty: false,
        last_commit_at: Some(Utc::now()),
        upstream_tracking: None,
        size_bytes: None,
        manifest_kind: None,
        readme_title: None,
        license_spdx: None,
        fingerprint: None,
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    };

    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![clone_a.clone(), clone_b.clone()],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plan = build_plan_with(
        &snapshot,
        PlanBuildOpts {
            merge_base: true,
            ..Default::default()
        },
    )
    .expect("build plan");
    assert_eq!(plan.clusters.len(), 1);

    let mb_evidence: Vec<_> = plan.clusters[0]
        .cluster
        .evidence
        .iter()
        .filter(|e| e.kind == "merge_base")
        .collect();
    assert_eq!(mb_evidence.len(), 1, "expected merge_base evidence");

    assert!(
        (mb_evidence[0].score_delta - 0.0).abs() < 0.001
            || (mb_evidence[0].score_delta - 8.0).abs() < 0.001,
        "unexpected merge_base score_delta: {}",
        mb_evidence[0].score_delta
    );

    let plan_no = build_plan_with(
        &snapshot,
        PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    )
    .expect("build plan");
    let has_mb = plan_no.clusters[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "merge_base");
    assert!(!has_mb, "merge_base evidence should be disabled");

    // Cleanup best-effort (not required for correctness).
    let _ = fs::remove_dir_all(&repo_a);
    let _ = fs::remove_dir_all(&repo_b);
}
