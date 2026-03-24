pub mod model;
pub mod remote_url;

pub use model::*;
pub use remote_url::normalize_remote_url;

#[cfg(test)]
mod golden_plan_tests {
    use super::PlanDocument;

    #[test]
    fn golden_plan_fixture_roundtrips() {
        let raw = include_str!("../../../fixtures/golden/plan-v1.json");
        let doc: PlanDocument = serde_json::from_str(raw).expect("parse golden fixture");
        assert_eq!(doc.schema_version, 1);
        assert_eq!(doc.scoring_rules_version, 4);
        assert_eq!(doc.generated_by, "nexus 0.1.0");
        assert_eq!(doc.clusters.len(), 1);
        let serialized = serde_json::to_string(&doc).expect("serialize");
        let doc2: PlanDocument = serde_json::from_str(&serialized).expect("re-parse");
        assert_eq!(doc2.clusters.len(), 1);
        assert_eq!(
            doc2.clusters[0].cluster.label,
            doc.clusters[0].cluster.label
        );
    }

    #[test]
    fn plan_without_schema_version_defaults_to_v1() {
        let raw =
            r#"{"generated_at":"2026-03-24T12:00:00Z","generated_by":"nexus 0.1.0","clusters":[]}"#;
        let doc: PlanDocument = serde_json::from_str(raw).expect("parse");
        assert_eq!(doc.schema_version, 1);
    }

    #[test]
    fn plan_without_scoring_rules_version_defaults_to_v1() {
        let raw = r#"{"schema_version":1,"generated_at":"2026-03-24T12:00:00Z","generated_by":"nexus 0.1.0","clusters":[]}"#;
        let doc: PlanDocument = serde_json::from_str(raw).expect("parse");
        assert_eq!(doc.scoring_rules_version, 1);
    }
}
