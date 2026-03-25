# Configuration (`nexus.toml`)

Nexus reads a TOML config file. Precedence (first match wins):

1. `--config /path/to/nexus.toml` on the CLI
2. Environment variable `NEXUS_CONFIG`
3. `./nexus.toml` in the current working directory
4. XDG-style config directory: `nexus.toml` under the OS config dir (`ProjectDirs`, qualifier `org.nexus.nexus`)
5. Built-in defaults (no file)

Relative `db_path` values are resolved against the **config file's parent directory** when a config file is found, or the process working directory when using built-in defaults. Tilde (`~`) is expanded to `$HOME`.

## Example

See `nexus.toml.example` in the repository root. Fields:

| Field | Purpose |
| --- | --- |
| `db_path` | SQLite database path (resolved relative to config file; keep under `.nexus/` or another ignored directory) |
| `default_roots` | Used when `nexus scan` is run with no path arguments |
| `github_owner` | Optional default for `gh`-based remote ingest |
| `include_hidden` | Whether to descend into hidden directories when scanning |
| `[scan]` | Scan behavior: `scan_mode` (`git_only` / `project_roots`), `max_depth`, `respect_gitignore`, `max_readme_bytes`, `max_hash_files` |
| `[planner]` | Ambiguity and publish-action thresholds; optional `canonical_pins`, `ignored_cluster_keys`, `archive_hint_cluster_keys`, `scoring_profile` (see `docs/SCORING_PROFILES.md`, `docs/CLI.md`) |
| `[ai]` | Optional AI-assisted explanations; `enabled`, `api_base`, `model`, `max_tokens`, `temperature` (see `docs/CLI.md` § AI integration) |

### Scan modes

| Mode | Behavior |
| --- | --- |
| `git_only` (default) | Only directories containing `.git` are treated as project roots |
| `project_roots` | Directories with `.git` or common manifests (`Cargo.toml`, `package.json`, etc.) are included |

When a `.git` root is found, subdirectories are **not** scanned for nested project roots (prevents monorepo sub-packages from appearing as separate entries).

Place a `.nexusignore` file in any scan root with glob patterns (one per line) to exclude matching directories.

### CLI `--profile` override

The `--profile` flag on `score`, `plan`, `report`, and `explain` overrides `planner.scoring_profile` from the config file. Accepted values: `default`, `publish`, `open_source`, `security`, `ai_handoff`.

## Environment

- `NEXUS_CONFIG` — path to a `nexus.toml` file (see `nexus-config` crate: `ENV_NEXUS_CONFIG`).
- `NEXUS_AI_API_KEY` — API key for AI features (takes precedence over `OPENAI_API_KEY`).
- `OPENAI_API_KEY` — fallback API key for AI features.
- `NEXUS_ADAPTER_TIMEOUT_SECS` — timeout for external adapter subprocesses (default: 180 seconds).
- `RUST_LOG` — standard `tracing` filter when you need verbose logs from components that emit them.

## SQLite

Nexus uses SQLite with WAL mode (`journal_mode=WAL`), `synchronous=NORMAL`, and a 5-second `busy_timeout` for safe concurrent access. Schema versioning is tracked in the `nexus_meta` table. The `doctor` command reports the resolved DB path and SQLite version.
