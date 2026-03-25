use nexus_db::Database;
use tempfile::tempdir;

#[test]
fn wal_mode_active_after_open() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("wal-test.db");
    let db = Database::open(&db_path).expect("open db");

    let mode: String = db
        .raw_query_row("PRAGMA journal_mode")
        .expect("query journal_mode");
    assert_eq!(mode.to_lowercase(), "wal");
}

#[test]
fn schema_version_returns_one() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("version-test.db");
    let db = Database::open(&db_path).expect("open db");

    assert_eq!(db.schema_version().unwrap(), 1);
}

#[test]
fn busy_timeout_is_set() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("timeout-test.db");
    let db = Database::open(&db_path).expect("open db");

    let timeout: String = db
        .raw_query_row("PRAGMA busy_timeout")
        .expect("query busy_timeout");
    assert_eq!(timeout, "5000");
}
