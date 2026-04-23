PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  roots_json TEXT NOT NULL,
  github_owner TEXT,
  version TEXT NOT NULL,
  stats_json TEXT
);

CREATE TABLE IF NOT EXISTS repos (
  id TEXT PRIMARY KEY,
  slug TEXT NOT NULL,
  display_name TEXT NOT NULL,
  primary_language TEXT,
  created_at TEXT NOT NULL,
  notes TEXT
);

CREATE TABLE IF NOT EXISTS clones (
  id TEXT PRIMARY KEY,
  repo_id TEXT,
  path TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  is_git INTEGER NOT NULL,
  head_oid TEXT,
  active_branch TEXT,
  default_branch TEXT,
  is_dirty INTEGER NOT NULL DEFAULT 0,
  last_commit_at TEXT,
  upstream_branch TEXT,
  ahead_count INTEGER,
  behind_count INTEGER,
  no_upstream_configured INTEGER,
  upstream_resolution_error TEXT,
  size_bytes INTEGER,
  manifest_kind TEXT,
  readme_title TEXT,
  license_spdx TEXT,
  fingerprint TEXT,
  scan_run_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repo_id) REFERENCES repos(id),
  FOREIGN KEY (scan_run_id) REFERENCES runs(id)
);

CREATE TABLE IF NOT EXISTS remotes (
  id TEXT PRIMARY KEY,
  repo_id TEXT,
  provider TEXT NOT NULL,
  owner TEXT,
  name TEXT,
  full_name TEXT,
  url TEXT NOT NULL,
  normalized_url TEXT NOT NULL,
  default_branch TEXT,
  is_fork INTEGER NOT NULL DEFAULT 0,
  is_archived INTEGER NOT NULL DEFAULT 0,
  is_private INTEGER NOT NULL DEFAULT 0,
  pushed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repo_id) REFERENCES repos(id)
);

CREATE TABLE IF NOT EXISTS clone_remote_links (
  clone_id TEXT NOT NULL,
  remote_id TEXT NOT NULL,
  relationship TEXT NOT NULL DEFAULT 'origin',
  PRIMARY KEY (clone_id, remote_id),
  FOREIGN KEY (clone_id) REFERENCES clones(id),
  FOREIGN KEY (remote_id) REFERENCES remotes(id)
);

CREATE TABLE IF NOT EXISTS clusters (
  id TEXT PRIMARY KEY,
  cluster_key TEXT NOT NULL UNIQUE,
  label TEXT NOT NULL,
  status TEXT NOT NULL,
  confidence REAL NOT NULL,
  canonical_clone_id TEXT,
  canonical_remote_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (canonical_clone_id) REFERENCES clones(id),
  FOREIGN KEY (canonical_remote_id) REFERENCES remotes(id)
);

CREATE TABLE IF NOT EXISTS cluster_members (
  cluster_id TEXT NOT NULL,
  member_kind TEXT NOT NULL,
  member_id TEXT NOT NULL,
  PRIMARY KEY (cluster_id, member_kind, member_id),
  FOREIGN KEY (cluster_id) REFERENCES clusters(id)
);

CREATE TABLE IF NOT EXISTS evidence (
  id TEXT PRIMARY KEY,
  cluster_id TEXT,
  subject_kind TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  score_delta REAL NOT NULL,
  detail TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (cluster_id) REFERENCES clusters(id)
);

CREATE TABLE IF NOT EXISTS assessments (
  id TEXT PRIMARY KEY,
  target_kind TEXT NOT NULL,
  target_id TEXT NOT NULL,
  canonical_score REAL,
  usability_score REAL,
  oss_readiness_score REAL,
  risk_score REAL,
  summary_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS actions (
  id TEXT PRIMARY KEY,
  cluster_id TEXT NOT NULL,
  priority TEXT NOT NULL,
  action_type TEXT NOT NULL,
  target_kind TEXT NOT NULL,
  target_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  commands_json TEXT,
  status TEXT NOT NULL DEFAULT 'proposed',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (cluster_id) REFERENCES clusters(id)
);

CREATE INDEX IF NOT EXISTS idx_clones_scan_run_id ON clones(scan_run_id);
CREATE INDEX IF NOT EXISTS idx_remotes_normalized_url ON remotes(normalized_url);
CREATE INDEX IF NOT EXISTS idx_evidence_subject ON evidence(subject_kind, subject_id);
CREATE INDEX IF NOT EXISTS idx_assessments_target ON assessments(target_kind, target_id);
CREATE INDEX IF NOT EXISTS idx_actions_cluster ON actions(cluster_id);
