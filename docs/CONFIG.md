# Configuration (`nexus.toml`)

Nexus reads a TOML config file. Precedence (first match wins):

1. `--config /path/to/nexus.toml` on the CLI
2. Environment variable `NEXUS_CONFIG`
3. `./nexus.toml` in the current working directory
4. XDG-style config directory: `nexus.toml` under the OS config dir (`ProjectDirs`, qualifier `org.nexus.nexus`)
5. Built-in defaults (no file)

Relative `db_path` values are resolved against the **process current working directory**, not the config file’s directory.

## Example

See `nexus.toml.example` in the repository root. Typical fields:

| Field | Purpose |
| --- | --- |
| `db_path` | SQLite database path (local state; keep under `.nexus/` or another ignored directory) |
| `default_roots` | Used when `nexus scan` is run with no path arguments |
| `github_owner` | Optional default for `gh`-based remote ingest |
| `include_hidden` | Whether to descend into hidden directories when scanning |
| `[scan]` | Read limits and gitignore behavior |
| `[planner]` | Ambiguity and publish-action thresholds; optional `canonical_pins`, `ignored_cluster_keys`, `archive_hint_cluster_keys`, `scoring_profile` (see `docs/SCORING_PROFILES.md`, `docs/CLI.md`) |

## Environment

- `NEXUS_CONFIG` — path to a `nexus.toml` file (see `nexus-config` crate: `ENV_NEXUS_CONFIG`).
- `RUST_LOG` — standard `tracing` filter when you need verbose logs from components that emit them.
