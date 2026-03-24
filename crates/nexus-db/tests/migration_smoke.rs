use nexus_db::Database;
use tempfile::tempdir;

#[test]
fn open_creates_file_and_applies_schema() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("nexus-test.db");
    assert!(!db_path.exists());

    let db = Database::open(&db_path).expect("open db");
    assert!(db_path.exists());
    assert!(!db.sqlite_version().unwrap().is_empty());
    assert!(db.has_table("runs").unwrap());
    assert!(db.has_table("clones").unwrap());
    assert!(db.has_table("remotes").unwrap());
}

#[test]
fn open_is_idempotent() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("reuse.db");
    Database::open(&db_path).unwrap();
    Database::open(&db_path).unwrap();
}
