//! Fixture-style `InventorySnapshot` cases for `resolve_clusters` (local + GitHub remote).

use chrono::Utc;
use nexus_core::{
    ActionType, CloneRecord, CloneRemoteLink, InventorySnapshot, ManifestKind, MemberKind,
    RemoteRecord,
};
use nexus_plan::{resolve_clusters, PlanBuildOpts};

fn sample_clone(id: &str, name: &str) -> CloneRecord {
    CloneRecord {
        id: id.into(),
        path: format!("/dev/{name}"),
        display_name: name.into(),
        is_git: true,
        head_oid: Some("abc".into()),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty: false,
        last_commit_at: Some(Utc::now()),
        size_bytes: None,
        manifest_kind: Some(ManifestKind::Cargo),
        readme_title: Some(name.into()),
        license_spdx: Some("MIT".into()),
        fingerprint: None,
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    }
}

fn sample_github_remote(id: &str, norm: &str) -> RemoteRecord {
    RemoteRecord {
        id: id.into(),
        provider: "github".into(),
        owner: Some("acme".into()),
        name: Some("proj".into()),
        full_name: Some("acme/proj".into()),
        url: "https://github.com/acme/proj".into(),
        normalized_url: norm.into(),
        default_branch: Some("main".into()),
        is_fork: false,
        is_archived: false,
        is_private: false,
        pushed_at: Some(Utc::now()),
    }
}

#[test]
fn two_clones_linked_to_same_remote_form_one_cluster() {
    let c1 = sample_clone("clone-a", "proj-a");
    let c2 = sample_clone("clone-b", "proj-b");
    let remote = sample_github_remote("remote-1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![c1.clone(), c2.clone()],
        remotes: vec![remote.clone()],
        links: vec![
            CloneRemoteLink {
                clone_id: c1.id.clone(),
                remote_id: remote.id.clone(),
                relationship: "origin".into(),
            },
            CloneRemoteLink {
                clone_id: c2.id.clone(),
                remote_id: remote.id.clone(),
                relationship: "origin".into(),
            },
        ],
        ..Default::default()
    };

    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    );
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].cluster.members.len(), 3);
    assert!(plans[0].cluster.cluster_key.starts_with("url:"));
}

#[test]
fn remote_only_unlinked_github_repo_is_single_cluster() {
    let remote = sample_github_remote("remote-orphan", "github.com/acme/orphan");
    let snapshot = InventorySnapshot {
        clones: vec![],
        remotes: vec![remote],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    );
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].cluster.members.len(), 1);
    assert!(plans[0].cluster.cluster_key.starts_with("url:"));
    assert!(
        plans[0].actions.iter().any(|a| {
            matches!(a.action_type, ActionType::CloneLocalWorkspace)
                && a.target_kind == MemberKind::Remote
        }),
        "expected CloneLocalWorkspace for remote-only: {:?}",
        plans[0].actions
    );
}

#[test]
fn local_only_clone_without_remote_stays_name_clustered() {
    let clone = sample_clone("solo", "solo-proj");
    let snapshot = InventorySnapshot {
        clones: vec![clone],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    );
    assert_eq!(plans.len(), 1);
    assert!(plans[0].cluster.cluster_key.starts_with("name:"));
    assert!(
        plans[0]
            .actions
            .iter()
            .any(|a| matches!(a.action_type, ActionType::CreateRemoteRepo)),
        "expected CreateRemoteRepo for local-only: {:?}",
        plans[0].actions
    );
}
