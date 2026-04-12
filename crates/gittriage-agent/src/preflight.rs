//! Compact agent preflight manifest (F3).

use crate::provenance::Provenance;
use crate::resolve::resolve_target;
use crate::verdict::automation_verdict_for_cluster;
use gittriage_core::{InventorySnapshot, MemberKind, PlanDocument};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PreflightOutput {
    pub schema_version: u32,
    pub kind: &'static str,
    #[serde(flatten)]
    pub provenance: Provenance,
    pub target: String,
    pub canonical_path: Option<String>,
    pub repo_root: Option<String>,
    pub blocked_paths: Vec<String>,
    pub ignored_alternates: Vec<String>,
    pub warnings: Vec<String>,
    pub recommended_next_action: String,
    pub cluster_id: Option<String>,
    #[serde(flatten)]
    pub verdict: crate::verdict::AutomationVerdict,
}

fn recommended_action(cp: &gittriage_core::ClusterPlan) -> String {
    use gittriage_core::ClusterStatus;
    match cp.cluster.status {
        ClusterStatus::Ambiguous => "review_ambiguous_cluster".into(),
        ClusterStatus::ManualReview => "human_review_required".into(),
        ClusterStatus::Resolved => {
            if cp
                .cluster
                .members
                .iter()
                .filter(|m| m.kind == MemberKind::Clone)
                .count()
                > 1
            {
                "verify_canonical_before_edit".into()
            } else {
                "proceed_with_canonical_checkout".into()
            }
        }
    }
}

/// Build a token-light manifest for a coding agent before it touches disk.
pub fn preflight(
    plan: &PlanDocument,
    snapshot: &InventorySnapshot,
    target: &str,
) -> PreflightOutput {
    let provenance = Provenance::from_snapshot(snapshot);
    let resolved = resolve_target(plan, snapshot, target);

    let mut warnings: Vec<String> = resolved.blocking_reasons.clone();
    if let Some(ref e) = resolved.error {
        warnings.push(format!("resolve: {e}"));
    }

    let cp_opt = resolved
        .cluster_id
        .as_ref()
        .and_then(|id| plan.clusters.iter().find(|c| c.cluster.id == *id));

    let verdict = cp_opt
        .map(|cp| automation_verdict_for_cluster(cp, snapshot))
        .unwrap_or_else(|| crate::verdict::AutomationVerdict {
            safe_to_read: false,
            safe_to_index: false,
            safe_to_modify: false,
            safe_to_commit: false,
            safe_to_archive: false,
            human_review_required: true,
            unsafe_for_automation: true,
            automation_verdict: crate::verdict::AutomationVerdictLabel::Blocked,
            blocking_reasons: vec!["Could not resolve target cluster.".into()],
        });

    let blocked_paths: Vec<String> = resolved.alternates.clone();
    let ignored_alternates = resolved.alternates.clone();

    let repo_root = resolved.canonical_path.clone();

    let recommended_next_action = cp_opt
        .map(recommended_action)
        .unwrap_or_else(|| "run_scan_and_resolve_target".into());

    if let Some(run) = snapshot.run.as_ref() {
        if let Some(ref st) = run.stats {
            for p in &st.skipped_nested_git {
                warnings.push(format!("nested git skipped: {p}"));
            }
        }
    }

    PreflightOutput {
        schema_version: 1,
        kind: "gittriage_preflight",
        provenance,
        target: target.to_string(),
        canonical_path: resolved.canonical_path.clone(),
        repo_root,
        blocked_paths,
        ignored_alternates,
        warnings,
        recommended_next_action,
        cluster_id: resolved.cluster_id.clone(),
        verdict,
    }
}
