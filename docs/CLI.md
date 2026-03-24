# CLI

## Stable core (v1 direction)

These commands are the **intended stable surface** for repo fleet triage (names and primary flags should remain compatible in v1.x):

| Command | Purpose |
| --- | --- |
| `scan` | Inventory local repos (and optional GitHub ingest) into SQLite |
| `score` | Compute **scores and evidence** per cluster (stdout only; does not persist a plan) |
| `plan` | Build clusters, scores, evidence, and **prioritized actions**; write `plan.json`; persist plan to SQLite |
| `report` | Render markdown or JSON from the current inventory (plan recomputed in-process) |
| `doctor` | Environment, toolchain, and DB checks (`--format json` for scripts) |
| `tools` | Optional external adapters on `PATH` (`--format json` for scripts) |
| `export` | JSON envelope with `inventory` (optional `--with-plan`) for backup or transfer |
| `import` | Replace DB inventory from export JSON (clears persisted plan); requires `--force` |
| `explain` | One cluster’s scores, evidence, and actions (by cluster query or clone/remote id) |

**Helpers / previews**

| Command | Purpose |
| --- | --- |
| `apply --dry-run` | Read-only preview: counts clusters and proposed actions (`--format json` supported). **Mutating apply is not implemented**; omitting `--dry-run` exits with an error. Stays as the v1 preview mechanism (not folded into `plan`/`report`). |
| `serve` | **Experimental** read-only JSON over local SQLite for scripting. **Not** a dashboard, not multi-user, **unstable API** until release notes say otherwise. May move behind a feature flag later; default product remains the CLI. |

New subcommands (e.g. `tui`) may be added alongside the core without removing these in v1.x.

See `docs/PRODUCT_STRATEGY.md` for roadmap and non-goals.

## Configuration

Precedence (first match wins):

1. `--config /path/to/nexus.toml`
2. `NEXUS_CONFIG` environment variable
3. `./nexus.toml` in the current working directory
4. XDG config: `nexus.toml` under the OS config dir (`ProjectDirs`, qualifier `org.nexus.nexus`)
5. Built-in defaults (no file)

Relative `db_path` values are resolved against the **current working directory**. See `nexus.toml.example`.

The **`[planner]`** table also drives planning: ambiguity cutoff (`ambiguous_cluster_threshold`), when to suggest archiving duplicates vs canonical strength (`archive_duplicate_threshold`), publish-hygiene actions vs `oss_readiness` (`oss_candidate_threshold`), optional **`canonical_pins`** (clone ids), **`ignored_cluster_keys`** / **`archive_hint_cluster_keys`** (exact `cluster_key` from JSON output), and optional **`scoring_profile`** (`docs/SCORING_PROFILES.md`). `score`, `plan`, `report`, `export --with-plan`, `explain`, and `apply --dry-run` all use the loaded config; experimental `serve` resolves config from its process working directory.

## Commands

### `nexus scan`

Discover local repositories and persist scan output.

Example:

```bash
nexus scan ~/Projects ~/code --github-owner your-github-login
```

### `nexus score`

Compute cluster **scores** and **evidence** from the latest inventory. Does **not** write a plan file and does **not** call `persist_plan` (use `nexus plan` to refresh the persisted plan and `plan.json`).

- `--format text` (default) — human-readable lines per cluster (canonical, repo health, recoverability, publish readiness, risk).
- `--format json` — JSON with `kind: "nexus_scores"`, `schema_version`, and a `clusters` array of `ClusterRecord` objects (same `scores` shape as `plan.json`, without per-cluster actions).
- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — when **gitleaks**, **semgrep**, **jscpd**, or **syft** are on `PATH`, run them on canonical clones and attach evidence (can be slow).

Example:

```bash
nexus score
nexus score --format json --no-merge-base
```

### `nexus plan`

Resolve clusters, score them, optionally attach external evidence, write a deterministic plan file, and **persist** the plan to SQLite (for `serve` and future consumers). Plan JSON includes `scoring_rules_version` (rule-set revision; see `docs/SCORING.md`).

- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — optional scanners on canonical clones (see above).

Example:

```bash
nexus plan --write nexus-plan.json
nexus plan --write plan.json --external
```

### `nexus report`

Render markdown or JSON reports from the current inventory (plan is rebuilt in memory; does not require a prior `plan --write`).

**Stable markdown sections (in order):** top-level title `Nexus Report`, run metadata bullets, optional `## Warnings` (ambiguous / low-confidence clusters), then per cluster: `## {label}`, cluster metadata bullets, `### Scores`, `### Score explanations`, `### Evidence`, `### Actions`. Tools that parse reports should key off these headings.

Example:

```bash
nexus report --format md
nexus report --format json
```

### `nexus doctor`

Validate environment and dependencies.

- `--format text` (default) — human-readable lines and tips.
- `--format json` — machine-readable document with `kind: "nexus_doctor"`, config paths, DB open/sqlite status, `path_tools` (`git`, `gh`, `cc`), optional scanner map, and `rustc_version` when available.

Example:

```bash
nexus doctor
nexus doctor --format json
```

### `nexus apply --dry-run`

Lists how many clusters/actions would be considered. v1 does not mutate repos; omitting `--dry-run` exits with an error.

- `--format text` (default) — one-line summary.
- `--format json` — `kind: "nexus_apply_dry_run"` with `cluster_count`, `action_count`, and `scoring_rules_version` (only with `--dry-run`).

Example:

```bash
nexus apply --dry-run
nexus apply --dry-run --format json
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

- `--format text` (default) — two-column list.
- `--format json` — `kind: "nexus_tools"` and a `tools` object (binary name → bool).

```bash
nexus tools
nexus tools --format json
```

### `nexus export`

Writes JSON to stdout or `-o`/`--output`:

- `schema_version`, `kind: "nexus_inventory_export_v1"`, `exported_at`, `generated_by`
- `inventory` — same shape as the in-memory snapshot (`clones`, `remotes`, `links`, and `run` when a row exists in SQLite — latest scan by `started_at`)
- optional `plan` when `--with-plan` — fresh plan (same flags as `plan` for merge-base and external scanners; not written to disk or persisted)

```bash
nexus export -o backup.json
nexus export --with-plan --external -o snapshot.json
```

### `nexus import`

Replaces **all** runs, clones, remotes, links, and **clears** persisted plan tables (`clusters`, `evidence`, `actions`, …). Expects either the export envelope (`inventory` key) or a raw `InventorySnapshot` JSON object. Requires `--force`.

```bash
nexus import backup.json --force
```

### `nexus explain`

Subcommands: `cluster <ID_OR_LABEL>`, `clone <CLONE_ID>`, `remote <REMOTE_ID>`. Resolves a cluster (exact id, case-insensitive label, or unique substring for `cluster`), then prints text or `--format json`. Uses the same `--no-merge-base` and `--external` switches as `score`/`plan`.

```bash
nexus explain cluster my-repo
nexus explain clone clone-abc --format json
```

## Planned next-layer commands

(Not necessarily in the first tagged v1 release.)

- `nexus tui` — minimal terminal UI for browsing clusters, sorting by score, overrides (see `docs/PRODUCT_STRATEGY.md`)
- `nexus suggest` — AI-assisted suggestions grounded in Nexus output (optional)
- Optional **AI**-enhanced natural language on top of deterministic `explain` (see `docs/PRODUCT_STRATEGY.md`)
