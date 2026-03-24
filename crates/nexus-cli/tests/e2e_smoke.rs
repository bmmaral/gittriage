use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn nexus_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nexus"))
}

#[test]
fn doctor_succeeds() {
    let status = Command::new(nexus_exe())
        .arg("doctor")
        .status()
        .expect("spawn nexus doctor");
    assert!(status.success(), "nexus doctor should exit 0");
}

#[test]
fn scan_score_plan_report_json_pipeline() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("state.db");
    let cfg = dir.path().join("nexus.toml");
    fs::write(
        &cfg,
        format!("db_path = \"{}\"\ndefault_roots = []\n", db.display()),
    )
    .expect("write nexus.toml");

    let empty_root = dir.path().join("empty");
    fs::create_dir_all(&empty_root).unwrap();

    assert!(
        Command::new(nexus_exe())
            .current_dir(dir.path())
            .args([
                "--config",
                cfg.to_str().unwrap(),
                "scan",
                empty_root.to_str().unwrap(),
            ])
            .status()
            .expect("scan")
            .success(),
        "scan"
    );

    let score_out = Command::new(nexus_exe())
        .current_dir(dir.path())
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "score",
            "--format",
            "json",
            "--no-merge-base",
        ])
        .output()
        .expect("score");
    assert!(
        score_out.status.success(),
        "score stderr: {}",
        String::from_utf8_lossy(&score_out.stderr)
    );
    let score_stdout = String::from_utf8_lossy(&score_out.stdout);
    assert!(
        score_stdout.contains("nexus_scores") && score_stdout.contains("clusters"),
        "score json: {score_stdout}"
    );

    let plan_path = dir.path().join("out-plan.json");
    assert!(
        Command::new(nexus_exe())
            .current_dir(dir.path())
            .args([
                "--config",
                cfg.to_str().unwrap(),
                "plan",
                "--write",
                plan_path.to_str().unwrap(),
                "--no-merge-base",
            ])
            .status()
            .expect("plan")
            .success(),
        "plan"
    );
    assert!(plan_path.is_file(), "plan file written");

    let output = Command::new(nexus_exe())
        .current_dir(dir.path())
        .args([
            "--config",
            cfg.to_str().unwrap(),
            "report",
            "--format",
            "json",
        ])
        .output()
        .expect("report");
    assert!(
        output.status.success(),
        "report stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("clusters"),
        "report json should mention clusters: {stdout}"
    );
}
