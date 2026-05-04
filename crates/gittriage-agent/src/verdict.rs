//! Deterministic automation safety verdicts (F2, F8).

use gittriage_core::{ClusterPlan, ClusterStatus, InventorySnapshot, MemberKind};
use serde::Serialize;

/// High-level verdict string for agents and JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationVerdictLabel {
    /// Clear canonical target; automation may proceed with normal care.
    Safe,
    /// Resolved but elevated risk (e.g. dirty tree, low confidence) — inspect before bulk edits.
    Caution,
    /// Planner could not pick a single safe target — stop for human review.
    HumanReviewRequired,
    /// Hard block: ambiguous cluster, missing canonical, or conflicting signals.
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationVerdictReasonCode {
    UnresolvedTarget,
    AmbiguousCanonicalSelection,
    ManualReviewRequired,
    LowConfidence,
    NestedGitRepoSkipped,
    MultipleClonesNoCanonical,
    CanonicalDirty,
    Safe,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutomationVerdict {
    pub safe_to_read: bool,
    pub safe_to_index: bool,
    pub safe_to_modify: bool,
    pub safe_to_commit: bool,
    pub safe_to_archive: bool,
    pub human_review_required: bool,
    /// Trust boundary: when true, agents must not perform mutating automation (F8).
    pub unsafe_for_automation: bool,
    pub automation_verdict: AutomationVerdictLabel,
    pub blocking_reasons: Vec<String>,
    pub reason_codes: Vec<AutomationVerdictReasonCode>,
    pub remediation_hints: Vec<String>,
}

const MIN_CONF_AUTOMATION: f64 = 0.6;
const MIN_CONF_ARCHIVE: f64 = 0.75;

fn has_nested_skipped_evidence(cp: &ClusterPlan) -> bool {
    cp.cluster
        .evidence
        .iter()
        .any(|e| e.kind == "nested_git_repo_skipped")
}

fn canonical_clone_dirty(cp: &ClusterPlan, snapshot: &InventorySnapshot) -> bool {
    let Some(cid) = cp.cluster.canonical_clone_id.as_ref() else {
        return false;
    };
    snapshot
        .clones
        .iter()
        .find(|c| c.id == *cid)
        .is_some_and(|c| c.is_dirty)
}

fn clone_member_count(cp: &ClusterPlan) -> usize {
    cp.cluster
        .members
        .iter()
        .filter(|m| m.kind == MemberKind::Clone)
        .count()
}

/// Deterministic verdict from cluster state and inventory (no ML).
/// Conservative verdict when the target does not resolve to a cluster.
pub fn automation_verdict_unresolved(message: &str) -> AutomationVerdict {
    AutomationVerdict {
        safe_to_read: false,
        safe_to_index: false,
        safe_to_modify: false,
        safe_to_commit: false,
        safe_to_archive: false,
        human_review_required: true,
        unsafe_for_automation: true,
        automation_verdict: AutomationVerdictLabel::Blocked,
        blocking_reasons: vec![message.to_string()],
        reason_codes: vec![AutomationVerdictReasonCode::UnresolvedTarget],
        remediation_hints: vec!["Ensure the path is correct and has been scanned.".to_string()],
    }
}

pub fn automation_verdict_for_cluster(
    cp: &ClusterPlan,
    snapshot: &InventorySnapshot,
) -> AutomationVerdict {
    let c = &cp.cluster;
    let mut blocking: Vec<String> = Vec::new();
    let mut reason_codes: Vec<AutomationVerdictReasonCode> = Vec::new();

    let human_review_required = matches!(
        c.status,
        ClusterStatus::Ambiguous | ClusterStatus::ManualReview
    ) || c.confidence < MIN_CONF_AUTOMATION;

    if matches!(c.status, ClusterStatus::Ambiguous) {
        blocking
            .push("Ambiguous canonical selection — do not assume a single source of truth.".into());
        reason_codes.push(AutomationVerdictReasonCode::AmbiguousCanonicalSelection);
    }
    if matches!(c.status, ClusterStatus::ManualReview) {
        blocking.push("Cluster status is ManualReview — human verification required.".into());
        reason_codes.push(AutomationVerdictReasonCode::ManualReviewRequired);
    }
    if c.confidence < MIN_CONF_AUTOMATION {
        blocking.push(format!(
            "Confidence {:.2} is below the automation threshold ({:.2}).",
            c.confidence, MIN_CONF_AUTOMATION
        ));
        reason_codes.push(AutomationVerdictReasonCode::LowConfidence);
    }

    if has_nested_skipped_evidence(cp) {
        blocking.push(
            "Nested git repositories were skipped under this tree — paths may be incomplete."
                .into(),
        );
        reason_codes.push(AutomationVerdictReasonCode::NestedGitRepoSkipped);
    }

    let n_clones = clone_member_count(cp);
    if n_clones > 1 && c.canonical_clone_id.is_none() {
        blocking.push("Multiple local checkouts without a recorded canonical clone id.".into());
        reason_codes.push(AutomationVerdictReasonCode::MultipleClonesNoCanonical);
    }

    let dirty = canonical_clone_dirty(cp, snapshot);
    if dirty {
        blocking.push(
            "Canonical checkout has a dirty worktree — avoid automated commits until reviewed."
                .into(),
        );
        reason_codes.push(AutomationVerdictReasonCode::CanonicalDirty);
    }

    if blocking.is_empty() {
        reason_codes.push(AutomationVerdictReasonCode::Safe);
    }

    let safe_to_read = !c.members.is_empty();
    let safe_to_index = safe_to_read && c.canonical_clone_id.is_some();

    let base_modify = matches!(c.status, ClusterStatus::Resolved)
        && c.confidence >= MIN_CONF_AUTOMATION
        && c.canonical_clone_id.is_some()
        && !has_nested_skipped_evidence(cp);

    let safe_to_modify = base_modify && !dirty;
    let safe_to_commit = safe_to_modify && !dirty;

    let safe_to_archive = matches!(c.status, ClusterStatus::Resolved)
        && c.confidence >= MIN_CONF_ARCHIVE
        && c.canonical_clone_id.is_some()
        && n_clones > 1
        && !human_review_required;

    let unsafe_for_automation =
        human_review_required || !safe_to_modify || matches!(c.status, ClusterStatus::Ambiguous);

    let automation_verdict = if matches!(c.status, ClusterStatus::Ambiguous) {
        AutomationVerdictLabel::Blocked
    } else if human_review_required {
        AutomationVerdictLabel::HumanReviewRequired
    } else if dirty || c.confidence < 0.75 {
        AutomationVerdictLabel::Caution
    } else {
        AutomationVerdictLabel::Safe
    };

    let remediation_hints = reason_codes.iter().map(remediation_hint_for_code).collect();

    AutomationVerdict {
        safe_to_read,
        safe_to_index,
        safe_to_modify,
        safe_to_commit,
        safe_to_archive,
        human_review_required,
        unsafe_for_automation,
        automation_verdict,
        blocking_reasons: blocking,
        reason_codes,
        remediation_hints,
    }
}

fn remediation_hint_for_code(code: &AutomationVerdictReasonCode) -> String {
    match code {
        AutomationVerdictReasonCode::UnresolvedTarget => "Ensure the path is correct and has been scanned.".into(),
        AutomationVerdictReasonCode::AmbiguousCanonicalSelection => "Run `gittriage explain` on the cluster to see why a canonical could not be chosen.".into(),
        AutomationVerdictReasonCode::ManualReviewRequired => "A human needs to review this cluster. Use `gittriage tui`.".into(),
        AutomationVerdictReasonCode::LowConfidence => "The confidence score is low. More information may be needed, or the cluster may be ambiguous.".into(),
        AutomationVerdictReasonCode::NestedGitRepoSkipped => "The scan skipped nested git repositories. Consider running a scan on the nested repositories directly.".into(),
        AutomationVerdictReasonCode::MultipleClonesNoCanonical => "The tool could not determine a canonical clone. You may need to manually specify one.".into(),
        AutomationVerdictReasonCode::CanonicalDirty => "Commit or stash the changes in the canonical repository.".into(),
        AutomationVerdictReasonCode::Safe => "No remediation needed.".into(),
    }
}
