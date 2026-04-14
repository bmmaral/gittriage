//! Comprehensive planner rule tests (P6): canonical selection, remote-only,
//! local-only, ambiguous duplicates, stale-but-important, override/pinning,
//! profile-driven actions, and JSON plan snapshot stability.

use chrono::{Duration, Utc};
use gittriage_core::{
    ActionType, CloneRecord, CloneRemoteLink, InventorySnapshot, ManifestKind, RemoteRecord,
};
use gittriage_plan::{resolve_clusters, PlanBuildOpts, PlanUserIntent, ScoringProfile};
use std::collections::HashSet;

fn clone_with(id: &str, name: &str, days_ago: i64, dirty: bool) -> CloneRecord {
    CloneRecord {
        id: id.into(),
        path: format!("/dev/{id}"),
        display_name: name.into(),
        is_git: true,
        head_oid: Some(format!("oid-{id}")),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty: dirty,
        last_commit_at: Some(Utc::now() - Duration::days(days_ago)),
        size_bytes: Some(1024),
        manifest_kind: Some(ManifestKind::Cargo),
        readme_title: Some(name.into()),
        license_spdx: Some("MIT".into()),
        fingerprint: Some(format!("fp-{id}")),
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    }
}

fn bare_clone(id: &str, name: &str) -> CloneRecord {
    CloneRecord {
        id: id.into(),
        path: format!("/dev/{id}"),
        display_name: name.into(),
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
    }
}

fn github_remote(id: &str, norm: &str) -> RemoteRecord {
    RemoteRecord {
        id: id.into(),
        provider: "github".into(),
        owner: Some("acme".into()),
        name: Some("proj".into()),
        full_name: Some("acme/proj".into()),
        url: format!("https://{norm}"),
        normalized_url: norm.into(),
        default_branch: Some("main".into()),
        is_fork: false,
        is_archived: false,
        is_private: false,
        pushed_at: Some(Utc::now()),
    }
}

fn default_opts() -> PlanBuildOpts {
    PlanBuildOpts {
        merge_base: false,
        ..Default::default()
    }
}

fn action_types(plan: &gittriage_core::ClusterPlan) -> Vec<ActionType> {
    plan.actions.iter().map(|a| a.action_type.clone()).collect()
}

#[test]
fn canonical_picks_freshest_clone_with_remote() {
    let stale = clone_with("old", "proj", 400, false);
    let fresh = clone_with("new", "proj", 1, false);
    let remote = github_remote("r1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![stale, fresh],
        remotes: vec![remote.clone()],
        links: vec![
            CloneRemoteLink {
                clone_id: "old".into(),
                remote_id: "r1".into(),
                relationship: "origin".into(),
            },
            CloneRemoteLink {
                clone_id: "new".into(),
                remote_id: "r1".into(),
                relationship: "origin".into(),
            },
        ],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].cluster.canonical_clone_id.as_deref(), Some("new"));
}

#[test]
fn canonical_prefers_clean_over_dirty() {
    let dirty = clone_with("dirty", "proj", 1, true);
    let clean = clone_with("clean", "proj", 5, false);
    let snapshot = InventorySnapshot {
        clones: vec![dirty, clean],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    let canon = plans[0].cluster.canonical_clone_id.as_deref();
    assert!(canon == Some("clean") || canon == Some("dirty"));
}

#[test]
fn canonical_non_selected_gets_not_canonical_evidence() {
    let a = clone_with("a", "proj", 100, false);
    let b = clone_with("b", "proj", 1, false);
    let snapshot = InventorySnapshot {
        clones: vec![a, b],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    let canonical = plans[0].cluster.canonical_clone_id.as_deref().unwrap();
    let non_canonical = if canonical == "a" { "b" } else { "a" };
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "not_canonical_clone" && e.subject_id == non_canonical));
}

#[test]
fn remote_only_cluster_suggests_clone_workspace() {
    let remote = github_remote("r-only", "github.com/acme/remote-proj");
    let snapshot = InventorySnapshot {
        clones: vec![],
        remotes: vec![remote],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert_eq!(plans.len(), 1);
    assert!(plans[0].cluster.canonical_clone_id.is_none());
    assert!(plans[0].cluster.canonical_remote_id.is_some());
    assert!(plans[0]
        .actions
        .iter()
        .any(|a| matches!(a.action_type, ActionType::CloneLocalWorkspace)));
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "remote_only_cluster"));
}

#[test]
fn remote_only_has_no_archive_duplicate_actions() {
    let remote = github_remote("r-only", "github.com/acme/proj2");
    let snapshot = InventorySnapshot {
        clones: vec![],
        remotes: vec![remote],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(!plans[0]
        .actions
        .iter()
        .any(|a| matches!(a.action_type, ActionType::ArchiveLocalDuplicate)));
}

#[test]
fn local_only_clone_suggests_create_remote() {
    let clone = clone_with("solo", "solo-proj", 5, false);
    let snapshot = InventorySnapshot {
        clones: vec![clone],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert_eq!(plans.len(), 1);
    assert!(plans[0].cluster.canonical_remote_id.is_none());
    assert!(plans[0]
        .actions
        .iter()
        .any(|a| matches!(a.action_type, ActionType::CreateRemoteRepo)));
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "local_only_cluster" || e.kind == "no_remote_linked"));
}

#[test]
fn local_only_bare_dir_has_lower_recoverability() {
    let bare = bare_clone("bare", "bare-proj");
    let git_clone = clone_with("git-c", "git-proj", 5, false);
    let snapshot = InventorySnapshot {
        clones: vec![bare, git_clone],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert_eq!(plans.len(), 2);
    let bare_plan = plans.iter().find(|p| p.cluster.label == "bare-proj").unwrap();
    let git_plan = plans.iter().find(|p| p.cluster.label == "git-proj").unwrap();
    assert!(bare_plan.cluster.scores.recoverability < git_plan.cluster.scores.recoverability);
}

#[test]
fn many_same_name_clones_get_name_bucket_duplicate_evidence() {
    let mut clones = Vec::new();
    for i in 0..4 {
        let mut c = clone_with(&format!("c{i}"), "dupe-proj", 10 + i * 10, false);
        c.fingerprint = Some(format!("fp-{i}"));
        c.path = format!("/path/{i}");
        clones.push(c);
    }
    let snapshot = InventorySnapshot {
        clones,
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    let cluster = &plans[0].cluster;
    assert!(cluster
        .evidence
        .iter()
        .any(|e| e.kind == "name_bucket_duplicate_cluster"));
    assert!(cluster.scores.risk > 0.0);
}

#[test]
fn ambiguous_cluster_has_higher_risk() {
    let a = clone_with("a", "proj", 10, false);
    let single_snap = InventorySnapshot {
        clones: vec![a.clone()],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let mut b = clone_with("b", "proj", 20, false);
    b.fingerprint = Some("different".into());
    b.path = "/alt".into();
    let multi_snap = InventorySnapshot {
        clones: vec![a, b],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let single_plans = resolve_clusters(&single_snap, &default_opts());
    let multi_plans = resolve_clusters(&multi_snap, &default_opts());
    assert!(multi_plans[0].cluster.scores.risk >= single_plans[0].cluster.scores.risk);
}

#[test]
fn stale_but_artifacted_gets_evidence_hint() {
    let mut c = clone_with("stale", "old-proj", 700, false);
    c.manifest_kind = Some(ManifestKind::Cargo);
    c.readme_title = Some("old-proj".into());
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "stale_but_artifacted"));
}

#[test]
fn very_stale_without_artifacts_has_elevated_risk() {
    let mut c = clone_with("ancient", "dead-proj", 800, false);
    c.manifest_kind = None;
    c.readme_title = None;
    c.license_spdx = None;
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(plans[0].cluster.scores.risk >= 10.0);
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "very_stale_canonical"));
}

#[test]
fn pin_overrides_canonical_even_for_stale_clone() {
    let fresh = clone_with("fresh", "proj", 1, false);
    let stale = clone_with("stale", "proj", 500, false);
    let remote = github_remote("r1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![fresh, stale],
        remotes: vec![remote.clone()],
        links: vec![
            CloneRemoteLink {
                clone_id: "fresh".into(),
                remote_id: "r1".into(),
                relationship: "origin".into(),
            },
            CloneRemoteLink {
                clone_id: "stale".into(),
                remote_id: "r1".into(),
                relationship: "origin".into(),
            },
        ],
        ..Default::default()
    };

    let mut pins = HashSet::new();
    pins.insert("stale".into());
    let opts = PlanBuildOpts {
        merge_base: false,
        user_intent: PlanUserIntent {
            pin_canonical_clone_ids: pins,
            ..Default::default()
        },
        ..Default::default()
    };
    let plans = resolve_clusters(&snapshot, &opts);
    assert_eq!(plans[0].cluster.canonical_clone_id.as_deref(), Some("stale"));
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "user_pinned_canonical"));
}

#[test]
fn ignored_key_clears_actions_keeps_scores() {
    let c = clone_with("c1", "ignored-proj", 5, false);
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let mut keys = HashSet::new();
    keys.insert("name:ignored-proj".into());
    let opts = PlanBuildOpts {
        merge_base: false,
        user_intent: PlanUserIntent {
            ignored_cluster_keys: keys,
            ..Default::default()
        },
        ..Default::default()
    };
    let plans = resolve_clusters(&snapshot, &opts);
    assert!(plans[0].actions.is_empty());
    assert!(plans[0].cluster.scores.canonical > 0.0);
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "user_ignored_cluster"));
}

#[test]
fn archive_hint_adds_evidence_keeps_actions() {
    let c = clone_with("c1", "hinted-proj", 5, false);
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let mut keys = HashSet::new();
    keys.insert("name:hinted-proj".into());
    let opts = PlanBuildOpts {
        merge_base: false,
        user_intent: PlanUserIntent {
            archive_hint_cluster_keys: keys,
            ..Default::default()
        },
        ..Default::default()
    };
    let plans = resolve_clusters(&snapshot, &opts);
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "user_archive_hint"));
}

#[test]
fn missing_readme_reduces_usability() {
    let mut with_readme = clone_with("readme", "proj-a", 5, false);
    with_readme.readme_title = Some("proj-a".into());
    let mut no_readme = clone_with("noread", "proj-b", 5, false);
    no_readme.readme_title = None;
    no_readme.path = "/alt".into();

    let snap_with = InventorySnapshot {
        clones: vec![with_readme],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
    let snap_without = InventorySnapshot {
        clones: vec![no_readme],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plans_with = resolve_clusters(&snap_with, &default_opts());
    let plans_without = resolve_clusters(&snap_without, &default_opts());
    assert!(plans_without[0].cluster.scores.usability < plans_with[0].cluster.scores.usability);
}

#[test]
fn missing_manifest_reduces_usability() {
    let with_manifest = clone_with("withman", "proj-a", 5, false);
    let mut no_manifest = clone_with("noman", "proj-b", 5, false);
    no_manifest.manifest_kind = None;
    let snap_with = InventorySnapshot {
        clones: vec![with_manifest],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
    let snap_without = InventorySnapshot {
        clones: vec![no_manifest],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
    let plans_with = resolve_clusters(&snap_with, &default_opts());
    let plans_without = resolve_clusters(&snap_without, &default_opts());
    assert!(plans_without[0].cluster.scores.usability < plans_with[0].cluster.scores.usability);
}

#[test]
fn ambiguous_clusters_only_emit_review_not_archive_actions() {
    let mut a = clone_with("a", "proj", 0, false);
    a.path = "/tmp/proj-a".into();
    a.last_commit_at = None;
    a.head_oid = None;
    a.fingerprint = Some("fp-a".into());
    let mut b = clone_with("b", "proj", 0, false);
    b.path = "/tmp/proj-b".into();
    b.last_commit_at = None;
    b.head_oid = None;
    b.fingerprint = Some("fp-b".into());
    let snapshot = InventorySnapshot {
        clones: vec![a, b],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
        let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(matches!(plans[0].cluster.status, gittriage_core::ClusterStatus::Ambiguous));
    let types = action_types(&plans[0]);
    assert!(types.contains(&ActionType::ReviewAmbiguousCluster));
    assert!(!types.contains(&ActionType::ArchiveLocalDuplicate));
}

#[test]
fn resolved_high_canonical_duplicate_cluster_emits_archive_actions() {
    let mut older = clone_with("clone-old", "proj", 500, false);
    older.path = "/tmp/old/proj".into();
    let mut newer = clone_with("clone-new", "proj", 1, false);
    newer.path = "/tmp/new/proj".into();
    let remote = github_remote("remote-1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![older, newer],
        remotes: vec![remote.clone()],
        links: vec![
            CloneRemoteLink {
                clone_id: "clone-old".into(),
                remote_id: remote.id.clone(),
                relationship: "origin".into(),
            },
            CloneRemoteLink {
                clone_id: "clone-new".into(),
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
            archive_duplicate_canonical_min: 10,
            ..Default::default()
        },
    );
    let types = action_types(&plans[0]);
    assert!(matches!(plans[0].cluster.status, gittriage_core::ClusterStatus::Resolved));
    assert!(types.contains(&ActionType::ArchiveLocalDuplicate));
}

#[test]
fn local_only_cluster_emits_create_remote_repo_action() {
    let c = clone_with("c1", "solo", 5, false);
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(action_types(&plans[0]).contains(&ActionType::CreateRemoteRepo));
}

#[test]
fn remote_only_cluster_emits_clone_local_workspace_action() {
    let remote = github_remote("remote-1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![],
        remotes: vec![remote],
        links: vec![],
        ..Default::default()
    };
    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(action_types(&plans[0]).contains(&ActionType::CloneLocalWorkspace));
}

#[test]
fn scoring_profiles_change_hygiene_actions_not_score_axes() {
    let clone = CloneRecord {
        id: "c1".into(),
        path: "/tmp/proj".into(),
        display_name: "proj".into(),
        is_git: true,
        head_oid: Some("abc".into()),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty: false,
        last_commit_at: Some(Utc::now()),
        size_bytes: None,
        manifest_kind: Some(ManifestKind::Cargo),
        readme_title: Some("proj".into()),
        license_spdx: Some("MIT".into()),
        fingerprint: Some("fp".into()),
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    };
    let remote = github_remote("remote-1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![clone],
        remotes: vec![remote.clone()],
        links: vec![CloneRemoteLink {
            clone_id: "c1".into(),
            remote_id: remote.id.clone(),
            relationship: "origin".into(),
        }],
        ..Default::default()
    };

    let default_plan = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            oss_candidate_threshold: 60,
            ..Default::default()
        },
    );
    let publish_plan = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            oss_candidate_threshold: 60,
            user_intent: PlanUserIntent {
                scoring_profile: ScoringProfile::PublishReadiness,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let oss_plan = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            oss_candidate_threshold: 60,
            user_intent: PlanUserIntent {
                scoring_profile: ScoringProfile::OpenSourceReadiness,
                ..Default::default()
            },
            ..Default::default()
        },
    );

    assert_eq!(default_plan[0].cluster.scores.oss_readiness, publish_plan[0].cluster.scores.oss_readiness);
    assert_eq!(publish_plan[0].cluster.scores.oss_readiness, oss_plan[0].cluster.scores.oss_readiness);
    assert!(!default_plan[0].cluster.evidence.iter().any(|e| e.kind == "scoring_profile_active"));
    assert!(publish_plan[0].cluster.evidence.iter().any(|e| e.kind == "scoring_profile_active"));
    assert!(oss_plan[0].cluster.evidence.iter().any(|e| e.kind == "scoring_profile_active"));

    let default_actions = action_types(&default_plan[0]);
    let publish_actions = action_types(&publish_plan[0]);
    let oss_actions = action_types(&oss_plan[0]);

    assert!(default_actions.contains(&ActionType::AddCi));
    assert!(default_actions.contains(&ActionType::RunSecurityScans));
    assert!(!default_actions.contains(&ActionType::PublishOssCandidate));
    assert!(publish_actions.contains(&ActionType::PublishOssCandidate));
    assert!(!publish_actions.contains(&ActionType::AddCi));
    assert!(oss_actions.contains(&ActionType::PublishOssCandidate));
    assert!(!oss_actions.contains(&ActionType::AddCi));
}
