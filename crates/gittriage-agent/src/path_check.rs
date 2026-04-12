//! Wrong-repo / non-canonical path detection (F4).

use crate::provenance::Provenance;
use crate::verdict::automation_verdict_for_cluster;
use gittriage_core::{InventorySnapshot, MemberKind, PlanDocument};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathDisposition {
    /// Path is the canonical checkout for its cluster.
    Canonical,
    /// Path is another clone in the same duplicate group — not the canonical pick.
    NonCanonicalAlternate,
    /// Path maps to inventory but is not under a known clone root (unexpected).
    UnknownMapping,
    /// Path is not under any inventoried clone.
    NotInInventory,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathCheckOutput {
    pub schema_version: u32,
    pub kind: &'static str,
    #[serde(flatten)]
    pub provenance: Provenance,
    pub path: String,
    pub disposition: PathDisposition,
    pub is_wrong_clone: bool,
    pub canonical_path: Option<String>,
    pub cluster_id: Option<String>,
    pub cluster_label: Option<String>,
    pub guidance: String,
    #[serde(flatten)]
    pub verdict: crate::verdict::AutomationVerdict,
}

fn longest_clone_for_path<'a>(
    plan: &'a PlanDocument,
    snapshot: &InventorySnapshot,
    query_path: &Path,
) -> Option<(&'a gittriage_core::ClusterPlan, String, bool)> {
    let query_str = std::fs::canonicalize(query_path)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| query_path.to_string_lossy().to_string());
    let query_str = query_str.trim_end_matches(['/', '\\']);

    let mut best: Option<(&'a gittriage_core::ClusterPlan, String, usize)> = None;
    for cp in &plan.clusters {
        for m in &cp.cluster.members {
            if m.kind != MemberKind::Clone {
                continue;
            }
            let Some(cl) = snapshot.clones.iter().find(|c| c.id == m.id) else {
                continue;
            };
            let p = cl.path.trim_end_matches(['/', '\\']);
            if query_str == p
                || query_str.starts_with(&format!("{p}/"))
                || query_str.starts_with(&format!("{p}\\"))
            {
                let len = p.len();
                if best.as_ref().map(|(_, _, l)| len > *l).unwrap_or(true) {
                    best = Some((cp, cl.path.clone(), len));
                }
            }
        }
    }
    let (cp, matched_path, _) = best?;
    let is_canon = cp
        .cluster
        .canonical_clone_id
        .as_ref()
        .and_then(|cid| snapshot.clones.iter().find(|c| c.id == *cid))
        .is_some_and(|c| {
            let cpth = c.path.trim_end_matches(['/', '\\']);
            query_str == cpth
                || query_str.starts_with(&format!("{cpth}/"))
                || query_str.starts_with(&format!("{cpth}\\"))
        });
    Some((cp, matched_path, is_canon))
}

/// Check whether `path` is the canonical repo root for automation, or a wrong/duplicate checkout.
pub fn check_path(
    plan: &PlanDocument,
    snapshot: &InventorySnapshot,
    path: &Path,
) -> PathCheckOutput {
    let provenance = Provenance::from_snapshot(snapshot);
    let path_display = path.to_string_lossy().to_string();

    let Some((cp, matched_clone_path, is_canonical_path)) =
        longest_clone_for_path(plan, snapshot, path)
    else {
        let v = crate::verdict::AutomationVerdict {
            safe_to_read: false,
            safe_to_index: false,
            safe_to_modify: false,
            safe_to_commit: false,
            safe_to_archive: false,
            human_review_required: true,
            unsafe_for_automation: true,
            automation_verdict: crate::verdict::AutomationVerdictLabel::Blocked,
            blocking_reasons: vec![
                "Path is not under any inventoried clone — run scan or fix path.".into(),
            ],
        };
        return PathCheckOutput {
            schema_version: 1,
            kind: "gittriage_check_path",
            provenance,
            path: path_display,
            disposition: PathDisposition::NotInInventory,
            is_wrong_clone: true,
            canonical_path: None,
            cluster_id: None,
            cluster_label: None,
            guidance: "Do not modify: path is outside GitTriage inventory. Run `gittriage scan` on a parent directory or use a known checkout path.".into(),
            verdict: v,
        };
    };

    let v = automation_verdict_for_cluster(cp, snapshot);
    let canonical_path = cp
        .cluster
        .canonical_clone_id
        .as_ref()
        .and_then(|cid| snapshot.clones.iter().find(|c| c.id == *cid))
        .map(|c| c.path.clone());

    let (disposition, is_wrong, guidance) = if is_canonical_path {
        (
            PathDisposition::Canonical,
            false,
            "This path is under the planner-selected canonical checkout. Prefer this tree for agent edits when verdict allows.".into(),
        )
    } else {
        (
            PathDisposition::NonCanonicalAlternate,
            true,
            format!(
                "Wrong clone for automation: you are under `{}` but canonical is `{}`. Do not edit here; switch to the canonical path or resolve duplicates first.",
                matched_clone_path,
                canonical_path.as_deref().unwrap_or("(unknown)")
            ),
        )
    };

    PathCheckOutput {
        schema_version: 1,
        kind: "gittriage_check_path",
        provenance,
        path: path_display,
        disposition,
        is_wrong_clone: is_wrong,
        canonical_path,
        cluster_id: Some(cp.cluster.id.clone()),
        cluster_label: Some(cp.cluster.label.clone()),
        guidance,
        verdict: v,
    }
}
