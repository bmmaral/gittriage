use chrono::Utc;
use nexus_adapters::attach_external_evidence;
use nexus_core::{
    ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus, InventorySnapshot, MemberKind,
    PlanDocument, ScoreBundle,
};
use std::fs;
use uuid::Uuid;

fn ev_kind_allowed(kind: &str) -> bool {
    matches!(
        kind,
        "gitleaks_detect" | "semgrep_scan" | "jscpd_scan" | "syft_sbom"
    )
}

#[test]
fn external_evidence_is_best_effort_and_non_blocking() {
    let base = std::env::temp_dir();
    let root = base.join(format!("nexus-adapter-root-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp root dir");

    let clone_id = "clone-1".to_string();
    let snapshot = InventorySnapshot {
        run: None,
        clones: vec![nexus_core::CloneRecord {
            id: clone_id.clone(),
            path: root.to_string_lossy().to_string(),
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
        }],
        remotes: vec![],
        links: vec![],
    };

    let mut plan = PlanDocument {
        generated_at: Utc::now(),
        generated_by: "test".into(),
        clusters: vec![ClusterPlan {
            cluster: ClusterRecord {
                id: "cluster-1".into(),
                cluster_key: "name:proj".into(),
                label: "proj".into(),
                status: ClusterStatus::Resolved,
                confidence: 0.9,
                canonical_clone_id: Some(clone_id.clone()),
                canonical_remote_id: None,
                members: vec![ClusterMember {
                    kind: MemberKind::Clone,
                    id: clone_id.clone(),
                }],
                evidence: vec![],
                scores: ScoreBundle::default(),
            },
            actions: vec![],
        }],
    };

    attach_external_evidence(&mut plan, &snapshot).expect("attach external evidence");

    let evidence = &plan.clusters[0].cluster.evidence;
    assert!(
        evidence.len() <= 4,
        "expected <= 4 evidence items, got {}",
        evidence.len()
    );

    for e in evidence {
        assert_eq!(e.subject_id, clone_id);
        assert_eq!(e.subject_kind, MemberKind::Clone);
        assert!(
            ev_kind_allowed(&e.kind),
            "unexpected evidence.kind: {}",
            e.kind
        );
        // Adapter evidence is best-effort. It should not contribute negative deltas.
        assert!(e.score_delta >= 0.0);
    }

    let _ = fs::remove_dir_all(&root);
}
