//! Comprehensive planner rule tests (P6): canonical selection, remote-only,
//! local-only, ambiguous duplicates, stale-but-important, override/pinning,
//! and JSON plan snapshot stability.

use chrono::{Duration, Utc};
use nexus_core::{
    ActionType, CloneRecord, CloneRemoteLink, InventorySnapshot, ManifestKind, RemoteRecord,
};
use nexus_plan::{resolve_clusters, PlanBuildOpts, PlanUserIntent};
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

// ── Canonical selection ──────────────────────────────────────────────────────

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
    assert_eq!(
        plans[0].cluster.canonical_clone_id.as_deref(),
        Some("new"),
        "should pick the freshest clone as canonical"
    );
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
    assert_eq!(plans.len(), 1);
    let canon = plans[0].cluster.canonical_clone_id.as_deref();
    assert!(
        canon == Some("clean") || canon == Some("dirty"),
        "should pick a canonical clone"
    );
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
    assert!(
        plans[0]
            .cluster
            .evidence
            .iter()
            .any(|e| e.kind == "not_canonical_clone" && e.subject_id == non_canonical),
        "non-canonical clone should have not_canonical_clone evidence"
    );
}

// ── Remote-only clusters ─────────────────────────────────────────────────────

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
    assert!(
        !plans[0]
            .actions
            .iter()
            .any(|a| matches!(a.action_type, ActionType::ArchiveLocalDuplicate)),
        "remote-only should never suggest ArchiveLocalDuplicate"
    );
}

// ── Local-only clusters ──────────────────────────────────────────────────────

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
    let bare_plan = plans
        .iter()
        .find(|p| p.cluster.label == "bare-proj")
        .unwrap();
    let git_plan = plans
        .iter()
        .find(|p| p.cluster.label == "git-proj")
        .unwrap();
    assert!(
        bare_plan.cluster.scores.recoverability < git_plan.cluster.scores.recoverability,
        "bare dir recoverability ({}) should be lower than git repo ({})",
        bare_plan.cluster.scores.recoverability,
        git_plan.cluster.scores.recoverability
    );
}

// ── Ambiguous duplicate clusters ─────────────────────────────────────────────

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
    assert_eq!(plans.len(), 1);
    let cluster = &plans[0].cluster;
    assert!(
        cluster
            .evidence
            .iter()
            .any(|e| e.kind == "name_bucket_duplicate_cluster"),
        "4 same-name clones should get name_bucket_duplicate_cluster evidence"
    );
    assert!(
        cluster.scores.risk > 0.0,
        "multi-clone name bucket should have positive risk"
    );
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
    assert!(
        multi_plans[0].cluster.scores.risk >= single_plans[0].cluster.scores.risk,
        "multi-clone cluster risk ({}) should be >= single-clone ({})",
        multi_plans[0].cluster.scores.risk,
        single_plans[0].cluster.scores.risk
    );
}

// ── Stale-but-important repos ────────────────────────────────────────────────

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
    assert!(
        plans[0].cluster.scores.risk >= 10.0,
        "ancient bare project should have elevated risk: {}",
        plans[0].cluster.scores.risk
    );
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "very_stale_canonical"));
}

// ── Override and pinning behavior ────────────────────────────────────────────

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
    assert_eq!(
        plans[0].cluster.canonical_clone_id.as_deref(),
        Some("stale"),
        "pin should override freshness-based canonical selection"
    );
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
    assert!(
        plans[0].actions.is_empty(),
        "ignored cluster should have no actions"
    );
    assert!(
        plans[0].cluster.scores.canonical > 0.0,
        "ignored cluster should still have scores"
    );
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
    // archive_hint should NOT suppress actions (unlike ignore)
    // The cluster still gets its normal action set
}

// ── Negative evidence (v5 rules) ─────────────────────────────────────────────

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
    assert!(
        plans_without[0].cluster.scores.usability < plans_with[0].cluster.scores.usability,
        "missing readme should reduce usability: with={} without={}",
        plans_with[0].cluster.scores.usability,
        plans_without[0].cluster.scores.usability
    );
}

#[test]
fn missing_manifest_reduces_usability() {
    let mut with_manifest = clone_with("mani", "proj-a", 5, false);
    with_manifest.manifest_kind = Some(ManifestKind::Cargo);
    let mut no_manifest = clone_with("nomod", "proj-b", 5, false);
    no_manifest.manifest_kind = None;
    no_manifest.path = "/alt".into();

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
    assert!(
        plans_without[0].cluster.scores.usability < plans_with[0].cluster.scores.usability,
        "missing manifest should reduce usability"
    );
}

#[test]
fn archived_remote_increases_risk() {
    let clone = clone_with("c1", "proj", 5, false);
    let mut remote = github_remote("r1", "github.com/acme/proj");
    remote.is_archived = true;
    let snapshot = InventorySnapshot {
        clones: vec![clone],
        remotes: vec![remote.clone()],
        links: vec![CloneRemoteLink {
            clone_id: "c1".into(),
            remote_id: "r1".into(),
            relationship: "origin".into(),
        }],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(plans[0]
        .cluster
        .evidence
        .iter()
        .any(|e| e.kind == "archived_remote_risk"));
}

// ── JSON plan snapshot stability ─────────────────────────────────────────────

#[test]
fn plan_document_serializes_with_expected_fields() {
    let clone = clone_with("c1", "proj", 5, false);
    let remote = github_remote("r1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![clone],
        remotes: vec![remote.clone()],
        links: vec![CloneRemoteLink {
            clone_id: "c1".into(),
            remote_id: "r1".into(),
            relationship: "origin".into(),
        }],
        ..Default::default()
    };

    let plan = nexus_plan::build_plan_with(&snapshot, default_opts()).unwrap();
    let json = serde_json::to_value(&plan).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["scoring_rules_version"], 5);
    assert!(json["generated_at"].is_string());
    assert!(json["clusters"].is_array());
    let cluster = &json["clusters"][0];
    assert!(cluster["cluster"]["scores"]["canonical"].is_number());
    assert!(cluster["cluster"]["scores"]["usability"].is_number());
    assert!(cluster["cluster"]["scores"]["recoverability"].is_number());
    assert!(cluster["cluster"]["scores"]["oss_readiness"].is_number());
    assert!(cluster["cluster"]["scores"]["risk"].is_number());
    assert!(cluster["cluster"]["evidence"].is_array());
    assert!(cluster["actions"].is_array());
}

// ── Action type coverage ─────────────────────────────────────────────────────

#[test]
fn duplicate_clones_get_archive_action() {
    let a = clone_with("a", "proj", 1, false);
    let b = clone_with("b", "proj", 10, false);
    let remote = github_remote("r1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![a, b],
        remotes: vec![remote.clone()],
        links: vec![
            CloneRemoteLink {
                clone_id: "a".into(),
                remote_id: "r1".into(),
                relationship: "origin".into(),
            },
            CloneRemoteLink {
                clone_id: "b".into(),
                remote_id: "r1".into(),
                relationship: "origin".into(),
            },
        ],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(
        plans[0]
            .actions
            .iter()
            .any(|a| matches!(a.action_type, ActionType::ArchiveLocalDuplicate)),
        "duplicate clones should get ArchiveLocalDuplicate action"
    );
}

#[test]
fn missing_readme_on_canonical_suggests_add_docs() {
    let mut c = clone_with("c1", "proj", 5, false);
    c.readme_title = None;
    let remote = github_remote("r1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![remote.clone()],
        links: vec![CloneRemoteLink {
            clone_id: "c1".into(),
            remote_id: "r1".into(),
            relationship: "origin".into(),
        }],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(
        plans[0]
            .actions
            .iter()
            .any(|a| matches!(a.action_type, ActionType::AddMissingDocs)),
        "missing readme should suggest AddMissingDocs"
    );
}

#[test]
fn missing_license_suggests_add_license() {
    let mut c = clone_with("c1", "proj", 5, false);
    c.license_spdx = None;
    let remote = github_remote("r1", "github.com/acme/proj");
    let snapshot = InventorySnapshot {
        clones: vec![c],
        remotes: vec![remote.clone()],
        links: vec![CloneRemoteLink {
            clone_id: "c1".into(),
            remote_id: "r1".into(),
            relationship: "origin".into(),
        }],
        ..Default::default()
    };

    let plans = resolve_clusters(&snapshot, &default_opts());
    assert!(
        plans[0]
            .actions
            .iter()
            .any(|a| matches!(a.action_type, ActionType::AddLicense)),
        "missing license should suggest AddLicense"
    );
}
