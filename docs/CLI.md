# CLI

## Stable core (v1 direction)

These commands are the **intended stable surface** for repo fleet triage (names and primary flags should remain compatible in v1.x):

| Command | Purpose |
| --- | --- |
| `scan` | Inventory local repos (and optional GitHub ingest) into SQLite |
| `score` | Compute **scores and evidence** per cluster (stdout only; does not persist a plan) |
| `plan` | Build clusters, scores, evidence, and **prioritized actions**; write `plan.json`; persist plan to SQLite |
| `report` | Render markdown or JSON from the current inventory (plan recomputed in-process) |
| `doctor` | Environment, toolchain, and DB checks |
| `tools` | Optional external adapters on `PATH` |

**Helpers / previews**

| Command | Purpose |
| --- | --- |
| `apply --dry-run` | Read-only preview: counts clusters and proposed actions. **Mutating apply is not implemented**; omitting `--dry-run` exits with an error. Stays as the v1 preview mechanism (not folded into `plan`/`report`). |
| `serve` | **Experimental** read-only JSON over local SQLite for scripting. **Not** a dashboard, not multi-user, **unstable API** until release notes say otherwise. May move behind a feature flag later; default product remains the CLI. |

New subcommands (e.g. `explain`, `export`, `import`, `tui`) may be added alongside the core without removing these in v1.x.

See `docs/PRODUCT_STRATEGY.md` for roadmap and non-goals.

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

### `nexus score`

Compute cluster **scores** and **evidence** from the latest inventory. Does **not** write a plan file and does **not** call `persist_plan` (use `nexus plan` to refresh the persisted plan and `plan.json`).

- `--format text` (default) — human-readable lines per cluster.
- `--format json` — JSON with `kind: "nexus_scores"`, `schema_version`, and a `clusters` array of `ClusterRecord` objects (same shape as inside `plan.json`, without per-cluster actions).
- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — when **gitleaks**, **semgrep**, **jscpd**, or **syft** are on `PATH`, run them on canonical clones and attach evidence (can be slow).

Example:

```bash
nexus score
nexus score --format json --no-merge-base
```

### `nexus plan`

Resolve clusters, score them, optionally attach external evidence, write a deterministic plan file, and **persist** the plan to SQLite (for `serve` and future consumers).

- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — optional scanners on canonical clones (see above).

Example:

```bash
nexus plan --write nexus-plan.json
nexus plan --write plan.json --external
```

### `nexus report`

Render markdown or JSON reports from the current inventory (plan is rebuilt in memory; does not require a prior `plan --write`).

**Stable markdown sections (in order):** top-level title `Nexus Report`, run metadata bullets, then per cluster: `## {label}`, cluster metadata bullets, `### Scores`, `### Evidence`, `### Actions`. Tools that parse reports should key off these headings.

Example:

```bash
nexus report --format md
nexus report --format json
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

### `nexus serve` (experimental)

Read-only HTTP JSON API (requires a configured/openable SQLite DB). Intended for **local** inspection only; not a web product. Treat URLs and JSON shapes as **unstable** until promoted in release notes.

- `GET /health`
- `GET /v1/plan` — current plan JSON (recomputed from inventory)
- `GET /v1/inventory` — clone / remote / link counts

Example:

```bash
nexus serve --port 3030
```

### `nexus tools`

Print whether optional external scanners are on `PATH`.

## Planned next-layer commands

(Not necessarily in the first tagged v1 release.)

- `nexus tui` — minimal terminal UI for browsing clusters, sorting by score, overrides (see `docs/PRODUCT_STRATEGY.md`)
- `nexus explain <cluster|repo|clone>` — deterministic explanation; optional AI layer later
- `nexus suggest` — AI-assisted suggestions grounded in Nexus output (optional)
- `nexus export` / `nexus import` — saved inventory / cross-machine comparison
