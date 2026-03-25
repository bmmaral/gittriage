//! P6: Tests for adapter absence/failure — adapters must never break the core pipeline.

use chrono::Utc;
use nexus_adapters::{
    attach_external_evidence, attach_external_evidence_cached, attach_filtered_evidence,
    AdapterCache, AdapterCategory, ExternalTool,
};
use nexus_core::{
    ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus, InventorySnapshot, MemberKind,
    PlanDocument, ScoreBundle,
};
use std::fs;
use uuid::Uuid;

fn make_plan(clone_id: &str, root_path: &str) -> (PlanDocument, InventorySnapshot) {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![nexus_core::CloneRecord {
            id: clone_id.into(),
            path: root_path.into(),
            display_name: "proj".into(),
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
        remotes: vec![],
        links: vec![],
    };

    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        clusters: vec![ClusterPlan {
            cluster: ClusterRecord {
                id: "cluster-1".into(),
                cluster_key: "name:proj".into(),
                label: "proj".into(),
                status: ClusterStatus::Resolved,
                confidence: 0.9,
                canonical_clone_id: Some(clone_id.into()),
                canonical_remote_id: None,
                members: vec![ClusterMember {
                    kind: MemberKind::Clone,
                    id: clone_id.into(),
                }],
                evidence: vec![],
                scores: ScoreBundle::default(),
            },
            actions: vec![],
        }],
    };

    (plan, snapshot)
}

#[test]
fn missing_adapters_produce_no_evidence_and_no_error() {
    let root = std::env::temp_dir().join(format!("nexus-absent-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();

    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());
    let result = attach_external_evidence(&mut plan, &snapshot);
    assert!(result.is_ok(), "missing adapters should not error");
    // Evidence count depends on which tools are actually on PATH in the test env.
    // The key invariant: no panic, no error.

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn nonexistent_directory_is_silently_skipped() {
    let (mut plan, snapshot) = make_plan("clone-1", "/nonexistent/path/that/does/not/exist");
    let result = attach_external_evidence(&mut plan, &snapshot);
    assert!(result.is_ok());
    assert!(
        plan.clusters[0].cluster.evidence.is_empty(),
        "nonexistent directory should produce no evidence"
    );
}

#[test]
fn no_canonical_clone_is_silently_skipped() {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![],
        remotes: vec![],
        links: vec![],
    };
    let mut plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        clusters: vec![ClusterPlan {
            cluster: ClusterRecord {
                id: "cluster-1".into(),
                cluster_key: "name:proj".into(),
                label: "proj".into(),
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

    let result = attach_external_evidence(&mut plan, &snapshot);
    assert!(result.is_ok());
    assert!(plan.clusters[0].cluster.evidence.is_empty());
}

#[test]
fn cache_prevents_duplicate_scans() {
    let root = std::env::temp_dir().join(format!("nexus-cache-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();

    let mut cache = AdapterCache::new();
    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());

    // Run twice with the same cache
    let _ = attach_external_evidence_cached(&mut plan, &snapshot, &mut cache);
    let ev_count_1 = plan.clusters[0].cluster.evidence.len();

    let _ = attach_external_evidence_cached(&mut plan, &snapshot, &mut cache);
    let ev_count_2 = plan.clusters[0].cluster.evidence.len();

    // Second run appends again (evidence is additive per call) but the cache
    // means the subprocess was only run once per tool — we can't observe this
    // directly without mocking, but we verify no crash and consistent counts.
    assert_eq!(ev_count_2, ev_count_1 * 2);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn filtered_evidence_respects_category() {
    let root = std::env::temp_dir().join(format!("nexus-filter-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();

    let mut cache = AdapterCache::new();
    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());

    // Only request supply-chain category (syft)
    let result = attach_filtered_evidence(
        &mut plan,
        &snapshot,
        &[AdapterCategory::SupplyChain],
        &mut cache,
    );
    assert!(result.is_ok());

    // Any evidence produced should only be from syft
    for ev in &plan.clusters[0].cluster.evidence {
        assert_eq!(
            ev.kind, "syft_sbom",
            "filtered evidence should only contain syft evidence, got: {}",
            ev.kind
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn empty_plan_with_no_clusters_is_fine() {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![],
        remotes: vec![],
        links: vec![],
    };
    let mut plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        clusters: vec![],
    };

    let result = attach_external_evidence(&mut plan, &snapshot);
    assert!(result.is_ok());
    assert!(plan.clusters.is_empty());
}

#[test]
fn all_evidence_kinds_are_recognized() {
    let valid = ["gitleaks_detect", "semgrep_scan", "jscpd_scan", "syft_sbom"];
    for tool in ExternalTool::ALL {
        assert!(
            valid.contains(&tool.evidence_kind()),
            "unknown evidence kind: {}",
            tool.evidence_kind()
        );
    }
}
