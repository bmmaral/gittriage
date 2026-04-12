use chrono::{DateTime, Utc};
use gittriage_core::{
    ActionType, ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus, EvidenceItem,
    InventorySnapshot, MemberKind, PlanAction, PlanDocument, Priority, ScoreBundle,
};

fn fixture_plan() -> PlanDocument {
    PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: "2026-01-01T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture time"),
        generated_by: "gittriage-test".into(),
        external_adapter_run: None,
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
    let md = gittriage_report::render_markdown(&fixture_plan()).expect("render");
    insta::assert_snapshot!("markdown_report", md);
}

#[test]
fn agent_preflight_report_demotes_per_cluster_score_narrative() {
    let plan = fixture_plan();
    let (l, m, r, e) = gittriage_report::scope_breakdown(&plan);
    let md = gittriage_report::render_markdown_with(
        &plan,
        gittriage_report::ReportExtras {
            local_only_count: l,
            mixed_count: m,
            remote_only_count: r,
            empty_count: e,
            agent_preflight_headings: true,
            inventory_snapshot: Some(InventorySnapshot::default()),
            ..Default::default()
        },
    )
    .expect("render");
    assert!(md.contains("### Scores (summary)"), "{md}");
    assert!(
        !md.contains("### Score explanations"),
        "long score narrative should be omitted in agent-preflight report mode"
    );
}
