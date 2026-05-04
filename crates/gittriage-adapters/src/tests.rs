use super::*;

#[test]
fn test_parse_gitleaks_output() {
    let fixture_path = "tests/fixtures/gitleaks_output.json";
    let content = std::fs::read_to_string(fixture_path).expect("failed to read fixture");
    let findings = parse_gitleaks_output(&content).expect("failed to parse");

    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.tool, ExternalTool::Gitleaks);
    assert_eq!(finding.path, "src/main.rs");
    assert_eq!(finding.line, Some(8));
    assert_eq!(finding.message, "Generic API Key");
    assert_eq!(
        finding.details.get("rule_id"),
        Some(&"generic-api-key".to_string())
    );
}

#[test]
fn test_parse_semgrep_output() {
    let fixture_path = "tests/fixtures/semgrep_output.json";
    let content = std::fs::read_to_string(fixture_path).expect("failed to read fixture");
    let findings = parse_semgrep_output(&content).expect("failed to parse");

    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.tool, ExternalTool::Semgrep);
    assert_eq!(finding.path, "keys/id_rsa");
    assert_eq!(finding.line, Some(1));
    assert_eq!(finding.message, "A private key was detected.");
    assert_eq!(
        finding.details.get("check_id"),
        Some(&"generic.secrets.security.detected-private-key.detected-private-key".to_string())
    );
}

#[test]
fn test_syft_summary_truncation() {
    let long_output = "a".repeat(300);
    let (bin, args) = ("echo", &[long_output.as_str()]);
    let result = run_capture(bin, args, Path::new(".")).unwrap();
    let summary = if result.1.len() > 240 {
        format!("{}…", &result.1[..240])
    } else {
        result.1
    };
    assert_eq!(summary.chars().count(), 241);
    assert!(summary.ends_with('…'));
}
