# CLI

## Agent / coding-agent commands

Deterministic **workspace truth** and **automation verdicts** for scripts and coding agents. All share `--format text|json`, `--no-merge-base`, `--external`, and `--profile` (same meaning as `plan` / `score`). JSON responses include **provenance**: `generated_at`, `inventory_run_id`, `scope`, `freshness`, `data_sources` where applicable.

| Command | Purpose |
| --- | --- |
| `preflight <TARGET>` | Compact manifest: `canonical_path`, `repo_root`, alternates as `blocked_paths` / `ignored_alternates`, `warnings`, flattened `AutomationVerdict` (`safe_to_*`, `unsafe_for_automation`, `blocking_reasons`), `recommended_next_action` |
| `resolve <QUERY>` | Resolve label, filesystem path, or remote URL → `canonical_path`, `cluster_id`, `alternates`, `confidence`, `automation_verdict`, `why_canonical`, `unsafe_for_automation` |
| `verdict <TARGET>` | Full `AutomationVerdict` for the resolved cluster (conservative block if the target does not resolve) |
| `check-path <PATH>` | Wrong-clone check: disposition vs canonical, guidance for non-canonical paths |
| `summary --agent [DIR]…` | Token-light rollup: duplicate groups, unsafe targets, canonical paths, dirty canonical checkouts, nested warnings, counts (`--agent` required) |

The read-only HTTP API mirrors these under **`GET /v2/agent/*`** (see `gittriage serve` below).

## Stable core

These commands are the **stable surface** for local git workspace triage (names and primary flags remain compatible in v1.x):

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
| `explain` | One cluster’s scores, evidence, and actions (by cluster query or clone/remote id); optional `--ai` narrative (non-deterministic) |
| `ai-doctor` | Print AI config status; optional `--probe-network` to GET `{api_base}/models` |

**Secondary (shipped, real)**

| Command | Purpose |
| --- | --- |
| `tui` | Interactive terminal table over the current plan; **Auto** column (automation verdict), canonical path in details; `v` cycles views (all / safe / unsafe / duplicates / local-only) |

**Experimental**

| Command | Purpose |
| --- | --- |
| `ai-summary` | Optional AI-generated narrative over the full plan (not deterministic; requires AI config + key) |
| `apply --dry-run` / `preview` | Read-only preview: counts clusters and proposed actions (`--format json` supported). Mutating apply is not implemented. |
| `serve` | Read-only JSON over local SQLite. **`/v1/*`** inventory/plan; **`/v2/agent/*`** agent operations (versioned contract; see below). |

New subcommands may be added alongside the core without removing these in v1.x.

See `docs/PRODUCT_STRATEGY.md` for roadmap and non-goals.

## Configuration

Precedence (first match wins):

1. `--config /path/to/gittriage.toml`
2. `GITTRIAGE_CONFIG` environment variable
3. `./gittriage.toml` in the current working directory
4. XDG config: `gittriage.toml` under the OS config dir (`ProjectDirs`, qualifier `org.gittriage.gittriage`)
5. Built-in defaults (no file)

Relative `db_path` values are resolved against the **config file's parent directory** when a config file is found, or the current working directory when using built-in defaults. Tilde (`~`) is expanded. See `gittriage.toml.example`.

The **`[scan]`** table controls scanning behavior: `scan_mode` (`git_only` default, `project_roots`), `max_depth` (optional traversal limit), `respect_gitignore`, `max_readme_bytes`, `max_hash_files`, and `include_nested_git` (discover nested `.git` dirs under an already-found root; off by default). Place a `.gittriageignore` file in any scan root with glob patterns to exclude directories.

Top-level **`github_owner_mode`** (`augment` default, `full_catalog`) controls how `github_owner` ingest combines with local remotes: `augment` keeps only GitHub repos whose URL matches a local-git remote from the scan; `full_catalog` ingests the full `gh repo list` for the owner. Override per run with `gittriage scan --github-owner-mode …`.

Optional **`tui_export_path`** sets the file written when pressing `o` in the TUI; if unset, a timestamped `gittriage-plan-tui-export-*.json` in the current directory is used.

### Git workspaces vs manifest-only discovery

- **`scan_mode = "git_only"` (default)** — Inventory is built from real `.git` directories (and worktrees). This is the path **`preflight` / `resolve` / `check-path` / `verdict`** assume: canonical paths point at actual git checkouts, and duplicate/nested semantics refer to inventoried clones.
- **`scan_mode = "project_roots"`** — Walk uses manifest / project-root heuristics (lockfiles, etc.). Treat this as **secondary**: clusters may not map cleanly to “the one true git repo” for an agent. Prefer `git_only` for coding-agent safety; use `project_roots` when you explicitly want broader directory cues, and still validate paths with `check-path` before automation.

See `gittriage.toml.example` (`[scan] scan_mode`) and `docs/CONFIG.md` for field details.

The **`[planner]`** table drives planning: ambiguity cutoff (`ambiguous_cluster_threshold`), when to suggest archiving duplicates vs canonical strength (`archive_duplicate_threshold`), publish-hygiene actions vs `oss_readiness` (`oss_candidate_threshold`), optional **`canonical_pins`** (clone ids), **`ignored_cluster_keys`** / **`archive_hint_cluster_keys`** (exact `cluster_key` from JSON output), and optional **`scoring_profile`** (`docs/SCORING_PROFILES.md`). The `--profile` flag on `score`, `plan`, `report`, and `explain` overrides the config value. `serve` loads config once at startup.

## Commands

### `gittriage scan`

Discover local repositories and persist scan output.

Example:

```bash
gittriage scan ~/Projects ~/code --github-owner your-github-login
gittriage scan ~/Projects --github-owner your-github-login --github-owner-mode full_catalog
```

Nested `.git` directories under another root are skipped by default; stderr lists them. Set `scan.include_nested_git = true` (or use a future release’s warnings-only mode) to include them.

### `gittriage score`

Compute cluster **scores** and **evidence** from the latest inventory. Does **not** write a plan file and does **not** call `persist_plan` (use `gittriage plan` to refresh the persisted plan and `plan.json`).

- `--format text` (default) — human-readable lines per cluster (canonical, repo health, recoverability, publish readiness, risk).
- `--format json` — JSON with `kind: "gittriage_scores"`, `schema_version`, and a `clusters` array of `ClusterRecord` objects (same `scores` shape as `plan.json`, without per-cluster actions).
- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — when **gitleaks**, **semgrep**, **jscpd**, or **syft** are on `PATH`, run them on canonical clones and attach evidence (can be slow).
- `--profile <NAME>` — override `planner.scoring_profile` from config. Accepts: `default`, `publish`, `open_source`, `security`, `ai_handoff`.

Example:

```bash
gittriage score
gittriage score --format json --no-merge-base
```

### `gittriage plan`

Resolve clusters, score them, optionally attach external evidence, write a deterministic plan file, and **persist** the plan to SQLite (for `serve` and future consumers). Plan JSON includes `scoring_rules_version` (rule-set revision; see `docs/SCORING.md`).

- `--no-merge-base` — skip pairwise `git merge-base` evidence between git clones in the same cluster.
- `--external` — optional scanners on canonical clones (see above). After attach, prints how many adapter tools are on `PATH`, how many adapter evidence rows were added, and a one-line `adapter_run …` summary (`tools_on_path`, spawn attempts, evidence attached, skipped paths, timeouts/non-zero exits). Plan JSON gains optional `external_adapter_run` with the same counters for export and persistence.
- `--profile <NAME>` — override `planner.scoring_profile` from config.

Example:

```bash
gittriage plan --write gittriage-plan.json
gittriage plan --write plan.json --external
```

### `gittriage report`

Render markdown or JSON reports from the current inventory. The plan is **always** recomputed in memory from the current inventory (same engine as `plan` / `score`); it does not read the last `plan --write` file. The markdown header still compares timing against any **SQLite-persisted** plan rows (which `scan` clears).

- `--scope <SCOPE>` — optional filter: `local-only` (clone members only, no remote row in cluster), `mixed` (clone + remote members), `remote-only` (GitHub/catalog-only rows). The markdown header still lists **full** scope counts across the whole plan; the body lists only matching clusters. JSON output is the filtered plan document.
- `--persist-plan` — after building the report plan, write it to SQLite (same as `gittriage plan` persistence, without writing a JSON file). Use this to refresh DB-backed consumers after a scan without running `plan` separately.
- Hidden `--recompute` — no-op; prints a one-line note to stderr that the report always recomputes (for explicit scripts).
- Markdown header also includes: **Local triage focus** (how many clusters involve a local checkout), latest inventory scan time (when present), SQLite persisted-plan row count / timestamp (and a note when `scan` cleared rows or the persisted plan predates the latest scan), and optional `## Skipped nested git repositories` from the last scan’s `runs.stats_json`.
- When there is no scope filter and the plan mixes local-involved clusters with remote-only clusters, markdown may split body sections into **Clusters with local checkouts** and **Remote-only clusters**.

**Stable markdown sections (in order):** top-level title `GitTriage Report`, run metadata bullets. For **unscoped** markdown reports, **agent-preflight sections** are inserted next: `## Unsafe for automation`, `## Duplicate groups`, `## Canonical repo paths`, `## Dirty canonical checkouts`, `## Nested / scan gaps`, then optional `## Skipped nested git repositories`, optional `## Warnings` (ambiguous / low-confidence clusters), then per cluster: `## {label}`, cluster metadata bullets (including **Member scope**), **`### Scores (summary)`** (one line + pointer to `explain` / `score` — not the long narrative), `### Evidence`, `### Actions`. **Scoped** reports (`--scope …`) omit agent-preflight blocks and keep the classic **`### Scores`** plus **`### Score explanations`** under each cluster. Tools that parse reports should key off these headings.

Example:

```bash
gittriage report --format md
gittriage report --format json
gittriage report --format md --scope remote-only
gittriage report --format md --persist-plan
```

### `gittriage doctor`

Validate environment and dependencies.

- `--format text` (default) — human-readable lines and tips.
- `--format json` — machine-readable document with `kind: "gittriage_doctor"`, config paths, DB open/sqlite status, `path_tools` (`git`, `gh`, `cc`), optional scanner map, and `rustc_version` when available.

Example:

```bash
gittriage doctor
gittriage doctor --format json
```

### `gittriage apply --dry-run` (alias: `gittriage preview`)

Lists how many clusters/actions would be considered. v1 does not mutate repos; omitting `--dry-run` exits with an error. There is no mutating apply path yet; future releases may add explicit opt-in automation.

- `--format text` (default) — one-line summary.
- `--format json` — `kind: "gittriage_apply_dry_run"` with `cluster_count`, `action_count`, and `scoring_rules_version` (only with `--dry-run`).

Example:

```bash
gittriage apply --dry-run
gittriage preview --dry-run
gittriage apply --dry-run --format json
```

### `gittriage serve` (experimental)

Read-only HTTP JSON API (requires a configured/openable SQLite DB). Intended for **local** inspection only; not a web product. Config is loaded once at startup (not per-request).

**Stability:** treat **`/v1/*`** as **experimental** (same rules as before: contract tests lock top-level keys for `/health`, `/v1/inventory`, `/v1/plan`). **`/v2/agent/*`** is the **versioned agent surface**: paths and the `kind` / `schema_version` fields are treated as stable within `0.1.x`; new JSON fields may be added. Breaking changes will be called out in release notes.

- `--port <PORT>` — listen port (default: 3030).
- `--listen <IP>` — bind address (default: `127.0.0.1`; use `0.0.0.0` for network access).
Routes (all `GET`, JSON bodies):

- `/health` — `{"ok": true, "service": "gittriage-api", "version": "<crate semver>"}` (service liveness).
- `/v1/inventory` — `{"clones": N, "remotes": N, "links": N}` (lightweight counts).
- `/v1/plan` — full plan document (recomputed from inventory using startup config; same shape as `plan --write`, including optional `external_adapter_run` when last built with adapters).

**`/v2/agent/*`** (query parameters):

- `/v2/agent/resolve?query=<label|path|url>` — same shape as CLI `resolve` (`kind: "gittriage_resolve"`).
- `/v2/agent/verdict?target=<QUERY>` — `kind: "gittriage_verdict"` plus embedded `verdict` object.
- `/v2/agent/preflight?target=<QUERY>` — same shape as CLI `preflight` (`kind: "gittriage_preflight"`; verdict fields flattened).
- `/v2/agent/check-path?path=<PATH>` — `kind: "gittriage_check_path"`.
- `/v2/agent/summary` — optional repeated `workspace=<DIR>` query params; `kind: "gittriage_agent_summary"`.
- `/v2/agent/duplicate-groups` — optional `workspace` params; `kind: "gittriage_duplicate_groups"`.
- `/v2/agent/unsafe-targets` — optional `workspace` params; `kind: "gittriage_unsafe_targets"`.

Example:

```bash
gittriage serve --port 3030
gittriage serve --port 8080 --listen 0.0.0.0
```

### `gittriage tools`

Print whether optional external scanners are on `PATH`.

- `--format text` (default) — two-column list.
- `--format json` — `kind: "gittriage_tools"` and a `tools` object (binary name → bool).

```bash
gittriage tools
gittriage tools --format json
```

### `gittriage export`

Writes JSON to stdout or `-o`/`--output`:

- `schema_version`, `kind: "gittriage_inventory_export_v1"`, `exported_at`, `generated_by`
- `inventory` — same shape as the in-memory snapshot (`clones`, `remotes`, `links`, optional `semantics` documenting git vs manifest-only root counts, and `run` when a row exists in SQLite — latest scan by `started_at`)
- optional `plan` when `--with-plan` — fresh plan (same flags as `plan` for merge-base and external scanners; not written to disk or persisted)

```bash
gittriage export -o backup.json
gittriage export --with-plan --external -o snapshot.json
```

### `gittriage import`

Replaces **all** runs, clones, remotes, links, and **clears** persisted plan tables (`clusters`, `evidence`, `actions`, …). Expects either the export envelope (`inventory` key) or a raw `InventorySnapshot` JSON object. Requires `--force`.

```bash
gittriage import backup.json --force
```

### `gittriage explain`

Subcommands: `cluster <ID_OR_LABEL>`, `clone <CLONE_ID>`, `remote <REMOTE_ID>`. Resolves a cluster (exact id, case-insensitive label, or unique substring for `cluster`), then prints text or `--format json`. Uses the same `--no-merge-base` and `--external` switches as `score`/`plan`.

- `--ai` — Append an AI-generated narrative after the deterministic block. Flag is **global** on `explain` (valid as `gittriage explain --ai cluster foo` or `gittriage explain cluster foo --ai`). If AI is disabled or no API key is set, a short **note** is printed to stderr and the command still **exits 0** after the deterministic output.
- `--profile <NAME>` — override `planner.scoring_profile` from config.

```bash
gittriage explain cluster my-repo
gittriage explain clone clone-abc --format json
gittriage explain --ai cluster my-repo
gittriage explain cluster my-repo --ai
```

### `gittriage ai-summary`

Generate an AI-powered executive summary of the full plan. When `ai.enabled` is false or no API key is set, prints a one-line note to stderr and **exits 0** (no summary). Otherwise requires `ai.enabled = true` and `GITTRIAGE_AI_API_KEY` or `OPENAI_API_KEY`. Output is model-generated and never modifies deterministic scores or actions.

```bash
gittriage ai-summary
gittriage ai-summary --no-merge-base --external
```

### `gittriage ai-doctor`

Prints whether AI is enabled, whether an API key is present, and the configured `api_base` / `model`. By default it does not call the network. With `--probe-network`, performs a short `GET` to `{api_base}/models` (OpenAI-compatible listing), sending `Authorization: Bearer …` when a key is set, to verify reachability and HTTP status.

```bash
gittriage ai-doctor
gittriage ai-doctor --probe-network
```

### `gittriage tui`

Rebuilds the plan in-process (same `gittriage.toml` `[planner]` fields and `--no-merge-base` / `--external` as `score`/`plan`). **Read-only:** no charts, no background services, no mutation of repos.

- `--scope <SCOPE>` — same member-scope buckets as `report --scope` (`local-only`, `mixed`, `remote-only`): only matching clusters appear in the table (help overlay shows when a scope filter is active).
- If the latest scan recorded skipped nested git paths, the initial status line reminds you to set `scan.include_nested_git` when you want them inventoried.

| Key | Action |
| --- | --- |
| `j` / `↓`, `k` / `↑` | Move selection |
| `g` / `G` | Jump to top / bottom |
| `PgUp` / `PgDn` | Page up / down |
| `s` | Cycle sort: label, canonical↓, health↓, risk↓, ambiguous-first |
| `/` | Edit filter substring (label + `cluster_key`); Enter apply, Esc cancel |
| `f` | Clear filter |
| `Tab` | Toggle bottom panel: Detail ↔ Actions |
| `a` | Switch to Actions panel |
| `e` | Full evidence overlay for selected cluster (Esc back) |
| `p` | Show `canonical_pins` TOML snippet for the canonical clone |
| `o` | Write full plan JSON to `tui_export_path` in config, or a timestamped `gittriage-plan-tui-export-*.json` in the current directory |
| `?` | Help overlay (Esc or `q` closes) |
| `q` / `Esc` / `Ctrl-c` | Quit |

Requires a TTY; exits with an error if stdout is not interactive.

```bash
gittriage tui
gittriage tui --no-merge-base --external
gittriage tui --scope local-only
```

## AI integration

GitTriage can optionally use an OpenAI-compatible LLM to generate narrative explanations grounded in deterministic plan data. AI never modifies scores, canonical selections, or actions.

**Configuration** (`gittriage.toml`):

```toml
[ai]
enabled = true
api_base = "https://api.openai.com/v1"   # or any OpenAI-compatible HTTP API
model = "gpt-4o-mini"
max_tokens = 1024
temperature = 0.2
```

For a **local** OpenAI-compatible server (Ollama with an OpenAI shim, LiteLLM proxy, etc.), point `api_base` at that service’s `/v1` URL and set `model` to whatever that server expects. Use `gittriage ai-doctor` to confirm the resolved settings before calling `explain --ai` or `ai-summary`.

**Environment:** Set `GITTRIAGE_AI_API_KEY` or `OPENAI_API_KEY`.

**Commands:** `gittriage explain --ai` (per-cluster narrative), `gittriage ai-summary` (plan-wide summary), `gittriage ai-doctor` (config check).

All AI output is clearly labeled as model-generated. For `explain --ai` and `ai-summary`, when AI is disabled or no API key is set, a short note is printed to stderr and the command **exits 0** after any deterministic output (no network call).

## Planned next-layer commands

(Not necessarily in the first tagged v1 release.)

- `gittriage suggest` — AI-assisted suggestions grounded in GitTriage output (optional)
