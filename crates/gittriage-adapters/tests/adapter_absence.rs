//! Tests for adapter absence/failure and fake-binary scenarios — adapters must never break the core pipeline.

use chrono::Utc;
use gittriage_adapters::{
    attach_external_evidence, attach_external_evidence_cached, attach_filtered_evidence,
    count_adapter_evidence, probe_all, AdapterCache, AdapterCategory, ExternalTool, SupportTier,
};
use gittriage_core::{
    CloneRecord, ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus, InventorySnapshot,
    ManifestKind, MemberKind, PlanDocument, ScoreBundle,
};
use std::env;
use std::fs;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

fn test_path_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn make_plan(clone_id: &str, root_path: &str) -> (PlanDocument, InventorySnapshot) {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![CloneRecord {
            id: clone_id.into(),
            path: root_path.into(),
            display_name: "proj".into(),
            is_git: true,
            head_oid: Some("abc".into()),
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
        }],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };

    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        external_adapter_run: None,
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

fn write_fake_tool(bin_dir: &std::path::Path, tool: &str) {
    #[cfg(windows)]
    let (path, script) = (
        bin_dir.join(format!("{tool}.cmd")),
        format!("@echo off\r\necho {tool}-ok\r\nexit /b 0\r\n"),
    );

    #[cfg(not(windows))]
    let (path, script) = (
        bin_dir.join(tool),
        format!("#!/bin/sh\necho {tool}-ok\nexit 0\n"),
    );

    fs::write(&path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
    }
}

#[test]
fn tool_metadata_is_consistent() {
    for tool in ExternalTool::ALL {
        assert!(!tool.bin_name().is_empty());
        assert!(!tool.evidence_kind().is_empty());
        let _ = tool.category();
        let _ = tool.support_tier();
    }
}

#[test]
fn probe_all_returns_four_entries() {
    let probes = probe_all();
    assert_eq!(probes.len(), 4);
}

#[test]
fn support_tiers_are_assigned() {
    assert_eq!(
        ExternalTool::Gitleaks.support_tier(),
        SupportTier::OfficiallySupported
    );
    assert_eq!(ExternalTool::Jscpd.support_tier(), SupportTier::BestEffort);
}

#[test]
fn categories_are_assigned() {
    assert_eq!(ExternalTool::Gitleaks.category(), AdapterCategory::Security);
    assert_eq!(ExternalTool::Syft.category(), AdapterCategory::SupplyChain);
    assert_eq!(ExternalTool::Jscpd.category(), AdapterCategory::Quality);
}

#[test]
fn missing_adapters_produce_no_evidence_and_no_error() {
    let _guard = test_path_lock().lock().unwrap();
    let old_path = env::var("PATH").unwrap_or_default();
    env::set_var("PATH", "/nonexistent");

    let root = std::env::temp_dir().join(format!("gittriage-absent-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();

    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());
    let result = attach_external_evidence(&mut plan, &snapshot);
    assert!(result.is_ok());
    assert_eq!(count_adapter_evidence(&plan), 0);
    let run = plan.external_adapter_run.as_ref().unwrap();
    assert_eq!(run.tools_on_path, 0);
    assert_eq!(run.evidence_items_attached, 0);

    let _ = fs::remove_dir_all(&root);
    env::set_var("PATH", old_path);
}

#[test]
fn nonexistent_directory_is_silently_skipped() {
    let _guard = test_path_lock().lock().unwrap();
    let old_path = env::var("PATH").unwrap_or_default();
    env::set_var("PATH", "/nonexistent");

    let (mut plan, snapshot) = make_plan("clone-1", "/nonexistent/path/that/does/not/exist");
    let result = attach_external_evidence(&mut plan, &snapshot);
    assert!(result.is_ok());
    assert!(plan.clusters[0].cluster.evidence.is_empty());

    env::set_var("PATH", old_path);
}

#[test]
fn no_canonical_clone_is_silently_skipped() {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
    let mut plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        external_adapter_run: None,
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
    let _guard = test_path_lock().lock().unwrap();
    let old_path = env::var("PATH").unwrap_or_default();
    env::set_var("PATH", "/nonexistent");

    let root = std::env::temp_dir().join(format!("gittriage-cache-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();

    let mut cache = AdapterCache::new();
    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());
    let _ = attach_external_evidence_cached(&mut plan, &snapshot, &mut cache);
    let ev_count_1 = plan.clusters[0].cluster.evidence.len();
    let _ = attach_external_evidence_cached(&mut plan, &snapshot, &mut cache);
    let ev_count_2 = plan.clusters[0].cluster.evidence.len();
    assert_eq!(ev_count_2, ev_count_1 * 2);

    let _ = fs::remove_dir_all(&root);
    env::set_var("PATH", old_path);
}

#[test]
fn filtered_evidence_respects_category() {
    let _guard = test_path_lock().lock().unwrap();
    let old_path = env::var("PATH").unwrap_or_default();
    env::set_var("PATH", "/nonexistent");

    let root = std::env::temp_dir().join(format!("gittriage-filter-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();

    let mut cache = AdapterCache::new();
    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());
    let result = attach_filtered_evidence(
        &mut plan,
        &snapshot,
        &[AdapterCategory::SupplyChain],
        &mut cache,
    );
    assert!(result.is_ok());
    for ev in &plan.clusters[0].cluster.evidence {
        assert_eq!(ev.kind, "syft_sbom");
    }

    let _ = fs::remove_dir_all(&root);
    env::set_var("PATH", old_path);
}

#[test]
fn empty_plan_with_no_clusters_is_fine() {
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![],
        remotes: vec![],
        links: vec![],
        ..Default::default()
    };
    let mut plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        external_adapter_run: None,
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
        assert!(valid.contains(&tool.evidence_kind()));
    }
}

#[test]
fn fake_adapter_binaries_produce_evidence_items() {
    let _guard = test_path_lock().lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    let root = temp.path().join("proj");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&root).unwrap();

    for tool in ["gitleaks", "semgrep", "jscpd", "syft"] {
        write_fake_tool(&bin_dir, tool);
    }

    let old_path = env::var("PATH").unwrap_or_default();
    let mut path_entries = env::split_paths(&old_path).collect::<Vec<_>>();
    path_entries.insert(0, bin_dir.clone());
    env::set_var("PATH", env::join_paths(path_entries).unwrap());

    let (mut plan, snapshot) = make_plan("clone-1", &root.to_string_lossy());
    attach_external_evidence(&mut plan, &snapshot).unwrap();

    let kinds: Vec<String> = plan.clusters[0]
        .cluster
        .evidence
        .iter()
        .map(|e| e.kind.clone())
        .collect();
    assert!(kinds.contains(&"gitleaks_detect".to_string()));
    assert!(kinds.contains(&"semgrep_scan".to_string()));
    assert!(kinds.contains(&"jscpd_scan".to_string()));
    assert!(kinds.contains(&"syft_sbom".to_string()));
    assert_eq!(count_adapter_evidence(&plan), 4);
    let run = plan.external_adapter_run.as_ref().unwrap();
    assert_eq!(run.tools_on_path, 4);
    assert_eq!(run.evidence_items_attached, 4);

    env::set_var("PATH", old_path);
}
