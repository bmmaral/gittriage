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

/// Optional scan-time metadata persisted in `runs.stats_json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RunScanStats {
    /// Nested `.git` directories skipped because `scan.include_nested_git` is false.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_nested_git: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub roots: Vec<String>,
    pub github_owner: Option<String>,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<RunScanStats>,
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

/// Filter markdown/TUI/plan views to one member-scope bucket ([`ClusterRecord::cluster_scope`]).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClusterScopeFilter {
    LocalOnly,
    Mixed,
    RemoteOnly,
}

impl ClusterScopeFilter {
    pub fn matches(self, scope: ClusterScope) -> bool {
        matches!(
            (scope, self),
            (ClusterScope::LocalOnly, Self::LocalOnly)
                | (ClusterScope::Mixed, Self::Mixed)
                | (ClusterScope::RemoteOnly, Self::RemoteOnly)
        )
    }
}

/// Whether a cluster has local clone members, remote-only members, or both.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClusterScope {
    /// At least one clone member, no remote members.
    LocalOnly,
    /// Both clone and remote members.
    Mixed,
    /// Remote members only (no local checkout in this cluster).
    RemoteOnly,
    /// No members (unexpected; treated as its own bucket for reporting).
    Empty,
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

impl ClusterRecord {
    /// Classify cluster by member kinds (local clones vs remotes in the inventory).
    pub fn cluster_scope(&self) -> ClusterScope {
        let has_clone = self.members.iter().any(|m| m.kind == MemberKind::Clone);
        let has_remote = self.members.iter().any(|m| m.kind == MemberKind::Remote);
        match (has_clone, has_remote) {
            (true, false) => ClusterScope::LocalOnly,
            (true, true) => ClusterScope::Mixed,
            (false, true) => ClusterScope::RemoteOnly,
            _ => ClusterScope::Empty,
        }
    }
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

/// Summary of the last `plan --external` / adapter attach (optional in JSON for backward compatibility).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PlanExternalAdapterRun {
    /// How many adapter binaries were on `PATH` at run start (gitleaks, semgrep, jscpd, syft).
    pub tools_on_path: u8,
    /// Clusters with a canonical clone path considered for adapter runs.
    pub canonical_clone_roots_considered: u32,
    /// Tool invocations attempted (up to 4 per canonical root).
    pub tool_spawn_attempts: u32,
    /// New adapter evidence rows attached (`jscpd_scan`, `semgrep_scan`, …).
    pub evidence_items_attached: u32,
    /// Canonical paths that were not directories on disk.
    pub skipped_clone_path_not_directory: u32,
    /// Runs that timed out or exited non-zero (still recorded as evidence when applicable).
    pub runs_timeout_or_nonzero_exit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDocument {
    /// JSON plan format version. Missing in older files deserializes as `1`.
    #[serde(default = "plan_schema_version")]
    pub schema_version: u32,
    /// Version of deterministic scoring rules (`gittriage-plan`); not the app semver.
    #[serde(default = "scoring_rules_version_default")]
    pub scoring_rules_version: u32,
    pub generated_at: DateTime<Utc>,
    pub generated_by: String,
    pub clusters: Vec<ClusterPlan>,
    /// Populated after optional external adapter attach.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_adapter_run: Option<PlanExternalAdapterRun>,
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

/// Explains [`InventorySnapshot::clones`] for exports and APIs (v1: all roots stay in `clones`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InventorySemantics {
    pub version: u32,
    pub git_checkout_count: usize,
    pub manifest_only_project_root_count: usize,
    /// Human-readable clarification for `project_roots` vs `git_only` consumers.
    pub note: String,
}

impl Default for InventorySemantics {
    fn default() -> Self {
        Self {
            version: 1,
            git_checkout_count: 0,
            manifest_only_project_root_count: 0,
            note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventorySnapshot {
    pub run: Option<RunRecord>,
    pub clones: Vec<CloneRecord>,
    pub remotes: Vec<RemoteRecord>,
    pub links: Vec<CloneRemoteLink>,
    /// Derived from `clones` on load/export; documents that `clones` is “project roots”, not only git checkouts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantics: Option<InventorySemantics>,
}

impl InventorySnapshot {
    /// Refreshes [`InventorySnapshot::semantics`] from [`Self::clones`].
    pub fn refresh_semantics(&mut self) {
        let git = self.clones.iter().filter(|c| c.is_git).count();
        let total = self.clones.len();
        self.semantics = Some(InventorySemantics {
            version: 1,
            git_checkout_count: git,
            manifest_only_project_root_count: total.saturating_sub(git),
            note: "Field `clones` lists every scanned project root. `is_git == true` means a `.git` directory was present; `false` is a manifest-only root when using scan_mode project_roots.".into(),
        });
    }
}
