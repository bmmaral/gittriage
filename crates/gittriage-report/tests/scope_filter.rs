//! `report --scope` filtering (member-scope buckets).
use chrono::{DateTime, Utc};
use gittriage_core::{
    ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus, MemberKind, PlanDocument, ScoreBundle,
};

fn sample_plan() -> PlanDocument {
    let mk = |id: &str, label: &str, members: Vec<ClusterMember>| ClusterPlan {
        cluster: ClusterRecord {
            id: id.to_string(),
            cluster_key: format!("key:{id}"),
            label: label.to_string(),
            status: ClusterStatus::Resolved,
            confidence: 0.9,
            canonical_clone_id: None,
            canonical_remote_id: None,
            members,
            evidence: vec![],
            scores: ScoreBundle::default(),
        },
        actions: vec![],
    };
    PlanDocument {
        schema_version: 1,
        scoring_rules_version: 1,
        generated_at: "2026-01-01T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        generated_by: "test".into(),
        external_adapter_run: None,
        clusters: vec![
            mk(
                "c1",
                "local",
                vec![ClusterMember {
                    kind: MemberKind::Clone,
                    id: "clone-1".to_string(),
                }],
            ),
            mk(
                "c2",
                "mixed",
                vec![
                    ClusterMember {
                        kind: MemberKind::Clone,
                        id: "clone-2".to_string(),
                    },
                    ClusterMember {
                        kind: MemberKind::Remote,
                        id: "remote-2".to_string(),
                    },
                ],
            ),
            mk(
                "c3",
                "remote",
                vec![ClusterMember {
                    kind: MemberKind::Remote,
                    id: "remote-3".to_string(),
                }],
            ),
        ],
    }
}

#[test]
fn filter_remote_only_keeps_single_cluster() {
    let p = sample_plan();
    let f = gittriage_report::filter_plan_by_scope(
        &p,
        Some(gittriage_report::ReportScopeFilter::RemoteOnly),
    );
    assert_eq!(f.clusters.len(), 1);
    assert_eq!(f.clusters[0].cluster.label, "remote");
}

#[test]
fn scope_breakdown_counts_all_buckets() {
    let p = sample_plan();
    let (l, m, r, e) = gittriage_report::scope_breakdown(&p);
    assert_eq!((l, m, r, e), (1, 1, 1, 0));
}

#[test]
fn filter_local_only_excludes_mixed() {
    let p = sample_plan();
    let f = gittriage_report::filter_plan_by_scope(
        &p,
        Some(gittriage_report::ReportScopeFilter::LocalOnly),
    );
    assert_eq!(f.clusters.len(), 1);
    assert_eq!(f.clusters[0].cluster.label, "local");
}
