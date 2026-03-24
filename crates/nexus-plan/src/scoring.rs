//! Deterministic cluster scoring. See `docs/SCORING.md`.
//!
//! Version is carried on [`nexus_core::PlanDocument::scoring_rules_version`], not the CLI semver.

use chrono::{Duration, Utc};
use nexus_core::{CloneRecord, ClusterStatus, EvidenceItem, MemberKind, RemoteRecord, ScoreBundle};
use uuid::Uuid;

/// Bump when rule weights or evidence kinds change materially (keep in sync with docs).
pub const SCORING_RULES_VERSION: u32 = 4;

/// Extra canonical confidence when `git merge-base` finds a common ancestor between two clones.
pub const MERGE_BASE_CANONICAL_BONUS: f64 = 8.0;

pub struct ClusterEvaluation {
    pub scores: ScoreBundle,
    pub evidence: Vec<EvidenceItem>,
    pub canonical_clone_id: Option<String>,
    pub canonical_remote_id: Option<String>,
    pub status: ClusterStatus,
    pub confidence: f64,
}

pub fn evaluate_cluster(
    clones: &[CloneRecord],
    remotes: &[RemoteRecord],
    ambiguous_confidence_threshold: f64,
) -> ClusterEvaluation {
    let mut evidence = Vec::new();
    let mut scores = ScoreBundle::default();

    let canonical_clone = clones
        .iter()
        .max_by_key(|c| (c.last_commit_at, c.is_git, !c.is_dirty));
    let canonical_remote = remotes.iter().max_by_key(|r| r.pushed_at);

    if let Some(clone) = canonical_clone {
        bump_canonical(
            &mut scores,
            14.0,
            &mut evidence,
            &clone.id,
            MemberKind::Clone,
            "canonical_clone_pick",
            "selected as canonical local candidate (freshness, git metadata, clean tree tie-break)",
        );

        if clone.is_git {
            bump_canonical(
                &mut scores,
                10.0,
                &mut evidence,
                &clone.id,
                MemberKind::Clone,
                "git_repo",
                ".git metadata present",
            );
        }
        if clone.head_oid.is_some() {
            bump_canonical(
                &mut scores,
                6.0,
                &mut evidence,
                &clone.id,
                MemberKind::Clone,
                "commit_head_present",
                "HEAD oid recorded",
            );
        }
        if clone.default_branch.is_some() {
            bump_canonical(
                &mut scores,
                5.0,
                &mut evidence,
                &clone.id,
                MemberKind::Clone,
                "default_branch_known",
                "default branch recorded",
            );
        }
        if clone.active_branch.is_some() {
            bump_canonical(
                &mut scores,
                4.0,
                &mut evidence,
                &clone.id,
                MemberKind::Clone,
                "active_branch_known",
                "active branch recorded",
            );
        }

        if let Some(t) = clone.last_commit_at {
            let age = Utc::now() - t;
            if age <= Duration::days(14) {
                bump_canonical(
                    &mut scores,
                    12.0,
                    &mut evidence,
                    &clone.id,
                    MemberKind::Clone,
                    "recent_activity",
                    "last commit within 14 days",
                );
            } else if age <= Duration::days(90) {
                bump_canonical(
                    &mut scores,
                    8.0,
                    &mut evidence,
                    &clone.id,
                    MemberKind::Clone,
                    "fresh_commits",
                    "last commit within 90 days",
                );
            } else if age <= Duration::days(365) {
                bump_canonical(
                    &mut scores,
                    4.0,
                    &mut evidence,
                    &clone.id,
                    MemberKind::Clone,
                    "stale_but_tracked",
                    "last commit within 12 months",
                );
            }
        }

        if clone.is_dirty {
            bump_canonical(
                &mut scores,
                -4.0,
                &mut evidence,
                &clone.id,
                MemberKind::Clone,
                "dirty_worktree",
                "uncommitted changes on canonical clone",
            );
        }

        // Repo health (usability)
        if clone.manifest_kind.is_some() {
            bump_usability(
                &mut scores,
                14.0,
                &mut evidence,
                &clone.id,
                "manifest_present",
                "project manifest detected",
            );
        }
        if clone.readme_title.is_some() {
            bump_usability(
                &mut scores,
                12.0,
                &mut evidence,
                &clone.id,
                "readme_present",
                "README / title detected",
            );
        }
        if clone.license_spdx.is_some() {
            bump_usability(
                &mut scores,
                6.0,
                &mut evidence,
                &clone.id,
                "license_signal_usability",
                "license metadata supports onboarding",
            );
        }
        if clone.fingerprint.is_some() {
            bump_usability(
                &mut scores,
                4.0,
                &mut evidence,
                &clone.id,
                "content_fingerprint",
                "scan fingerprint present",
            );
        }

        // Recoverability
        if clone.is_git {
            bump_recoverability(
                &mut scores,
                18.0,
                &mut evidence,
                &clone.id,
                "git_object_db",
                "full git history available locally",
            );
        }
        if clone.head_oid.is_some() {
            bump_recoverability(
                &mut scores,
                10.0,
                &mut evidence,
                &clone.id,
                "resolved_head",
                "HEAD resolved for checkout",
            );
        }
        if clone.default_branch.is_some() {
            bump_recoverability(
                &mut scores,
                10.0,
                &mut evidence,
                &clone.id,
                "default_branch_recover",
                "default branch aids clone/sync",
            );
        }
        if clone.active_branch.is_some() {
            bump_recoverability(
                &mut scores,
                6.0,
                &mut evidence,
                &clone.id,
                "active_branch_recover",
                "active branch indicates working state",
            );
        }
        if !clone.is_dirty {
            bump_recoverability(
                &mut scores,
                8.0,
                &mut evidence,
                &clone.id,
                "clean_worktree_recover",
                "clean tree easier to sync",
            );
        }
        if let Some(t) = clone.last_commit_at {
            let age = Utc::now() - t;
            if age <= Duration::days(90) {
                bump_recoverability(
                    &mut scores,
                    12.0,
                    &mut evidence,
                    &clone.id,
                    "recent_sync_signal",
                    "recent commit supports recovery confidence",
                );
            }
        }
        if !remotes.is_empty() {
            bump_recoverability(
                &mut scores,
                16.0,
                &mut evidence,
                &clone.id,
                "remote_backup_path",
                "cluster has linked remote(s)",
            );
        }

        // Publish readiness
        if clone.license_spdx.is_some() {
            bump_publish(
                &mut scores,
                18.0,
                &mut evidence,
                &clone.id,
                MemberKind::Clone,
                "license_present",
                "SPDX / license file signal",
            );
        }
    }

    if let Some(remote) = canonical_remote {
        bump_canonical(
            &mut scores,
            18.0,
            &mut evidence,
            &remote.id,
            MemberKind::Remote,
            "upstream_remote",
            "canonical remote candidate (push recency)",
        );
        if remote.default_branch.is_some() {
            bump_canonical(
                &mut scores,
                5.0,
                &mut evidence,
                &remote.id,
                MemberKind::Remote,
                "remote_default_branch",
                "upstream default branch known",
            );
        }

        if !remote.is_archived {
            bump_publish(
                &mut scores,
                12.0,
                &mut evidence,
                &remote.id,
                MemberKind::Remote,
                "remote_active",
                "remote not archived",
            );
        } else {
            bump_publish(
                &mut scores,
                -8.0,
                &mut evidence,
                &remote.id,
                MemberKind::Remote,
                "remote_archived",
                "archived upstream reduces publish readiness",
            );
        }
        if !remote.is_fork {
            bump_publish(
                &mut scores,
                6.0,
                &mut evidence,
                &remote.id,
                MemberKind::Remote,
                "not_fork_signal",
                "upstream appears primary (not marked fork)",
            );
        } else {
            bump_publish(
                &mut scores,
                -4.0,
                &mut evidence,
                &remote.id,
                MemberKind::Remote,
                "fork_signal",
                "fork flag on remote metadata",
            );
        }
        if let Some(t) = remote.pushed_at {
            let age = Utc::now() - t;
            if age <= Duration::days(90) {
                bump_publish(
                    &mut scores,
                    6.0,
                    &mut evidence,
                    &remote.id,
                    MemberKind::Remote,
                    "remote_recent_push",
                    "push activity within 90 days",
                );
            }
        }
    }

    if clones.len() > 1 {
        let sid = clones.first().map(|c| c.id.as_str()).unwrap_or("cluster");
        bump_risk(
            &mut scores,
            24.0,
            &mut evidence,
            sid,
            MemberKind::Clone,
            "multiple_clones",
            "more than one local clone in cluster",
        );
    }

    if remotes.is_empty() && !clones.is_empty() {
        if let Some(c) = canonical_clone {
            bump_risk(
                &mut scores,
                14.0,
                &mut evidence,
                &c.id,
                MemberKind::Clone,
                "no_remote_linked",
                "local-only cluster: no remote rows",
            );
            bump_canonical(
                &mut scores,
                -6.0,
                &mut evidence,
                &c.id,
                MemberKind::Clone,
                "local_only_cluster",
                "no remote linkage lowers canonical confidence",
            );
        }
    }

    if clones.is_empty() && !remotes.is_empty() {
        let r = remotes.first().expect("non-empty");
        bump_risk(
            &mut scores,
            12.0,
            &mut evidence,
            &r.id,
            MemberKind::Remote,
            "remote_only_cluster",
            "no local checkout in cluster",
        );
    }

    let confidence = cluster_confidence(clones, remotes);
    let status = if confidence < ambiguous_confidence_threshold {
        ClusterStatus::Ambiguous
    } else {
        ClusterStatus::Resolved
    };

    if matches!(status, ClusterStatus::Ambiguous) {
        let (sid, sk) = canonical_subject_tuple(canonical_clone, canonical_remote, clones, remotes);
        push_ev(
            &mut evidence,
            sid,
            sk,
            "ambiguous_cluster",
            0.0,
            "cluster confidence is below threshold; canonical selection is tentative—verify before cleanup or automation",
        );
    }

    ClusterEvaluation {
        scores,
        evidence,
        canonical_clone_id: canonical_clone.map(|c| c.id.clone()),
        canonical_remote_id: canonical_remote.map(|r| r.id.clone()),
        status,
        confidence,
    }
}

fn cluster_confidence(clones: &[CloneRecord], remotes: &[RemoteRecord]) -> f64 {
    if clones.len() + remotes.len() <= 1 {
        0.5
    } else if clones.len() > 1 && remotes.is_empty() {
        0.55
    } else {
        0.8
    }
}

fn canonical_subject_tuple<'a>(
    canonical_clone: Option<&'a CloneRecord>,
    canonical_remote: Option<&'a RemoteRecord>,
    clones: &'a [CloneRecord],
    remotes: &'a [RemoteRecord],
) -> (&'a str, MemberKind) {
    if let Some(c) = canonical_clone {
        return (c.id.as_str(), MemberKind::Clone);
    }
    if let Some(r) = canonical_remote {
        return (r.id.as_str(), MemberKind::Remote);
    }
    if let Some(c) = clones.first() {
        return (c.id.as_str(), MemberKind::Clone);
    }
    if let Some(r) = remotes.first() {
        return (r.id.as_str(), MemberKind::Remote);
    }
    ("cluster", MemberKind::Clone)
}

fn bump_canonical(
    scores: &mut ScoreBundle,
    delta: f64,
    evidence: &mut Vec<EvidenceItem>,
    subject_id: &str,
    subject_kind: MemberKind,
    kind: &str,
    detail: &str,
) {
    scores.canonical += delta;
    push_ev(evidence, subject_id, subject_kind, kind, delta, detail);
}

fn bump_usability(
    scores: &mut ScoreBundle,
    delta: f64,
    evidence: &mut Vec<EvidenceItem>,
    subject_id: &str,
    kind: &str,
    detail: &str,
) {
    scores.usability += delta;
    push_ev(evidence, subject_id, MemberKind::Clone, kind, delta, detail);
}

fn bump_recoverability(
    scores: &mut ScoreBundle,
    delta: f64,
    evidence: &mut Vec<EvidenceItem>,
    subject_id: &str,
    kind: &str,
    detail: &str,
) {
    scores.recoverability += delta;
    push_ev(evidence, subject_id, MemberKind::Clone, kind, delta, detail);
}

fn bump_publish(
    scores: &mut ScoreBundle,
    delta: f64,
    evidence: &mut Vec<EvidenceItem>,
    subject_id: &str,
    subject_kind: MemberKind,
    kind: &str,
    detail: &str,
) {
    scores.oss_readiness += delta;
    push_ev(evidence, subject_id, subject_kind, kind, delta, detail);
}

fn bump_risk(
    scores: &mut ScoreBundle,
    delta: f64,
    evidence: &mut Vec<EvidenceItem>,
    subject_id: &str,
    subject_kind: MemberKind,
    kind: &str,
    detail: &str,
) {
    scores.risk += delta;
    push_ev(evidence, subject_id, subject_kind, kind, delta, detail);
}

fn push_ev(
    evidence: &mut Vec<EvidenceItem>,
    subject_id: &str,
    subject_kind: MemberKind,
    kind: &str,
    delta: f64,
    detail: &str,
) {
    evidence.push(EvidenceItem {
        id: format!("ev-{}", Uuid::new_v4()),
        subject_kind,
        subject_id: subject_id.into(),
        kind: kind.into(),
        score_delta: delta,
        detail: detail.into(),
    });
}

pub fn finalize_scores(mut scores: ScoreBundle) -> ScoreBundle {
    scores.canonical = scores.canonical.clamp(0.0, 100.0);
    scores.usability = scores.usability.clamp(0.0, 100.0);
    scores.recoverability = scores.recoverability.clamp(0.0, 100.0);
    scores.oss_readiness = scores.oss_readiness.clamp(0.0, 100.0);
    scores.risk = scores.risk.clamp(0.0, 100.0);
    scores
}
