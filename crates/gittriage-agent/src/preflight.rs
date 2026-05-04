//! Compact agent preflight manifest (F3).

use crate::provenance::Provenance;
use crate::resolve::resolve_target;
use crate::verdict::{automation_verdict_for_cluster, automation_verdict_unresolved};
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

    let mut warnings: Vec<String> = resolved
        .as_ref()
        .map(|r| r.blocking_reasons.clone())
        .unwrap_or_default();
    if let Err(ref e) = resolved {
        warnings.push(format!("resolve: {e}"));
    }

    let cp_opt = resolved
        .as_ref()
        .ok()
        .and_then(|r| r.cluster_id.as_ref())
        .and_then(|id| plan.clusters.iter().find(|c| c.cluster.id == *id));

    let verdict = cp_opt
        .map(|cp| automation_verdict_for_cluster(cp, snapshot))
        .unwrap_or_else(|| automation_verdict_unresolved("Could not resolve target cluster."));

    let blocked_paths: Vec<String> = resolved
        .as_ref()
        .map(|r| r.alternates.clone())
        .unwrap_or_default();
    let ignored_alternates = blocked_paths.clone();

    let repo_root = resolved
        .as_ref()
        .ok()
        .and_then(|r| r.canonical_path.clone());

    let cluster_id = resolved.as_ref().ok().and_then(|r| r.cluster_id.clone());

    let canonical_path = resolved
        .as_ref()
        .ok()
        .and_then(|r| r.canonical_path.clone());

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
        canonical_path,
        repo_root,
        blocked_paths,
        ignored_alternates,
        warnings,
        recommended_next_action,
        cluster_id,
        verdict,
    }
}
