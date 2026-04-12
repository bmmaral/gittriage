use chrono::Utc;
use gittriage_core::PlanDocument;
use gittriage_db::Database;
use tempfile::tempdir;

#[test]
fn persist_plan_empty_succeeds() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("plan.db");
    let mut db = Database::open(&db_path).expect("open");
    let plan = PlanDocument {
        schema_version: 1,
        scoring_rules_version: 5,
        generated_at: Utc::now(),
        generated_by: "test".into(),
        external_adapter_run: None,
        clusters: vec![],
    };
    db.persist_plan(&plan).expect("persist");
}
