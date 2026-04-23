use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn gittriage_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_gittriage"))
}

fn toml_escape_basic(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

struct Harness {
    _dir: TempDir,
    cfg: PathBuf,
    repo_root: PathBuf,
    alt_root: PathBuf,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("state.db");
        let cfg = dir.path().join("gittriage.toml");
        let escaped_db = toml_escape_basic(&db.to_string_lossy());
        fs::write(
            &cfg,
            format!(
                "db_path = \"{}\"\ndefault_roots = []\n[scan]\nscan_mode = \"git_only\"\nrespect_gitignore = true\ninclude_nested_git = false\n",
                escaped_db
            ),
        )
        .expect("write config");

        let repo_root = dir.path().join("ws").join("primary").join("canon-repo");
        let alt_root = dir.path().join("ws").join("secondary").join("canon-repo");
        let nested = repo_root.join("vendor/nested");

        for root in [&repo_root, &alt_root, &nested] {
            fs::create_dir_all(root.join(".git")).unwrap();
        }
        fs::write(
            repo_root.join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(repo_root.join("README.md"), "# Demo Repo\n").unwrap();
        fs::write(repo_root.join("LICENSE"), "MIT License\n").unwrap();

        fs::write(
            alt_root.join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(alt_root.join("README.md"), "# Demo Repo\n").unwrap();
        fs::write(alt_root.join("LICENSE"), "MIT License\n").unwrap();

        let status = Command::new(gittriage_exe())
            .current_dir(dir.path())
            .args([
                "--config",
                cfg.to_str().unwrap(),
                "scan",
                dir.path().join("ws").to_str().unwrap(),
            ])
            .status()
            .expect("scan");
        assert!(status.success(), "scan should succeed");

        Self {
            _dir: dir,
            cfg,
            repo_root,
            alt_root,
        }
    }

    fn run_json(&self, args: &[&str]) -> Value {
        let output = Command::new(gittriage_exe())
            .args(args)
            .output()
            .expect("spawn gittriage");
        assert_success(&output);
        serde_json::from_slice(&output.stdout).expect("json stdout")
    }

    fn with_config<'a>(&'a self, extra: &'a [&'a str]) -> Vec<&'a str> {
        let mut args = vec!["--config", self.cfg.to_str().unwrap()];
        args.extend_from_slice(extra);
        args
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn agent_cli_contracts_are_consistent_for_unresolved_target() {
    let h = Harness::new();

    let resolve = h.run_json(&h.with_config(&[
        "resolve",
        "missing-target",
        "--format",
        "json",
        "--no-merge-base",
    ]));
    assert_eq!(resolve["kind"], "gittriage_resolve");
    assert_eq!(resolve["query"], "missing-target");
    assert_eq!(resolve["automation_verdict"], "blocked");
    assert_eq!(resolve["unsafe_for_automation"], true);
    assert!(resolve.get("generated_at").is_some());
    assert!(resolve.get("freshness").is_some());
    assert!(resolve["error"].is_string());

    let verdict = h.run_json(&h.with_config(&[
        "verdict",
        "missing-target",
        "--format",
        "json",
        "--no-merge-base",
    ]));
    assert_eq!(verdict["kind"], "gittriage_verdict");
    assert_eq!(verdict["target"], "missing-target");
    assert_eq!(verdict["verdict"]["automation_verdict"], "blocked");
    assert_eq!(verdict["verdict"]["unsafe_for_automation"], true);
    assert!(!verdict["verdict"]["blocking_reasons"]
        .as_array()
        .unwrap()
        .is_empty());

    let preflight = h.run_json(&h.with_config(&[
        "preflight",
        "missing-target",
        "--format",
        "json",
        "--no-merge-base",
    ]));
    assert_eq!(preflight["kind"], "gittriage_preflight");
    assert_eq!(preflight["target"], "missing-target");
    assert_eq!(preflight["automation_verdict"], "blocked");
    assert_eq!(preflight["unsafe_for_automation"], true);
    assert_eq!(
        preflight["recommended_next_action"],
        "run_scan_and_resolve_target"
    );

    let check = h.run_json(&h.with_config(&[
        "check-path",
        "/definitely/not/inventory",
        "--format",
        "json",
        "--no-merge-base",
    ]));
    assert_eq!(check["kind"], "gittriage_check_path");
    assert_eq!(check["disposition"], "not_in_inventory");
    assert_eq!(check["is_wrong_clone"], true);
    assert_eq!(check["unsafe_for_automation"], true);
}

#[test]
fn agent_summary_endpoints_have_expected_shape_after_real_scan() {
    let h = Harness::new();

    let summary = h.run_json(&h.with_config(&[
        "summary",
        "--agent",
        h.repo_root.parent().unwrap().to_str().unwrap(),
        "--format",
        "json",
        "--no-merge-base",
    ]));
    assert_eq!(summary["kind"], "gittriage_agent_summary");
    assert!(summary["total_clusters_considered"].as_u64().unwrap() >= 1);
    assert!(summary["canonical_paths"].is_array());
    assert!(summary["unsafe_targets"].is_array());
    assert!(summary["duplicate_groups"].is_array());
    assert!(summary["nested_repo_warnings"].is_array());

    let duplicates = summary["duplicate_groups"].as_array().unwrap();
    let nested = summary["nested_repo_warnings"].as_array().unwrap();
    assert!(!nested.is_empty(), "expected skipped nested git warning");
    assert!(
        duplicates.iter().any(|g| {
            g["canonical_path"] == Value::String(h.repo_root.to_string_lossy().to_string())
                && g["alternate_paths"]
                    .as_array()
                    .map(|arr| {
                        arr.contains(&Value::String(h.alt_root.to_string_lossy().to_string()))
                    })
                    .unwrap_or(false)
        }),
        "expected a duplicate group referencing the scanned repos: {summary}"
    );
}
