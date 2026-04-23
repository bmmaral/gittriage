//! Optional CLI adapters (jscpd, semgrep, gitleaks, syft). They never block the core pipeline.
//!
//! Adapters are **best-effort**: missing tools are silently skipped, timeouts are caught,
//! and failures produce informational evidence rather than errors.

use anyhow::Result;
use gittriage_core::{EvidenceItem, InventorySnapshot, MemberKind, PlanDocument};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// ── Tool registry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExternalTool {
    Jscpd,
    Semgrep,
    Gitleaks,
    Syft,
}

impl ExternalTool {
    pub fn bin_name(self) -> &'static str {
        match self {
            Self::Jscpd => "jscpd",
            Self::Semgrep => "semgrep",
            Self::Gitleaks => "gitleaks",
            Self::Syft => "syft",
        }
    }

    pub fn evidence_kind(self) -> &'static str {
        match self {
            Self::Jscpd => "jscpd_scan",
            Self::Semgrep => "semgrep_scan",
            Self::Gitleaks => "gitleaks_detect",
            Self::Syft => "syft_sbom",
        }
    }

    /// Which category this adapter belongs to for profile filtering.
    pub fn category(self) -> AdapterCategory {
        match self {
            Self::Gitleaks => AdapterCategory::Security,
            Self::Semgrep => AdapterCategory::Security,
            Self::Jscpd => AdapterCategory::Quality,
            Self::Syft => AdapterCategory::SupplyChain,
        }
    }

    /// Official support status.
    pub fn support_tier(self) -> SupportTier {
        match self {
            Self::Gitleaks => SupportTier::OfficiallySupported,
            Self::Semgrep => SupportTier::OfficiallySupported,
            Self::Syft => SupportTier::OfficiallySupported,
            Self::Jscpd => SupportTier::BestEffort,
        }
    }

    pub const ALL: [ExternalTool; 4] = [Self::Gitleaks, Self::Semgrep, Self::Jscpd, Self::Syft];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterCategory {
    Security,
    Quality,
    SupplyChain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportTier {
    /// Tested in CI, documented, breakage is a bug.
    OfficiallySupported,
    /// Works when available, not guaranteed across versions.
    BestEffort,
}

// ── Probe ────────────────────────────────────────────────────────────────────

/// Whether each supported tool is on `PATH`.
pub fn probe_all() -> Vec<(ExternalTool, bool)> {
    ExternalTool::ALL
        .into_iter()
        .map(|t| (t, tool_on_path(t.bin_name())))
        .collect()
}

// ── Per-run cache ────────────────────────────────────────────────────────────

/// Caches adapter results keyed by (tool, directory path) so the same clone
/// is never scanned twice in a single `--external` invocation.
#[derive(Default)]
pub struct AdapterCache {
    results: HashMap<(ExternalTool, String), Option<AdapterResult>>,
}

#[derive(Debug, Clone)]
pub struct AdapterResult {
    pub exit_code: i32,
    pub summary: String,
}

impl AdapterCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_or_run(&mut self, tool: ExternalTool, root: &Path) -> &Option<AdapterResult> {
        let key = (tool, root.to_string_lossy().to_string());
        self.results.entry(key).or_insert_with_key(|k| {
            let (tool, _) = k;
            run_tool(*tool, root)
        })
    }
}

fn run_tool(tool: ExternalTool, root: &Path) -> Option<AdapterResult> {
    let (args, bin): (&[&str], &str) = match tool {
        ExternalTool::Gitleaks => (
            &["detect", "-s", ".", "--exit-code", "0", "--no-banner"],
            "gitleaks",
        ),
        ExternalTool::Semgrep => (
            &["scan", "--config", "p/ci", "--quiet", "--error", "."],
            "semgrep",
        ),
        ExternalTool::Jscpd => (&[".", "--silent", "--min-lines", "10"], "jscpd"),
        ExternalTool::Syft => (&[".", "-o", "json"], "syft"),
    };

    run_capture(bin, args, root).map(|(code, summary)| {
        let summary = if tool == ExternalTool::Syft && summary.len() > 240 {
            format!("{}…", &summary[..240])
        } else {
            summary
        };
        AdapterResult {
            exit_code: code,
            summary,
        }
    })
}

// ── Normalized evidence ──────────────────────────────────────────────────────

/// Structured adapter evidence with a consistent schema regardless of tool.
fn adapter_evidence(tool: ExternalTool, clone_id: &str, result: &AdapterResult) -> EvidenceItem {
    let prefix = tool.bin_name();
    let detail = format!("{prefix}: {}", result.summary);
    evid(clone_id, tool.evidence_kind(), 0.0, detail)
}

// ── Main entry point ─────────────────────────────────────────────────────────

/// Append lightweight evidence rows for each cluster's canonical clone when tools exist.
///
/// Adapters are best-effort: missing tools are silently skipped, timeouts produce
/// informational evidence, and failures never propagate as errors. A per-run cache
/// prevents scanning the same directory twice.
pub fn attach_external_evidence(
    plan: &mut PlanDocument,
    snapshot: &InventorySnapshot,
) -> Result<()> {
    let mut cache = AdapterCache::new();
    attach_external_evidence_cached(plan, snapshot, &mut cache)
}

/// Count evidence rows produced by optional CLI adapters (for `--external` summaries).
pub fn count_adapter_evidence(plan: &PlanDocument) -> usize {
    const KINDS: &[&str] = &["jscpd_scan", "semgrep_scan", "gitleaks_detect", "syft_sbom"];
    plan.clusters
        .iter()
        .flat_map(|cp| &cp.cluster.evidence)
        .filter(|e| KINDS.contains(&e.kind.as_str()))
        .count()
}

/// Same as [`attach_external_evidence`] but accepts a reusable cache.
pub fn attach_external_evidence_cached(
    plan: &mut PlanDocument,
    snapshot: &InventorySnapshot,
    cache: &mut AdapterCache,
) -> Result<()> {
    let tools_on_path = probe_all().iter().filter(|(_, ok)| *ok).count() as u8;
    let evidence_before = count_adapter_evidence(plan);
    let by_id: HashMap<_, _> = snapshot.clones.iter().map(|c| (c.id.clone(), c)).collect();

    let mut canonical_clone_roots_considered: u32 = 0;
    let mut skipped_clone_path_not_directory: u32 = 0;
    let mut tool_spawn_attempts: u32 = 0;
    let mut runs_timeout_or_nonzero_exit: u32 = 0;

    for cp in &mut plan.clusters {
        let Some(cid) = cp.cluster.canonical_clone_id.as_ref() else {
            continue;
        };
        let Some(clone) = by_id.get(cid) else {
            continue;
        };
        let root = Path::new(clone.path.as_str());
        canonical_clone_roots_considered = canonical_clone_roots_considered.saturating_add(1);
        if !root.is_dir() {
            skipped_clone_path_not_directory = skipped_clone_path_not_directory.saturating_add(1);
            continue;
        }

        for tool in ExternalTool::ALL {
            tool_spawn_attempts = tool_spawn_attempts.saturating_add(1);
            if let Some(result) = cache.get_or_run(tool, root).clone() {
                if result.exit_code != 0 {
                    runs_timeout_or_nonzero_exit = runs_timeout_or_nonzero_exit.saturating_add(1);
                }
                cp.cluster
                    .evidence
                    .push(adapter_evidence(tool, cid, &result));
            }
        }
    }

    let evidence_after = count_adapter_evidence(plan);
    plan.external_adapter_run = Some(gittriage_core::PlanExternalAdapterRun {
        tools_on_path,
        canonical_clone_roots_considered,
        tool_spawn_attempts,
        evidence_items_attached: evidence_after.saturating_sub(evidence_before) as u32,
        skipped_clone_path_not_directory,
        runs_timeout_or_nonzero_exit,
    });

    Ok(())
}

// ── Profile-filtered variant ─────────────────────────────────────────────────

/// Only run adapters whose category matches the requested set.
pub fn attach_filtered_evidence(
    plan: &mut PlanDocument,
    snapshot: &InventorySnapshot,
    categories: &[AdapterCategory],
    cache: &mut AdapterCache,
) -> Result<()> {
    let by_id: HashMap<_, _> = snapshot.clones.iter().map(|c| (c.id.clone(), c)).collect();

    for cp in &mut plan.clusters {
        let Some(cid) = cp.cluster.canonical_clone_id.as_ref() else {
            continue;
        };
        let Some(clone) = by_id.get(cid) else {
            continue;
        };
        let root = Path::new(clone.path.as_str());
        if !root.is_dir() {
            continue;
        }

        for tool in ExternalTool::ALL {
            if !categories.contains(&tool.category()) {
                continue;
            }
            if let Some(result) = cache.get_or_run(tool, root).clone() {
                cp.cluster
                    .evidence
                    .push(adapter_evidence(tool, cid, &result));
            }
        }
    }

    Ok(())
}

// ── Subprocess runner ────────────────────────────────────────────────────────

/// Wall-clock limit per adapter invocation. Override with `GITTRIAGE_ADAPTER_TIMEOUT_SECS` (1–86400).
fn adapter_timeout() -> Duration {
    const DEFAULT: Duration = Duration::from_secs(180);
    std::env::var("GITTRIAGE_ADAPTER_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&s| (1..=86_400).contains(&s))
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT)
}

fn run_capture(bin: &str, args: &[&str], cwd: &Path) -> Option<(i32, String)> {
    let _ = tool_on_path(bin).then_some(())?;
    let timeout = adapter_timeout();

    #[cfg(windows)]
    let mut child = {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(bin).args(args);
        command
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .ok()?
    };

    #[cfg(not(windows))]
    let mut child = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_handle = thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut out) = stdout {
            let _ = out.read_to_string(&mut s);
        }
        s
    });
    let stderr_handle = thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut err) = stderr {
            let _ = err.read_to_string(&mut s);
        }
        s
    });

    let start = Instant::now();
    loop {
        let waited = match child.try_wait() {
            Ok(s) => s,
            Err(_) => return None,
        };
        match waited {
            Some(status) => {
                let code = status.code().unwrap_or(-1);
                let stdout = stdout_handle.join().unwrap_or_default();
                let stderr = stderr_handle.join().unwrap_or_default();
                let msg = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.trim().to_string()
                };
                let short = msg.lines().next().unwrap_or("").to_string();
                return Some((
                    code,
                    if short.is_empty() {
                        format!("exit {code}")
                    } else {
                        short
                    },
                ));
            }
            None => {
                if start.elapsed() >= timeout {
                    if let Err(e) = child.kill() {
                        tracing::warn!(tool = %bin, error = %e, "failed to kill timed-out adapter");
                    }
                    let _ = child.wait();
                    let _ = stdout_handle.join();
                    let _ = stderr_handle.join();
                    let secs = timeout.as_secs();
                    tracing::warn!(tool = %bin, seconds = secs, "external adapter timed out");
                    return Some((124, format!("timed out after {secs}s")));
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn tool_on_path(bin: &str) -> bool {
    if which::which(bin).is_ok() {
        return true;
    }

    #[cfg(windows)]
    {
        for candidate in [
            format!("{bin}.cmd"),
            format!("{bin}.bat"),
            format!("{bin}.exe"),
        ] {
            if which::which(candidate).is_ok() {
                return true;
            }
        }
    }

    false
}

fn evid(clone_id: &str, kind: &str, delta: f64, detail: String) -> EvidenceItem {
    EvidenceItem {
        id: format!("ext-{}", uuid::Uuid::new_v4()),
        subject_kind: MemberKind::Clone,
        subject_id: clone_id.into(),
        kind: kind.into(),
        score_delta: delta,
        detail,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────
