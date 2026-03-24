use anyhow::Result;
use chrono::Utc;
use nexus_core::{
    ActionType, CloneRecord, ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus,
    EvidenceItem, InventorySnapshot, MemberKind, PlanAction, PlanDocument, Priority, RemoteRecord,
    ScoreBundle,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PlanBuildOpts {
    /// Run pairwise `git merge-base` across git clones in a cluster (when object DB overlaps).
    pub merge_base: bool,
}

impl Default for PlanBuildOpts {
    fn default() -> Self {
        Self { merge_base: true }
    }
}

pub fn build_plan(snapshot: &InventorySnapshot) -> Result<PlanDocument> {
    build_plan_with(snapshot, PlanBuildOpts::default())
}

pub fn build_plan_with(snapshot: &InventorySnapshot, opts: PlanBuildOpts) -> Result<PlanDocument> {
    let clusters = resolve_clusters(snapshot, opts.merge_base);
    Ok(PlanDocument {
        generated_at: Utc::now(),
        generated_by: format!("nexus {}", env!("CARGO_PKG_VERSION")),
        clusters,
    })
}

pub fn resolve_clusters(snapshot: &InventorySnapshot, merge_base: bool) -> Vec<ClusterPlan> {
    let remote_by_id: HashMap<String, &RemoteRecord> =
        snapshot.remotes.iter().map(|r| (r.id.clone(), r)).collect();

    let mut clone_urls: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut clone_remote_ids: HashMap<String, Vec<String>> = HashMap::new();

    for link in &snapshot.links {
        let Some(remote) = remote_by_id.get(&link.remote_id) else {
            continue;
        };
        clone_urls
            .entry(link.clone_id.clone())
            .or_default()
            .insert(remote.normalized_url.clone());
        clone_remote_ids
            .entry(link.clone_id.clone())
            .or_default()
            .push(link.remote_id.clone());
    }

    let mut buckets: BTreeMap<String, (Vec<CloneRecord>, Vec<RemoteRecord>)> = BTreeMap::new();

    for clone in &snapshot.clones {
        let key = if let Some(urls) = clone_urls.get(&clone.id) {
            if urls.is_empty() {
                format!("name:{}", sanitize_name(&clone.display_name))
            } else {
                format!("url:{}", urls.iter().next().expect("non-empty set"))
            }
        } else {
            format!("name:{}", sanitize_name(&clone.display_name))
        };
        push_clone_unique(&mut buckets, &key, clone);

        if let Some(rids) = clone_remote_ids.get(&clone.id) {
            for rid in rids {
                if let Some(r) = remote_by_id.get(rid) {
                    push_remote_unique(&mut buckets, &key, *r);
                }
            }
        }
    }

    let mut remote_seen: HashSet<String> = HashSet::new();
    for (_, (_, rs)) in &buckets {
        for r in rs {
            remote_seen.insert(r.id.clone());
        }
    }

    for remote in &snapshot.remotes {
        if !remote_seen.contains(&remote.id) {
            let key = format!("url:{}", remote.normalized_url);
            push_remote_unique(&mut buckets, &key, remote);
            remote_seen.insert(remote.id.clone());
        }
    }

    let mut plans = Vec::new();
    for (cluster_key, (cluster_clones, cluster_remotes)) in buckets {
        let label = derive_label(&cluster_clones, &cluster_remotes);
        let (mut scores, mut evidence, canonical_clone_id, canonical_remote_id, status, confidence) =
            assess_cluster(&cluster_clones, &cluster_remotes);

        if cluster_key.starts_with("url:")
            && (!cluster_clones.is_empty() || !cluster_remotes.is_empty())
        {
            let norm = &cluster_key[4..];
            scores.canonical = (scores.canonical + 25.0).min(100.0);
            let subject = canonical_clone_id
                .as_ref()
                .and_then(|id| cluster_clones.iter().find(|c| c.id == *id))
                .map(|c| (c.id.as_str(), MemberKind::Clone))
                .or_else(|| {
                    canonical_remote_id.as_ref().and_then(|id| {
                        cluster_remotes
                            .iter()
                            .find(|r| r.id == *id)
                            .map(|r| (r.id.as_str(), MemberKind::Remote))
                    })
                })
                .or_else(|| {
                    cluster_remotes
                        .first()
                        .map(|r| (r.id.as_str(), MemberKind::Remote))
                })
                .or_else(|| {
                    cluster_clones
                        .first()
                        .map(|c| (c.id.as_str(), MemberKind::Clone))
                });

            if let Some((sid, kind)) = subject {
                evidence.push(ev(
                    sid,
                    kind,
                    "remote_url_match",
                    25.0,
                    &format!("clustered by normalized remote URL `{norm}`"),
                ));
            }
        }

        let mut cluster = ClusterRecord {
            id: format!("cluster-{}", Uuid::new_v4()),
            cluster_key,
            label,
            status,
            confidence,
            canonical_clone_id,
            canonical_remote_id,
            members: build_members(&cluster_clones, &cluster_remotes),
            evidence: evidence.clone(),
            scores: scores.clone(),
        };

        if merge_base {
            enrich_merge_base_evidence(&mut cluster, &cluster_clones);
        }

        let actions = build_actions(&cluster, &cluster_clones, &cluster_remotes);
        plans.push(ClusterPlan { cluster, actions });
    }

    plans.sort_by(|a, b| a.cluster.label.cmp(&b.cluster.label));
    plans
}

fn enrich_merge_base_evidence(cluster: &mut ClusterRecord, clones: &[CloneRecord]) {
    let git: Vec<&CloneRecord> = clones.iter().filter(|c| c.is_git).collect();
    if git.len() < 2 {
        return;
    }
    for i in 0..git.len() {
        for j in (i + 1)..git.len() {
            let a = git[i];
            let b = git[j];
            let hint = match nexus_git::merge_base_between_local_clones(
                Path::new(&a.path),
                Path::new(&b.path),
            ) {
                Ok(h) => h,
                Err(e) => {
                    tracing::debug!(error = %e, "merge-base skipped for pair");
                    continue;
                }
            };
            let delta = if hint.merge_base_oid.is_some() {
                8.0
            } else {
                0.0
            };
            cluster.evidence.push(ev(
                &a.id,
                MemberKind::Clone,
                "merge_base",
                delta,
                &hint.detail,
            ));
            if hint.merge_base_oid.is_some() {
                cluster.scores.canonical = (cluster.scores.canonical + delta).min(100.0);
            }
        }
    }
}

fn sanitize_name(s: &str) -> String {
    s.trim().to_lowercase().replace(' ', "-")
}

fn push_clone_unique(
    buckets: &mut BTreeMap<String, (Vec<CloneRecord>, Vec<RemoteRecord>)>,
    key: &str,
    clone: &CloneRecord,
) {
    let e = buckets.entry(key.to_string()).or_default();
    if !e.0.iter().any(|c| c.id == clone.id) {
        e.0.push(clone.clone());
    }
}

fn push_remote_unique(
    buckets: &mut BTreeMap<String, (Vec<CloneRecord>, Vec<RemoteRecord>)>,
    key: &str,
    remote: &RemoteRecord,
) {
    let e = buckets.entry(key.to_string()).or_default();
    if !e.1.iter().any(|r| r.id == remote.id) {
        e.1.push(remote.clone());
    }
}

fn derive_label(clones: &[CloneRecord], remotes: &[RemoteRecord]) -> String {
    remotes
        .iter()
        .find(|r| r.provider == "github")
        .and_then(|r| r.name.clone())
        .or_else(|| remotes.first().and_then(|r| r.name.clone()))
        .or_else(|| clones.first().map(|c| c.display_name.clone()))
        .unwrap_or_else(|| "unknown".into())
}

fn build_members(clones: &[CloneRecord], remotes: &[RemoteRecord]) -> Vec<ClusterMember> {
    let mut members = Vec::new();
    for clone in clones {
        members.push(ClusterMember {
            kind: MemberKind::Clone,
            id: clone.id.clone(),
        });
    }
    for remote in remotes {
        members.push(ClusterMember {
            kind: MemberKind::Remote,
            id: remote.id.clone(),
        });
    }
    members
}

fn assess_cluster(
    clones: &[CloneRecord],
    remotes: &[RemoteRecord],
) -> (
    ScoreBundle,
    Vec<EvidenceItem>,
    Option<String>,
    Option<String>,
    ClusterStatus,
    f64,
) {
    let mut evidence = Vec::new();
    let mut scores = ScoreBundle::default();

    let canonical_clone = clones
        .iter()
        .max_by_key(|c| (c.last_commit_at, c.is_git, !c.is_dirty));
    let canonical_remote = remotes.iter().max_by_key(|r| r.pushed_at);

    if let Some(clone) = canonical_clone {
        scores.canonical += 30.0;
        evidence.push(ev(
            &clone.id,
            MemberKind::Clone,
            "freshest_clone",
            30.0,
            "selected as best local candidate",
        ));

        if clone.is_git {
            scores.canonical += 10.0;
            evidence.push(ev(
                &clone.id,
                MemberKind::Clone,
                "git_repo",
                10.0,
                "has .git metadata",
            ));
        }
        if clone.manifest_kind.is_some() {
            scores.usability += 15.0;
            evidence.push(ev(
                &clone.id,
                MemberKind::Clone,
                "manifest_present",
                15.0,
                "project manifest detected",
            ));
        }
        if clone.readme_title.is_some() {
            scores.usability += 10.0;
            evidence.push(ev(
                &clone.id,
                MemberKind::Clone,
                "readme_present",
                10.0,
                "readme title detected",
            ));
        }
        if clone.license_spdx.is_some() {
            scores.oss_readiness += 15.0;
            evidence.push(ev(
                &clone.id,
                MemberKind::Clone,
                "license_present",
                15.0,
                "license file detected",
            ));
        }
    }

    if let Some(remote) = canonical_remote {
        scores.canonical += 20.0;
        evidence.push(ev(
            &remote.id,
            MemberKind::Remote,
            "freshest_remote",
            20.0,
            "selected as best remote candidate",
        ));

        if !remote.is_archived {
            scores.oss_readiness += 10.0;
        }
        if !remote.is_fork {
            scores.oss_readiness += 5.0;
        }
    }

    if clones.len() > 1 {
        scores.risk += 25.0;
        evidence.push(ev(
            &clones[0].id,
            MemberKind::Clone,
            "multiple_clones",
            25.0,
            "more than one local clone in cluster",
        ));
    }

    if remotes.is_empty() && !clones.is_empty() {
        scores.risk += 15.0;
        scores.canonical -= 5.0;
    }

    if clones.is_empty() && !remotes.is_empty() {
        scores.risk += 10.0;
    }

    let confidence = if clones.len() + remotes.len() <= 1 {
        0.5
    } else if clones.len() > 1 && remotes.is_empty() {
        0.55
    } else {
        0.8
    };

    let status = if confidence < 0.6 {
        ClusterStatus::Ambiguous
    } else {
        ClusterStatus::Resolved
    };

    (
        normalize_scores(scores),
        evidence,
        canonical_clone.map(|c| c.id.clone()),
        canonical_remote.map(|r| r.id.clone()),
        status,
        confidence,
    )
}

fn build_actions(
    cluster: &ClusterRecord,
    clones: &[CloneRecord],
    remotes: &[RemoteRecord],
) -> Vec<PlanAction> {
    let mut actions = Vec::new();

    if matches!(cluster.status, ClusterStatus::Ambiguous) {
        actions.push(action(
            Priority::High,
            ActionType::ReviewAmbiguousCluster,
            cluster
                .canonical_clone_id
                .as_ref()
                .map(|_| MemberKind::Clone)
                .unwrap_or(MemberKind::Remote),
            cluster
                .canonical_clone_id
                .clone()
                .or_else(|| cluster.canonical_remote_id.clone())
                .unwrap_or_else(|| "unknown".into()),
            "Cluster confidence is low; manual review required",
        ));
    }

    if clones.len() > 1 {
        for clone in clones {
            if Some(&clone.id) != cluster.canonical_clone_id.as_ref() {
                actions.push(action(
                    Priority::High,
                    ActionType::ArchiveLocalDuplicate,
                    MemberKind::Clone,
                    clone.id.clone(),
                    "Lower-priority duplicate clone in same cluster",
                ));
            }
        }
    }

    let canonical = cluster.scores.oss_readiness;
    if canonical < 70.0 {
        if let Some(clone_id) = cluster.canonical_clone_id.clone() {
            actions.push(action(
                Priority::Medium,
                ActionType::AddLicense,
                MemberKind::Clone,
                clone_id.clone(),
                "OSS readiness below threshold: ensure license metadata exists",
            ));
            actions.push(action(
                Priority::Medium,
                ActionType::AddCi,
                MemberKind::Clone,
                clone_id.clone(),
                "OSS readiness below threshold: add CI baseline",
            ));
            actions.push(action(
                Priority::Medium,
                ActionType::RunSecurityScans,
                MemberKind::Clone,
                clone_id,
                "OSS readiness below threshold: run semgrep/gitleaks/syft",
            ));
        }
    }

    if remotes.is_empty() && !clones.is_empty() {
        if let Some(clone_id) = cluster.canonical_clone_id.clone() {
            actions.push(action(
                Priority::Medium,
                ActionType::CreateRemoteRepo,
                MemberKind::Clone,
                clone_id,
                "Canonical local project has no linked remote",
            ));
        }
    }

    actions
}

fn action(
    priority: Priority,
    action_type: ActionType,
    target_kind: MemberKind,
    target_id: String,
    reason: &str,
) -> PlanAction {
    PlanAction {
        id: format!("action-{}", Uuid::new_v4()),
        priority,
        action_type,
        target_kind,
        target_id,
        reason: reason.into(),
        commands: Vec::new(),
    }
}

fn ev(
    subject_id: &str,
    subject_kind: MemberKind,
    kind: &str,
    delta: f64,
    detail: &str,
) -> EvidenceItem {
    EvidenceItem {
        id: format!("ev-{}", Uuid::new_v4()),
        subject_kind,
        subject_id: subject_id.into(),
        kind: kind.into(),
        score_delta: delta,
        detail: detail.into(),
    }
}

fn normalize_scores(mut scores: ScoreBundle) -> ScoreBundle {
    scores.canonical = scores.canonical.clamp(0.0, 100.0);
    scores.usability = scores.usability.clamp(0.0, 100.0);
    scores.oss_readiness = scores.oss_readiness.clamp(0.0, 100.0);
    scores.risk = scores.risk.clamp(0.0, 100.0);
    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use nexus_core::{CloneRemoteLink, ManifestKind};

    fn sample_clone(id: &str, name: &str) -> CloneRecord {
        CloneRecord {
            id: id.into(),
            path: format!("/dev/{name}"),
            display_name: name.into(),
            is_git: true,
            head_oid: Some("abc".into()),
            active_branch: Some("main".into()),
            default_branch: Some("main".into()),
            is_dirty: false,
            last_commit_at: Some(Utc::now()),
            size_bytes: None,
            manifest_kind: Some(ManifestKind::Cargo),
            readme_title: Some(name.into()),
            license_spdx: Some("MIT".into()),
            fingerprint: None,
        }
    }

    fn sample_github_remote(id: &str, norm: &str) -> RemoteRecord {
        RemoteRecord {
            id: id.into(),
            provider: "github".into(),
            owner: Some("acme".into()),
            name: Some("proj".into()),
            full_name: Some("acme/proj".into()),
            url: format!("https://github.com/acme/proj"),
            normalized_url: norm.into(),
            default_branch: Some("main".into()),
            is_fork: false,
            is_archived: false,
            is_private: false,
            pushed_at: Some(Utc::now()),
        }
    }

    #[test]
    fn resolve_merges_clone_and_github_remote_by_link() {
        let clone = sample_clone("clone-1", "proj");
        let remote = sample_github_remote("remote-gh-1", "github.com/acme/proj");
        let snapshot = InventorySnapshot {
            clones: vec![clone.clone()],
            remotes: vec![remote.clone()],
            links: vec![CloneRemoteLink {
                clone_id: clone.id.clone(),
                remote_id: remote.id.clone(),
                relationship: "origin".into(),
            }],
            ..Default::default()
        };

        let plans = resolve_clusters(&snapshot, false);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].cluster.members.len(), 2);
        assert!(plans[0].cluster.cluster_key.starts_with("url:"));
    }

    #[test]
    fn resolve_isolates_unlinked_clone_by_name() {
        let c1 = sample_clone("c1", "aaa");
        let c2 = sample_clone("c2", "bbb");
        let snapshot = InventorySnapshot {
            clones: vec![c1, c2],
            remotes: vec![],
            links: vec![],
            ..Default::default()
        };
        let plans = resolve_clusters(&snapshot, false);
        assert_eq!(plans.len(), 2);
    }
}
