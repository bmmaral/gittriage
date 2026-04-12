# Configuration (`gittriage.toml`)

GitTriage reads a TOML config file. Precedence (first match wins):

1. `--config /path/to/gittriage.toml` on the CLI
2. Environment variable `GITTRIAGE_CONFIG`
3. `./gittriage.toml` in the current working directory
4. XDG-style config directory: `gittriage.toml` under the OS config dir (`ProjectDirs`, qualifier `org.gittriage.gittriage`)
5. Built-in defaults (no file)

Relative `db_path` values are resolved against the **config file's parent directory** when a config file is found, or the process working directory when using built-in defaults. Tilde (`~`) is expanded to `$HOME`.

## Example

See `gittriage.toml.example` in the repository root. Fields:

| Field | Purpose |
| --- | --- |
| `db_path` | SQLite database path (resolved relative to config file; keep under `.gittriage/` or another ignored directory) |
| `default_roots` | Used when `gittriage scan` is run with no path arguments |
| `github_owner` | Optional default for `gh`-based remote ingest |
| `github_owner_mode` | `augment` (default): only GitHub repos whose URL matches a local remote from the scan; `full_catalog`: full `gh repo list` for the owner |
| `include_hidden` | Whether to descend into hidden directories when scanning |
| `tui_export_path` | Optional path for TUI `o` export; if unset, a timestamped file in the working directory is used |
| `[scan]` | Scan behavior: `scan_mode` (`git_only` / `project_roots`), `max_depth`, `respect_gitignore`, `max_readme_bytes`, `max_hash_files`, `include_nested_git` (discover nested `.git` under a root; off by default) |
| `[planner]` | Ambiguity and publish-action thresholds; optional `canonical_pins`, `ignored_cluster_keys`, `archive_hint_cluster_keys`, `scoring_profile` (see `docs/SCORING_PROFILES.md`, `docs/CLI.md`) |
| `[ai]` | Optional AI-assisted explanations; `enabled`, `api_base`, `model`, `max_tokens`, `temperature` (see `docs/CLI.md` § AI integration) |

The latest `runs` row may include JSON in `stats_json` (e.g. `skipped_nested_git` paths when nested repos were skipped); `gittriage report` surfaces those paths when present.

### Scan modes

| Mode | Behavior | Agent / canonical path note |
| --- | --- | --- |
| `git_only` (default) | Only directories containing `.git` are treated as project roots | Use this for **coding-agent preflight**: `resolve`, `check-path`, and canonical paths refer to real git checkouts in inventory. |
| `project_roots` | Directories with `.git` or common manifests (`Cargo.toml`, `package.json`, etc.) are included | **Secondary** discovery mode: not every row is a git root; treat automation targets as tentative until verified with `check-path` / a `git_only` scan. |

When a `.git` root is found, subdirectories are **not** scanned for nested project roots (prevents monorepo sub-packages from appearing as separate entries). Nested `.git` directories inside another root are **skipped** unless `scan.include_nested_git = true`; when skipped, paths are listed on stderr during `scan`.

Place a `.gittriageignore` file in any scan root with glob patterns (one per line) to exclude matching directories.

### CLI `--profile` override

The `--profile` flag on `score`, `plan`, `report`, and `explain` overrides `planner.scoring_profile` from the config file. Accepted values: `default`, `publish`, `open_source`, `security`, `ai_handoff`.

## Environment

- `GITTRIAGE_CONFIG` — path to a `gittriage.toml` file (see `gittriage-config` crate: `ENV_GITTRIAGE_CONFIG`).
- `GITTRIAGE_AI_API_KEY` — API key for AI features (takes precedence over `OPENAI_API_KEY`).
- `OPENAI_API_KEY` — fallback API key for AI features.
- `GITTRIAGE_ADAPTER_TIMEOUT_SECS` — timeout for external adapter subprocesses (default: 180 seconds).
- `RUST_LOG` — standard `tracing` filter when you need verbose logs from components that emit them.

## SQLite

GitTriage uses SQLite with WAL mode (`journal_mode=WAL`), `synchronous=NORMAL`, and a 5-second `busy_timeout` for safe concurrent access. Schema versioning is tracked in the `gittriage_meta` table. The `doctor` command reports the resolved DB path and SQLite version.
