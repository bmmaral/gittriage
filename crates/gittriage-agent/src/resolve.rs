//! Canonical repo resolution (F1).

use crate::verdict::{automation_verdict_for_cluster, AutomationVerdictLabel};
use anyhow::{bail, Result};
use gittriage_core::{
    normalize_remote_url, ClusterPlan, InventorySnapshot, MemberKind, PlanDocument,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::provenance::Provenance;

#[derive(Debug, Clone, Serialize)]
pub struct ResolveOutput {
    pub schema_version: u32,
    pub kind: &'static str,
    #[serde(flatten)]
    pub provenance: Provenance,
    pub query: String,
    pub canonical_path: Option<String>,
    pub cluster_id: Option<String>,
    pub cluster_label: Option<String>,
    pub alternates: Vec<String>,
    pub confidence: Option<f64>,
    pub automation_verdict: AutomationVerdictLabel,
    pub blocking_reasons: Vec<String>,
    pub unsafe_for_automation: bool,
    pub why_canonical: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn expand_path_hint(q: &str) -> PathBuf {
    let q = q.trim();
    if let Some(rest) = q.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(q)
}

fn normalize_fs_path(p: &Path) -> Option<String> {
    std::fs::canonicalize(p)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Longest clone path prefix match for `query_path` (filesystem-normalized when possible).
fn cluster_for_path<'a>(
    plan: &'a PlanDocument,
    snapshot: &InventorySnapshot,
    query_path: &Path,
) -> Option<&'a ClusterPlan> {
    let query_str =
        normalize_fs_path(query_path).unwrap_or_else(|| query_path.to_string_lossy().to_string());
    let query_str = query_str.trim_end_matches(['/', '\\']);

    let mut best: Option<(&'a ClusterPlan, usize)> = None;
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
                if best.as_ref().map(|(_, l)| len > *l).unwrap_or(true) {
                    best = Some((cp, len));
                }
            }
        }
    }
    best.map(|(cp, _)| cp)
}

fn cluster_for_remote_url<'a>(
    plan: &'a PlanDocument,
    snapshot: &InventorySnapshot,
    url_norm: &str,
) -> Option<&'a ClusterPlan> {
    if url_norm.is_empty() {
        return None;
    }
    let mut clone_ids: Vec<String> = Vec::new();
    for r in &snapshot.remotes {
        if normalize_remote_url(&r.normalized_url) == url_norm
            || normalize_remote_url(&r.normalized_url).contains(url_norm)
            || url_norm.contains(&normalize_remote_url(&r.normalized_url))
        {
            for link in &snapshot.links {
                if link.remote_id == r.id {
                    clone_ids.push(link.clone_id.clone());
                }
            }
        }
    }
    clone_ids.sort();
    clone_ids.dedup();
    for cid in clone_ids {
        if let Ok(cp) = resolve_cluster_plan_by_member(plan, MemberKind::Clone, &cid) {
            return Some(cp);
        }
    }
    None
}

fn why_canonical_lines(cp: &ClusterPlan) -> Vec<String> {
    let c = &cp.cluster;
    let mut lines = Vec::new();
    match c.status {
        gittriage_core::ClusterStatus::Resolved => {
            lines.push("Cluster status is Resolved.".into());
        }
        gittriage_core::ClusterStatus::Ambiguous => {
            lines.push("Cluster status is Ambiguous — canonical is tentative.".into());
        }
        gittriage_core::ClusterStatus::ManualReview => {
            lines.push("Cluster status is ManualReview.".into());
        }
    }
    lines.push(format!(
        "Planner confidence {:.2}; canonical axis score {:.1}.",
        c.confidence, c.scores.canonical
    ));
    if let Some(cc) = &c.canonical_clone_id {
        lines.push(format!("Selected canonical_clone_id: {cc}."));
    }
    for ev in c
        .evidence
        .iter()
        .filter(|e| {
            matches!(
                e.kind.as_str(),
                "canonical_selected"
                    | "merge_base"
                    | "path_semantic_penalty"
                    | "not_canonical"
                    | "multiple_clones"
            )
        })
        .take(4)
    {
        lines.push(format!("{} — {}", ev.kind, ev.detail));
    }
    lines
}

/// Resolve a fuzzy label, path, or remote URL to a cluster and canonical checkout path.
pub fn resolve_target(
    plan: &PlanDocument,
    snapshot: &InventorySnapshot,
    query: &str,
) -> ResolveOutput {
    let provenance = Provenance::from_snapshot(snapshot);
    let q = query.trim();
    if q.is_empty() {
        return ResolveOutput {
            schema_version: 1,
            kind: "gittriage_resolve",
            provenance,
            query: query.to_string(),
            canonical_path: None,
            cluster_id: None,
            cluster_label: None,
            alternates: vec![],
            confidence: None,
            automation_verdict: AutomationVerdictLabel::Blocked,
            blocking_reasons: vec!["Empty query.".into()],
            unsafe_for_automation: true,
            why_canonical: vec![],
            error: Some("query is empty".into()),
        };
    }

    let path_guess = expand_path_hint(q);
    let by_path = if path_guess.as_os_str().len() > 0
        && (path_guess.exists() || q.contains('/') || q.contains('\\') || q.starts_with('~'))
    {
        cluster_for_path(plan, snapshot, &path_guess)
    } else {
        None
    };

    let url_norm = if q.contains("://") || q.starts_with("git@") {
        normalize_remote_url(q)
    } else {
        String::new()
    };
    let by_url = if !url_norm.is_empty() {
        cluster_for_remote_url(plan, snapshot, &url_norm)
    } else {
        None
    };

    let cp_result = if let Some(cp) = by_path {
        Ok(cp)
    } else if let Some(cp) = by_url {
        Ok(cp)
    } else {
        resolve_cluster_plan(plan, q)
    };

    let cp = match cp_result {
        Ok(cp) => cp,
        Err(e) => {
            return ResolveOutput {
                schema_version: 1,
                kind: "gittriage_resolve",
                provenance,
                query: query.to_string(),
                canonical_path: None,
                cluster_id: None,
                cluster_label: None,
                alternates: vec![],
                confidence: None,
                automation_verdict: AutomationVerdictLabel::Blocked,
                blocking_reasons: vec![format!("{e:#}")],
                unsafe_for_automation: true,
                why_canonical: vec![],
                error: Some(format!("{e:#}")),
            };
        }
    };

    let v = automation_verdict_for_cluster(cp, snapshot);
    let canonical_path = cp
        .cluster
        .canonical_clone_id
        .as_ref()
        .and_then(|cid| snapshot.clones.iter().find(|c| c.id == *cid))
        .map(|c| c.path.clone());

    let mut alternates: Vec<String> = Vec::new();
    for m in &cp.cluster.members {
        if m.kind != MemberKind::Clone {
            continue;
        }
        if Some(&m.id) == cp.cluster.canonical_clone_id.as_ref() {
            continue;
        }
        if let Some(cl) = snapshot.clones.iter().find(|c| c.id == m.id) {
            alternates.push(cl.path.clone());
        }
    }
    alternates.sort();

    ResolveOutput {
        schema_version: 1,
        kind: "gittriage_resolve",
        provenance,
        query: query.to_string(),
        canonical_path,
        cluster_id: Some(cp.cluster.id.clone()),
        cluster_label: Some(cp.cluster.label.clone()),
        alternates,
        confidence: Some(cp.cluster.confidence),
        automation_verdict: v.automation_verdict,
        blocking_reasons: v.blocking_reasons.clone(),
        unsafe_for_automation: v.unsafe_for_automation,
        why_canonical: why_canonical_lines(cp),
        error: None,
    }
}

/// Internal: same algorithm as `gittriage::explain::resolve_cluster_plan` (duplicated to keep agent crate independent).
pub(crate) fn resolve_cluster_plan<'a>(
    plan: &'a PlanDocument,
    query: &str,
) -> Result<&'a ClusterPlan> {
    let q = query.trim();
    if q.is_empty() {
        bail!("cluster query is empty");
    }

    let by_id: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| cp.cluster.id == q)
        .collect();
    if by_id.len() == 1 {
        return Ok(by_id[0]);
    }

    let by_label: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| cp.cluster.label.eq_ignore_ascii_case(q))
        .collect();
    if by_label.len() == 1 {
        return Ok(by_label[0]);
    }

    let q_lower = q.to_lowercase();
    let substr: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| {
            cp.cluster.id.contains(q) || cp.cluster.label.to_lowercase().contains(&q_lower)
        })
        .collect();

    if substr.is_empty() {
        bail!("no cluster matches {:?}", q);
    }
    if substr.len() > 1 {
        let ids: Vec<String> = substr
            .iter()
            .map(|cp| format!("{} ({})", cp.cluster.label, cp.cluster.id))
            .collect();
        bail!("ambiguous cluster query {:?}: {}", q, ids.join("; "));
    }
    Ok(substr[0])
}

pub(crate) fn resolve_cluster_plan_by_member<'a>(
    plan: &'a PlanDocument,
    kind: MemberKind,
    id: &str,
) -> Result<&'a ClusterPlan> {
    let id = id.trim();
    if id.is_empty() {
        bail!("member id is empty");
    }
    let matches: Vec<_> = plan
        .clusters
        .iter()
        .filter(|cp| {
            cp.cluster
                .members
                .iter()
                .any(|m| m.kind == kind && m.id == id)
        })
        .collect();
    match matches.len() {
        0 => bail!("no cluster contains member {}", id),
        1 => Ok(matches[0]),
        _ => bail!("internal error: multiple clusters contain member {:?}", id),
    }
}
