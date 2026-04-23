use chrono::{Duration, Utc};
use gittriage_agent::{
    agent_summary, automation_verdict_for_cluster, check_path, list_duplicate_groups,
    list_unsafe_targets, preflight, resolve_target, AutomationVerdictLabel, PathDisposition,
};
use gittriage_core::{
    ActionType, CloneRecord, CloneRemoteLink, ClusterMember, ClusterPlan, ClusterRecord,
    ClusterStatus, EvidenceItem, InventorySnapshot, ManifestKind, MemberKind, PlanDocument,
    RemoteRecord, RunRecord, RunScanStats, ScoreBundle,
};
use gittriage_plan::{resolve_clusters, PlanBuildOpts};
use std::path::PathBuf;

fn clone(id: &str, path: &str) -> CloneRecord {
    CloneRecord {
        id: id.into(),
        path: path.into(),
        display_name: "proj".into(),
        is_git: true,
        head_oid: Some(format!("head-{id}")),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty: false,
        last_commit_at: Some(Utc::now()),
        upstream_tracking: None,
        size_bytes: Some(1),
        manifest_kind: Some(ManifestKind::Cargo),
        readme_title: Some("proj".into()),
        license_spdx: Some("MIT".into()),
        fingerprint: Some("fp".into()),
        has_lockfile: false,
        has_ci: false,
        has_tests_dir: false,
    }
}

fn remote(id: &str) -> RemoteRecord {
    RemoteRecord {
        id: id.into(),
        provider: "github".into(),
        owner: Some("acme".into()),
        name: Some("proj".into()),
        full_name: Some("acme/proj".into()),
        url: "https://github.com/acme/proj".into(),
        normalized_url: "github.com/acme/proj".into(),
        default_branch: Some("main".into()),
        is_fork: false,
        is_archived: false,
        is_private: false,
        pushed_at: Some(Utc::now()),
    }
}

fn clone_record(id: &str, path: &str, is_dirty: bool) -> CloneRecord {
    CloneRecord {
        id: id.into(),
        path: path.into(),
        display_name: PathBuf::from(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        is_git: true,
        head_oid: Some(format!("{id}-head")),
        active_branch: Some("main".into()),
        default_branch: Some("main".into()),
        is_dirty,
        last_commit_at: Some(Utc::now()),
        upstream_tracking: None,
        size_bytes: Some(123),
        manifest_kind: Some(ManifestKind::Cargo),
        readme_title: Some("Example".into()),
        license_spdx: Some("MIT".into()),
        fingerprint: Some("fp-same".into()),
        has_lockfile: true,
        has_ci: true,
        has_tests_dir: true,
    }
}

fn evidence(kind: &str, subject_id: &str) -> EvidenceItem {
    EvidenceItem {
        id: format!("ev-{kind}-{subject_id}"),
        subject_kind: MemberKind::Clone,
        subject_id: subject_id.into(),
        kind: kind.into(),
        score_delta: 0.0,
        detail: format!("detail for {kind}"),
    }
}

fn resolved_duplicate_fixture() -> (InventorySnapshot, PlanDocument) {
    let c1 = clone_record("clone-1", "/tmp/ws/canon", false);
    let c2 = clone_record("clone-2", "/tmp/ws/alt", false);
    let cluster = ClusterRecord {
        id: "cluster-1".into(),
        cluster_key: "name:example".into(),
        label: "example".into(),
        status: ClusterStatus::Resolved,
        confidence: 0.91,
        canonical_clone_id: Some(c1.id.clone()),
        canonical_remote_id: None,
        members: vec![
            ClusterMember {
                kind: MemberKind::Clone,
                id: c1.id.clone(),
            },
            ClusterMember {
                kind: MemberKind::Clone,
                id: c2.id.clone(),
            },
        ],
        evidence: vec![
            evidence("canonical_clone_pick", &c1.id),
            evidence("multiple_clones", &c1.id),
        ],
        scores: ScoreBundle {
            canonical: 70.0,
            usability: 32.0,
            recoverability: 74.0,
            oss_readiness: 44.0,
            risk: 28.0,
        },
    };
    let snapshot = InventorySnapshot {
        run: Some(RunRecord {
            id: "run-1".into(),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            roots: vec!["/tmp/ws".into()],
            github_owner: None,
            version: "test".into(),
            stats: Some(RunScanStats {
                skipped_nested_git: vec!["/tmp/ws/canon/vendor/nested".into()],
            }),
        }),
        clones: vec![c1.clone(), c2.clone()],
        remotes: vec![],
        links: vec![],
        semantics: None,
    };
    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "tests".into(),
        clusters: vec![ClusterPlan {
            cluster,
            actions: vec![],
        }],
        external_adapter_run: None,
    };
    (snapshot, plan)
}

#[test]
fn resolve_by_path_returns_canonical_and_alternates() {
    let (snapshot, plan) = resolved_duplicate_fixture();
    let out = resolve_target(&plan, &snapshot, "/tmp/ws/canon/src/main.rs");
    assert_eq!(out.kind, "gittriage_resolve");
    assert_eq!(out.schema_version, 1);
    assert_eq!(out.cluster_id.as_deref(), Some("cluster-1"));
    assert_eq!(out.canonical_path.as_deref(), Some("/tmp/ws/canon"));
    assert_eq!(out.alternates, vec!["/tmp/ws/alt".to_string()]);
    assert_eq!(out.automation_verdict, AutomationVerdictLabel::Safe);
    assert!(!out.unsafe_for_automation);
    assert!(out.why_canonical.iter().any(|l| l.contains("Resolved")));
}

#[test]
fn preflight_and_verdict_match_for_resolved_cluster() {
    let (snapshot, plan) = resolved_duplicate_fixture();
    let cp = &plan.clusters[0];
    let direct = automation_verdict_for_cluster(cp, &snapshot);
    let out = preflight(&plan, &snapshot, "/tmp/ws/canon");

    assert_eq!(out.kind, "gittriage_preflight");
    assert_eq!(out.cluster_id.as_deref(), Some("cluster-1"));
    assert_eq!(out.canonical_path.as_deref(), Some("/tmp/ws/canon"));
    assert_eq!(out.repo_root.as_deref(), Some("/tmp/ws/canon"));
    assert_eq!(out.blocked_paths, vec!["/tmp/ws/alt".to_string()]);
    assert_eq!(out.ignored_alternates, out.blocked_paths);
    assert_eq!(out.recommended_next_action, "verify_canonical_before_edit");
    assert_eq!(out.verdict.safe_to_modify, direct.safe_to_modify);
    assert_eq!(out.verdict.safe_to_commit, direct.safe_to_commit);
    assert_eq!(out.verdict.automation_verdict, direct.automation_verdict);
    assert!(out
        .warnings
        .iter()
        .any(|w| w.contains("nested git skipped: /tmp/ws/canon/vendor/nested")));
}

#[test]
fn check_path_distinguishes_canonical_and_alternate() {
    let (snapshot, plan) = resolved_duplicate_fixture();

    let canon = check_path(
        &plan,
        &snapshot,
        PathBuf::from("/tmp/ws/canon/src").as_path(),
    );
    assert_eq!(canon.disposition, PathDisposition::Canonical);
    assert!(!canon.is_wrong_clone);
    assert_eq!(canon.canonical_path.as_deref(), Some("/tmp/ws/canon"));
    assert!(canon.guidance.contains("canonical checkout"));

    let alt = check_path(
        &plan,
        &snapshot,
        PathBuf::from("/tmp/ws/alt/tests").as_path(),
    );
    assert_eq!(alt.disposition, PathDisposition::NonCanonicalAlternate);
    assert!(alt.is_wrong_clone);
    assert_eq!(alt.canonical_path.as_deref(), Some("/tmp/ws/canon"));
    assert!(alt.guidance.contains("Wrong clone for automation"));
}

#[test]
fn unresolved_outputs_fail_closed() {
    let snapshot = InventorySnapshot::default();
    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "tests".into(),
        clusters: vec![],
        external_adapter_run: None,
    };

    let resolved = resolve_target(&plan, &snapshot, "missing");
    assert_eq!(resolved.automation_verdict, AutomationVerdictLabel::Blocked);
    assert!(resolved.unsafe_for_automation);
    assert!(resolved.error.is_some());

    let pre = preflight(&plan, &snapshot, "missing");
    assert!(pre.verdict.unsafe_for_automation);
    assert_eq!(
        pre.verdict.automation_verdict,
        AutomationVerdictLabel::Blocked
    );
    assert_eq!(pre.recommended_next_action, "run_scan_and_resolve_target");

    let path = check_path(&plan, &snapshot, PathBuf::from("/does/not/exist").as_path());
    assert_eq!(path.disposition, PathDisposition::NotInInventory);
    assert!(path.is_wrong_clone);
    assert!(path.verdict.unsafe_for_automation);
}

#[test]
fn summary_lists_duplicates_unsafe_targets_and_workspace_filtering() {
    let (mut snapshot, plan) = resolved_duplicate_fixture();
    if let Some(canon) = snapshot.clones.iter_mut().find(|c| c.id == "clone-1") {
        canon.is_dirty = true;
    }

    let full = agent_summary(&plan, &snapshot, &[]);
    assert_eq!(full.kind, "gittriage_agent_summary");
    assert_eq!(full.total_clusters_considered, 1);
    assert_eq!(full.duplicate_groups.len(), 1);
    assert_eq!(
        full.duplicate_groups[0].canonical_path.as_deref(),
        Some("/tmp/ws/canon")
    );
    assert_eq!(full.unsafe_targets.len(), 1);
    assert!(full.unsafe_targets[0].reason.contains("dirty worktree"));
    assert_eq!(full.total_unsafe_for_automation, 1);
    assert_eq!(full.dirty_canonical_repos.len(), 1);
    assert!(full
        .nested_repo_warnings
        .iter()
        .any(|p| p.contains("vendor/nested")));

    let dups = list_duplicate_groups(&plan, &snapshot, &[PathBuf::from("/tmp/ws")]);
    assert_eq!(dups.len(), 1);
    let unsafe_targets = list_unsafe_targets(&plan, &snapshot, &[PathBuf::from("/tmp/ws")]);
    assert_eq!(unsafe_targets.len(), 1);

    let filtered = agent_summary(&plan, &snapshot, &[PathBuf::from("/other")]);
    assert_eq!(filtered.total_clusters_considered, 0);
    assert!(filtered.duplicate_groups.is_empty());
    assert!(filtered.unsafe_targets.is_empty());
}

#[test]
fn ambiguous_duplicate_cluster_is_blocked_and_has_review_action() {
    let mut a = clone("a", "/tmp/proj-a");
    a.head_oid = None;
    a.last_commit_at = None;
    a.fingerprint = Some("fp-a".into());
    let mut b = clone("b", "/tmp/proj-b");
    b.head_oid = None;
    b.last_commit_at = None;
    b.fingerprint = Some("fp-b".into());

    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![a, b],
        remotes: vec![],
        links: vec![],
        semantics: None,
    };
    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    );
    let cp = &plans[0];
    let verdict = automation_verdict_for_cluster(cp, &snapshot);

    assert!(matches!(cp.cluster.status, ClusterStatus::Ambiguous));
    assert!(verdict.unsafe_for_automation);
    assert!(verdict.human_review_required);
    assert!(cp
        .actions
        .iter()
        .any(|a| a.action_type == ActionType::ReviewAmbiguousCluster));
    assert!(!cp
        .actions
        .iter()
        .any(|a| a.action_type == ActionType::ArchiveLocalDuplicate));
}

#[test]
fn resolved_duplicate_cluster_can_archive_when_verdict_allows_mutation() {
    let mut older = clone("older", "/tmp/old/proj");
    older.last_commit_at = Some(Utc::now() - Duration::days(500));
    let newer = clone("newer", "/tmp/new/proj");
    let r = remote("r1");

    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![older, newer],
        remotes: vec![r.clone()],
        links: vec![
            CloneRemoteLink {
                clone_id: "older".into(),
                remote_id: r.id.clone(),
                relationship: "origin".into(),
            },
            CloneRemoteLink {
                clone_id: "newer".into(),
                remote_id: r.id.clone(),
                relationship: "origin".into(),
            },
        ],
        semantics: None,
    };
    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            archive_duplicate_canonical_min: 10,
            ..Default::default()
        },
    );
    let cp = &plans[0];
    let verdict = automation_verdict_for_cluster(cp, &snapshot);

    assert!(verdict.safe_to_modify);
    assert!(!verdict.unsafe_for_automation);
    assert!(cp
        .actions
        .iter()
        .any(|a| a.action_type == ActionType::ArchiveLocalDuplicate));
}

#[test]
fn dirty_canonical_cluster_blocks_mutation_even_if_plan_has_other_actions() {
    let mut c = clone("c1", "/tmp/proj");
    c.is_dirty = true;
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![c],
        remotes: vec![],
        links: vec![],
        semantics: None,
    };
    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    );
    let cp = &plans[0];
    let verdict = automation_verdict_for_cluster(cp, &snapshot);

    assert!(!verdict.safe_to_modify);
    assert!(verdict.unsafe_for_automation);
    assert!(verdict
        .blocking_reasons
        .iter()
        .any(|r| r.contains("dirty worktree")));
    assert!(cp
        .actions
        .iter()
        .any(|a| a.action_type == ActionType::CreateRemoteRepo));
}

#[test]
fn remote_only_cluster_suggests_clone_but_verdict_stays_conservative() {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![],
        remotes: vec![remote("r1")],
        links: vec![],
        semantics: None,
    };
    let plans = resolve_clusters(
        &snapshot,
        &PlanBuildOpts {
            merge_base: false,
            ..Default::default()
        },
    );
    let cp = &plans[0];
    let verdict = automation_verdict_for_cluster(cp, &snapshot);

    assert!(!verdict.safe_to_modify);
    assert!(verdict.unsafe_for_automation);
    assert!(cp
        .actions
        .iter()
        .any(|a| a.action_type == ActionType::CloneLocalWorkspace));
    assert!(cp
        .cluster
        .members
        .iter()
        .all(|m| m.kind == MemberKind::Remote));
}
