mod scoring;

pub use scoring::SCORING_RULES_VERSION;

use anyhow::Result;
use chrono::{Duration, Utc};
use gittriage_core::{
    ActionType, CloneRecord, ClusterMember, ClusterPlan, ClusterRecord, ClusterStatus,
    EvidenceItem, InventorySnapshot, MemberKind, PlanAction, PlanDocument, Priority, RemoteRecord,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

/// Optional scoring profile: adjusts hygiene action thresholds only; default `ScoreBundle` axes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScoringProfile {
    #[default]
    Default,
    PublishReadiness,
    OpenSourceReadiness,
    SecuritySupplyChain,
    AiHandoff,
}

impl ScoringProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::PublishReadiness => "publish_readiness",
            Self::OpenSourceReadiness => "open_source_readiness",
            Self::SecuritySupplyChain => "security_supply_chain",
            Self::AiHandoff => "ai_handoff",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlanUserIntent {
    pub pin_canonical_clone_ids: HashSet<String>,
    pub ignored_cluster_keys: HashSet<String>,
    pub archive_hint_cluster_keys: HashSet<String>,
    pub scoring_profile: ScoringProfile,
}

#[derive(Debug, Clone)]
pub struct PlanBuildOpts {
    /// Run pairwise `git merge-base` across git clones in a cluster (when object DB overlaps).
    pub merge_base: bool,
    /// Ambiguous status when planner confidence is strictly below this percent (1–99).
    pub ambiguous_cluster_threshold_pct: u8,
    pub oss_candidate_threshold: u8,
    /// Suggest archiving non-canonical clones only when canonical score is at least this (0–100).
    pub archive_duplicate_canonical_min: u8,
    pub user_intent: PlanUserIntent,
}

impl Default for PlanBuildOpts {
    fn default() -> Self {
        Self {
            merge_base: true,
            ambiguous_cluster_threshold_pct: 60,
            oss_candidate_threshold: 70,
            archive_duplicate_canonical_min: 80,
            user_intent: PlanUserIntent::default(),
        }
    }
}

pub fn build_plan(snapshot: &InventorySnapshot) -> Result<PlanDocument> {
    build_plan_with(snapshot, PlanBuildOpts::default())
}

pub fn build_plan_with(snapshot: &InventorySnapshot, opts: PlanBuildOpts) -> Result<PlanDocument> {
    let clusters = resolve_clusters(snapshot, &opts);
    let mut plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: crate::scoring::SCORING_RULES_VERSION,
        generated_at: Utc::now(),
        generated_by: format!("gittriage {}", env!("CARGO_PKG_VERSION")),
        clusters,
        external_adapter_run: None,
    };
    attach_inventory_insights(&mut plan, snapshot);
    Ok(plan)
}

/// Attach scan-derived evidence (e.g. skipped nested git repos under inventoried roots).
pub fn attach_inventory_insights(plan: &mut PlanDocument, snapshot: &InventorySnapshot) {
    let Some(stats) = snapshot.run.as_ref().and_then(|r| r.stats.as_ref()) else {
        return;
    };
    if stats.skipped_nested_git.is_empty() {
        return;
    }

    let skipped: Vec<std::path::PathBuf> = stats
        .skipped_nested_git
        .iter()
        .map(std::path::PathBuf::from)
        .collect();

    let clone_by_id: HashMap<&str, &CloneRecord> =
        snapshot.clones.iter().map(|c| (c.id.as_str(), c)).collect();

    for cp in &mut plan.clusters {
        let mut nested_under: Vec<String> = Vec::new();
        for m in &cp.cluster.members {
            if m.kind != MemberKind::Clone {
                continue;
            }
            let Some(cl) = clone_by_id.get(m.id.as_str()) else {
                continue;
            };
            let parent = Path::new(cl.path.as_str());
            for sp in &skipped {
                if sp.starts_with(parent) && sp.as_path() != parent {
                    nested_under.push(sp.display().to_string());
                }
            }
        }
        nested_under.sort();
        nested_under.dedup();
        if nested_under.is_empty() {
            continue;
        }

        let subject = cp.cluster.canonical_clone_id.clone().or_else(|| {
            cp.cluster
                .members
                .iter()
                .find(|m| m.kind == MemberKind::Clone)
                .map(|m| m.id.clone())
        });
        let Some(sid) = subject else {
            continue;
        };

        let detail = format!(
            "Nested git repo(s) under this tree were not inventoried (set scan.include_nested_git = true to include): {}",
            nested_under.join("; ")
        );
        cp.cluster.evidence.push(EvidenceItem {
            id: format!("scan-nested-{}", Uuid::new_v4()),
            subject_kind: MemberKind::Clone,
            subject_id: sid,
            kind: "nested_git_repo_skipped".into(),
            score_delta: 0.0,
            detail,
        });
    }
}

pub fn resolve_clusters(snapshot: &InventorySnapshot, opts: &PlanBuildOpts) -> Vec<ClusterPlan> {
    let ambiguous_threshold = (opts.ambiguous_cluster_threshold_pct.clamp(1, 99) as f64) / 100.0;
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
                    push_remote_unique(&mut buckets, &key, r);
                }
            }
        }
    }

    let mut remote_seen: HashSet<String> = HashSet::new();
    for (_, rs) in buckets.values() {
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

    // Merge buckets that share a normalized remote URL or an identical scan fingerprint so
    // local-only scans still group duplicate checkouts (e.g. "aegis" vs "aegis copy").
    let buckets = merge_cluster_buckets(buckets, snapshot);

    let mut plans = Vec::new();
    for (cluster_key, (cluster_clones, cluster_remotes)) in buckets {
        let cluster_key_for_hints = cluster_key.clone();
        let label = derive_label(&cluster_clones, &cluster_remotes);
        let mut eval = crate::scoring::evaluate_cluster(
            &cluster_clones,
            &cluster_remotes,
            ambiguous_threshold,
        );
        apply_canonical_pin_to_eval(&mut eval, &cluster_clones, opts);
        eval.scores = crate::scoring::finalize_scores(eval.scores);

        if cluster_key.starts_with("url:")
            && (!cluster_clones.is_empty() || !cluster_remotes.is_empty())
        {
            let norm = &cluster_key[4..];
            eval.scores.canonical = (eval.scores.canonical + 25.0).min(100.0);
            let subject = eval
                .canonical_clone_id
                .as_ref()
                .and_then(|id| cluster_clones.iter().find(|c| c.id == *id))
                .map(|c| (c.id.as_str(), MemberKind::Clone))
                .or_else(|| {
                    eval.canonical_remote_id.as_ref().and_then(|id| {
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
                eval.evidence.push(ev(
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
            status: eval.status,
            confidence: eval.confidence,
            canonical_clone_id: eval.canonical_clone_id,
            canonical_remote_id: eval.canonical_remote_id,
            members: build_members(&cluster_clones, &cluster_remotes),
            evidence: eval.evidence.clone(),
            scores: eval.scores.clone(),
        };

        if opts.merge_base {
            enrich_merge_base_evidence(&mut cluster, &cluster_clones);
        }

        append_non_canonical_clone_evidence(&mut cluster, &cluster_clones);
        attach_cluster_shape_hints(&mut cluster, &cluster_clones, &cluster_key_for_hints);

        let mut actions = build_actions(&cluster, &cluster_clones, &cluster_remotes, opts);
        apply_user_intent_post(&mut cluster, &mut actions, opts);
        plans.push(ClusterPlan { cluster, actions });
    }

    attach_cross_cluster_duplicate_hints(&mut plans, snapshot);

    plans.sort_by(|a, b| a.cluster.label.cmp(&b.cluster.label));
    plans
}

/// In-cluster signals: weak name-only clustering and “stale but still looks like a project.”
fn attach_cluster_shape_hints(
    cluster: &mut ClusterRecord,
    clones: &[CloneRecord],
    cluster_key: &str,
) {
    if cluster_key.starts_with("name:") && clones.len() > 1 {
        let (sid, sk) = evidence_subject_cluster(cluster);
        cluster.evidence.push(ev(
            &sid,
            sk,
            "name_bucket_duplicate_cluster",
            0.0,
            "multiple local clones grouped by display name only; confirm shared lineage before treating as duplicates",
        ));
    }

    let Some(cid) = cluster.canonical_clone_id.as_ref() else {
        return;
    };
    let Some(canon) = clones.iter().find(|c| &c.id == cid) else {
        return;
    };
    if let Some(t) = canon.last_commit_at {
        if Utc::now() - t > Duration::days(400)
            && canon.manifest_kind.is_some()
            && canon.readme_title.is_some()
        {
            cluster.evidence.push(ev(
                cid,
                MemberKind::Clone,
                "stale_but_artifacted",
                0.0,
                "last commit is old but manifest and README are present—may be slow-cycle, archival, or still important; review before deprioritizing",
            ));
        }
    }
}

fn evidence_subject_cluster(cluster: &ClusterRecord) -> (String, MemberKind) {
    if let Some(ref id) = cluster.canonical_clone_id {
        return (id.clone(), MemberKind::Clone);
    }
    for m in &cluster.members {
        if m.kind == MemberKind::Clone {
            return (m.id.clone(), MemberKind::Clone);
        }
    }
    if let Some(m) = cluster.members.first() {
        return (m.id.clone(), m.kind.clone());
    }
    ("cluster".into(), MemberKind::Clone)
}

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] = self.rank[ra].saturating_add(1);
        }
    }
}

fn merge_cluster_buckets(
    buckets: BTreeMap<String, (Vec<CloneRecord>, Vec<RemoteRecord>)>,
    snapshot: &InventorySnapshot,
) -> BTreeMap<String, (Vec<CloneRecord>, Vec<RemoteRecord>)> {
    if buckets.len() <= 1 {
        return buckets;
    }
    let clone_urls = snapshot_clone_normalized_urls(snapshot);
    let entries: Vec<(String, Vec<CloneRecord>, Vec<RemoteRecord>)> = buckets
        .into_iter()
        .map(|(k, (cl, rm))| (k, cl, rm))
        .collect();
    let n = entries.len();
    let mut uf = UnionFind::new(n);

    for i in 0..n {
        for j in i + 1..n {
            let mut merge = false;
            for ca in &entries[i].1 {
                let Some(ua) = clone_urls.get(&ca.id) else {
                    continue;
                };
                for cb in &entries[j].1 {
                    let Some(ub) = clone_urls.get(&cb.id) else {
                        continue;
                    };
                    if ua.iter().any(|u| ub.contains(u)) {
                        merge = true;
                        break;
                    }
                }
                if merge {
                    break;
                }
            }
            if merge {
                uf.union(i, j);
            }
        }
    }

    let mut by_root: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        by_root.entry(uf.find(i)).or_default().push(i);
    }

    let mut out: BTreeMap<String, (Vec<CloneRecord>, Vec<RemoteRecord>)> = BTreeMap::new();
    for idxs in by_root.into_values() {
        let mut old_keys: Vec<String> = Vec::new();
        let mut merged_clones: Vec<CloneRecord> = Vec::new();
        let mut merged_remotes: Vec<RemoteRecord> = Vec::new();
        let mut seen_c: HashSet<String> = HashSet::new();
        let mut seen_r: HashSet<String> = HashSet::new();
        for &idx in &idxs {
            old_keys.push(entries[idx].0.clone());
            for c in &entries[idx].1 {
                if seen_c.insert(c.id.clone()) {
                    merged_clones.push(c.clone());
                }
            }
            for r in &entries[idx].2 {
                if seen_r.insert(r.id.clone()) {
                    merged_remotes.push(r.clone());
                }
            }
        }
        let cluster_key = derive_merged_cluster_key(&old_keys, &merged_clones, &clone_urls);
        let mut key = cluster_key.clone();
        let mut suf = 0u32;
        while out.contains_key(&key) {
            suf += 1;
            key = format!("{cluster_key}#{suf}");
        }
        out.insert(key, (merged_clones, merged_remotes));
    }
    out
}

fn derive_merged_cluster_key(
    old_keys: &[String],
    clones: &[CloneRecord],
    clone_urls: &HashMap<String, BTreeSet<String>>,
) -> String {
    let mut urls: BTreeSet<String> = BTreeSet::new();
    for k in old_keys {
        if let Some(rest) = k.strip_prefix("url:") {
            urls.insert(rest.to_string());
        }
    }
    for c in clones {
        if let Some(u) = clone_urls.get(&c.id) {
            urls.extend(u.iter().cloned());
        }
    }
    if let Some(u) = urls.iter().next() {
        format!("url:{u}")
    } else if let Some(c) = clones.first() {
        format!("name:{}", sanitize_name(&c.display_name))
    } else {
        format!("name:merged-{}", Uuid::new_v4())
    }
}

fn snapshot_clone_normalized_urls(
    snapshot: &InventorySnapshot,
) -> HashMap<String, BTreeSet<String>> {
    let remote_by_id: HashMap<String, &RemoteRecord> =
        snapshot.remotes.iter().map(|r| (r.id.clone(), r)).collect();
    let mut clone_urls: HashMap<String, BTreeSet<String>> = HashMap::new();
    for link in &snapshot.links {
        let Some(remote) = remote_by_id.get(&link.remote_id) else {
            continue;
        };
        clone_urls
            .entry(link.clone_id.clone())
            .or_default()
            .insert(remote.normalized_url.clone());
    }
    clone_urls
}

/// Same fingerprint or same display name across **different** clusters (possible duplicate / pivot).
fn attach_cross_cluster_duplicate_hints(plans: &mut [ClusterPlan], snapshot: &InventorySnapshot) {
    let clone_cluster: HashMap<String, String> = plans
        .iter()
        .flat_map(|cp| {
            cp.cluster
                .members
                .iter()
                .filter(|m| m.kind == MemberKind::Clone)
                .map(|m| (m.id.clone(), cp.cluster.id.clone()))
        })
        .collect();

    // Index plan positions by cluster id for O(1) lookup instead of O(n) find
    let plan_idx: HashMap<String, usize> = plans
        .iter()
        .enumerate()
        .map(|(i, cp)| (cp.cluster.id.clone(), i))
        .collect();

    let mut fp_to_clusters: HashMap<String, BTreeSet<String>> = HashMap::new();
    for c in &snapshot.clones {
        let Some(fp) = c
            .fingerprint
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let Some(cluster_id) = clone_cluster.get(&c.id) else {
            continue;
        };
        fp_to_clusters
            .entry(fp.to_string())
            .or_default()
            .insert(cluster_id.clone());
    }

    for (_fp, cluster_ids) in fp_to_clusters {
        if cluster_ids.len() < 2 {
            continue;
        }
        let ids: Vec<String> = cluster_ids.into_iter().collect();
        let detail = format!(
            "scan fingerprint matches across clusters `{}` — likely duplicate trees split by name/url buckets; verify before cleanup",
            ids.join("`, `")
        );
        for cid in &ids {
            if let Some(&idx) = plan_idx.get(cid) {
                let (sid, sk) = evidence_subject_cluster(&plans[idx].cluster);
                plans[idx].cluster.evidence.push(ev(
                    &sid,
                    sk,
                    "fingerprint_split_clusters",
                    0.0,
                    &detail,
                ));
            }
        }
    }

    let mut by_display: HashMap<String, Vec<&CloneRecord>> = HashMap::new();
    for c in &snapshot.clones {
        by_display
            .entry(sanitize_name(&c.display_name))
            .or_default()
            .push(c);
    }

    let clone_urls = snapshot_clone_normalized_urls(snapshot);

    for (_norm, group) in by_display {
        if group.len() < 2 {
            continue;
        }
        let cluster_ids: BTreeSet<String> = group
            .iter()
            .filter_map(|c| clone_cluster.get(&c.id).cloned())
            .collect();
        if cluster_ids.len() < 2 {
            continue;
        }

        let url_set: BTreeSet<String> = group
            .iter()
            .flat_map(|c| {
                clone_urls
                    .get(&c.id)
                    .into_iter()
                    .flat_map(|urls| urls.iter().cloned())
            })
            .collect();

        let detail = if url_set.len() >= 2 {
            format!(
                "{} clones share display name `{}` but sit in different clusters with different remotes—possible fork/rename/pivot; reconcile origin URLs",
                group.len(),
                group[0].display_name
            )
        } else {
            format!(
                "{} clones share display name `{}` but sit in different clusters—weak clustering signal; confirm they are unrelated before merging inventory",
                group.len(),
                group[0].display_name
            )
        };

        for cid in &cluster_ids {
            if let Some(&idx) = plan_idx.get(cid) {
                let (sid, sk) = evidence_subject_cluster(&plans[idx].cluster);
                plans[idx].cluster.evidence.push(ev(
                    &sid,
                    sk,
                    "duplicate_name_split_clusters",
                    0.0,
                    &detail,
                ));
            }
        }
    }
}

fn append_non_canonical_clone_evidence(cluster: &mut ClusterRecord, clones: &[CloneRecord]) {
    let Some(canonical_id) = cluster.canonical_clone_id.as_ref() else {
        return;
    };
    let Some(canonical) = clones.iter().find(|c| &c.id == canonical_id) else {
        return;
    };
    for clone in clones {
        if &clone.id == canonical_id {
            continue;
        }
        let detail = explain_non_canonical_clone(clone, canonical);
        cluster.evidence.push(ev(
            &clone.id,
            MemberKind::Clone,
            "not_canonical_clone",
            0.0,
            &detail,
        ));
    }
}

fn explain_non_canonical_clone(c: &CloneRecord, canon: &CloneRecord) -> String {
    let mut parts: Vec<String> = Vec::new();
    match (c.last_commit_at, canon.last_commit_at) {
        (Some(tc), Some(tn)) if tc < tn => {
            parts.push(format!(
                "last commit {} older than canonical {}",
                tc.format("%Y-%m-%d"),
                tn.format("%Y-%m-%d")
            ));
        }
        (None, Some(_)) => {
            parts.push("no recorded last commit on this clone; canonical has activity".into());
        }
        (Some(_), None) => {
            parts.push("planner still preferred the other clone as canonical".into());
        }
        _ => {}
    }
    if c.is_dirty && !canon.is_dirty {
        parts.push("dirty worktree vs clean canonical".into());
    }
    if parts.is_empty() {
        "ranked below canonical on planner tie-break (freshness, git metadata)".into()
    } else {
        parts.join("; ")
    }
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
            let hint = match gittriage_git::merge_base_between_local_clones(
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
                crate::scoring::MERGE_BASE_CANONICAL_BONUS
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

struct ActionExtras<'a> {
    evidence_summary: Option<&'a str>,
    confidence: Option<f64>,
    risk_note: Option<&'a str>,
}

fn effective_oss_threshold(base: u8, profile: ScoringProfile) -> f64 {
    let b = i16::from(base.min(100));
    let adj = match profile {
        ScoringProfile::Default => b,
        ScoringProfile::PublishReadiness => b - 5,
        ScoringProfile::OpenSourceReadiness => b - 10,
        ScoringProfile::SecuritySupplyChain => b,
        ScoringProfile::AiHandoff => b - 5,
    }
    .clamp(0, 100);
    adj as f64
}

fn apply_canonical_pin_to_eval(
    eval: &mut crate::scoring::ClusterEvaluation,
    clones: &[CloneRecord],
    opts: &PlanBuildOpts,
) {
    let pins = &opts.user_intent.pin_canonical_clone_ids;
    if pins.is_empty() {
        return;
    }
    for c in clones {
        if pins.contains(&c.id) {
            eval.canonical_clone_id = Some(c.id.clone());
            eval.evidence.push(ev(
                c.id.as_str(),
                MemberKind::Clone,
                "user_pinned_canonical",
                5.0,
                "clone ID listed in planner.canonical_pins (gittriage.toml)",
            ));
            eval.scores.canonical = (eval.scores.canonical + 5.0).min(100.0);
            return;
        }
    }
}

fn apply_user_intent_post(
    cluster: &mut ClusterRecord,
    actions: &mut Vec<PlanAction>,
    opts: &PlanBuildOpts,
) {
    let ui = &opts.user_intent;
    if ui.ignored_cluster_keys.contains(&cluster.cluster_key) {
        let (sid, sk) = evidence_subject_cluster(cluster);
        cluster.evidence.push(ev(
            &sid,
            sk,
            "user_ignored_cluster",
            0.0,
            "planner.ignored_cluster_keys — plan actions suppressed; scores unchanged",
        ));
        actions.clear();
    }
    if ui.archive_hint_cluster_keys.contains(&cluster.cluster_key) {
        let (sid, sk) = evidence_subject_cluster(cluster);
        cluster.evidence.push(ev(
            &sid,
            sk,
            "user_archive_hint",
            0.0,
            "planner.archive_hint_cluster_keys — user hint to review for archival; no automation",
        ));
    }
    if ui.scoring_profile != ScoringProfile::Default {
        let (sid, sk) = evidence_subject_cluster(cluster);
        let name = ui.scoring_profile.as_str();
        cluster.evidence.push(ev(
            &sid,
            sk,
            "scoring_profile_active",
            0.0,
            &format!("optional profile `{name}` — see docs/SCORING_PROFILES.md; headline score axes unchanged"),
        ));
    }
}

fn build_actions(
    cluster: &ClusterRecord,
    clones: &[CloneRecord],
    remotes: &[RemoteRecord],
    opts: &PlanBuildOpts,
) -> Vec<PlanAction> {
    let mut actions = Vec::with_capacity(8);
    let oss_line = effective_oss_threshold(
        opts.oss_candidate_threshold,
        opts.user_intent.scoring_profile,
    );
    let archive_min = opts.archive_duplicate_canonical_min.min(100) as f64;

    // --- Ambiguous cluster review ---
    if matches!(cluster.status, ClusterStatus::Ambiguous) {
        actions.push(plan_action(
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
            ActionExtras {
                evidence_summary: Some("see `ambiguous_cluster` and related evidence"),
                confidence: Some(cluster.confidence),
                risk_note: Some(
                    "acting on canonical or duplicates may be wrong until the cluster is disambiguated",
                ),
            },
        ));
    }

    // --- Mark canonical (resolved, high confidence) ---
    if matches!(cluster.status, ClusterStatus::Resolved)
        && cluster.scores.canonical >= 60.0
        && cluster.canonical_clone_id.is_some()
    {
        actions.push(plan_action(
            Priority::Low,
            ActionType::MarkCanonical,
            MemberKind::Clone,
            cluster.canonical_clone_id.clone().unwrap(),
            "Resolved cluster with strong canonical signal; confirm this is the primary copy",
            ActionExtras {
                evidence_summary: Some(
                    "canonical score ≥ 60; planner picked by freshness + git metadata",
                ),
                confidence: Some((cluster.scores.canonical / 100.0).min(0.95)),
                risk_note: None,
            },
        ));
    }

    // --- Archive duplicates ---
    if matches!(cluster.status, ClusterStatus::Resolved)
        && clones.len() > 1
        && cluster.scores.canonical >= archive_min
    {
        for clone in clones {
            if Some(&clone.id) != cluster.canonical_clone_id.as_ref() {
                actions.push(plan_action(
                    Priority::High,
                    ActionType::ArchiveLocalDuplicate,
                    MemberKind::Clone,
                    clone.id.clone(),
                    "Lower-priority duplicate clone in same cluster",
                    ActionExtras {
                        evidence_summary: Some("see `not_canonical_clone` evidence for this clone"),
                        confidence: Some(0.65),
                        risk_note: Some(
                            "confirm no unpushed branches or local-only work before removing",
                        ),
                    },
                ));
            }
        }
    }

    // --- Merge diverged clones (merge-base detected shared history but different HEADs) ---
    if clones.len() > 1 {
        let has_merge_base = cluster
            .evidence
            .iter()
            .any(|e| e.kind == "merge_base" && e.score_delta > 0.0);
        if has_merge_base {
            let canon_head = clones
                .iter()
                .find(|c| Some(&c.id) == cluster.canonical_clone_id.as_ref())
                .and_then(|c| c.head_oid.clone());
            for clone in clones {
                if Some(&clone.id) != cluster.canonical_clone_id.as_ref()
                    && clone.head_oid != canon_head
                {
                    actions.push(plan_action(
                        Priority::Medium,
                        ActionType::MergeDivergedClone,
                        MemberKind::Clone,
                        clone.id.clone(),
                        "Clone shares git history with canonical but has diverged; consider merging",
                        ActionExtras {
                            evidence_summary: Some(
                                "merge-base evidence confirms shared ancestor; HEADs differ",
                            ),
                            confidence: Some(0.55),
                            risk_note: Some(
                                "merge conflicts possible; review diff before merging",
                            ),
                        },
                    ));
                }
            }
        }
    }

    // --- Publish readiness actions ---
    let publish_score = cluster.scores.oss_readiness;
    if let Some(clone_id) = cluster.canonical_clone_id.clone() {
        let canon = clones.iter().find(|c| c.id == clone_id);

        // AddMissingDocs: when README is absent
        if canon.map(|c| c.readme_title.is_none()).unwrap_or(false) {
            actions.push(plan_action(
                Priority::Medium,
                ActionType::AddMissingDocs,
                MemberKind::Clone,
                clone_id.clone(),
                "No README detected; add documentation for onboarding and discoverability",
                ActionExtras {
                    evidence_summary: Some("see `no_readme` evidence"),
                    confidence: Some(0.8),
                    risk_note: None,
                },
            ));
        }

        // AddLicense: when license is absent
        if canon.map(|c| c.license_spdx.is_none()).unwrap_or(false) {
            actions.push(plan_action(
                Priority::Medium,
                ActionType::AddLicense,
                MemberKind::Clone,
                clone_id.clone(),
                "No license file detected; add license for legal clarity",
                ActionExtras {
                    evidence_summary: Some("see `no_license` evidence"),
                    confidence: Some(0.7),
                    risk_note: Some("handoff and publication often require explicit licensing"),
                },
            ));
        }

        if publish_score < oss_line {
            actions.push(plan_action(
                Priority::Medium,
                ActionType::AddCi,
                MemberKind::Clone,
                clone_id.clone(),
                "Publish readiness below threshold: add CI baseline",
                ActionExtras {
                    evidence_summary: Some("no strong CI signal from scan heuristics"),
                    confidence: Some(0.5),
                    risk_note: Some("CI gaps increase regression risk for collaborators"),
                },
            ));
            actions.push(plan_action(
                Priority::Medium,
                ActionType::RunSecurityScans,
                MemberKind::Clone,
                clone_id.clone(),
                "Publish readiness below threshold: run semgrep/gitleaks/syft",
                ActionExtras {
                    evidence_summary: Some("optional adapters when installed (`gittriage tools`)"),
                    confidence: Some(0.5),
                    risk_note: Some("scanners can be noisy; triage findings before acting"),
                },
            ));
            actions.push(plan_action(
                Priority::Low,
                ActionType::GenerateSbom,
                MemberKind::Clone,
                clone_id.clone(),
                "Generate software bill of materials for supply-chain visibility",
                ActionExtras {
                    evidence_summary: Some("publish readiness below threshold"),
                    confidence: Some(0.45),
                    risk_note: Some(
                        "requires syft or similar tool on PATH (`gittriage tools` to check)",
                    ),
                },
            ));
        }

        // PublishOssCandidate: when publish score is high enough
        if publish_score >= oss_line && !remotes.is_empty() {
            let has_github = remotes.iter().any(|r| r.provider == "github");
            if has_github {
                actions.push(plan_action(
                    Priority::Low,
                    ActionType::PublishOssCandidate,
                    MemberKind::Clone,
                    clone_id.clone(),
                    "Publish readiness meets threshold; candidate for open-source release",
                    ActionExtras {
                        evidence_summary: Some("license, README, manifest, and remote all present"),
                        confidence: Some((publish_score / 100.0).min(0.9)),
                        risk_note: Some(
                            "review IP, secrets, and internal references before publishing",
                        ),
                    },
                ));
            }
        }
    }

    // --- No remote: suggest creating one ---
    if remotes.is_empty() && !clones.is_empty() {
        if let Some(clone_id) = cluster.canonical_clone_id.clone() {
            actions.push(plan_action(
                Priority::Medium,
                ActionType::CreateRemoteRepo,
                MemberKind::Clone,
                clone_id,
                "Canonical local project has no linked remote",
                ActionExtras {
                    evidence_summary: Some(
                        "see `no_remote_linked` / `local_only_cluster` evidence",
                    ),
                    confidence: Some(0.45),
                    risk_note: Some(
                        "may be intentional offline work; verify before creating or linking remotes",
                    ),
                },
            ));
        }
    }

    // --- Remote-only: suggest cloning locally ---
    if clones.is_empty() && !remotes.is_empty() {
        if let Some(remote_id) = cluster.canonical_remote_id.clone() {
            actions.push(plan_action(
                Priority::Medium,
                ActionType::CloneLocalWorkspace,
                MemberKind::Remote,
                remote_id,
                "Remote-only cluster: add a local clone when you need filesystem scan or merge-base evidence",
                ActionExtras {
                    evidence_summary: Some(
                        "see `remote_only_cluster` evidence; no Clone members in cluster",
                    ),
                    confidence: Some(0.55),
                    risk_note: Some(
                        "GitHub-only triage is still useful; cloning is optional unless you need local tooling",
                    ),
                },
            ));
        }
    }

    actions
}

fn plan_action(
    priority: Priority,
    action_type: ActionType,
    target_kind: MemberKind,
    target_id: String,
    reason: &str,
    extras: ActionExtras<'_>,
) -> PlanAction {
    PlanAction {
        id: format!("action-{}", Uuid::new_v4()),
        priority,
        action_type,
        target_kind,
        target_id,
        reason: reason.into(),
        commands: Vec::new(),
        evidence_summary: extras.evidence_summary.map(str::to_string),
        confidence: extras.confidence,
        risk_note: extras.risk_note.map(str::to_string),
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
