use chrono::Utc;
use nexus_core::PlanDocument;
use nexus_db::Database;
use tempfile::tempdir;

#[test]
fn persist_plan_empty_succeeds() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("plan.db");
    let db = Database::open(&db_path).expect("open");
    let plan = PlanDocument {
        generated_at: Utc::now(),
        generated_by: "test".into(),
        clusters: vec![],
    };
    db.persist_plan(&plan).expect("persist");
}
