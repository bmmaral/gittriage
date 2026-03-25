use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManifestKind {
    Cargo,
    PackageJson,
    PyProject,
    RequirementsTxt,
    CMake,
    Makefile,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClusterStatus {
    Resolved,
    Ambiguous,
    ManualReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemberKind {
    Clone,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionType {
    MarkCanonical,
    ArchiveLocalDuplicate,
    ReviewAmbiguousCluster,
    MergeDivergedClone,
    CreateRemoteRepo,
    /// Inventory has this remote but no local clone; clone locally when filesystem scans are needed.
    CloneLocalWorkspace,
    AddMissingDocs,
    AddLicense,
    AddCi,
    RunSecurityScans,
    GenerateSbom,
    PublishOssCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub roots: Vec<String>,
    pub github_owner: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneRecord {
    pub id: String,
    pub path: String,
    pub display_name: String,
    pub is_git: bool,
    pub head_oid: Option<String>,
    pub active_branch: Option<String>,
    pub default_branch: Option<String>,
    pub is_dirty: bool,
    pub last_commit_at: Option<DateTime<Utc>>,
    pub size_bytes: Option<u64>,
    pub manifest_kind: Option<ManifestKind>,
    pub readme_title: Option<String>,
    pub license_spdx: Option<String>,
    pub fingerprint: Option<String>,
    /// Scan-time only: true when a lockfile (Cargo.lock, package-lock.json, …) is present.
    /// Not persisted in SQLite; defaults to false when loaded from DB.
    #[serde(default)]
    pub has_lockfile: bool,
    /// Scan-time only: true when a CI configuration (.github/workflows, .gitlab-ci.yml, …) is present.
    #[serde(default)]
    pub has_ci: bool,
    /// Scan-time only: true when a test directory (tests/, test/, spec/, …) is present.
    #[serde(default)]
    pub has_tests_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRecord {
    pub id: String,
    pub provider: String,
    pub owner: Option<String>,
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub url: String,
    pub normalized_url: String,
    pub default_branch: Option<String>,
    pub is_fork: bool,
    pub is_archived: bool,
    pub is_private: bool,
    pub pushed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub kind: MemberKind,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: String,
    pub subject_kind: MemberKind,
    pub subject_id: String,
    pub kind: String,
    pub score_delta: f64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBundle {
    pub canonical: f64,
    /// Repo health / project hygiene (manifest, README, …).
    pub usability: f64,
    /// Ability to recover or resync the project (git metadata, remotes, recency, clean tree).
    #[serde(default)]
    pub recoverability: f64,
    /// Publish / handoff readiness (JSON field name unchanged for compatibility).
    pub oss_readiness: f64,
    pub risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterRecord {
    pub id: String,
    pub cluster_key: String,
    pub label: String,
    pub status: ClusterStatus,
    pub confidence: f64,
    pub canonical_clone_id: Option<String>,
    pub canonical_remote_id: Option<String>,
    pub members: Vec<ClusterMember>,
    pub evidence: Vec<EvidenceItem>,
    pub scores: ScoreBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAction {
    pub id: String,
    pub priority: Priority,
    pub action_type: ActionType,
    pub target_kind: MemberKind,
    pub target_id: String,
    pub reason: String,
    pub commands: Vec<String>,
    /// Short summary of evidence motivating this action (optional in JSON for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_summary: Option<String>,
    /// Planner confidence in this recommendation, 0.0–1.0 (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Risk or trade-off the user should weigh (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPlan {
    pub cluster: ClusterRecord,
    pub actions: Vec<PlanAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDocument {
    /// JSON plan format version. Missing in older files deserializes as `1`.
    #[serde(default = "plan_schema_version")]
    pub schema_version: u32,
    /// Version of deterministic scoring rules (`nexus-plan`); not the app semver.
    #[serde(default = "scoring_rules_version_default")]
    pub scoring_rules_version: u32,
    pub generated_at: DateTime<Utc>,
    pub generated_by: String,
    pub clusters: Vec<ClusterPlan>,
}

const fn plan_schema_version() -> u32 {
    1
}

const fn scoring_rules_version_default() -> u32 {
    1
}

/// Association between a scanned local clone and a persisted remote row (git origin, GitHub match, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloneRemoteLink {
    pub clone_id: String,
    pub remote_id: String,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventorySnapshot {
    pub run: Option<RunRecord>,
    pub clones: Vec<CloneRecord>,
    pub remotes: Vec<RemoteRecord>,
    pub links: Vec<CloneRemoteLink>,
}
