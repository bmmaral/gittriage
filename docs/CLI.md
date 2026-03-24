# CLI

## Configuration

Precedence (first match wins):

1. `--config /path/to/nexus.toml`
2. `NEXUS_CONFIG` environment variable
3. `./nexus.toml` in the current working directory
4. XDG config: `nexus.toml` under the OS config dir (`ProjectDirs`, qualifier `org.nexus.nexus`)
5. Built-in defaults (no file)

Relative `db_path` values are resolved against the **current working directory**. See `nexus.toml.example`.

## Commands

### `nexus scan`
Discover local repositories and persist scan output.

Example:
```bash
nexus scan ~/Projects ~/code --github-owner your-github-login
```

### `nexus plan`
Resolve clusters, score them, and write a deterministic plan.

- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — when **gitleaks**, **semgrep**, **jscpd**, or **syft** are on `PATH`, run them on each cluster’s canonical clone and attach summary evidence (can be slow).

Example:
```bash
nexus plan --write nexus-plan.json
nexus plan --write plan.json --external
```

### `nexus report`
Render markdown or JSON reports from the persisted state.

Example:
```bash
nexus report --format md
```

### `nexus doctor`
Validate environment and dependencies.

Example:
```bash
nexus doctor
```

### `nexus apply --dry-run`
Lists how many clusters/actions would be considered. v1 does not mutate repos; omitting `--dry-run` exits with an error.

Example:
```bash
nexus apply --dry-run
```

### `nexus serve`
Read-only HTTP JSON API (requires a configured/openable SQLite DB).

- `GET /health`
- `GET /v1/plan` — current plan JSON (recomputed from inventory)
- `GET /v1/inventory` — clone / remote / link counts

Example:
```bash
nexus serve --port 3030
```

### `nexus tools`
Print whether optional external scanners are on `PATH`.

## Future commands
- `nexus explain cluster <id>`
- `nexus export`
