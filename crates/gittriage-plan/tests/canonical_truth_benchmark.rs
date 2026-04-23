//! Corpus-driven benchmark for canonical correctness.
//!
//! Each fixture describes a small inventory snapshot plus the canonical member
//! the planner should select. The harness keeps the cases stable and prints a
//! summary that can be tracked in CI logs.

use gittriage_core::{ClusterPlan, InventorySnapshot};
use gittriage_plan::{resolve_clusters, PlanBuildOpts};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct CanonicalTruthCase {
    name: String,
    #[serde(default)]
    notes: Option<String>,
    snapshot: InventorySnapshot,
    expected: ExpectedCanonical,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct ExpectedCanonical {
    cluster_count: usize,
    cluster_key: String,
    canonical_clone_id: Option<String>,
    canonical_remote_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ActualCanonical {
    cluster_count: usize,
    cluster_key: String,
    canonical_clone_id: Option<String>,
    canonical_remote_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    name: String,
    notes: Option<String>,
    passed: bool,
    expected: ExpectedCanonical,
    actual: Option<ActualCanonical>,
    failure: Option<String>,
}

#[derive(Debug, Serialize)]
struct BenchmarkSummary {
    total: usize,
    passed: usize,
    failed: usize,
    results: Vec<CaseResult>,
}

fn default_opts() -> PlanBuildOpts {
    PlanBuildOpts {
        merge_base: false,
        ..Default::default()
    }
}

fn load_cases() -> Vec<CanonicalTruthCase> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/canonical_truth");
    let mut cases = Vec::new();

    for entry in fs::read_dir(&dir).expect("read canonical truth fixture dir") {
        let entry = entry.expect("read fixture entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
        let case: CanonicalTruthCase = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse fixture {}: {e}", path.display()));
        cases.push(case);
    }

    cases.sort_by(|a, b| a.name.cmp(&b.name));
    cases
}

fn summary_output_path() -> PathBuf {
    let dir = env::var_os("GITTRIAGE_CANONICAL_TRUTH_SUMMARY_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/truth-benchmark")
        });
    dir.join("canonical_truth_summary.json")
}

fn actual_for_cluster(cluster: &ClusterPlan, cluster_count: usize) -> ActualCanonical {
    ActualCanonical {
        cluster_count,
        cluster_key: cluster.cluster.cluster_key.clone(),
        canonical_clone_id: cluster.cluster.canonical_clone_id.clone(),
        canonical_remote_id: cluster.cluster.canonical_remote_id.clone(),
    }
}

fn evaluate_case(case: &CanonicalTruthCase) -> CaseResult {
    let plans = resolve_clusters(&case.snapshot, &default_opts());
    let actual = plans
        .iter()
        .find(|plan| plan.cluster.cluster_key == case.expected.cluster_key)
        .map(|plan| actual_for_cluster(plan, plans.len()));

    let mut issues = Vec::new();

    match &actual {
        Some(actual) => {
            if actual.cluster_count != case.expected.cluster_count {
                issues.push(format!(
                    "cluster_count expected {} got {}",
                    case.expected.cluster_count, actual.cluster_count
                ));
            }
            if actual.cluster_key != case.expected.cluster_key {
                issues.push(format!(
                    "cluster_key expected {:?} got {:?}",
                    case.expected.cluster_key, actual.cluster_key
                ));
            }
            if actual.canonical_clone_id != case.expected.canonical_clone_id {
                issues.push(format!(
                    "canonical_clone_id expected {:?} got {:?}",
                    case.expected.canonical_clone_id, actual.canonical_clone_id
                ));
            }
            if actual.canonical_remote_id != case.expected.canonical_remote_id {
                issues.push(format!(
                    "canonical_remote_id expected {:?} got {:?}",
                    case.expected.canonical_remote_id, actual.canonical_remote_id
                ));
            }
        }
        None => {
            issues.push(format!(
                "cluster {:?} not found in {} resolved clusters",
                case.expected.cluster_key,
                plans.len()
            ));
        }
    }

    let passed = issues.is_empty();
    CaseResult {
        name: case.name.clone(),
        notes: case.notes.clone(),
        passed,
        expected: case.expected.clone(),
        actual,
        failure: if passed {
            None
        } else {
            Some(issues.join("; "))
        },
    }
}

#[test]
fn canonical_truth_corpus_matches_expected() {
    let cases = load_cases();
    assert!(!cases.is_empty(), "canonical truth corpus is empty");

    let results: Vec<CaseResult> = cases.iter().map(evaluate_case).collect();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let summary = BenchmarkSummary {
        total: results.len(),
        passed,
        failed,
        results,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("serialize benchmark summary")
    );
    if let Some(parent) = summary_output_path().parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        summary_output_path(),
        serde_json::to_vec_pretty(&summary).expect("serialize benchmark summary"),
    );

    assert_eq!(failed, 0, "canonical truth benchmark regressions detected");
}
