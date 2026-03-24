use chrono::{DateTime, Utc};
use nexus_core::{
    ActionType, ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus, EvidenceItem, MemberKind,
    PlanAction, PlanDocument, Priority, ScoreBundle,
};

fn fixture_plan() -> PlanDocument {
    PlanDocument {
        schema_version: 1,
        scoring_rules_version: 3,
        generated_at: "2026-01-01T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture time"),
        generated_by: "nexus-test".into(),
        clusters: vec![ClusterPlan {
            cluster: ClusterRecord {
                id: "cluster-snap-1".into(),
                cluster_key: "url:github.com/demo/repo".into(),
                label: "demo".into(),
                status: ClusterStatus::Resolved,
                confidence: 0.9,
                canonical_clone_id: Some("clone-1".into()),
                canonical_remote_id: Some("remote-1".into()),
                members: vec![
                    ClusterMember {
                        kind: MemberKind::Clone,
                        id: "clone-1".into(),
                    },
                    ClusterMember {
                        kind: MemberKind::Remote,
                        id: "remote-1".into(),
                    },
                ],
                evidence: vec![EvidenceItem {
                    id: "ev-1".into(),
                    subject_kind: MemberKind::Clone,
                    subject_id: "clone-1".into(),
                    kind: "readme_present".into(),
                    score_delta: 10.0,
                    detail: "readme title detected".into(),
                }],
                scores: ScoreBundle {
                    canonical: 50.0,
                    usability: 40.0,
                    recoverability: 55.0,
                    oss_readiness: 30.0,
                    risk: 10.0,
                },
            },
            actions: vec![PlanAction {
                id: "act-1".into(),
                priority: Priority::Medium,
                action_type: ActionType::AddLicense,
                target_kind: MemberKind::Clone,
                target_id: "clone-1".into(),
                reason: "test".into(),
                commands: vec![],
                evidence_summary: None,
                confidence: None,
                risk_note: None,
            }],
        }],
    }
}

#[test]
fn markdown_report_snapshot() {
    let md = nexus_report::render_markdown(&fixture_plan()).expect("render");
    insta::assert_snapshot!("markdown_report", md);
}
