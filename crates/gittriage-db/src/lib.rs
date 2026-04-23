use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, types::Value, Connection};
use std::fs;
use std::path::Path;

use gittriage_core::{
    ActionType, CloneRecord, CloneRemoteLink, ClusterStatus, InventorySnapshot, MemberKind,
    PlanDocument, Priority, RemoteRecord, RunRecord, RunScanStats,
};
use uuid::Uuid;

pub const MIGRATION_0001: &str = include_str!("../../../migrations/0001_init.sql");

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Returns the SQLite library version string (e.g. `3.45.1`).
    pub fn sqlite_version(&self) -> Result<String> {
        let v: String = self
            .conn
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .context("failed to query sqlite_version")?;
        Ok(v)
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create db dir {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("failed to open sqlite db {}", path.display()))?;

        // Best-effort hardening pragmas.
        //
        // In CI/test environments multiple processes may open the same default DB path in
        // parallel (e2e smoke tests spawn `gittriage doctor` multiple times). Some pragmas (e.g.
        // `journal_mode`) can require stronger locking and may fail transiently with "database
        // is locked" or similar. GitTriage should still be able to open and migrate the DB.
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
        let _ = conn.execute_batch("PRAGMA synchronous=NORMAL;");
        let _ = conn.execute_batch("PRAGMA busy_timeout=5000;");
        let _ = conn.execute_batch("PRAGMA foreign_keys=ON;");

        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(MIGRATION_0001)
            .context("failed to apply migration 0001")?;

        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS gittriage_meta (
                   key TEXT PRIMARY KEY,
                   value TEXT NOT NULL
                 );
                 INSERT OR IGNORE INTO gittriage_meta (key, value) VALUES ('schema_version', '1');",
            )
            .context("failed to create gittriage_meta table")?;

        Ok(())
    }

    /// Execute a single-column query and return the first row as a `String`.
    /// Useful for reading PRAGMAs in tests and diagnostics (SQLite may return int or text).
    pub fn raw_query_row(&self, sql: &str) -> Result<String> {
        let v: Value = self
            .conn
            .query_row(sql, [], |row| row.get(0))
            .with_context(|| format!("raw_query_row: {sql}"))?;
        Ok(match v {
            Value::Null => String::new(),
            Value::Integer(i) => i.to_string(),
            Value::Real(f) => f.to_string(),
            Value::Text(s) => s,
            Value::Blob(b) => String::from_utf8_lossy(&b).into_owned(),
        })
    }

    pub fn schema_version(&self) -> Result<u32> {
        let v: String = self
            .conn
            .query_row(
                "SELECT value FROM gittriage_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "0".to_string());
        Ok(v.parse().unwrap_or(0))
    }

    /// Returns whether a table from the schema exists (used by tests and diagnostics).
    pub fn has_table(&self, name: &str) -> Result<bool> {
        let n: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get(0),
            )
            .context("failed to inspect sqlite_master")?;
        Ok(n > 0)
    }

    /// Rows in the `clusters` table (persisted plan). Useful after `import` / `replace_inventory_snapshot`.
    pub fn cluster_count(&self) -> Result<u64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM clusters", [], |row| row.get(0))
            .context("count clusters")?;
        Ok(n as u64)
    }

    /// Clears inventory and plan tables in FK-safe order before a new `scan` persist.
    ///
    /// `scan` uses `INSERT OR REPLACE` on `clones` by primary key; without this, replacing a row
    /// that shares a unique `path` with an older row would delete the old clone while persisted
    /// `clusters` still reference it → SQLite error 787 (`FOREIGN KEY constraint failed`).
    pub fn prepare_for_scan_persist(&mut self) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("begin prepare_for_scan_persist transaction")?;
        tx.execute("DELETE FROM actions", [])
            .context("delete actions before scan")?;
        tx.execute("DELETE FROM evidence", [])
            .context("delete evidence before scan")?;
        tx.execute("DELETE FROM cluster_members", [])
            .context("delete cluster_members before scan")?;
        tx.execute("DELETE FROM clusters", [])
            .context("delete clusters before scan")?;
        tx.execute("DELETE FROM clone_remote_links", [])
            .context("delete clone_remote_links before scan")?;
        tx.execute("DELETE FROM clones", [])
            .context("delete clones before scan")?;
        tx.execute("DELETE FROM remotes", [])
            .context("delete remotes before scan")?;
        tx.commit()
            .context("commit prepare_for_scan_persist transaction")?;
        Ok(())
    }

    pub fn save_run(&self, run: &RunRecord) -> Result<()> {
        let stats_json = run
            .stats
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("serialize run.stats")?;
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO runs (id, started_at, finished_at, roots_json, github_owner, version, stats_json)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                run.id,
                run.started_at.to_rfc3339(),
                run.finished_at.map(|dt| dt.to_rfc3339()),
                serde_json::to_string(&run.roots)?,
                run.github_owner,
                run.version,
                stats_json,
            ],
        )?;
        Ok(())
    }

    /// Rows in `clusters` and the latest `updated_at` among them (SQLite persisted plan), if any.
    pub fn persisted_plan_cluster_stats(
        &self,
    ) -> Result<(u64, Option<chrono::DateTime<chrono::Utc>>)> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM clusters", [], |row| row.get(0))
            .context("count clusters for stats")?;
        if n == 0 {
            return Ok((0, None));
        }
        let max_s: String = self
            .conn
            .query_row("SELECT MAX(updated_at) FROM clusters", [], |row| row.get(0))
            .context("max cluster updated_at")?;
        let ts = chrono::DateTime::parse_from_rfc3339(&max_s)
            .ok()
            .map(|d| d.with_timezone(&Utc));
        Ok((n as u64, ts))
    }

    pub fn save_clones(&mut self, run_id: &str, clones: &[CloneRecord]) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tx = self
            .conn
            .transaction()
            .context("begin save_clones transaction")?;
        for clone in clones {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO clones
                (id, repo_id, path, display_name, is_git, head_oid, active_branch, default_branch, is_dirty,
                 last_commit_at, size_bytes, manifest_kind, readme_title, license_spdx, fingerprint,
                 scan_run_id, created_at, updated_at)
                VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, COALESCE((SELECT created_at FROM clones WHERE id = ?1), ?16), ?16)
                "#,
                params![
                    clone.id,
                    clone.path,
                    clone.display_name,
                    clone.is_git as i32,
                    clone.head_oid,
                    clone.active_branch,
                    clone.default_branch,
                    clone.is_dirty as i32,
                    clone.last_commit_at.map(|dt| dt.to_rfc3339()),
                    clone.size_bytes.map(|v| v as i64),
                    clone.manifest_kind.as_ref().map(|m| format!("{m:?}")),
                    clone.readme_title,
                    clone.license_spdx,
                    clone.fingerprint,
                    run_id,
                    now
                ],
            )?;
        }
        tx.commit().context("commit save_clones")?;
        Ok(())
    }

    pub fn replace_clone_remote_links(&mut self, links: &[CloneRemoteLink]) -> Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("begin replace_clone_remote_links transaction")?;
        tx.execute("DELETE FROM clone_remote_links", [])
            .context("failed to clear clone_remote_links")?;
        for link in links {
            tx.execute(
                r#"
                INSERT INTO clone_remote_links (clone_id, remote_id, relationship)
                VALUES (?1, ?2, ?3)
                "#,
                params![link.clone_id, link.remote_id, link.relationship],
            )?;
        }
        tx.commit().context("commit replace_clone_remote_links")?;
        Ok(())
    }

    pub fn save_remotes(&mut self, remotes: &[RemoteRecord]) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tx = self
            .conn
            .transaction()
            .context("begin save_remotes transaction")?;
        for remote in remotes {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO remotes
                (id, repo_id, provider, owner, name, full_name, url, normalized_url, default_branch,
                 is_fork, is_archived, is_private, pushed_at, created_at, updated_at)
                VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, COALESCE((SELECT created_at FROM remotes WHERE id = ?1), ?13), ?13)
                "#,
                params![
                    remote.id,
                    remote.provider,
                    remote.owner,
                    remote.name,
                    remote.full_name,
                    remote.url,
                    remote.normalized_url,
                    remote.default_branch,
                    remote.is_fork as i32,
                    remote.is_archived as i32,
                    remote.is_private as i32,
                    remote.pushed_at.map(|dt| dt.to_rfc3339()),
                    now
                ],
            )?;
        }
        tx.commit().context("commit save_remotes")?;
        Ok(())
    }

    pub fn load_inventory(&self) -> Result<InventorySnapshot> {
        let mut clones_stmt = self.conn.prepare(
            r#"
            SELECT id, path, display_name, is_git, head_oid, active_branch, default_branch, is_dirty,
                   last_commit_at, size_bytes, manifest_kind, readme_title, license_spdx, fingerprint
            FROM clones
            ORDER BY updated_at DESC
            "#,
        )?;

        let clones = clones_stmt
            .query_map([], |row| {
                Ok(CloneRecord {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    display_name: row.get(2)?,
                    is_git: row.get::<_, i32>(3)? != 0,
                    head_oid: row.get(4)?,
                    active_branch: row.get(5)?,
                    default_branch: row.get(6)?,
                    is_dirty: row.get::<_, i32>(7)? != 0,
                    last_commit_at: row
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    size_bytes: row.get::<_, Option<i64>>(9)?.map(|v| v as u64),
                    manifest_kind: row.get::<_, Option<String>>(10)?.and_then(|s| {
                        match s.as_str() {
                            "Cargo" => Some(gittriage_core::ManifestKind::Cargo),
                            "PackageJson" => Some(gittriage_core::ManifestKind::PackageJson),
                            "PyProject" => Some(gittriage_core::ManifestKind::PyProject),
                            "RequirementsTxt" => {
                                Some(gittriage_core::ManifestKind::RequirementsTxt)
                            }
                            "CMake" => Some(gittriage_core::ManifestKind::CMake),
                            "Makefile" => Some(gittriage_core::ManifestKind::Makefile),
                            _ => None,
                        }
                    }),
                    readme_title: row.get(11)?,
                    license_spdx: row.get(12)?,
                    fingerprint: row.get(13)?,
                    has_lockfile: false,
                    has_ci: false,
                    has_tests_dir: false,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut remotes_stmt = self.conn.prepare(
            r#"
            SELECT id, provider, owner, name, full_name, url, normalized_url, default_branch,
                   is_fork, is_archived, is_private, pushed_at
            FROM remotes
            ORDER BY updated_at DESC
            "#,
        )?;

        let remotes = remotes_stmt
            .query_map([], |row| {
                Ok(RemoteRecord {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    owner: row.get(2)?,
                    name: row.get(3)?,
                    full_name: row.get(4)?,
                    url: row.get(5)?,
                    normalized_url: row.get(6)?,
                    default_branch: row.get(7)?,
                    is_fork: row.get::<_, i32>(8)? != 0,
                    is_archived: row.get::<_, i32>(9)? != 0,
                    is_private: row.get::<_, i32>(10)? != 0,
                    pushed_at: row
                        .get::<_, Option<String>>(11)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut links_stmt = self.conn.prepare(
            r#"
            SELECT clone_id, remote_id, relationship FROM clone_remote_links
            "#,
        )?;

        let links = links_stmt
            .query_map([], |row| {
                Ok(CloneRemoteLink {
                    clone_id: row.get(0)?,
                    remote_id: row.get(1)?,
                    relationship: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let run = load_latest_run(&self.conn).context("load latest run")?;

        let mut snap = InventorySnapshot {
            run,
            clones,
            remotes,
            links,
            semantics: None,
        };
        snap.refresh_semantics();
        Ok(snap)
    }

    /// Wipes plan tables, clone/remote/link rows, and all runs, then inserts the snapshot
    /// (synthetic [`RunRecord`] when `snapshot.run` is `None`). Use for `gittriage import`.
    pub fn replace_inventory_snapshot(
        &mut self,
        snapshot: &InventorySnapshot,
        app_version: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tx = self
            .conn
            .transaction()
            .context("begin replace_inventory_snapshot transaction")?;

        tx.execute("DELETE FROM actions", [])
            .context("delete actions")?;
        tx.execute("DELETE FROM evidence", [])
            .context("delete evidence")?;
        tx.execute("DELETE FROM cluster_members", [])
            .context("delete cluster_members")?;
        tx.execute("DELETE FROM clusters", [])
            .context("delete clusters")?;
        tx.execute("DELETE FROM clone_remote_links", [])
            .context("delete clone_remote_links")?;
        tx.execute("DELETE FROM clones", [])
            .context("delete clones")?;
        tx.execute("DELETE FROM remotes", [])
            .context("delete remotes")?;
        tx.execute("DELETE FROM runs", []).context("delete runs")?;

        let run = snapshot.run.clone().unwrap_or_else(|| RunRecord {
            id: format!("import-{}", Uuid::new_v4()),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            roots: vec!["<gittriage import>".to_string()],
            github_owner: None,
            version: app_version.to_string(),
            stats: None,
        });

        let stats_json = run
            .stats
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .context("serialize run.stats for import")?;

        tx.execute(
            r#"
            INSERT OR REPLACE INTO runs (id, started_at, finished_at, roots_json, github_owner, version, stats_json)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                run.id,
                run.started_at.to_rfc3339(),
                run.finished_at.map(|dt| dt.to_rfc3339()),
                serde_json::to_string(&run.roots)?,
                run.github_owner,
                run.version,
                stats_json,
            ],
        )
        .context("insert run")?;

        for clone in &snapshot.clones {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO clones
                (id, repo_id, path, display_name, is_git, head_oid, active_branch, default_branch, is_dirty,
                 last_commit_at, size_bytes, manifest_kind, readme_title, license_spdx, fingerprint,
                 scan_run_id, created_at, updated_at)
                VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
                "#,
                params![
                    clone.id,
                    clone.path,
                    clone.display_name,
                    clone.is_git as i32,
                    clone.head_oid,
                    clone.active_branch,
                    clone.default_branch,
                    clone.is_dirty as i32,
                    clone.last_commit_at.map(|dt| dt.to_rfc3339()),
                    clone.size_bytes.map(|v| v as i64),
                    clone.manifest_kind.as_ref().map(|m| format!("{m:?}")),
                    clone.readme_title,
                    clone.license_spdx,
                    clone.fingerprint,
                    run.id,
                    now
                ],
            )
            .context("insert clone")?;
        }

        for remote in &snapshot.remotes {
            tx.execute(
                r#"
                INSERT OR REPLACE INTO remotes
                (id, repo_id, provider, owner, name, full_name, url, normalized_url, default_branch,
                 is_fork, is_archived, is_private, pushed_at, created_at, updated_at)
                VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
                "#,
                params![
                    remote.id,
                    remote.provider,
                    remote.owner,
                    remote.name,
                    remote.full_name,
                    remote.url,
                    remote.normalized_url,
                    remote.default_branch,
                    remote.is_fork as i32,
                    remote.is_archived as i32,
                    remote.is_private as i32,
                    remote.pushed_at.map(|dt| dt.to_rfc3339()),
                    now
                ],
            )
            .context("insert remote")?;
        }

        for link in &snapshot.links {
            tx.execute(
                r#"
                INSERT INTO clone_remote_links (clone_id, remote_id, relationship)
                VALUES (?1, ?2, ?3)
                "#,
                params![link.clone_id, link.remote_id, link.relationship],
            )
            .context("insert clone_remote_link")?;
        }

        tx.commit().context("commit replace_inventory_snapshot")?;
        Ok(())
    }

    /// Replaces clustering / plan tables with a fresh plan snapshot (v1 is recompute-only).
    pub fn persist_plan(&mut self, plan: &PlanDocument) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let tx = self
            .conn
            .transaction()
            .context("begin persist_plan transaction")?;

        tx.execute("DELETE FROM actions", [])
            .context("delete actions")?;
        tx.execute("DELETE FROM evidence", [])
            .context("delete evidence")?;
        tx.execute("DELETE FROM cluster_members", [])
            .context("delete cluster_members")?;
        tx.execute("DELETE FROM clusters", [])
            .context("delete clusters")?;

        for cp in &plan.clusters {
            let c = &cp.cluster;
            tx.execute(
                r#"
                INSERT INTO clusters (id, cluster_key, label, status, confidence, canonical_clone_id, canonical_remote_id, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
                "#,
                params![
                    c.id,
                    c.cluster_key,
                    c.label,
                    cluster_status_db(&c.status),
                    c.confidence,
                    c.canonical_clone_id,
                    c.canonical_remote_id,
                    now,
                ],
            )
            .context("insert cluster")?;

            for m in &c.members {
                tx.execute(
                    r#"
                    INSERT INTO cluster_members (cluster_id, member_kind, member_id)
                    VALUES (?1, ?2, ?3)
                    "#,
                    params![c.id, member_kind_db(&m.kind), m.id],
                )?;
            }

            for ev in &c.evidence {
                tx.execute(
                    r#"
                    INSERT INTO evidence (id, cluster_id, subject_kind, subject_id, kind, score_delta, detail, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    "#,
                    params![
                        ev.id,
                        c.id,
                        member_kind_db(&ev.subject_kind),
                        ev.subject_id,
                        ev.kind,
                        ev.score_delta,
                        ev.detail,
                        now,
                    ],
                )?;
            }

            for a in &cp.actions {
                tx.execute(
                    r#"
                    INSERT INTO actions (id, cluster_id, priority, action_type, target_kind, target_id, reason, commands_json, status, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'proposed', ?9, ?9)
                    "#,
                    params![
                        a.id,
                        c.id,
                        priority_db(&a.priority),
                        action_type_db(&a.action_type),
                        member_kind_db(&a.target_kind),
                        a.target_id,
                        a.reason,
                        serde_json::to_string(&a.commands)?,
                        now,
                    ],
                )?;
            }
        }

        tx.commit().context("commit persist_plan")?;
        Ok(())
    }
}

fn load_latest_run(conn: &rusqlite::Connection) -> Result<Option<RunRecord>> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, started_at, finished_at, roots_json, github_owner, version, stats_json
            FROM runs
            ORDER BY datetime(started_at) DESC
            LIMIT 1
            "#,
        )
        .context("prepare latest run query")?;

    let mut rows = stmt.query([]).context("query latest run")?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    let stats = row
        .get::<_, Option<String>>(6)?
        .map(|s| serde_json::from_str::<RunScanStats>(&s))
        .transpose()
        .context("deserialize run.stats_json")?;

    let started: String = row.get(1)?;
    let started_at = chrono::DateTime::parse_from_rfc3339(&started)
        .map(|d| d.with_timezone(&Utc))
        .context("parse run.started_at")?;

    let finished_at = row
        .get::<_, Option<String>>(2)?
        .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
        .transpose()
        .context("parse run.finished_at")?
        .map(|d| d.with_timezone(&Utc));

    let roots_json: String = row.get(3)?;
    let roots: Vec<String> =
        serde_json::from_str(&roots_json).context("deserialize run.roots_json")?;

    Ok(Some(RunRecord {
        id: row.get(0)?,
        started_at,
        finished_at,
        roots,
        github_owner: row.get(4)?,
        version: row.get(5)?,
        stats,
    }))
}

fn cluster_status_db(s: &ClusterStatus) -> &'static str {
    match s {
        ClusterStatus::Resolved => "resolved",
        ClusterStatus::Ambiguous => "ambiguous",
        ClusterStatus::ManualReview => "manual_review",
    }
}

fn member_kind_db(k: &MemberKind) -> &'static str {
    match k {
        MemberKind::Clone => "clone",
        MemberKind::Remote => "remote",
    }
}

fn priority_db(p: &Priority) -> &'static str {
    match p {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
    }
}

fn action_type_db(t: &ActionType) -> &'static str {
    match t {
        ActionType::MarkCanonical => "mark_canonical",
        ActionType::ArchiveLocalDuplicate => "archive_local_duplicate",
        ActionType::ReviewAmbiguousCluster => "review_ambiguous_cluster",
        ActionType::MergeDivergedClone => "merge_diverged_clone",
        ActionType::CreateRemoteRepo => "create_remote_repo",
        ActionType::CloneLocalWorkspace => "clone_local_workspace",
        ActionType::AddMissingDocs => "add_missing_docs",
        ActionType::AddLicense => "add_license",
        ActionType::AddCi => "add_ci",
        ActionType::RunSecurityScans => "run_security_scans",
        ActionType::GenerateSbom => "generate_sbom",
        ActionType::PublishOssCandidate => "publish_oss_candidate",
    }
}
