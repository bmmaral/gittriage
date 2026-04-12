//! Compact workspace summary for agents (F5).

use crate::verdict::automation_verdict_for_cluster;
use gittriage_core::{InventorySnapshot, MemberKind, PlanDocument};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::provenance::Provenance;

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroupSummary {
    pub cluster_id: String,
    pub label: String,
    pub canonical_path: Option<String>,
    pub alternate_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnsafeTargetSummary {
    pub cluster_id: String,
    pub label: String,
    pub reason: String,
    pub unsafe_for_automation: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirtyCanonicalSummary {
    pub cluster_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSummaryOutput {
    pub schema_version: u32,
    pub kind: &'static str,
    #[serde(flatten)]
    pub provenance: Provenance,
    /// When non-empty, only clusters touching these directory prefixes are included.
    pub workspace_roots: Vec<String>,
    pub duplicate_groups: Vec<DuplicateGroupSummary>,
    pub unsafe_targets: Vec<UnsafeTargetSummary>,
    pub canonical_paths: Vec<String>,
    pub dirty_canonical_repos: Vec<DirtyCanonicalSummary>,
    pub nested_repo_warnings: Vec<String>,
    pub total_unsafe_for_automation: usize,
    pub total_clusters_considered: usize,
}

fn cluster_touches_workspace(
    cp: &gittriage_core::ClusterPlan,
    snapshot: &InventorySnapshot,
    roots: &[PathBuf],
) -> bool {
    if roots.is_empty() {
        return true;
    }
    for m in &cp.cluster.members {
        if m.kind != MemberKind::Clone {
            continue;
        }
        let Some(cl) = snapshot.clones.iter().find(|c| c.id == m.id) else {
            continue;
        };
        let p = Path::new(cl.path.as_str());
        for root in roots {
            if let (Ok(a), Ok(b)) = (p.canonicalize(), root.canonicalize()) {
                if a == b || a.starts_with(&b) {
                    return true;
                }
            } else if cl.path.contains(root.to_string_lossy().as_ref()) {
                return true;
            }
        }
    }
    false
}

/// Agent-oriented rollup: duplicates, unsafe clusters, canonical paths, dirty trees, nested skips.
pub fn agent_summary(
    plan: &PlanDocument,
    snapshot: &InventorySnapshot,
    workspace_roots: &[PathBuf],
) -> AgentSummaryOutput {
    let provenance = Provenance::from_snapshot(snapshot);

    let mut nested_repo_warnings: Vec<String> = Vec::new();
    if let Some(run) = snapshot.run.as_ref() {
        if let Some(ref st) = run.stats {
            nested_repo_warnings = st.skipped_nested_git.clone();
        }
    }

    let mut duplicate_groups = Vec::new();
    let mut unsafe_targets = Vec::new();
    let mut canonical_paths = Vec::new();
    let mut dirty_canonical = Vec::new();
    let mut considered = 0usize;

    for cp in &plan.clusters {
        if !cluster_touches_workspace(cp, snapshot, workspace_roots) {
            continue;
        }
        considered += 1;
        let c = &cp.cluster;
        let v = automation_verdict_for_cluster(cp, snapshot);

        let clone_members: Vec<_> = c
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Clone)
            .collect();
        let paths: Vec<String> = clone_members
            .iter()
            .filter_map(|m| snapshot.clones.iter().find(|cl| cl.id == m.id))
            .map(|cl| cl.path.clone())
            .collect();

        let canon = c
            .canonical_clone_id
            .as_ref()
            .and_then(|cid| snapshot.clones.iter().find(|cl| cl.id == *cid))
            .map(|cl| cl.path.clone());

        if let Some(ref p) = canon {
            if !canonical_paths.contains(p) {
                canonical_paths.push(p.clone());
            }
        }

        if clone_members.len() > 1 {
            let mut alts: Vec<String> = paths
                .iter()
                .filter(|p| Some(*p) != canon.as_ref())
                .cloned()
                .collect();
            alts.sort();
            duplicate_groups.push(DuplicateGroupSummary {
                cluster_id: c.id.clone(),
                label: c.label.clone(),
                canonical_path: canon.clone(),
                alternate_paths: alts,
            });
        }

        if v.unsafe_for_automation {
            let reason = v.blocking_reasons.first().cloned().unwrap_or_else(|| {
                "Unsafe for automation — see cluster status and confidence.".into()
            });
            unsafe_targets.push(UnsafeTargetSummary {
                cluster_id: c.id.clone(),
                label: c.label.clone(),
                reason,
                unsafe_for_automation: true,
            });
        }

        if let Some(cid) = c.canonical_clone_id.as_ref() {
            if let Some(cl) = snapshot.clones.iter().find(|cl| cl.id == *cid) {
                if cl.is_git && cl.is_dirty {
                    dirty_canonical.push(DirtyCanonicalSummary {
                        cluster_id: c.id.clone(),
                        path: cl.path.clone(),
                    });
                }
            }
        }
    }

    canonical_paths.sort();

    let total_unsafe = unsafe_targets.len();

    AgentSummaryOutput {
        schema_version: 1,
        kind: "gittriage_agent_summary",
        provenance,
        workspace_roots: workspace_roots
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        duplicate_groups,
        unsafe_targets,
        canonical_paths,
        dirty_canonical_repos: dirty_canonical,
        nested_repo_warnings,
        total_unsafe_for_automation: total_unsafe,
        total_clusters_considered: considered,
    }
}
