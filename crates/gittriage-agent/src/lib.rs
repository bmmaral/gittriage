//! # GitTriage Agent Logic Core
//!
//! This crate contains the central, deterministic logic for all agent-facing commands
//! (`resolve`, `verdict`, `preflight`, `check-path`, etc.). Both the CLI and the API
//! server are thin wrappers around the functions in this crate. This ensures a
//! consistent data contract and identical behavior across all entry points.
//!
//! To maintain this consistency, any new agent-facing logic or data structures
//! should be added here, not in the CLI or API crates directly.

pub mod error;
mod path_check;
mod preflight;
mod provenance;
mod resolve;
mod summary;
mod verdict;

pub use error::{AgentError, AgentErrorCode};
pub use path_check::{check_path, PathCheckOutput, PathDisposition};
pub use preflight::{preflight, PreflightOutput};
pub use provenance::{Freshness, Provenance};
pub use resolve::{resolve_target, ResolveOutput};
pub use summary::{agent_summary, AgentSummaryOutput, DuplicateGroupSummary, UnsafeTargetSummary};
pub use verdict::{
    automation_verdict_for_cluster, automation_verdict_unresolved, AutomationVerdict,
    AutomationVerdictLabel,
};

/// Duplicate groups touching optional `workspace_roots` (empty = all clusters).
pub fn list_duplicate_groups(
    plan: &gittriage_core::PlanDocument,
    snapshot: &gittriage_core::InventorySnapshot,
    workspace_roots: &[std::path::PathBuf],
) -> Vec<DuplicateGroupSummary> {
    agent_summary(plan, snapshot, workspace_roots).duplicate_groups
}

/// Clusters marked unsafe for automation within optional workspace roots.
pub fn list_unsafe_targets(
    plan: &gittriage_core::PlanDocument,
    snapshot: &gittriage_core::InventorySnapshot,
    workspace_roots: &[std::path::PathBuf],
) -> Vec<UnsafeTargetSummary> {
    agent_summary(plan, snapshot, workspace_roots).unsafe_targets
}
